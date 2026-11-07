//! Dossier élève et personnes (socle 2, lot S2-1 ; D-020, socle 1 B7).
//!
//! * **Personne** (`ent_personne`) : responsable, contact ou élève majeur, avec ou
//!   sans compte de connexion, propre à l'établissement (D-013 / D-014).
//! * **Lien de responsabilité** (`ent_eleve_lien`) : personne ↔ élève, autorité
//!   parentale, garde, dates, suspension, droits fins existants.
//! * **Point unique des contrôles famille** : la vue `ent_eleve_lien_actif`
//!   (migration 121), lue par `PgFoyerRepo::lien_parent_eleve` /
//!   `liens_for_parent` (donc par `PgEnt::lien_parent_eleve`, utilisé par tous les
//!   contrôles famille du service), par les envois aux familles et par le
//!   consommateur de notifications. Un lien suspendu ou hors de ses dates
//!   n'ouvre plus rien, partout, dès la requête suivante.
//! * Contacts d'urgence, payeurs, fusion réversible de doublons, fiche 360° et
//!   chronologie de l'élève.
//!
//! Toute écriture : une transaction = fait + journal (`ent_journal_modif`) +
//! événement dans l'outbox (`Publier`, lot S2-0).

use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::{assert_parts_paiement, today_utc, DomainError, DossierSections};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::{json, Value};
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("dossier_personnes", err))
}

fn invalide(msg: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(msg)
}

/// Délai pendant lequel une fusion de personnes peut être annulée.
pub const DELAI_DEFUSION_JOURS: i64 = 30;

/// Valeurs admises pour la garde (`NULL` = non renseignée).
pub const GARDES: [&str; 3] = ["exclusive", "alternee", "aucune"];

/// Lien réservé à l'élève majeur représenté comme personne (payeur, contact).
pub const LIEN_ELEVE_MAJEUR: &str = "eleve_majeur";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Personne (responsable, contact, élève majeur), distincte du compte.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Personne {
    pub id: Uuid,
    pub nom: String,
    pub prenom: Option<String>,
    pub civilite: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub telephone_2: Option<String>,
    pub adresse: Option<String>,
    pub user_id: Option<Uuid>,
    pub eleve_id: Option<Uuid>,
    pub fusionnee_dans: Option<Uuid>,
    pub source: String,
}

impl Personne {
    /// Libellé « Prénom NOM » (ou nom seul).
    pub fn libelle(&self) -> String {
        match self.prenom.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
            Some(p) => format!("{p} {}", self.nom),
            None => self.nom.clone(),
        }
    }

    /// Retire les coordonnées (lecture par un autre responsable : minimisation).
    pub fn sans_coordonnees(mut self) -> Self {
        self.email = None;
        self.telephone = None;
        self.telephone_2 = None;
        self.adresse = None;
        self
    }
}

/// Saisie d'une personne (création ou modification de l'identité / coordonnées).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonneSaisie {
    pub nom: String,
    pub prenom: Option<String>,
    pub civilite: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub telephone_2: Option<String>,
    pub adresse: Option<String>,
}

fn opt(v: &Option<String>) -> Option<String> {
    v.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

impl PersonneSaisie {
    /// Normalise et valide (règles simples ; la base garde les bornes).
    pub fn normaliser(&self) -> Result<PersonneSaisie, EntDbError> {
        let nom = self.nom.trim().to_string();
        if nom.is_empty() || nom.chars().count() > 120 {
            return Err(invalide("nom de la personne obligatoire (120 caractères au plus)"));
        }
        let civilite = opt(&self.civilite);
        if let Some(c) = &civilite {
            if !["mme", "m", "autre"].contains(&c.as_str()) {
                return Err(invalide("civilité inconnue"));
            }
        }
        let email = opt(&self.email).map(|e| e.to_lowercase());
        if let Some(e) = &email {
            let ok = e.len() <= 254
                && e.split_once('@')
                    .map(|(a, d)| !a.is_empty() && d.contains('.') && !d.starts_with('.'))
                    .unwrap_or(false)
                && !e.chars().any(char::is_whitespace);
            if !ok {
                return Err(invalide("adresse e-mail mal formée"));
            }
        }
        for t in [&self.telephone, &self.telephone_2] {
            if let Some(t) = opt(t) {
                if t.len() > 40
                    || !t
                        .chars()
                        .all(|c| c.is_ascii_digit() || " +.-()".contains(c))
                {
                    return Err(invalide("numéro de téléphone mal formé"));
                }
            }
        }
        let prenom = opt(&self.prenom);
        if prenom.as_ref().is_some_and(|p| p.chars().count() > 120) {
            return Err(invalide("prénom trop long"));
        }
        let adresse = opt(&self.adresse);
        if adresse.as_ref().is_some_and(|a| a.chars().count() > 500) {
            return Err(invalide("adresse trop longue"));
        }
        Ok(PersonneSaisie {
            nom,
            prenom,
            civilite,
            email,
            telephone: opt(&self.telephone),
            telephone_2: opt(&self.telephone_2),
            adresse,
        })
    }
}

/// Lien de responsabilité tel qu'affiché sur la fiche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Responsable {
    pub lien_id: Uuid,
    pub eleve_id: Uuid,
    pub personne: Personne,
    pub foyer_id: Option<Uuid>,
    pub foyer_libelle: Option<String>,
    pub lien: String,
    pub autorite_parentale: bool,
    pub garde: Option<String>,
    pub date_debut: Option<NaiveDate>,
    pub date_fin: Option<NaiveDate>,
    pub suspendu_le: Option<DateTime<Utc>>,
    pub suspension_motif: Option<String>,
    pub suspension_reference: Option<String>,
    /// Actif aujourd'hui (lu dans `ent_eleve_lien_actif`).
    pub actif: bool,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
    pub est_payeur: bool,
    pub part_paiement: Option<i16>,
}

/// Saisie d'un lien de responsabilité (création : `personne_id` requis).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LienSaisie {
    pub personne_id: Option<Uuid>,
    pub foyer_id: Option<Uuid>,
    pub lien: String,
    pub autorite_parentale: bool,
    pub garde: Option<String>,
    pub date_debut: Option<NaiveDate>,
    pub date_fin: Option<NaiveDate>,
    pub peut_voir_notes: bool,
    pub peut_voir_vs: bool,
    pub peut_justifier_absence: bool,
    pub peut_autoriser_sorties: bool,
}

impl LienSaisie {
    fn valider(&self) -> Result<(String, Option<String>), EntDbError> {
        let lien = self.lien.trim();
        if lien.is_empty() || lien.chars().count() > 40 {
            return Err(invalide("nature du lien obligatoire (40 caractères au plus)"));
        }
        let garde = opt(&self.garde);
        if let Some(g) = &garde {
            if !GARDES.contains(&g.as_str()) {
                return Err(invalide("garde inconnue"));
            }
        }
        if let (Some(d), Some(f)) = (self.date_debut, self.date_fin) {
            if f < d {
                return Err(invalide("date de fin du lien antérieure à son début"));
            }
        }
        Ok((lien.to_string(), garde))
    }
}

/// Contact d'urgence / personne autorisée à récupérer l'élève.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactUrgence {
    pub id: Uuid,
    pub personne: Personne,
    pub relation: String,
    pub ordre: i16,
    pub peut_recuperer: bool,
    pub statut: String,
    pub commentaire: Option<String>,
}

/// Saisie d'un contact : personne existante, ou nouvelle personne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactSaisie {
    pub personne_id: Option<Uuid>,
    pub nouvelle_personne: Option<PersonneSaisie>,
    pub relation: String,
    pub ordre: i16,
    pub peut_recuperer: bool,
    pub commentaire: Option<String>,
}

/// Action métier proposée depuis la fiche (catalogue `ent_auth::Action`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionFiche {
    pub code: &'static str,
    pub libelle: &'static str,
}

/// Résumé du contenu des sections existantes visibles par l'acteur.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResumeSections {
    pub notes: usize,
    pub sanctions: usize,
    pub incidents: usize,
    pub carnet: usize,
    pub documents: usize,
    pub options: usize,
    pub dispositifs: usize,
    pub bourses: usize,
}

/// Fiche élève 360° (en-tête + personnes + accès aux sections + actions).
#[derive(Debug, Clone)]
pub struct FicheEleve {
    pub eleve: domain_eleve::Eleve,
    pub age_ans: Option<u32>,
    pub majeur: bool,
    pub sections: DossierSections,
    pub resume: ResumeSections,
    pub responsables: Vec<Responsable>,
    pub contacts: Vec<ContactUrgence>,
    pub actions: Vec<ActionFiche>,
    pub alertes: Vec<String>,
}

/// Entrée de la chronologie de l'élève (titre neutre, jamais de texte libre
/// d'un événement ; santé/psy réservés à leur métier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntreeChronologie {
    pub quand: DateTime<Utc>,
    pub source: &'static str,
    pub domaine: &'static str,
    pub titre: String,
}

/// Personne trouvée par la recherche (sélection, détection de doublons).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonneTrouvee {
    pub personne: Personne,
    pub nb_liens: i64,
}

/// Fusion de personnes (réversible).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct FusionPersonne {
    pub id: Uuid,
    pub source_id: Uuid,
    pub cible_id: Uuid,
    pub motif: String,
    pub fusionne_le: DateTime<Utc>,
    pub annule_le: Option<DateTime<Utc>>,
}

/// Ce que l'acteur peut lire des personnes d'un élève.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LectureFamille {
    /// Personnel habilité (section « famille ») : tout, coordonnées comprises.
    Complete,
    /// Personnel habilité à la section « famille » sans administrer les
    /// personnes (professeur, CPE, comptabilité…) : liens actifs seulement,
    /// coordonnées conservées, sans décision de suspension (motif, référence,
    /// date) ; `payeurs` n'est vrai que pour la comptabilité.
    Pedagogique { payeurs: bool },
    /// Responsable lié et actif : ses coordonnées, les noms des autres.
    Responsable(Uuid),
    Aucune,
}

// ---------------------------------------------------------------------------
// Chronologie : classement pur des événements (testé sans base)
// ---------------------------------------------------------------------------

/// Domaine d'une entrée de chronologie, décide de sa visibilité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomaineChrono {
    Dossier,
    Scolarite,
    VieScolaire,
    Notes,
    Documents,
    Accompagnement,
    Repas,
    Finances,
    Sante,
    Psy,
}

impl DomaineChrono {
    pub fn code(self) -> &'static str {
        match self {
            Self::Dossier => "dossier",
            Self::Scolarite => "scolarite",
            Self::VieScolaire => "vie_scolaire",
            Self::Notes => "notes",
            Self::Documents => "documents",
            Self::Accompagnement => "accompagnement",
            Self::Repas => "repas",
            Self::Finances => "finances",
            Self::Sante => "sante",
            Self::Psy => "psy",
        }
    }
}

/// Événement → (domaine, titre neutre). Liste blanche : un type absent n'apparaît pas.
pub fn classer_evenement(type_name: &str) -> Option<(DomaineChrono, &'static str)> {
    use DomaineChrono::*;
    let r = match type_name {
        types::ELEVE_CREATED => (Dossier, "Dossier créé"),
        types::ELEVE_STATUS_CHANGED => (Dossier, "Statut du dossier modifié"),
        types::ELEVE_IDENTITY_CHANGED => (Dossier, "Identité modifiée"),
        types::PHOTO_ELEVE => (Dossier, "Photo mise à jour"),
        types::ELEVE_LIEN_UPDATED => (Dossier, "Responsable lié ou modifié"),
        types::LIEN_SUSPENDU => (Dossier, "Lien d'un responsable suspendu"),
        types::LIEN_RETABLI => (Dossier, "Lien d'un responsable rétabli"),
        types::ELEVE_CONTACTS_URGENCE_CHANGES => (Dossier, "Contacts d'urgence modifiés"),
        types::ELEVE_COMPTE_LIE => (Dossier, "Compte élève rattaché"),
        types::ELEVE_COMPTE_INVITE => (Dossier, "Invitation au compte envoyée"),
        types::ELEVE_COMPTE_REVOQUE => (Dossier, "Compte élève révoqué"),
        types::SCOLARITE_CLASSE_CHANGED => (Scolarite, "Changement de classe"),
        types::SCOLARITE_MEF_CHANGED => (Scolarite, "Changement de formation"),
        types::SCOLARITE_SITE_CHANGED => (Scolarite, "Changement de site"),
        types::ELEVE_REGIME_CHANGED => (Scolarite, "Changement de régime"),
        types::ELEVE_SORTIE => (Scolarite, "Sortie de l'établissement"),
        types::ELEVE_OPTION_ADDED => (Scolarite, "Option ajoutée"),
        types::ELEVE_SCOLARITE_ANNEE_ENREGISTREE => (Scolarite, "Année de scolarité enregistrée"),
        types::ELEVE_MONTEE_RAPIDE | types::ELEVE_MONTEE_LOT => (Scolarite, "Passage dans l'année suivante"),
        types::ELEVE_REINSCRIPTION_CONFIRMEE => (Scolarite, "Réinscription confirmée"),
        types::INSCRIPTION_VALIDEE => (Scolarite, "Inscription validée"),
        types::SCOLARITE_DEPART_ANNONCE => (Scolarite, "Départ annoncé par la famille"),
        types::ELEVE_OPTION_N1_CHOISIE => (Scolarite, "Option choisie pour l'année suivante"),
        types::ELEVE_ABSENCE_DECLARED => (VieScolaire, "Absence enregistrée"),
        types::ELEVE_ABSENCE_JUSTIF_SOUMIS => (VieScolaire, "Justificatif d'absence déposé"),
        types::ELEVE_ABSENCE_JUSTIFIED => (VieScolaire, "Absence justifiée"),
        types::ELEVE_ABSENCE_REFUSED => (VieScolaire, "Justificatif d'absence refusé"),
        types::ELEVE_ABSENCE_ANNULEE_PRESENT => (VieScolaire, "Absence annulée (élève présent)"),
        types::ELEVE_ABSENCE_SEUIL_ATTEINT => (VieScolaire, "Seuil d'absences atteint"),
        types::RETARD_DECLARE => (VieScolaire, "Retard"),
        types::ELEVE_INCIDENT_DECLARED => (VieScolaire, "Incident"),
        types::ELEVE_SANCTION_PRONONCEE => (VieScolaire, "Sanction prononcée"),
        types::ELEVE_SANCTION_ANNULEE => (VieScolaire, "Sanction annulée"),
        types::ELEVE_CARNET_ENTREE => (VieScolaire, "Mot dans le carnet"),
        types::ELEVE_CARNET_SIGNEE => (VieScolaire, "Mot du carnet signé"),
        types::DISPENSE_VS => (VieScolaire, "Dispense"),
        types::DISPENSE_VS_CLOTURE => (VieScolaire, "Fin de dispense"),
        types::NOTE_SAISIE => (Notes, "Note saisie"),
        types::NOTE_BULLETIN_PUBLIE => (Notes, "Bulletin publié"),
        types::COMPETENCE_EVALUEE => (Notes, "Compétence évaluée"),
        types::ELEVE_DOCUMENT_UPLOADED => (Documents, "Document ajouté au dossier"),
        types::DOC_A_SIGNER => (Documents, "Document à signer"),
        types::DOC_SIGNE => (Documents, "Document signé"),
        types::ELEVE_DISPOSITIF_AJOUTE => (Accompagnement, "Dispositif d'accompagnement"),
        types::ELEVE_GRILLE_REPAS_POSED => (Repas, "Inscription à la restauration"),
        types::ELEVE_GRILLE_REPAS_CLOSE => (Repas, "Fin d'inscription à la restauration"),
        types::ELEVE_DISPENSE_REPAS_POSED => (Repas, "Dispense de repas"),
        types::ELEVE_BOURSE_AFFECTEE | types::ELEVE_BOURSE_DECIDEE => (Finances, "Bourse"),
        types::FOYER_PAYEUR_DEFINI => (Finances, "Payeurs définis"),
        types::INFIRMERIE_PASSAGE => (Sante, "Passage à l'infirmerie"),
        types::INFIRMERIE_PAI => (Sante, "Projet d'accueil individualisé"),
        types::PSY_ENTRETIEN | types::PSY_RDV => (Psy, "Suivi psychologique"),
        _ => return None,
    };
    Some(r)
}

/// Domaine d'une entrée du journal (`ent_journal_modif.domaine`).
pub fn classer_journal(domaine: &str) -> Option<DomaineChrono> {
    Some(match domaine {
        "dossier" | "administratif" => DomaineChrono::Dossier,
        "scolarite" => DomaineChrono::Scolarite,
        "vs" => DomaineChrono::VieScolaire,
        "notes" => DomaineChrono::Notes,
        "docs" => DomaineChrono::Documents,
        _ => return None,
    })
}

/// Visibilité d'un domaine pour un lecteur (sections déjà calculées).
/// Familles : scolarité, vie scolaire et notes seulement (selon leurs droits).
/// Santé / psy : jamais hors de leur métier, titre neutre seulement.
pub fn domaine_visible(d: DomaineChrono, s: &DossierSections, role: Role) -> bool {
    let famille = matches!(role, Role::Parent | Role::Eleve);
    match d {
        DomaineChrono::Scolarite => s.scolarite,
        DomaineChrono::VieScolaire => s.vs,
        DomaineChrono::Notes => s.notes,
        _ if famille => false,
        DomaineChrono::Dossier => s.famille,
        DomaineChrono::Documents => s.documents,
        DomaineChrono::Accompagnement => s.dispositifs,
        DomaineChrono::Repas => s.repas,
        DomaineChrono::Finances => s.bourses,
        DomaineChrono::Sante => role == Role::Infirmerie,
        DomaineChrono::Psy => role == Role::Psy,
    }
}

/// Alertes de cohérence de la fiche (D-014) : calcul pur.
pub fn alertes_fiche(responsables: &[Responsable]) -> Vec<String> {
    let mut a = Vec::new();
    let actifs: Vec<&Responsable> = responsables
        .iter()
        .filter(|r| r.actif && r.lien != LIEN_ELEVE_MAJEUR)
        .collect();
    let mut foyers: Vec<Uuid> = actifs.iter().filter_map(|r| r.foyer_id).collect();
    foyers.sort();
    foyers.dedup();
    if foyers.len() > 1 {
        for r in &actifs {
            if r.autorite_parentale && r.personne.email.is_none() {
                a.push(format!(
                    "Parents séparés : {} n'a pas d'adresse e-mail (une adresse par parent est requise).",
                    r.personne.libelle()
                ));
            }
        }
    }
    let payeurs: Vec<&Responsable> = responsables.iter().filter(|r| r.est_payeur).collect();
    if payeurs.iter().any(|r| r.foyer_id.is_none()) {
        a.push("Un payeur n'est rattaché à aucun foyer (facturation impossible).".into());
    }
    if payeurs.iter().any(|r| r.suspendu_le.is_some()) {
        a.push("Un payeur a un lien suspendu : vérifier la facturation.".into());
    }
    if !actifs.iter().any(|r| r.autorite_parentale) && !responsables.is_empty() {
        a.push("Aucun responsable actif n'exerce l'autorité parentale.".into());
    }
    a
}

// ---------------------------------------------------------------------------
// Lectures SQL communes
// ---------------------------------------------------------------------------

const COLS_PERSONNE: &str = "p.id, p.nom, p.prenom, p.civilite, p.email, p.telephone, \
     p.telephone_2, p.adresse, p.user_id, p.eleve_id, p.fusionnee_dans, p.source";

#[derive(sqlx::FromRow)]
struct ResponsableRow {
    lien_id: Uuid,
    lien_eleve_id: Uuid,
    foyer_id: Option<Uuid>,
    foyer_libelle: Option<String>,
    lien: String,
    autorite_parentale: bool,
    garde: Option<String>,
    date_debut: Option<NaiveDate>,
    date_fin: Option<NaiveDate>,
    suspendu_le: Option<DateTime<Utc>>,
    suspension_motif: Option<String>,
    suspension_reference: Option<String>,
    actif: bool,
    peut_voir_notes: bool,
    peut_voir_vs: bool,
    peut_justifier_absence: bool,
    peut_autoriser_sorties: bool,
    est_payeur: bool,
    part_paiement: Option<i16>,
    #[sqlx(flatten)]
    personne: Personne,
}

impl From<ResponsableRow> for Responsable {
    fn from(r: ResponsableRow) -> Self {
        Responsable {
            lien_id: r.lien_id,
            eleve_id: r.lien_eleve_id,
            personne: r.personne,
            foyer_id: r.foyer_id,
            foyer_libelle: r.foyer_libelle,
            lien: r.lien,
            autorite_parentale: r.autorite_parentale,
            garde: r.garde,
            date_debut: r.date_debut,
            date_fin: r.date_fin,
            suspendu_le: r.suspendu_le,
            suspension_motif: r.suspension_motif,
            suspension_reference: r.suspension_reference,
            actif: r.actif,
            peut_voir_notes: r.peut_voir_notes,
            peut_voir_vs: r.peut_voir_vs,
            peut_justifier_absence: r.peut_justifier_absence,
            peut_autoriser_sorties: r.peut_autoriser_sorties,
            est_payeur: r.est_payeur,
            part_paiement: r.part_paiement,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ContactRow {
    contact_id: Uuid,
    relation: String,
    ordre: i16,
    peut_recuperer: bool,
    statut: String,
    commentaire: Option<String>,
    #[sqlx(flatten)]
    personne: Personne,
}

async fn personne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<Option<Personne>, EntDbError> {
    let sql = format!(
        "SELECT {COLS_PERSONNE} FROM ent_personne p WHERE p.etablissement_id = $1 AND p.id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as::<_, Personne>(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)
}

/// Écrit une entrée de journal dans la transaction (domaine « dossier »).
#[allow(clippy::too_many_arguments)]
async fn journal_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Option<Uuid>,
    action: &str,
    libelle: &str,
    payload: Value,
    visible_famille: bool,
    actor: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,$3,'dossier',$4,$5,$6,$7,$8, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve_id)
    .bind(action)
    .bind(libelle)
    .bind(payload)
    .bind(visible_famille)
    .bind(actor)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn publier_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate: AggregateKind,
    aggregate_id: Uuid,
    eleve_id: Option<Uuid>,
    payload: Value,
    actor: Uuid,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate,
        aggregate_id,
        eleve_id,
        payload,
        actor_user_id: Some(actor.to_string()),
    })
    .await?;
    Ok(())
}

/// Parts de paiement de TOUS les liens d'un élève (avec ou sans compte).
async fn verifier_parts_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
) -> Result<(), EntDbError> {
    let parts: Vec<(bool, Option<i16>)> = sqlx::query_as(
        "SELECT est_payeur, part_paiement FROM ent_eleve_lien
          WHERE etablissement_id = $1 AND eleve_id = $2",
    )
    .bind(etab)
    .bind(eleve_id)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    assert_parts_paiement(&parts)?;
    Ok(())
}

async fn commit(tx: Transaction<'_, Postgres>) -> Result<(), EntDbError> {
    tx.commit().await.map_err(db)
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

impl PgEnt {
    async fn dossier_exiger_action(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        action: Action,
    ) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, actor, role, action).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    async fn dossier_exiger_eleve(&self, etab: Uuid, eleve_id: Uuid) -> Result<domain_eleve::Eleve, EntDbError> {
        self.get_eleve(etab, eleve_id)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))
    }

    async fn lecture_famille(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<LectureFamille, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Ok(LectureFamille::Aucune);
        };
        if parsed == Role::Parent {
            return Ok(match self.lien_parent_eleve(etab, actor, eleve_id).await? {
                Some(l) => LectureFamille::Responsable(l.id),
                None => LectureFamille::Aucune,
            });
        }
        if !parsed.is_staff() {
            return Ok(LectureFamille::Aucune);
        }
        let s = self
            .dossier_sections_for_actor(etab, actor, role, eleve_id)
            .await?;
        if !s.famille {
            return Ok(LectureFamille::Aucune);
        }
        for a in [Action::PersonnesGerer, Action::LiensSuspendre] {
            if self.a_action_etablissement(etab, actor, role, a).await? {
                return Ok(LectureFamille::Complete);
            }
        }
        let mut payeurs = false;
        for a in [Action::PayeursDefinir, Action::FacturationLire] {
            if self.a_action_etablissement(etab, actor, role, a).await? {
                payeurs = true;
                break;
            }
        }
        Ok(LectureFamille::Pedagogique { payeurs })
    }

    pub(crate) async fn responsables_bruts(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Responsable>, EntDbError> {
        let sql = format!(
            "SELECT l.id AS lien_id, l.eleve_id AS lien_eleve_id, l.foyer_id, f.libelle AS foyer_libelle, l.lien,
                    l.autorite_parentale, l.garde, l.date_debut, l.date_fin, l.suspendu_le,
                    l.suspension_motif, l.suspension_reference,
                    EXISTS (SELECT 1 FROM ent_eleve_lien_actif a WHERE a.id = l.id) AS actif,
                    l.peut_voir_notes, l.peut_voir_vs, l.peut_justifier_absence,
                    l.peut_autoriser_sorties, l.est_payeur, l.part_paiement, {COLS_PERSONNE}
               FROM ent_eleve_lien l
               JOIN ent_personne p ON p.id = l.personne_id AND p.etablissement_id = l.etablissement_id
               LEFT JOIN ent_foyer f ON f.id = l.foyer_id AND f.etablissement_id = l.etablissement_id
              WHERE l.etablissement_id = $1 AND l.eleve_id = $2
              ORDER BY l.autorite_parentale DESC, l.created_at, l.id"
        );
        let rows = sqlx::query_as::<_, ResponsableRow>(&sql)
            .bind(etab)
            .bind(eleve_id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        Ok(rows.into_iter().map(Responsable::from).collect())
    }

    pub(crate) async fn contacts_bruts(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<ContactUrgence>, EntDbError> {
        let sql = format!(
            "SELECT c.id AS contact_id, c.relation, c.ordre, c.peut_recuperer, c.statut,
                    c.commentaire, {COLS_PERSONNE}
               FROM ent_eleve_contact_urgence c
               JOIN ent_personne p ON p.id = c.personne_id AND p.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.eleve_id = $2
              ORDER BY c.ordre, c.created_at, c.id"
        );
        let rows = sqlx::query_as::<_, ContactRow>(&sql)
            .bind(etab)
            .bind(eleve_id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| ContactUrgence {
                id: r.contact_id,
                personne: r.personne,
                relation: r.relation,
                ordre: r.ordre,
                peut_recuperer: r.peut_recuperer,
                statut: r.statut,
                commentaire: r.commentaire,
            })
            .collect())
    }

    /// Responsables d'un élève vus par l'acteur (minimisation pour les familles :
    /// les autres responsables sans coordonnées, liens inactifs masqués).
    pub async fn responsables_eleve_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<Responsable>, EntDbError> {
        let lecture = self.lecture_famille(etab, actor, role, eleve_id).await?;
        let tous = self.responsables_bruts(etab, eleve_id).await?;
        Ok(match lecture {
            LectureFamille::Aucune => return Err(EntDbError::Forbidden),
            LectureFamille::Complete => tous,
            LectureFamille::Pedagogique { payeurs } => tous
                .into_iter()
                .filter(|r| r.actif)
                .map(|mut r| {
                    r.suspendu_le = None;
                    r.suspension_motif = None;
                    r.suspension_reference = None;
                    if !payeurs {
                        r.est_payeur = false;
                        r.part_paiement = None;
                    }
                    r
                })
                .collect(),
            LectureFamille::Responsable(mon_lien) => tous
                .into_iter()
                .filter(|r| r.actif)
                .map(|mut r| {
                    if r.lien_id != mon_lien {
                        r.personne = r.personne.sans_coordonnees();
                        r.est_payeur = false;
                        r.part_paiement = None;
                    }
                    r.suspension_motif = None;
                    r.suspension_reference = None;
                    r
                })
                .collect(),
        })
    }

    /// Contacts d'urgence (personnel habilité, ou responsable actif de l'élève).
    pub async fn contacts_eleve_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<ContactUrgence>, EntDbError> {
        match self.lecture_famille(etab, actor, role, eleve_id).await? {
            LectureFamille::Aucune => Err(EntDbError::Forbidden),
            _ => self.contacts_bruts(etab, eleve_id).await,
        }
    }

    /// Fiche 360° : identité (minimisée selon le rôle), sections existantes,
    /// personnes, actions possibles et alertes de cohérence.
    pub async fn fiche_eleve_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<FicheEleve>, EntDbError> {
        let jour = today_utc();
        let Some(mut dossier) = self
            .get_dossier_for_actor(etab, actor, role, eleve_id, jour)
            .await?
        else {
            return Ok(None);
        };
        dossier.minimise(PgEnt::niveau_identite(role));
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let (responsables, contacts) = if dossier.sections.famille {
            match self.lecture_famille(etab, actor, role, eleve_id).await? {
                LectureFamille::Aucune => (Vec::new(), Vec::new()),
                _ => (
                    self.responsables_eleve_for_actor(etab, actor, role, eleve_id)
                        .await?,
                    self.contacts_bruts(etab, eleve_id).await?,
                ),
            }
        } else {
            (Vec::new(), Vec::new())
        };
        let majeur = self.eleve_est_majeur(etab, eleve_id, jour).await?;
        let mut actions = Vec::new();
        if parsed.is_staff() {
            for a in [
                Action::NotesLire,
                Action::NotesSaisir,
                Action::AbsencesLire,
                Action::AbsencesJustifier,
                Action::JournalLire,
                Action::MessagerieEleve,
                Action::SuiviEleve,
            ] {
                if self.autoriser_eleve(etab, actor, role, a, eleve_id).await? {
                    actions.push(ActionFiche {
                        code: a.code(),
                        libelle: a.libelle(),
                    });
                }
            }
            for a in [
                Action::PersonnesGerer,
                Action::PayeursDefinir,
                Action::LiensSuspendre,
                Action::PersonnesFusionner,
            ] {
                if self.a_action_etablissement(etab, actor, role, a).await? {
                    actions.push(ActionFiche {
                        code: a.code(),
                        libelle: a.libelle(),
                    });
                }
            }
        }
        let alertes = if parsed.is_staff() && dossier.sections.famille {
            alertes_fiche(&responsables)
        } else {
            Vec::new()
        };
        let resume = ResumeSections {
            notes: dossier.notes.len(),
            sanctions: dossier.sanctions.len(),
            incidents: dossier.incidents.len(),
            carnet: dossier.carnet.len(),
            documents: dossier.documents.len(),
            options: dossier.options.len(),
            dispositifs: dossier.dispositifs.len(),
            bourses: dossier.bourses.len(),
        };
        Ok(Some(FicheEleve {
            eleve: dossier.eleve,
            age_ans: dossier.age_ans,
            majeur,
            sections: dossier.sections,
            resume,
            responsables,
            contacts,
            actions,
            alertes,
        }))
    }

    /// Chronologie de l'élève (PLT-07) : journal + événements, filtrés par les
    /// droits du lecteur, du plus récent au plus ancien.
    pub async fn chronologie_eleve_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
        limite: usize,
    ) -> Result<Vec<EntreeChronologie>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        // RP1-04 : la chronologie n'est lue que par le personnel qui suit l'élève.
        self.exiger_perimetre_prof(etab, actor, role, eleve_id).await?;
        if self
            .get_eleve_for_actor(etab, actor, role, eleve_id)
            .await?
            .is_none()
        {
            return Err(EntDbError::Forbidden);
        }
        let sections = self
            .dossier_sections_for_actor(etab, actor, role, eleve_id)
            .await?;
        let limite = limite.clamp(1, 500);
        let mut out: Vec<EntreeChronologie> = Vec::new();
        let evenements: Vec<(DateTime<Utc>, String)> = sqlx::query_as(
            "SELECT created_at, type FROM ent_metier_event
              WHERE etablissement_id = $1 AND eleve_id = $2
              ORDER BY created_at DESC
              LIMIT 2000",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        for (quand, t) in evenements {
            if let Some((d, titre)) = classer_evenement(&t) {
                if domaine_visible(d, &sections, parsed) {
                    out.push(EntreeChronologie {
                        quand,
                        source: "evenement",
                        domaine: d.code(),
                        titre: titre.to_string(),
                    });
                }
            }
        }
        // Journal : mêmes règles que sa route (droit `journal.lire`, `peut_voir_vs`
        // pour les familles, entrées « visibles famille » seulement).
        match self
            .list_journal_eleve_for_actor(etab, actor, role, eleve_id, 200)
            .await
        {
            Ok(entrees) => {
                for e in entrees {
                    // Les entrées de ce module doublent leurs événements.
                    if e.domaine == "dossier" {
                        continue;
                    }
                    if let Some(d) = classer_journal(&e.domaine) {
                        if domaine_visible(d, &sections, parsed) {
                            out.push(EntreeChronologie {
                                quand: e.created_at,
                                source: "journal",
                                domaine: d.code(),
                                titre: e.libelle.chars().take(160).collect(),
                            });
                        }
                    }
                }
            }
            Err(EntDbError::Forbidden) => {}
            Err(e) => return Err(e),
        }
        out.sort_by(|a, b| b.quand.cmp(&a.quand));
        out.truncate(limite);
        Ok(out)
    }

    // -- Personnes ---------------------------------------------------------

    /// Recherche de personnes (homonymes, e-mail, téléphone) : personnel habilité.
    pub async fn rechercher_personnes_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        q: &str,
    ) -> Result<Vec<PersonneTrouvee>, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let q = q.trim();
        if q.chars().count() < 2 {
            return Ok(Vec::new());
        }
        let motif = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        let sql = format!(
            "SELECT {COLS_PERSONNE},
                    (SELECT count(*) FROM ent_eleve_lien l WHERE l.personne_id = p.id) AS nb_liens
               FROM ent_personne p
              WHERE p.etablissement_id = $1 AND p.fusionnee_dans IS NULL
                AND (p.nom ILIKE $2 OR p.prenom ILIKE $2 OR p.email ILIKE $2
                     OR (p.prenom || ' ' || p.nom) ILIKE $2 OR p.telephone ILIKE $2)
              ORDER BY lower(p.nom), lower(coalesce(p.prenom, '')), p.id
              LIMIT 50"
        );
        #[derive(sqlx::FromRow)]
        struct Row {
            #[sqlx(flatten)]
            personne: Personne,
            nb_liens: i64,
        }
        let rows = sqlx::query_as::<_, Row>(&sql)
            .bind(etab)
            .bind(motif)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| PersonneTrouvee {
                personne: r.personne,
                nb_liens: r.nb_liens,
            })
            .collect())
    }

    pub async fn get_personne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
    ) -> Result<Personne, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let mut conn = self.pool().acquire().await.map_err(db)?;
        personne_tx(&mut conn, etab, personne_id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))
    }

    /// DOS-03 : créer une personne (avec ou sans compte, sans e-mail possible).
    pub async fn creer_personne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        saisie: &PersonneSaisie,
    ) -> Result<Personne, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let s = saisie.normaliser()?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let id = self.inserer_personne_tx(&mut tx, etab, &s, actor).await?;
        let p = personne_tx(&mut tx, etab, id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        commit(tx).await?;
        Ok(p)
    }

    async fn inserer_personne_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        s: &PersonneSaisie,
        actor: Uuid,
    ) -> Result<Uuid, EntDbError> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_personne (id, etablissement_id, nom, prenom, civilite, email,
                                       telephone, telephone_2, adresse, source)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'saisie')",
        )
        .bind(id)
        .bind(etab)
        .bind(&s.nom)
        .bind(&s.prenom)
        .bind(&s.civilite)
        .bind(&s.email)
        .bind(&s.telephone)
        .bind(&s.telephone_2)
        .bind(&s.adresse)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        journal_tx(
            conn,
            etab,
            None,
            "personne_creee",
            "Personne créée",
            json!({ "personne_id": id, "sans_email": s.email.is_none() }),
            false,
            actor,
        )
        .await?;
        publier_tx(
            conn,
            etab,
            types::PERSONNE_CREEE,
            AggregateKind::Personne,
            id,
            None,
            json!({ "personne_id": id }),
            actor,
        )
        .await?;
        Ok(id)
    }

    /// Modifier l'identité / les coordonnées d'une personne.
    pub async fn modifier_personne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        saisie: &PersonneSaisie,
    ) -> Result<Personne, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let s = saisie.normaliser()?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let avant = personne_tx(&mut tx, etab, personne_id, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if avant.fusionnee_dans.is_some() {
            return Err(EntDbError::ConflitDossier("personne fusionnée dans une autre fiche"));
        }
        let mut champs: Vec<&str> = Vec::new();
        for (nom, a, b) in [
            ("nom", Some(avant.nom.clone()), Some(s.nom.clone())),
            ("prenom", avant.prenom.clone(), s.prenom.clone()),
            ("civilite", avant.civilite.clone(), s.civilite.clone()),
            ("email", avant.email.clone(), s.email.clone()),
            ("telephone", avant.telephone.clone(), s.telephone.clone()),
            ("telephone_2", avant.telephone_2.clone(), s.telephone_2.clone()),
            ("adresse", avant.adresse.clone(), s.adresse.clone()),
        ] {
            if a != b {
                champs.push(nom);
            }
        }
        if !champs.is_empty() {
            sqlx::query(
                "UPDATE ent_personne SET nom = $3, prenom = $4, civilite = $5, email = $6,
                        telephone = $7, telephone_2 = $8, adresse = $9, updated_at = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(personne_id)
            .bind(&s.nom)
            .bind(&s.prenom)
            .bind(&s.civilite)
            .bind(&s.email)
            .bind(&s.telephone)
            .bind(&s.telephone_2)
            .bind(&s.adresse)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            // Charge utile minimale : noms des champs, jamais leurs valeurs.
            journal_tx(
                &mut tx,
                etab,
                None,
                "personne_modifiee",
                "Coordonnées d'une personne modifiées",
                json!({ "personne_id": personne_id, "champs": champs }),
                false,
                actor,
            )
            .await?;
            publier_tx(
                &mut tx,
                etab,
                types::PERSONNE_COORDONNEES_MODIFIEES,
                AggregateKind::Personne,
                personne_id,
                None,
                json!({ "personne_id": personne_id, "champs": champs }),
                actor,
            )
            .await?;
        }
        let p = personne_tx(&mut tx, etab, personne_id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        commit(tx).await?;
        Ok(p)
    }

    /// Rattacher un compte existant de l'établissement à une personne
    /// (le compte doit être membre de l'établissement, profil parent).
    pub async fn rattacher_compte_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        user_id: Uuid,
    ) -> Result<Personne, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = personne_tx(&mut tx, etab, personne_id, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if p.fusionnee_dans.is_some() || p.eleve_id.is_some() {
            return Err(EntDbError::ConflitDossier("cette fiche ne peut pas recevoir de compte"));
        }
        if p.user_id.is_some() {
            return Err(EntDbError::ConflitDossier("la personne a déjà un compte"));
        }
        let membre: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etab)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        // Compte d'un autre établissement : indiscernable d'un compte inconnu.
        let Some((profil,)) = membre else {
            return Err(EntDbError::Domain(DomainError::NotFound));
        };
        if profil != Role::Parent.as_str() {
            return Err(invalide("seul un compte de responsable peut être rattaché"));
        }
        sqlx::query(
            "UPDATE ent_personne SET user_id = $3, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(personne_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            None,
            "personne_compte_rattache",
            "Compte rattaché à une personne",
            json!({ "personne_id": personne_id }),
            false,
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNE_COMPTE_RATTACHE,
            AggregateKind::Personne,
            personne_id,
            None,
            json!({ "personne_id": personne_id }),
            actor,
        )
        .await?;
        let p = personne_tx(&mut tx, etab, personne_id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        commit(tx).await?;
        Ok(p)
    }

    /// Fiche « personne » représentant l'élève lui-même (élève majeur : payeur,
    /// signataire). Créée une seule fois ; refusée pour un élève mineur.
    pub async fn personne_eleve_majeur_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Personne, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let eleve = self.dossier_exiger_eleve(etab, eleve_id).await?;
        if !self.eleve_est_majeur(etab, eleve_id, today_utc()).await? {
            return Err(invalide("l'élève n'est pas majeur (date de naissance)"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let existante: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND eleve_id = $2",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let id = match existante {
            Some((id,)) => id,
            None => {
                let id = Uuid::new_v4();
                // Pas de compte sur cette fiche : l'élève agit avec son compte
                // élève ; la fiche ne sert qu'aux rôles « responsable » (payeur).
                sqlx::query(
                    "INSERT INTO ent_personne (id, etablissement_id, nom, prenom, eleve_id, source)
                     VALUES ($1,$2,$3,$4,$5,'saisie')",
                )
                .bind(id)
                .bind(etab)
                .bind(&eleve.nom)
                .bind(&eleve.prenom)
                .bind(eleve_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                publier_tx(
                    &mut tx,
                    etab,
                    types::PERSONNE_CREEE,
                    AggregateKind::Personne,
                    id,
                    Some(eleve_id),
                    json!({ "personne_id": id, "eleve_majeur": true }),
                    actor,
                )
                .await?;
                id
            }
        };
        let p = personne_tx(&mut tx, etab, id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        commit(tx).await?;
        Ok(p)
    }

    // -- Liens de responsabilité ------------------------------------------

    /// DOS-05 : lier une personne à un élève (lien, autorité, garde, dates, droits).
    pub async fn lier_personne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
        saisie: &LienSaisie,
    ) -> Result<Responsable, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        self.dossier_exiger_eleve(etab, eleve_id).await?;
        let (lien, garde) = saisie.valider()?;
        let personne_id = saisie
            .personne_id
            .ok_or(invalide("personne à lier obligatoire"))?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = personne_tx(&mut tx, etab, personne_id, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if p.fusionnee_dans.is_some() {
            return Err(EntDbError::ConflitDossier("personne fusionnée dans une autre fiche"));
        }
        // L'élève majeur lié à lui-même : sans autorité parentale, sans droit
        // de famille (ses accès viennent de son compte élève).
        let soi_meme = p.eleve_id.is_some();
        if soi_meme && (p.eleve_id != Some(eleve_id) || lien != LIEN_ELEVE_MAJEUR) {
            return Err(invalide("la fiche d'un élève ne se lie qu'à lui-même"));
        }
        if !soi_meme && lien == LIEN_ELEVE_MAJEUR {
            return Err(invalide("lien réservé à l'élève majeur"));
        }
        if let Some(f) = saisie.foyer_id {
            self.exiger_foyer_tx(&mut tx, etab, f).await?;
        }
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_eleve_lien (
                 id, etablissement_id, eleve_id, personne_id, foyer_id, lien,
                 autorite_parentale, garde, date_debut, date_fin,
                 peut_voir_notes, peut_voir_vs, peut_justifier_absence, peut_autoriser_sorties,
                 est_payeur, part_paiement, created_at, updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,false,NULL,now(),now())",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve_id)
        .bind(personne_id)
        .bind(saisie.foyer_id)
        .bind(&lien)
        .bind(saisie.autorite_parentale && !soi_meme)
        .bind(&garde)
        .bind(saisie.date_debut)
        .bind(saisie.date_fin)
        .bind(saisie.peut_voir_notes && !soi_meme)
        .bind(saisie.peut_voir_vs && !soi_meme)
        .bind(saisie.peut_justifier_absence && !soi_meme)
        .bind(saisie.peut_autoriser_sorties && !soi_meme)
        .execute(&mut *tx)
        .await;
        if let Err(e) = r {
            let d = map_sqlx("dossier_personnes", e);
            return Err(if d == DomainError::Conflit {
                EntDbError::ConflitDossier("cette personne est déjà liée à l'élève")
            } else {
                EntDbError::Domain(d)
            });
        }
        let payload = json!({
            "lien_id": id, "personne_id": personne_id, "autorite_parentale": saisie.autorite_parentale && !soi_meme,
            "droits": {
                "notes": saisie.peut_voir_notes && !soi_meme, "vs": saisie.peut_voir_vs && !soi_meme,
                "justifier": saisie.peut_justifier_absence && !soi_meme,
                "sorties": saisie.peut_autoriser_sorties && !soi_meme,
            }
        });
        journal_tx(
            &mut tx,
            etab,
            Some(eleve_id),
            "lien_cree",
            "Responsable lié à l'élève",
            payload.clone(),
            false,
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::ELEVE_LIEN_UPDATED,
            AggregateKind::Eleve,
            eleve_id,
            Some(eleve_id),
            payload,
            actor,
        )
        .await?;
        commit(tx).await?;
        self.responsable_par_lien(etab, id).await
    }

    async fn exiger_foyer_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        foyer_id: Uuid,
    ) -> Result<(), EntDbError> {
        let ok: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_foyer WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(foyer_id)
                .fetch_optional(conn)
                .await
                .map_err(db)?;
        ok.map(|_| ()).ok_or(EntDbError::Domain(DomainError::ReferenceInconnue))
    }

    async fn responsable_par_lien(&self, etab: Uuid, lien_id: Uuid) -> Result<Responsable, EntDbError> {
        let eleve: Option<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lien_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let Some((eleve_id,)) = eleve else {
            return Err(EntDbError::Domain(DomainError::NotFound));
        };
        self.responsables_bruts(etab, eleve_id)
            .await?
            .into_iter()
            .find(|r| r.lien_id == lien_id)
            .ok_or(EntDbError::Domain(DomainError::NotFound))
    }

    /// Modifier un lien (nature, autorité, garde, dates, droits fins, foyer).
    pub async fn modifier_lien_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        lien_id: Uuid,
        saisie: &LienSaisie,
    ) -> Result<Responsable, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let (lien, garde) = saisie.valider()?;
        let avant = self.responsable_par_lien(etab, lien_id).await?;
        let soi_meme = avant.lien == LIEN_ELEVE_MAJEUR;
        if soi_meme != (lien == LIEN_ELEVE_MAJEUR) {
            return Err(invalide("lien réservé à l'élève majeur"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        if let Some(f) = saisie.foyer_id {
            self.exiger_foyer_tx(&mut tx, etab, f).await?;
        }
        sqlx::query(
            "UPDATE ent_eleve_lien SET foyer_id = $3, lien = $4, autorite_parentale = $5,
                    garde = $6, date_debut = $7, date_fin = $8, peut_voir_notes = $9,
                    peut_voir_vs = $10, peut_justifier_absence = $11,
                    peut_autoriser_sorties = $12, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lien_id)
        .bind(saisie.foyer_id)
        .bind(&lien)
        .bind(saisie.autorite_parentale && !soi_meme)
        .bind(&garde)
        .bind(saisie.date_debut)
        .bind(saisie.date_fin)
        .bind(saisie.peut_voir_notes && !soi_meme)
        .bind(saisie.peut_voir_vs && !soi_meme)
        .bind(saisie.peut_justifier_absence && !soi_meme)
        .bind(saisie.peut_autoriser_sorties && !soi_meme)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let payload = json!({
            "lien_id": lien_id, "personne_id": avant.personne.id,
            "autorite_parentale": saisie.autorite_parentale && !soi_meme,
            "date_fin": saisie.date_fin,
            "droits": {
                "notes": saisie.peut_voir_notes && !soi_meme, "vs": saisie.peut_voir_vs && !soi_meme,
                "justifier": saisie.peut_justifier_absence && !soi_meme,
                "sorties": saisie.peut_autoriser_sorties && !soi_meme,
            }
        });
        journal_tx(
            &mut tx,
            etab,
            Some(avant.eleve_id),
            "lien_modifie",
            "Lien d'un responsable modifié",
            payload.clone(),
            false,
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::ELEVE_LIEN_UPDATED,
            AggregateKind::Eleve,
            avant.eleve_id,
            Some(avant.eleve_id),
            payload,
            actor,
        )
        .await?;
        commit(tx).await?;
        self.responsable_par_lien(etab, lien_id).await
    }

    /// DOS-06 ★ : suspendre le lien d'un responsable (décision de justice).
    /// Effet immédiat : tous les contrôles famille lisent les liens actifs.
    pub async fn suspendre_lien_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        lien_id: Uuid,
        motif: &str,
        reference: Option<&str>,
    ) -> Result<Responsable, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::LiensSuspendre)
            .await?;
        let motif = motif.trim();
        if motif.chars().count() < 3 || motif.chars().count() > 500 {
            return Err(invalide("motif de suspension obligatoire (3 à 500 caractères)"));
        }
        let reference = reference
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .map(|r| r.chars().take(120).collect::<String>());
        let avant = self.responsable_par_lien(etab, lien_id).await?;
        if avant.suspendu_le.is_some() {
            return Err(EntDbError::ConflitDossier("lien déjà suspendu"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_eleve_lien SET suspendu_le = now(), suspendu_par = $3,
                    suspension_motif = $4, suspension_reference = $5, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2 AND suspendu_le IS NULL",
        )
        .bind(etab)
        .bind(lien_id)
        .bind(actor)
        .bind(motif)
        .bind(&reference)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Le motif reste dans la table (accès restreint) ; ni journal visible
        // famille ni événement ne le recopient (règle 3 de la toile).
        let payload = json!({ "lien_id": lien_id, "personne_id": avant.personne.id });
        journal_tx(
            &mut tx,
            etab,
            Some(avant.eleve_id),
            "lien_suspendu",
            "Lien d'un responsable suspendu",
            json!({ "lien_id": lien_id, "personne_id": avant.personne.id, "reference": reference }),
            false,
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::LIEN_SUSPENDU,
            AggregateKind::Eleve,
            avant.eleve_id,
            Some(avant.eleve_id),
            payload,
            actor,
        )
        .await?;
        commit(tx).await?;
        self.responsable_par_lien(etab, lien_id).await
    }

    /// Rétablir un lien suspendu.
    pub async fn retablir_lien_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        lien_id: Uuid,
    ) -> Result<Responsable, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::LiensSuspendre)
            .await?;
        let avant = self.responsable_par_lien(etab, lien_id).await?;
        if avant.suspendu_le.is_none() {
            return Err(EntDbError::ConflitDossier("lien non suspendu"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_eleve_lien SET suspendu_le = NULL, suspendu_par = NULL,
                    suspension_motif = NULL, suspension_reference = NULL, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(lien_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let payload = json!({ "lien_id": lien_id, "personne_id": avant.personne.id });
        journal_tx(
            &mut tx,
            etab,
            Some(avant.eleve_id),
            "lien_retabli",
            "Lien d'un responsable rétabli",
            payload.clone(),
            false,
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::LIEN_RETABLI,
            AggregateKind::Eleve,
            avant.eleve_id,
            Some(avant.eleve_id),
            payload,
            actor,
        )
        .await?;
        commit(tx).await?;
        self.responsable_par_lien(etab, lien_id).await
    }

    /// DOS-07 : désigner les payeurs (parts en %, somme = 100) ; les autres liens
    /// cessent d'être payeurs. Un payeur doit appartenir à un foyer (débiteur des
    /// factures existantes, `ent_fac_*` par foyer).
    pub async fn definir_payeurs_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
        payeurs: &[(Uuid, i16)],
    ) -> Result<Vec<Responsable>, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PayeursDefinir)
            .await?;
        self.dossier_exiger_eleve(etab, eleve_id).await?;
        let liens = self.responsables_bruts(etab, eleve_id).await?;
        let mut vus = std::collections::HashSet::new();
        for (lien_id, part) in payeurs {
            if !vus.insert(*lien_id) {
                return Err(invalide("payeur cité deux fois"));
            }
            let Some(l) = liens.iter().find(|l| l.lien_id == *lien_id) else {
                return Err(EntDbError::Domain(DomainError::NotFound));
            };
            if l.foyer_id.is_none() {
                return Err(invalide("un payeur doit être rattaché à un foyer"));
            }
            if !(1..=100).contains(part) {
                return Err(invalide("part de paiement entre 1 et 100 %"));
            }
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_eleve_lien SET est_payeur = false, part_paiement = NULL, updated_at = now()
              WHERE etablissement_id = $1 AND eleve_id = $2",
        )
        .bind(etab)
        .bind(eleve_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (lien_id, part) in payeurs {
            sqlx::query(
                "UPDATE ent_eleve_lien SET est_payeur = true, part_paiement = $4, updated_at = now()
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3",
            )
            .bind(etab)
            .bind(eleve_id)
            .bind(lien_id)
            .bind(part)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        verifier_parts_tx(&mut tx, etab, eleve_id).await?;
        let payload = json!({
            "payeurs": payeurs.iter().map(|(l, p)| json!({ "lien_id": l, "part": p })).collect::<Vec<_>>()
        });
        journal_tx(
            &mut tx,
            etab,
            Some(eleve_id),
            "payeurs_definis",
            "Payeurs définis",
            payload.clone(),
            false,
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::FOYER_PAYEUR_DEFINI,
            AggregateKind::Eleve,
            eleve_id,
            Some(eleve_id),
            payload,
            actor,
        )
        .await?;
        commit(tx).await?;
        self.responsables_bruts(etab, eleve_id).await
    }

    // -- Contacts d'urgence -------------------------------------------------

    /// DOS-08 : ajouter un contact. Personnel habilité → contact validé ;
    /// responsable actif de l'élève → proposition (nouvelle personne seulement),
    /// à valider par l'établissement.
    pub async fn ajouter_contact_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        eleve_id: Uuid,
        saisie: &ContactSaisie,
    ) -> Result<ContactUrgence, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let proposition = if parsed == Role::Parent {
            if self.lien_parent_eleve(etab, actor, eleve_id).await?.is_none() {
                return Err(EntDbError::Forbidden);
            }
            if saisie.personne_id.is_some() {
                // Un responsable ne parcourt pas les personnes de l'établissement.
                return Err(EntDbError::Forbidden);
            }
            true
        } else {
            self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
                .await?;
            false
        };
        self.dossier_exiger_eleve(etab, eleve_id).await?;
        let relation = saisie.relation.trim();
        if relation.is_empty() || relation.chars().count() > 60 {
            return Err(invalide("relation avec l'élève obligatoire (60 caractères au plus)"));
        }
        if !(1..=20).contains(&saisie.ordre) {
            return Err(invalide("ordre d'appel entre 1 et 20"));
        }
        let commentaire = opt(&saisie.commentaire);
        if commentaire.as_ref().is_some_and(|c| c.chars().count() > 500) {
            return Err(invalide("commentaire trop long"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let personne_id = match (&saisie.personne_id, &saisie.nouvelle_personne) {
            (Some(id), None) => {
                let p = personne_tx(&mut tx, etab, *id, true)
                    .await?
                    .ok_or(EntDbError::Domain(DomainError::NotFound))?;
                if p.fusionnee_dans.is_some() {
                    return Err(EntDbError::ConflitDossier("personne fusionnée dans une autre fiche"));
                }
                *id
            }
            (None, Some(n)) => {
                let s = n.normaliser()?;
                if s.telephone.is_none() && s.telephone_2.is_none() {
                    return Err(invalide("un contact d'urgence doit avoir un téléphone"));
                }
                self.inserer_personne_tx(&mut tx, etab, &s, actor).await?
            }
            _ => return Err(invalide("indiquer une personne existante ou une nouvelle personne")),
        };
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_eleve_contact_urgence (id, etablissement_id, eleve_id, personne_id,
                    relation, ordre, peut_recuperer, statut, commentaire)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(eleve_id)
        .bind(personne_id)
        .bind(relation)
        .bind(saisie.ordre)
        .bind(saisie.peut_recuperer)
        .bind(if proposition { "propose" } else { "valide" })
        .bind(&commentaire)
        .execute(&mut *tx)
        .await;
        if let Err(e) = r {
            let d = map_sqlx("dossier_personnes", e);
            return Err(if d == DomainError::Conflit {
                EntDbError::ConflitDossier("cette personne est déjà un contact de l'élève")
            } else {
                EntDbError::Domain(d)
            });
        }
        self.trace_contacts_tx(&mut tx, etab, eleve_id, "contact_ajoute", id, actor)
            .await?;
        commit(tx).await?;
        self.contacts_bruts(etab, eleve_id)
            .await?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or(EntDbError::Domain(DomainError::NotFound))
    }

    async fn trace_contacts_tx(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        eleve_id: Uuid,
        action: &str,
        contact_id: Uuid,
        actor: Uuid,
    ) -> Result<(), EntDbError> {
        let payload = json!({ "contact_id": contact_id, "action": action });
        journal_tx(
            conn,
            etab,
            Some(eleve_id),
            action,
            "Contacts d'urgence modifiés",
            payload.clone(),
            false,
            actor,
        )
        .await?;
        publier_tx(
            conn,
            etab,
            types::ELEVE_CONTACTS_URGENCE_CHANGES,
            AggregateKind::Eleve,
            eleve_id,
            Some(eleve_id),
            payload,
            actor,
        )
        .await
    }

    /// Valider un contact proposé par un responsable.
    pub async fn valider_contact_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        contact_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_eleve_contact_urgence SET statut = 'valide', updated_at = now()
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'propose'
              RETURNING eleve_id",
        )
        .bind(etab)
        .bind(contact_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((eleve_id,)) = row else {
            return Err(EntDbError::Domain(DomainError::NotFound));
        };
        self.trace_contacts_tx(&mut tx, etab, eleve_id, "contact_valide", contact_id, actor)
            .await?;
        commit(tx).await
    }

    /// Retirer un contact d'urgence (la personne reste).
    pub async fn retirer_contact_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        contact_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "DELETE FROM ent_eleve_contact_urgence WHERE etablissement_id = $1 AND id = $2
              RETURNING eleve_id",
        )
        .bind(etab)
        .bind(contact_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((eleve_id,)) = row else {
            return Err(EntDbError::Domain(DomainError::NotFound));
        };
        self.trace_contacts_tx(&mut tx, etab, eleve_id, "contact_retire", contact_id, actor)
            .await?;
        commit(tx).await
    }

    // -- Fusion réversible --------------------------------------------------

    /// DOS-15 (personnes) ★ : absorber `source` dans `cible`. Les liens et
    /// contacts de la source passent à la cible ; en cas de doublon (même élève),
    /// celui de la source est retiré et conservé dans l'instantané. Le compte de
    /// la source passe à la cible si celle-ci n'en a pas ; deux comptes
    /// différents → refus (pas de fusion de comptes, D-013).
    pub async fn fusionner_personnes_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        source_id: Uuid,
        cible_id: Uuid,
        motif: &str,
    ) -> Result<FusionPersonne, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesFusionner)
            .await?;
        let motif = motif.trim();
        if motif.chars().count() < 3 || motif.chars().count() > 500 {
            return Err(invalide("motif de fusion obligatoire (3 à 500 caractères)"));
        }
        if source_id == cible_id {
            return Err(invalide("une personne ne se fusionne pas avec elle-même"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        // Verrous dans un ordre stable (pas d'interblocage entre deux fusions croisées).
        let (a, b) = if source_id < cible_id {
            (source_id, cible_id)
        } else {
            (cible_id, source_id)
        };
        let pa = personne_tx(&mut tx, etab, a, true).await?;
        let pb = personne_tx(&mut tx, etab, b, true).await?;
        let (Some(pa), Some(pb)) = (pa, pb) else {
            return Err(EntDbError::Domain(DomainError::NotFound));
        };
        let (source, cible) = if pa.id == source_id { (pa, pb) } else { (pb, pa) };
        if source.fusionnee_dans.is_some() || cible.fusionnee_dans.is_some() {
            return Err(EntDbError::ConflitDossier("une des personnes est déjà fusionnée"));
        }
        if source.eleve_id.is_some() || cible.eleve_id.is_some() {
            return Err(EntDbError::ConflitDossier("la fiche d'un élève ne se fusionne pas"));
        }
        if let (Some(u1), Some(u2)) = (source.user_id, cible.user_id) {
            if u1 != u2 {
                return Err(EntDbError::ConflitDossier(
                    "deux comptes différents : pas de fusion de comptes",
                ));
            }
        }
        let compte_transfere = source.user_id.filter(|_| cible.user_id.is_none());
        if let Some(u) = compte_transfere {
            sqlx::query(
                "UPDATE ent_personne SET user_id = NULL, updated_at = now() \
                  WHERE id = $1 AND etablissement_id = $2",
            )
                .bind(source.id)
                .bind(etab)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            sqlx::query(
                "UPDATE ent_personne SET user_id = $2, updated_at = now() \
                  WHERE id = $1 AND etablissement_id = $3",
            )
                .bind(cible.id)
                .bind(u)
                .bind(etab)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        // Liens : doublons (même élève) retirés et gardés tels quels, autres déplacés.
        let liens_supprimes: Vec<(Uuid, Value)> = sqlx::query_as(
            "DELETE FROM ent_eleve_lien l
              WHERE l.etablissement_id = $1 AND l.personne_id = $2
                AND EXISTS (SELECT 1 FROM ent_eleve_lien c
                             WHERE c.etablissement_id = l.etablissement_id
                               AND c.eleve_id = l.eleve_id AND c.personne_id = $3)
              RETURNING l.eleve_id, to_jsonb(l.*)",
        )
        .bind(etab)
        .bind(source.id)
        .bind(cible.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let liens_deplaces: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "UPDATE ent_eleve_lien SET personne_id = $3, updated_at = now()
              WHERE etablissement_id = $1 AND personne_id = $2
              RETURNING id, eleve_id",
        )
        .bind(etab)
        .bind(source.id)
        .bind(cible.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let contacts_supprimes: Vec<(Value,)> = sqlx::query_as(
            "DELETE FROM ent_eleve_contact_urgence c
              WHERE c.etablissement_id = $1 AND c.personne_id = $2
                AND EXISTS (SELECT 1 FROM ent_eleve_contact_urgence d
                             WHERE d.etablissement_id = c.etablissement_id
                               AND d.eleve_id = c.eleve_id AND d.personne_id = $3)
              RETURNING to_jsonb(c.*)",
        )
        .bind(etab)
        .bind(source.id)
        .bind(cible.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let contacts_deplaces: Vec<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_eleve_contact_urgence SET personne_id = $3, updated_at = now()
              WHERE etablissement_id = $1 AND personne_id = $2
              RETURNING id",
        )
        .bind(etab)
        .bind(source.id)
        .bind(cible.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_personne SET fusionnee_dans = $2, updated_at = now() \
              WHERE id = $1 AND etablissement_id = $3",
        )
        .bind(source.id)
        .bind(cible.id)
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut eleves: Vec<Uuid> = liens_supprimes
            .iter()
            .map(|(e, _)| *e)
            .chain(liens_deplaces.iter().map(|(_, e)| *e))
            .collect();
        eleves.sort();
        eleves.dedup();
        for e in &eleves {
            verifier_parts_tx(&mut tx, etab, *e).await?;
        }
        let instantane = json!({
            "compte_transfere": compte_transfere,
            "liens_deplaces": liens_deplaces.iter().map(|(id, _)| id).collect::<Vec<_>>(),
            "liens_supprimes": liens_supprimes.iter().map(|(_, v)| v).collect::<Vec<_>>(),
            "contacts_deplaces": contacts_deplaces.iter().map(|(id,)| id).collect::<Vec<_>>(),
            "contacts_supprimes": contacts_supprimes.iter().map(|(v,)| v).collect::<Vec<_>>(),
        });
        let fusion_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_personne_fusion (id, etablissement_id, source_id, cible_id,
                    instantane, motif, fusionne_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(fusion_id)
        .bind(etab)
        .bind(source.id)
        .bind(cible.id)
        .bind(&instantane)
        .bind(motif)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let payload = json!({ "fusion_id": fusion_id, "source_id": source.id, "cible_id": cible.id });
        journal_tx(
            &mut tx,
            etab,
            None,
            "personnes_fusionnees",
            "Personnes en doublon fusionnées",
            payload.clone(),
            false,
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNE_FUSIONNEE,
            AggregateKind::Personne,
            cible.id,
            None,
            payload,
            actor,
        )
        .await?;
        let f = fusion_tx(&mut tx, etab, fusion_id).await?;
        commit(tx).await?;
        Ok(f)
    }

    /// Annuler une fusion (≤ 30 jours) : restaure exactement l'état de la source.
    pub async fn defusionner_personnes_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        fusion_id: Uuid,
    ) -> Result<FusionPersonne, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesFusionner)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let row: Option<(Uuid, Uuid, Value, DateTime<Utc>, Option<DateTime<Utc>>)> =
            sqlx::query_as(
                "SELECT source_id, cible_id, instantane, fusionne_le, annule_le
                   FROM ent_personne_fusion WHERE etablissement_id = $1 AND id = $2
                  FOR UPDATE",
            )
            .bind(etab)
            .bind(fusion_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        let Some((source_id, cible_id, inst, fusionne_le, annule_le)) = row else {
            return Err(EntDbError::Domain(DomainError::NotFound));
        };
        if annule_le.is_some() {
            return Err(EntDbError::ConflitDossier("fusion déjà annulée"));
        }
        if Utc::now() - fusionne_le > Duration::days(DELAI_DEFUSION_JOURS) {
            return Err(EntDbError::ConflitDossier("délai d'annulation de la fusion dépassé"));
        }
        let (a, b) = if source_id < cible_id {
            (source_id, cible_id)
        } else {
            (cible_id, source_id)
        };
        personne_tx(&mut tx, etab, a, true).await?;
        personne_tx(&mut tx, etab, b, true).await?;
        let cible = personne_tx(&mut tx, etab, cible_id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let uuids = |cle: &str| -> Vec<Uuid> {
            inst.get(cle)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                        .collect()
                })
                .unwrap_or_default()
        };
        let lignes = |cle: &str| -> Vec<Value> {
            inst.get(cle)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        sqlx::query(
            "UPDATE ent_personne SET fusionnee_dans = NULL, updated_at = now() \
              WHERE id = $1 AND etablissement_id = $2",
        )
            .bind(source_id)
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if let Some(u) = inst
            .get("compte_transfere")
            .and_then(Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok())
        {
            if cible.user_id != Some(u) {
                return Err(EntDbError::ConflitDossier("le compte de la fiche a changé depuis la fusion"));
            }
            sqlx::query(
                "UPDATE ent_personne SET user_id = NULL, updated_at = now() \
                  WHERE id = $1 AND etablissement_id = $2",
            )
                .bind(cible_id)
                .bind(etab)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            sqlx::query(
                "UPDATE ent_personne SET user_id = $2, updated_at = now() \
                  WHERE id = $1 AND etablissement_id = $3",
            )
                .bind(source_id)
                .bind(u)
                .bind(etab)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let mut eleves: Vec<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_eleve_lien SET personne_id = $3, updated_at = now()
              WHERE etablissement_id = $1 AND personne_id = $2 AND id = ANY($4)
              RETURNING eleve_id",
        )
        .bind(etab)
        .bind(cible_id)
        .bind(source_id)
        .bind(uuids("liens_deplaces"))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for ligne in lignes("liens_supprimes") {
            let r: Result<(Uuid,), _> = sqlx::query_as(
                "INSERT INTO ent_eleve_lien
                 SELECT * FROM jsonb_populate_record(NULL::ent_eleve_lien, $1)
                 RETURNING eleve_id",
            )
            .bind(&ligne)
            .fetch_one(&mut *tx)
            .await;
            match r {
                Ok(e) => eleves.push(e),
                Err(e) => {
                    let d = map_sqlx("dossier_personnes", e);
                    return Err(if d == DomainError::Conflit {
                        EntDbError::ConflitDossier("un lien restauré existe déjà")
                    } else {
                        EntDbError::Domain(d)
                    });
                }
            }
        }
        sqlx::query(
            "UPDATE ent_eleve_contact_urgence SET personne_id = $3, updated_at = now()
              WHERE etablissement_id = $1 AND personne_id = $2 AND id = ANY($4)",
        )
        .bind(etab)
        .bind(cible_id)
        .bind(source_id)
        .bind(uuids("contacts_deplaces"))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for ligne in lignes("contacts_supprimes") {
            sqlx::query(
                "INSERT INTO ent_eleve_contact_urgence
                 SELECT * FROM jsonb_populate_record(NULL::ent_eleve_contact_urgence, $1)",
            )
            .bind(&ligne)
            .execute(&mut *tx)
            .await
            .map_err(|e| match map_sqlx("dossier_personnes", e) {
                DomainError::Conflit => EntDbError::ConflitDossier("un contact restauré existe déjà"),
                d => EntDbError::Domain(d),
            })?;
        }
        eleves.sort();
        eleves.dedup();
        for (e,) in &eleves {
            verifier_parts_tx(&mut tx, etab, *e).await?;
        }
        sqlx::query(
            "UPDATE ent_personne_fusion SET annule_le = now(), annule_par = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(fusion_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let payload = json!({ "fusion_id": fusion_id, "source_id": source_id, "cible_id": cible_id });
        journal_tx(
            &mut tx,
            etab,
            None,
            "personnes_defusionnees",
            "Fusion de personnes annulée",
            payload.clone(),
            false,
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNE_DEFUSIONNEE,
            AggregateKind::Personne,
            source_id,
            None,
            payload,
            actor,
        )
        .await?;
        let f = fusion_tx(&mut tx, etab, fusion_id).await?;
        commit(tx).await?;
        Ok(f)
    }

    /// Fusions récentes de l'établissement (annulables ou non).
    pub async fn list_fusions_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<FusionPersonne>, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesFusionner)
            .await?;
        sqlx::query_as::<_, FusionPersonne>(
            "SELECT id, source_id, cible_id, motif, fusionne_le, annule_le
               FROM ent_personne_fusion WHERE etablissement_id = $1
              ORDER BY fusionne_le DESC, id LIMIT 100",
        )
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }
}

async fn fusion_tx(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<FusionPersonne, EntDbError> {
    sqlx::query_as::<_, FusionPersonne>(
        "SELECT id, source_id, cible_id, motif, fusionne_le, annule_le
           FROM ent_personne_fusion WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_one(conn)
    .await
    .map_err(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sections(tout: bool) -> DossierSections {
        DossierSections {
            identite: true,
            scolarite: true,
            famille: tout,
            options: tout,
            notes: tout,
            vs: tout,
            documents: tout,
            dispositifs: tout,
            repas: tout,
            bourses: tout,
            continuum: tout,
        }
    }

    #[test]
    fn chronologie_liste_blanche_et_titres_neutres() {
        assert_eq!(classer_evenement("type.inconnu"), None);
        assert_eq!(classer_evenement(types::MAIL_ENVOYE), None);
        let (d, t) = classer_evenement(types::INFIRMERIE_PASSAGE).unwrap();
        assert_eq!(d, DomaineChrono::Sante);
        assert!(!t.contains('{'));
        assert_eq!(classer_evenement(types::LIEN_SUSPENDU).unwrap().0, DomaineChrono::Dossier);
    }

    #[test]
    fn sante_et_psy_reserves_a_leur_metier() {
        let s = sections(true);
        for r in [Role::Direction, Role::Cpe, Role::Prof, Role::Parent, Role::Admin] {
            assert!(!domaine_visible(DomaineChrono::Sante, &s, r), "{r:?}");
            assert!(!domaine_visible(DomaineChrono::Psy, &s, r), "{r:?}");
        }
        assert!(domaine_visible(DomaineChrono::Sante, &s, Role::Infirmerie));
        assert!(domaine_visible(DomaineChrono::Psy, &s, Role::Psy));
    }

    #[test]
    fn famille_ne_voit_que_scolarite_vs_notes_selon_droits() {
        let mut s = sections(true);
        assert!(!domaine_visible(DomaineChrono::Dossier, &s, Role::Parent));
        assert!(!domaine_visible(DomaineChrono::Finances, &s, Role::Parent));
        assert!(domaine_visible(DomaineChrono::VieScolaire, &s, Role::Parent));
        s.vs = false;
        assert!(!domaine_visible(DomaineChrono::VieScolaire, &s, Role::Parent));
        assert!(domaine_visible(DomaineChrono::Dossier, &s, Role::Direction));
        assert!(!domaine_visible(DomaineChrono::Dossier, &sections(false), Role::Prof));
    }

    fn resp(foyer: u8, email: bool, autorite: bool) -> Responsable {
        Responsable {
            lien_id: Uuid::from_u128(u128::from(foyer) + 100),
            eleve_id: Uuid::nil(),
            personne: Personne {
                id: Uuid::from_u128(u128::from(foyer)),
                nom: "Durand".into(),
                prenom: Some("Alex".into()),
                civilite: None,
                email: email.then(|| "a@exemple.fr".into()),
                telephone: None,
                telephone_2: None,
                adresse: None,
                user_id: None,
                eleve_id: None,
                fusionnee_dans: None,
                source: "saisie".into(),
            },
            foyer_id: Some(Uuid::from_u128(u128::from(foyer) + 1000)),
            foyer_libelle: None,
            lien: "père".into(),
            autorite_parentale: autorite,
            garde: None,
            date_debut: None,
            date_fin: None,
            suspendu_le: None,
            suspension_motif: None,
            suspension_reference: None,
            actif: true,
            peut_voir_notes: true,
            peut_voir_vs: true,
            peut_justifier_absence: true,
            peut_autoriser_sorties: false,
            est_payeur: false,
            part_paiement: None,
        }
    }

    #[test]
    fn alertes_parents_separes_sans_email() {
        assert!(alertes_fiche(&[resp(1, false, true)]).is_empty());
        let a = alertes_fiche(&[resp(1, false, true), resp(2, true, true)]);
        assert_eq!(a.len(), 1);
        assert!(a[0].contains("Parents séparés"));
        let a = alertes_fiche(&[resp(1, true, false)]);
        assert!(a[0].contains("autorité parentale"));
    }

    #[test]
    fn saisie_personne_normalisee() {
        let s = PersonneSaisie {
            nom: "  Martin ".into(),
            prenom: Some(" ".into()),
            email: Some(" Paul@Exemple.FR ".into()),
            telephone: Some("+33 6 12 34 56 78".into()),
            ..Default::default()
        }
        .normaliser()
        .unwrap();
        assert_eq!(s.nom, "Martin");
        assert_eq!(s.prenom, None);
        assert_eq!(s.email.as_deref(), Some("paul@exemple.fr"));
        assert!(PersonneSaisie { nom: " ".into(), ..Default::default() }.normaliser().is_err());
        assert!(PersonneSaisie {
            nom: "X".into(),
            email: Some("pas-une-adresse".into()),
            ..Default::default()
        }
        .normaliser()
        .is_err());
        assert!(PersonneSaisie {
            nom: "X".into(),
            telephone: Some("<script>".into()),
            ..Default::default()
        }
        .normaliser()
        .is_err());
    }
}
