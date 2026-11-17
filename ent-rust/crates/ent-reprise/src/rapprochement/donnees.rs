//! Données comparées par le rapprochement (lot R3) : ce que dit l'export (côté **source**, extrait du
//! décodage strict de R2) et ce que contient ScolIA (côté **cible**, instantané lu par `ent-db` avec le
//! filtre d'établissement). Aucune de ces structures ne sait lire une base : le rapprochement est une
//! fonction pure de ces deux entrées.

use crate::decodage::{Decodage, LigneAcceptee, TableChargee};
use chrono::NaiveDate;
use std::collections::{BTreeMap, BTreeSet};

// ───────────────────────────── Source (export Next.js) ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEleve {
    pub source_id: String,
    pub nom: String,
    pub prenom: String,
    pub ine: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub classe: Option<String>,
    pub date_sortie: Option<NaiveDate>,
    pub statut: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceResponsable {
    pub source_id: String,
    pub foyer_id: Option<String>,
    pub nom: String,
    pub prenom: String,
    pub email: Option<String>,
    pub telephone: Option<String>,
}

/// Personnel (`personnel`) ou copie d'enseignant (`enseignant`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePersonnel {
    pub objet: &'static str,
    pub source_id: String,
    pub nom: String,
    pub prenom: String,
    pub email_pro: Option<String>,
    pub email: Option<String>,
    pub email_perso: Option<String>,
    pub categorie: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCompte {
    pub source_id: String,
    pub email: Option<String>,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub administrateur_plateforme: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAnnee {
    pub source_id: String,
    pub libelle: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub courante: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSite {
    pub source_id: String,
    pub libelle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceClasse {
    pub source_id: String,
    pub nom: String,
    pub code_siecle: Option<String>,
}

/// Tout ce que le rapprochement lit de l'export, extrait du décodage.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceRapprochement {
    pub eleves: Vec<SourceEleve>,
    pub responsables: Vec<SourceResponsable>,
    pub personnels: Vec<SourcePersonnel>,
    pub comptes: Vec<SourceCompte>,
    pub annees: Vec<SourceAnnee>,
    pub sites: Vec<SourceSite>,
    pub classes: Vec<SourceClasse>,
    /// foyer source → élèves source (table `eleve_foyer_link`).
    pub foyers_eleves: BTreeMap<String, BTreeSet<String>>,
}

struct Acces<'a> {
    t: &'a TableChargee,
}

impl<'a> Acces<'a> {
    fn de(d: &'a Decodage, table: &str) -> Option<Acces<'a>> {
        d.tables.get(table).map(|t| Acces { t })
    }

    fn texte(&self, l: &LigneAcceptee, col: &str) -> Option<String> {
        let i = self.t.colonnes.iter().position(|c| c == col)?;
        l.cellules
            .get(i)?
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    fn jour(&self, l: &LigneAcceptee, col: &str) -> Option<NaiveDate> {
        // Cellule de type `date` déjà validée par le décodage strict ; une valeur impossible reste `None`.
        NaiveDate::parse_from_str(&self.texte(l, col)?, "%Y-%m-%d").ok()
    }

    fn booleen(&self, l: &LigneAcceptee, col: &str) -> bool {
        matches!(self.texte(l, col).as_deref(), Some("t" | "true" | "1"))
    }
}

impl SourceRapprochement {
    /// Extrait les fiches à rapprocher du décodage. Une ligne sans identifiant ou sans nom exploitable
    /// n'est pas devinée : elle est conservée avec un nom vide (donc jamais rapprochée sur le nom).
    pub fn extraire(d: &Decodage) -> SourceRapprochement {
        let mut s = SourceRapprochement::default();
        if let Some(a) = Acces::de(d, "eleve") {
            for l in &a.t.lignes {
                s.eleves.push(SourceEleve {
                    source_id: l.cle.clone(),
                    nom: a.texte(l, "nom").unwrap_or_default(),
                    prenom: a.texte(l, "prenom").unwrap_or_default(),
                    ine: a.texte(l, "ine"),
                    date_naissance: a.jour(l, "date_naissance"),
                    classe: a.texte(l, "classe"),
                    date_sortie: a.jour(l, "date_sortie"),
                    statut: a.texte(l, "status"),
                });
            }
        }
        if let Some(a) = Acces::de(d, "foyer_responsable") {
            for l in &a.t.lignes {
                s.responsables.push(SourceResponsable {
                    source_id: l.cle.clone(),
                    foyer_id: a.texte(l, "foyer_id"),
                    nom: a.texte(l, "nom").unwrap_or_default(),
                    prenom: a.texte(l, "prenom").unwrap_or_default(),
                    email: a.texte(l, "email"),
                    telephone: a.texte(l, "telephone"),
                });
            }
        }
        if let Some(a) = Acces::de(d, "personnel") {
            for l in &a.t.lignes {
                s.personnels.push(SourcePersonnel {
                    objet: "personnel",
                    source_id: l.cle.clone(),
                    nom: a.texte(l, "last_name").unwrap_or_default(),
                    prenom: a.texte(l, "first_name").unwrap_or_default(),
                    email_pro: a.texte(l, "email_pro"),
                    email: a.texte(l, "email"),
                    email_perso: a.texte(l, "email_perso"),
                    categorie: a.texte(l, "category"),
                });
            }
        }
        if let Some(a) = Acces::de(d, "enseignant") {
            for l in &a.t.lignes {
                s.personnels.push(SourcePersonnel {
                    objet: "enseignant",
                    source_id: l.cle.clone(),
                    nom: a.texte(l, "nom").unwrap_or_default(),
                    prenom: a.texte(l, "prenom").unwrap_or_default(),
                    email_pro: a.texte(l, "email_pro"),
                    email: a.texte(l, "email"),
                    email_perso: None,
                    categorie: None,
                });
            }
        }
        if let Some(a) = Acces::de(d, "user") {
            for l in &a.t.lignes {
                s.comptes.push(SourceCompte {
                    source_id: l.cle.clone(),
                    email: a.texte(l, "email"),
                    nom: a.texte(l, "last_name"),
                    prenom: a.texte(l, "first_name"),
                    administrateur_plateforme: a.booleen(l, "platform_admin"),
                });
            }
        }
        if let Some(a) = Acces::de(d, "annee_scolaire") {
            for l in &a.t.lignes {
                let (Some(debut), Some(fin)) = (a.jour(l, "starts_on"), a.jour(l, "ends_on"))
                else {
                    continue;
                };
                s.annees.push(SourceAnnee {
                    source_id: l.cle.clone(),
                    libelle: a.texte(l, "label").unwrap_or_default(),
                    debut,
                    fin,
                    courante: a.booleen(l, "is_current"),
                });
            }
        }
        if let Some(a) = Acces::de(d, "etablissement_site") {
            for l in &a.t.lignes {
                s.sites.push(SourceSite {
                    source_id: l.cle.clone(),
                    libelle: a.texte(l, "label").unwrap_or_default(),
                });
            }
        }
        if let Some(a) = Acces::de(d, "classe_site_mapping") {
            for l in &a.t.lignes {
                s.classes.push(SourceClasse {
                    source_id: l.cle.clone(),
                    nom: a.texte(l, "class_name").unwrap_or_default(),
                    code_siecle: a.texte(l, "siecle_code"),
                });
            }
        }
        if let Some(a) = Acces::de(d, "eleve_foyer_link") {
            for l in &a.t.lignes {
                if let (Some(e), Some(f)) = (a.texte(l, "eleve_id"), a.texte(l, "foyer_id")) {
                    s.foyers_eleves.entry(f).or_default().insert(e);
                }
            }
        }
        s
    }
}

// ───────────────────────────── Cible (ScolIA) ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CibleEleve {
    pub id: String,
    pub nom: String,
    pub nom_naissance: Option<String>,
    pub prenom: String,
    pub prenoms: Vec<String>,
    pub ine: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    pub classe: Option<String>,
    pub actif: bool,
}

/// Personne de ScolIA (responsable, contact) et les élèves auxquels elle est liée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiblePersonne {
    pub id: String,
    pub nom: String,
    pub prenom: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub eleves: Vec<String>,
}

/// Agent de ScolIA (personnel) : identifiant de l'agent, nom, adresses connues (fiche et compte).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CibleAgent {
    pub id: String,
    pub nom: String,
    pub prenom: Option<String>,
    pub emails: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CibleCompte {
    pub id: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CibleAnnee {
    pub id: String,
    pub libelle: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CibleSite {
    pub id: String,
    pub code: String,
    pub libelle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CibleClasse {
    pub id: String,
    pub code: String,
    pub libelle: String,
}

/// Équivalences de libellés **validées par une personne** : (domaine, clé de la valeur source) → cible.
/// Domaines : `classe`, `site`, `matiere`.
pub type Synonymes = BTreeMap<(String, String), String>;

/// Instantané de ScolIA lu pour le rapprochement (établissement cible seulement).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CibleRapprochement {
    pub eleves: Vec<CibleEleve>,
    pub personnes: Vec<CiblePersonne>,
    pub agents: Vec<CibleAgent>,
    pub comptes: Vec<CibleCompte>,
    pub annees: Vec<CibleAnnee>,
    pub sites: Vec<CibleSite>,
    /// Classes de l'année courante.
    pub classes: Vec<CibleClasse>,
    pub synonymes: Synonymes,
}
