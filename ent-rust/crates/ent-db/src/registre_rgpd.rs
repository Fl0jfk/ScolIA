//! **Registre des traitements généré** (socle 27, lot G2, `27_RGPD_ARCHIVES.md` §8.1, §14.1).
//!
//! Le logiciel **calcule** ce qu'il sait (catégories de personnes, tables et données, destinataires réels,
//! durées du référentiel, sous-traitants de la plateforme, état du module) et l'établissement **complète**
//! ce qu'il ne peut pas savoir (base légale, responsable, traitements hors ScolIA). Un registre **qui ne
//! ment pas** : si une donnée change après validation (nouveau rôle lecteur, nouvelle table de données,
//! sous-traitant, module activé), la fiche passe « à revoir » (R-RG10, RGP-108).
//!
//! * Génération : [`generer_fiche`] est une fonction **pure** (entrées explicites) ; la lecture en base
//!   ne fait que rassembler les entrées.
//! * Validation : une version figée par fiche (`rgp_traitement_version`), avec les **empreintes** SHA-256
//!   des facettes calculées. Aucune donnée de personne n'est copiée.
//! * Cloisonnement : tout passe par `TenantTx` ; rien n'est lu hors établissement.
//! * ScolIA ne s'affiche **jamais « conforme »** (R-RG9) : il décrit ses mesures et les choix de
//!   l'établissement.

use crate::conservation::{categorie, categorie_de_table, Categorie};
use crate::conservation_regles::{EtatAffichage, Politique, RegleVue};
use crate::registre_donnees::{ClasseTable, REGISTRE};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Utc};
use domain_eleve::DomainError;
use ent_auth::{modele_par_defaut, Action, Role};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

/// Un traitement de base proposé par ScolIA, relié aux domaines du registre des tables.
#[derive(Clone, Copy, Debug)]
pub struct TraitementBase {
    pub code: &'static str,
    pub libelle: &'static str,
    pub finalite: &'static str,
    /// Catégories de personnes concernées.
    pub personnes: &'static [&'static str],
    /// Domaines du registre des tables (`ClasseTable::domaine`).
    pub domaines: &'static [&'static str],
    /// Actions de lecture du domaine : celles qui font les **destinataires réels**.
    pub lecture: &'static [Action],
    /// Module à activer pour que le traitement existe (santé).
    pub module: Option<&'static str>,
}

use Action::*;

/// Traitements de base : tout domaine du registre des tables appartient à **un seul** traitement (test).
pub const TRAITEMENTS_BASE: &[TraitementBase] = &[
    TraitementBase {
        code: "scolarite_dossier",
        libelle: "Scolarité et dossier de l'élève",
        finalite: "Tenir le dossier de l'élève, sa scolarité, ses responsables et ses coordonnées, pour assurer l'instruction et le suivi.",
        personnes: &["élèves", "responsables légaux"],
        domaines: &["dossier_eleve", "dossier_personnes", "structure_scolaire"],
        lecture: &[DossierLire, ElevesLire, ResponsablesLire, ResponsablesAdressesLire],
        module: None,
    },
    TraitementBase {
        code: "inscriptions",
        libelle: "Inscriptions et réinscriptions",
        finalite: "Recueillir et traiter les demandes d'admission, d'inscription et de réinscription.",
        personnes: &["élèves", "responsables légaux", "candidats"],
        domaines: &["inscriptions"],
        lecture: &[DossierLire],
        module: None,
    },
    TraitementBase {
        code: "vie_scolaire_assiduite",
        libelle: "Assiduité et vie scolaire",
        finalite: "Suivre les présences, absences et retards, et leurs justificatifs.",
        personnes: &["élèves", "responsables légaux"],
        domaines: &["assiduite"],
        lecture: &[AbsencesLire, AbsencesJustificatifLire],
        module: None,
    },
    TraitementBase {
        code: "discipline",
        libelle: "Discipline : incidents, punitions et sanctions",
        finalite: "Gérer les incidents, les punitions, les sanctions et les procédures disciplinaires.",
        personnes: &["élèves", "responsables légaux", "personnels"],
        domaines: &["vie_scolaire"],
        lecture: &[DisciplineGerer, DossierLire],
        module: None,
    },
    TraitementBase {
        code: "evaluations_bulletins",
        libelle: "Évaluations, bulletins et orientation",
        finalite: "Saisir les notes et compétences, éditer les bulletins et préparer l'orientation.",
        personnes: &["élèves", "responsables légaux", "enseignants"],
        domaines: &["evaluations_pedagogie"],
        lecture: &[NotesLire],
        module: None,
    },
    TraitementBase {
        code: "cahier_textes",
        libelle: "Cahier de textes et travail à faire",
        finalite: "Publier le contenu des séances, le travail à faire et les remises.",
        personnes: &["élèves", "responsables légaux", "enseignants"],
        domaines: &["cahier_textes"],
        lecture: &[CahierLire],
        module: None,
    },
    TraitementBase {
        code: "communication",
        libelle: "Messagerie et communication",
        finalite: "Échanger avec les familles et les élèves, notifier et informer.",
        personnes: &["élèves", "responsables légaux", "personnels"],
        domaines: &["messagerie", "communication_technique"],
        lecture: &[MessagerieEleve, MessagerieFilsReserves],
        module: None,
    },
    TraitementBase {
        code: "finances_familles",
        libelle: "Finances des familles",
        finalite: "Facturer, encaisser et suivre les paiements et prélèvements des familles, et les bourses.",
        personnes: &["responsables légaux", "élèves"],
        domaines: &["finances"],
        lecture: &[FacturationLire],
        module: None,
    },
    TraitementBase {
        code: "restauration_internat",
        libelle: "Restauration et internat",
        finalite: "Organiser la restauration, l'internat et les présences associées.",
        personnes: &["élèves", "responsables légaux"],
        domaines: &["vie_pratique"],
        lecture: &[DossierLire, AbsencesLire],
        module: None,
    },
    TraitementBase {
        code: "sorties_voyages",
        libelle: "Sorties, voyages et stages",
        finalite: "Organiser les sorties et voyages scolaires et les stages en entreprise.",
        personnes: &["élèves", "responsables légaux", "personnels", "prestataires"],
        domaines: &["sorties_voyages", "stages"],
        lecture: &[VoyagesLire, StagesLire],
        module: None,
    },
    TraitementBase {
        code: "sante",
        libelle: "Santé : PAI, infirmerie, psychologue (module désactivé par défaut)",
        finalite: "Suivre les projets d'accueil individualisés, les passages à l'infirmerie et l'accompagnement psychologique.",
        personnes: &["élèves", "responsables légaux"],
        domaines: &["sante"],
        lecture: &[SanteLire, PsyLire, SanteUrgenceLire],
        module: Some("sante"),
    },
    TraitementBase {
        code: "documents",
        libelle: "Documents, certificats et attestations",
        finalite: "Émettre, conserver et mettre à disposition les documents officiels (certificats, attestations, relevés) et leurs exemplaires ; faire signer ces documents (circuits, preuve chaînée, dossier de preuve) et en garder la preuve.",
        personnes: &["élèves", "responsables légaux", "personnels"],
        domaines: &["documents"],
        lecture: &[DocumentsLire, DocumentsRegistreLire, DocumentsEmettre, DocumentsSignaturesGerer],
        module: None,
    },
    TraitementBase {
        code: "personnels",
        libelle: "Personnels et absences des personnels",
        finalite: "Tenir l'annuaire des personnels, leurs missions, leurs absences et les remplacements.",
        personnes: &["personnels"],
        domaines: &["personnels", "personnels_absences"],
        lecture: &[PersonnelsPersonnelLire, PersonnelsLire, PersonnelsAbsencesLireOrganisation],
        module: None,
    },
    TraitementBase {
        code: "organisation_pilotage",
        libelle: "Organisation : emploi du temps, agenda, demandes et pilotage",
        finalite: "Planifier les cours, les rendez-vous et les salles, traiter les demandes internes et piloter l'établissement.",
        personnes: &["personnels", "élèves", "responsables légaux"],
        domaines: &["emploi_du_temps_structure", "portails", "demandes", "etablissement", "referentiel", "plateforme", "autre"],
        lecture: &[AgendaLireStructure, EdtGerer, DemandesLireToutes],
        module: None,
    },
    TraitementBase {
        code: "comptes_securite",
        libelle: "Comptes, accès et sécurité",
        finalite: "Authentifier les utilisateurs, gérer les rôles et délégations, et assurer la sécurité du service.",
        personnes: &["personnels", "responsables légaux", "élèves"],
        domaines: &["identite_acces"],
        lecture: &[PersonnelsAccesGerer],
        module: None,
    },
    TraitementBase {
        code: "reprise_mise_en_service",
        libelle: "Reprise des données de l'ancien outil à la mise en service",
        finalite: "Reprendre les données de l'ancien outil (Next.js) à la mise en service de l'établissement : décoder l'export, simuler la reprise, tracer les comptes, les refus et la correspondance des identifiants. Aucune donnée de santé n'est reprise. Les données de la zone de transit sont purgées après la reprise ; seuls les comptes, les refus sans donnée personnelle et la correspondance des identifiants sont conservés comme preuve. Destinataire : l'opérateur ScolIA seulement, jamais un utilisateur de l'ENT.",
        personnes: &["élèves", "responsables légaux", "personnels"],
        domaines: &["mise_en_service"],
        // Aucune action de lecture applicative : la zone n'est lisible que par l'outil de reprise (rôle `ent_reprise`).
        lecture: &[],
        module: None,
    },
    TraitementBase {
        code: "conformite",
        libelle: "Conformité : registre, conservation et décisions",
        finalite: "Tenir le registre des traitements, les règles de conservation et le journal des décisions de conformité.",
        personnes: &["personnels"],
        domaines: &["conformite"],
        lecture: &[ConformiteLire],
        module: None,
    },
];

/// Rôles de base du logiciel (le catalogue d'actions donne leur modèle par défaut).
const ROLES_DE_BASE: [Role; 12] = [
    Role::Admin,
    Role::Direction,
    Role::Administratif,
    Role::Cpe,
    Role::Compta,
    Role::Prof,
    Role::Infirmerie,
    Role::Psy,
    Role::Accueil,
    Role::Personnel,
    Role::Parent,
    Role::Eleve,
];

/// Mesures de sécurité de la plateforme (texte de la plateforme, identique pour tous les traitements).
pub const MESURES_SECURITE: &[&str] = &[
    "Cloisonnement strict par établissement (isolation en base de données).",
    "Droits contrôlés côté serveur ; moindre privilège ; rôles paramétrables.",
    "Mots de passe hachés (Argon2id) ; clé d'accès (passkey) ; sessions révocables.",
    "Journal des modifications en ajout seul ; consultations sensibles tracées avant d'être servies.",
    "Fichiers privés servis par liens pré-signés de courte durée après contrôle des droits.",
];

/// Transferts hors Union européenne : aucun déclaré par défaut (à confirmer par l'éditeur, VERIF R59).
pub const TRANSFERTS_HORS_UE: &str = "Aucun transfert hors Union européenne déclaré.";

/// Documents de conformité suivis par établissement (catalogue minimal de G2 ; le reste vient avec G9).
pub const DOCUMENTS: &[(&str, &str)] = &[
    ("registre", "Registre des traitements (version validée)"),
    ("politique_conservation", "Politique de conservation"),
    ("mentions_information", "Mentions d'information"),
    (
        "procedure_droits",
        "Procédure de traitement des demandes de droits",
    ),
    (
        "procedure_violation",
        "Procédure de gestion des violations de données",
    ),
    ("aipd", "Analyse d'impact (si requise)"),
];

/// Ce que la plateforme sait d'un sous-traitant, pour une fiche.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SousTraitantFiche {
    pub nom: String,
    pub service: String,
    pub pays: String,
    pub categories: Vec<String>,
}

/// Entrées de la génération d'une fiche (tout est explicite : fonction pure).
#[derive(Clone, Debug, Default)]
pub struct EntreesFiche {
    /// Noms des rôles paramétrés de l'établissement qui portent une action de lecture du domaine.
    pub roles_etablissement: BTreeSet<String>,
    /// Destinataires externes déclarés (libellé, nature).
    pub externes: BTreeSet<(String, String)>,
    pub soustraitants: Vec<SousTraitantFiche>,
    /// Règles de conservation, par catégorie (durées et état).
    pub regles: BTreeMap<String, RegleVue>,
    pub module_actif: bool,
}

/// Fiche générée d'un traitement de base.
#[derive(Clone, Debug, Serialize)]
pub struct FicheGeneree {
    pub code: String,
    pub libelle: String,
    pub finalite: String,
    pub personnes: Vec<String>,
    pub domaines: Vec<String>,
    pub categories_donnees: Vec<String>,
    pub nb_tables_donnees_personnelles: usize,
    pub destinataires_internes: Vec<String>,
    pub destinataires_externes: Vec<String>,
    pub soustraitants: Vec<String>,
    pub durees: Vec<LigneDuree>,
    pub transferts_hors_ue: String,
    pub mesures_securite: Vec<String>,
    /// Le module requis est-il actif ? (sans module : le traitement n'existe pas encore)
    pub actif: bool,
    #[serde(skip)]
    pub facettes: Facettes,
}

#[derive(Clone, Debug, Serialize)]
pub struct LigneDuree {
    pub categorie: String,
    pub libelle: String,
    pub utilite_mois: Option<i32>,
    pub intermediaire_mois: Option<i32>,
    pub etat: EtatAffichage,
}

/// Empreintes des facettes : un écart se détecte en comparant ces cinq valeurs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facettes {
    pub destinataires: String,
    pub donnees: String,
    pub soustraitants: String,
    pub durees: String,
    pub activation: String,
}

fn empreinte(lignes: impl IntoIterator<Item = String>) -> String {
    let mut h = Sha256::new();
    for l in lignes {
        h.update(l.as_bytes());
        h.update([0u8]);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn tables_du_traitement(base: &TraitementBase) -> impl Iterator<Item = &'static ClasseTable> + '_ {
    REGISTRE
        .iter()
        .filter(move |t| base.domaines.contains(&t.domaine))
}

/// Destinataires **internes** réels : rôles de base dont le modèle par défaut porte une action de lecture du
/// domaine, plus les rôles de l'établissement qui la portent (la liste réelle, pas la théorie).
fn destinataires_internes(
    base: &TraitementBase,
    roles_etab: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for r in ROLES_DE_BASE {
        let m = modele_par_defaut(r);
        if base.lecture.iter().any(|a| m.iter().any(|(x, _)| x == a)) {
            out.insert(format!("rôle {}", r.as_str()));
        }
    }
    for n in roles_etab {
        out.insert(format!("rôle de l'établissement : {n}"));
    }
    out
}

/// Génère la fiche d'un traitement de base à partir des entrées. Fonction pure.
pub fn generer_fiche(base: &TraitementBase, e: &EntreesFiche) -> FicheGeneree {
    let tables: Vec<&ClasseTable> = tables_du_traitement(base).collect();
    let perso: Vec<&&ClasseTable> = tables.iter().filter(|t| t.donnees_personnelles).collect();
    let mut categories_donnees = BTreeSet::new();
    if !perso.is_empty() {
        categories_donnees.insert("Données personnelles courantes".to_string());
    }
    if tables.iter().any(|t| t.sante) {
        categories_donnees.insert("Données de santé (catégorie particulière)".to_string());
    }
    if tables
        .iter()
        .any(|t| t.fichiers != crate::registre_donnees::Fichiers::Aucun)
    {
        categories_donnees.insert("Fichiers et pièces jointes".to_string());
    }
    let internes = destinataires_internes(base, &e.roles_etablissement);
    let externes: BTreeSet<String> = e
        .externes
        .iter()
        .map(|(l, n)| format!("{l} ({n})"))
        .collect();
    // Catégories de conservation des tables du traitement.
    let codes: BTreeSet<&'static str> = tables.iter().map(|t| categorie_de_table(t)).collect();
    let mut durees = Vec::new();
    for code in &codes {
        let Some(c): Option<&Categorie> = categorie(code) else {
            continue;
        };
        let (u, i, etat) = match e.regles.get(*code) {
            Some(r) => (r.utilite_mois, r.intermediaire_mois, r.etat),
            None => (
                c.utilite_mois.map(i32::from),
                c.intermediaire_mois.map(i32::from),
                EtatAffichage::EnAttenteDpo,
            ),
        };
        durees.push(LigneDuree {
            categorie: c.code.to_string(),
            libelle: c.libelle.to_string(),
            utilite_mois: u,
            intermediaire_mois: i,
            etat,
        });
    }
    let st: BTreeSet<String> = e
        .soustraitants
        .iter()
        .filter(|s| s.categories.iter().any(|c| codes.contains(c.as_str())))
        .map(|s| format!("{} — {} ({})", s.nom, s.service, s.pays))
        .collect();
    let facettes = Facettes {
        destinataires: empreinte(internes.iter().cloned().chain(externes.iter().cloned())),
        donnees: empreinte(
            perso.iter().map(|t| t.nom.to_string()).chain(
                tables
                    .iter()
                    .filter(|t| t.sante)
                    .map(|t| format!("sante:{}", t.nom)),
            ),
        ),
        soustraitants: empreinte(st.iter().cloned()),
        // Les durées (pas leur état de validation, qui change sans changer le traitement) : voir R-RG10.
        durees: empreinte(durees.iter().map(|d| {
            format!(
                "{}|{:?}|{:?}",
                d.categorie, d.utilite_mois, d.intermediaire_mois
            )
        })),
        activation: empreinte([e.module_actif.to_string()]),
    };
    FicheGeneree {
        code: base.code.to_string(),
        libelle: base.libelle.to_string(),
        finalite: base.finalite.to_string(),
        personnes: base.personnes.iter().map(|s| s.to_string()).collect(),
        domaines: base.domaines.iter().map(|s| s.to_string()).collect(),
        categories_donnees: categories_donnees.into_iter().collect(),
        nb_tables_donnees_personnelles: perso.len(),
        destinataires_internes: internes.into_iter().collect(),
        destinataires_externes: externes.into_iter().collect(),
        soustraitants: st.into_iter().collect(),
        durees,
        transferts_hors_ue: TRANSFERTS_HORS_UE.to_string(),
        mesures_securite: MESURES_SECURITE.iter().map(|s| s.to_string()).collect(),
        actif: base.module.is_none() || e.module_actif,
        facettes,
    }
}

/// Nature d'écart pour chaque facette qui diffère de la version validée.
pub fn ecarts_entre(validee: &Facettes, courante: &Facettes) -> Vec<&'static str> {
    let mut v = Vec::new();
    if validee.destinataires != courante.destinataires {
        v.push("destinataire_ajoute");
    }
    if validee.donnees != courante.donnees {
        v.push("donnees_changees");
    }
    if validee.soustraitants != courante.soustraitants {
        v.push("soustraitant_change");
    }
    if validee.durees != courante.durees {
        v.push("categorie_changee");
    }
    if validee.activation != courante.activation {
        v.push("module_active");
    }
    v
}

// ---------------------------------------------------------------------------------------------
// Lecture en base
// ---------------------------------------------------------------------------------------------

/// Fiche telle que l'établissement la voit : générée + ce que l'établissement a complété.
#[derive(Clone, Debug, Serialize)]
pub struct FicheRegistre {
    #[serde(flatten)]
    pub fiche: FicheGeneree,
    /// `non_initialise`, `a_completer`, `complete`, `a_revoir` ou `valide`.
    pub etat: String,
    pub base_legale: String,
    pub base_legale_motif: Option<String>,
    pub responsable: String,
    pub responsable_libelle: Option<String>,
    pub version_validee: Option<i32>,
    pub valide_le: Option<DateTime<Utc>>,
    pub ecarts_ouverts: Vec<String>,
}

/// Traitement hors ScolIA, déclaratif (Charlemagne, papier, messagerie académique…) : sans donnée.
#[derive(Clone, Debug, Serialize)]
pub struct TraitementLocal {
    pub code: String,
    pub libelle: String,
    pub finalite: String,
    pub base_legale: String,
    pub etat: String,
}

/// Le registre complet.
#[derive(Clone, Debug, Serialize)]
pub struct Registre {
    pub fiches: Vec<FicheRegistre>,
    pub traitements_hors_scolia: Vec<TraitementLocal>,
    pub nb_valides: usize,
    pub nb_a_completer: usize,
    pub nb_a_revoir: usize,
    pub synthese: String,
}

pub(crate) struct LigneTraitement {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub finalite: String,
    pub base_legale: String,
    pub base_legale_motif: Option<String>,
    pub responsable: String,
    pub responsable_libelle: Option<String>,
    pub hors_scolia: bool,
    pub etat: String,
}

impl LigneTraitement {
    fn depuis(r: &sqlx::postgres::PgRow) -> Self {
        Self {
            id: r.get("id"),
            code: r.get("code"),
            libelle: r.get("libelle"),
            finalite: r.get("finalite"),
            base_legale: r.get("base_legale"),
            base_legale_motif: r.get("base_legale_motif"),
            responsable: r.get("responsable"),
            responsable_libelle: r.get("responsable_libelle"),
            hors_scolia: r.get("hors_scolia"),
            etat: r.get("etat"),
        }
    }
}

const COLONNES_TRAITEMENT: &str =
    "id, code, libelle, finalite, base_legale, base_legale_motif, responsable, \
     responsable_libelle, hors_scolia, etat";

pub(crate) async fn traitements_de(
    c: &mut PgConnection,
    etab: Uuid,
) -> Result<Vec<LigneTraitement>, EntDbError> {
    let rows = sqlx::query(&format!(
        "SELECT {COLONNES_TRAITEMENT} FROM rgp_traitement WHERE etablissement_id = $1 ORDER BY code"
    ))
    .bind(etab)
    .fetch_all(&mut *c)
    .await
    .map_err(db)?;
    Ok(rows.iter().map(LigneTraitement::depuis).collect())
}

/// Rassemble les entrées de génération de **tous** les traitements de base d'un établissement.
async fn entrees(
    c: &mut PgConnection,
    etab: Uuid,
    politique: &Politique,
    module_sante: bool,
    soustraitants: Vec<SousTraitantFiche>,
    externes: &BTreeMap<Uuid, BTreeSet<(String, String)>>,
    traitements: &[LigneTraitement],
) -> Result<BTreeMap<&'static str, EntreesFiche>, EntDbError> {
    // Rôles de l'établissement et leurs actions.
    let rows = sqlx::query(
        "SELECT r.nom, d.action FROM ent_role r JOIN ent_role_droit d
           ON d.role_id = r.id AND d.etablissement_id = r.etablissement_id
          WHERE r.etablissement_id = $1",
    )
    .bind(etab)
    .fetch_all(&mut *c)
    .await
    .map_err(db)?;
    let mut roles_par_action: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in rows {
        roles_par_action
            .entry(r.get::<String, _>("action"))
            .or_default()
            .insert(r.get::<String, _>("nom"));
    }
    let regles: BTreeMap<String, RegleVue> = politique
        .regles
        .iter()
        .map(|r| (r.categorie.clone(), r.clone()))
        .collect();
    let mut out = BTreeMap::new();
    for b in TRAITEMENTS_BASE {
        let mut roles = BTreeSet::new();
        for a in b.lecture {
            if let Some(ns) = roles_par_action.get(a.code()) {
                roles.extend(ns.iter().cloned());
            }
        }
        let ext = traitements
            .iter()
            .find(|t| t.code == b.code)
            .and_then(|t| externes.get(&t.id))
            .cloned()
            .unwrap_or_default();
        out.insert(
            b.code,
            EntreesFiche {
                roles_etablissement: roles,
                externes: ext,
                soustraitants: soustraitants.clone(),
                regles: regles.clone(),
                module_actif: b.module.is_none() || module_sante,
            },
        );
    }
    Ok(out)
}

async fn externes_de(
    c: &mut PgConnection,
    etab: Uuid,
) -> Result<BTreeMap<Uuid, BTreeSet<(String, String)>>, EntDbError> {
    let rows = sqlx::query("SELECT traitement_id, libelle, nature FROM rgp_destinataire_externe WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_all(&mut *c)
        .await
        .map_err(db)?;
    let mut m: BTreeMap<Uuid, BTreeSet<(String, String)>> = BTreeMap::new();
    for r in rows {
        m.entry(r.get("traitement_id"))
            .or_default()
            .insert((r.get("libelle"), r.get("nature")));
    }
    Ok(m)
}

/// Sous-traitants actifs de la plateforme avec les catégories de données qu'ils traitent.
pub(crate) async fn soustraitants_plateforme(
    c: &mut PgConnection,
    aujourdhui: chrono::NaiveDate,
) -> Result<Vec<SousTraitantFiche>, EntDbError> {
    let rows = sqlx::query(
        "SELECT s.id, s.nom, s.service, s.pays,
                COALESCE((SELECT array_agg(d.categorie_code ORDER BY d.categorie_code)
                            FROM rgp_soustraitant_donnee d WHERE d.soustraitant_id = s.id), ARRAY[]::text[]) AS cats
           FROM rgp_soustraitant s
          WHERE s.entree_le <= $1 AND (s.sortie_le IS NULL OR s.sortie_le >= $1)
          ORDER BY s.nom, s.service",
    )
    .bind(aujourdhui)
    .fetch_all(&mut *c)
    .await
    .map_err(db)?;
    Ok(rows
        .iter()
        .map(|r| SousTraitantFiche {
            nom: r.get("nom"),
            service: r.get("service"),
            pays: r.get("pays"),
            categories: r.get("cats"),
        })
        .collect())
}

/// Tout ce qu'il faut pour générer, lu dans une seule transaction.
struct Contexte {
    traitements: Vec<LigneTraitement>,
    entrees: BTreeMap<&'static str, EntreesFiche>,
    politique: Politique,
}

impl PgEnt {
    async fn contexte_rgpd(
        &self,
        c: &mut PgConnection,
        etab: Uuid,
    ) -> Result<Contexte, EntDbError> {
        let politique = self.politique_conservation_tx(c, etab).await?;
        let traitements = traitements_de(c, etab).await?;
        let externes = externes_de(c, etab).await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let st = soustraitants_plateforme(c, jour).await?;
        let module = self.module_sante_actif(etab).await?;
        let entrees = entrees(c, etab, &politique, module, st, &externes, &traitements).await?;
        Ok(Contexte {
            traitements,
            entrees,
            politique,
        })
    }

    /// Initialise le registre : une fiche « à compléter » par traitement de base (RGP-103). Idempotent.
    /// Droit `conformite.gerer`. Renvoie le nombre de fiches créées.
    pub async fn initialiser_registre(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<usize, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let mut crees = 0;
        for b in TRAITEMENTS_BASE {
            let r = sqlx::query(
                "INSERT INTO rgp_traitement (id, etablissement_id, code, libelle, finalite, aipd_requise)
                 VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (etablissement_id, code) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(b.code)
            .bind(b.libelle)
            .bind(b.finalite)
            .bind(b.module.is_some())
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            crees += r.rows_affected() as usize;
        }
        tx.commit().await?;
        Ok(crees)
    }

    /// Le registre **généré** (RGP-104), avec ce que l'établissement a complété. Droit `conformite.lire`.
    /// Une fiche non encore initialisée est annoncée `non_initialise` (jamais présentée comme remplie).
    pub async fn registre_genere(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Registre, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ctx = self.contexte_rgpd(&mut tx, etab).await?;
        let versions = versions_courantes(&mut tx, etab).await?;
        let ecarts = ecarts_ouverts_par_traitement(&mut tx, etab).await?;
        Ok(assembler_registre(&ctx, &versions, &ecarts))
    }

    /// Complète une fiche : base légale retenue, responsable (RGP-102, RGP-105). Droit `conformite.gerer`.
    /// Un motif est exigé pour s'écarter de la base proposée (mission d'intérêt public). Une fiche déjà
    /// validée repasse « complète » : elle devra être validée de nouveau.
    #[allow(clippy::too_many_arguments)]
    pub async fn completer_fiche(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        code: &str,
        base_legale: &str,
        motif: Option<&str>,
        responsable: &str,
        responsable_libelle: Option<&str>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        if !matches!(
            base_legale,
            "mission_interet_public"
                | "obligation_legale"
                | "contrat"
                | "consentement"
                | "interet_legitime"
                | "sauvegarde_interets_vitaux"
        ) {
            return Err(invalide("base légale inconnue"));
        }
        if !matches!(responsable, "etablissement" | "ogec" | "autre") {
            return Err(invalide("responsable de traitement inconnu"));
        }
        let motif = motif.map(str::trim).filter(|m| !m.is_empty());
        if base_legale != "mission_interet_public" && motif.is_none() {
            return Err(invalide(
                "un motif est exigé pour une base légale différente de la base proposée",
            ));
        }
        if motif.is_some_and(|m| {
            m.chars().count() < 3
                || m.chars().count() > 500
                || m.chars().any(|c| c.is_control() && c != '\n')
        }) {
            return Err(invalide("motif invalide"));
        }
        let resp_lib = responsable_libelle.map(str::trim).filter(|m| !m.is_empty());
        if responsable != "etablissement" && resp_lib.is_none() {
            return Err(invalide("nom du responsable exigé"));
        }
        if resp_lib.is_some_and(|m| m.chars().count() > 200 || m.chars().any(char::is_control)) {
            return Err(invalide("nom du responsable invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let n = sqlx::query(
            "UPDATE rgp_traitement SET base_legale = $3, base_legale_motif = $4, responsable = $5, responsable_libelle = $6,
                    etat = 'complete', modifie_le = now()
              WHERE etablissement_id = $1 AND code = $2",
        )
        .bind(etab)
        .bind(code)
        .bind(base_legale)
        .bind(motif)
        .bind(responsable)
        .bind(resp_lib)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        tx.commit().await?;
        Ok(())
    }

    /// Ajoute un traitement **hors ScolIA** (Charlemagne, papier, messagerie académique : RGP-106). Déclaratif,
    /// sans donnée. Droit `conformite.gerer`. Le code est dérivé du libellé.
    pub async fn ajouter_traitement_local(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        libelle: &str,
        finalite: &str,
    ) -> Result<String, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        let libelle = libelle.trim();
        let finalite = finalite.trim();
        if libelle.is_empty()
            || libelle.chars().count() > 200
            || libelle.chars().any(char::is_control)
        {
            return Err(invalide("libellé invalide"));
        }
        if finalite.is_empty()
            || finalite.chars().count() > 1000
            || finalite.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err(invalide("finalité invalide"));
        }
        let slug: String = libelle
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>()
            .split('_')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("_");
        let slug: String = slug.chars().take(50).collect();
        if slug.len() < 2 {
            return Err(invalide("libellé invalide"));
        }
        let code = format!("local_{slug}");
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "INSERT INTO rgp_traitement (id, etablissement_id, code, libelle, finalite, hors_scolia)
             VALUES ($1, $2, $3, $4, $5, true) ON CONFLICT (etablissement_id, code) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(&code)
        .bind(libelle)
        .bind(finalite)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier(
                "ce traitement hors ScolIA existe déjà",
            ));
        }
        tx.commit().await?;
        Ok(code)
    }

    /// Déclare un destinataire externe d'un traitement (administration, partenaire) : RGP-106 et §8.1.
    /// Droit `conformite.gerer`. Change la facette « destinataires » : une fiche validée passera « à revoir ».
    pub async fn declarer_destinataire_externe(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        code_traitement: &str,
        libelle: &str,
        nature: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        let libelle = libelle.trim();
        if libelle.is_empty()
            || libelle.chars().count() > 200
            || libelle.chars().any(char::is_control)
        {
            return Err(invalide("libellé invalide"));
        }
        if !matches!(nature, "administration" | "partenaire") {
            return Err(invalide("nature de destinataire inconnue"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM rgp_traitement WHERE etablissement_id = $1 AND code = $2",
        )
        .bind(etab)
        .bind(code_traitement)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let id = id.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        sqlx::query(
            "INSERT INTO rgp_destinataire_externe (id, etablissement_id, traitement_id, libelle, nature)
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (etablissement_id, traitement_id, libelle) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(libelle)
        .bind(nature)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(())
    }

    /// Valide une **version du registre** (RGP-107) : toutes les fiches actives doivent être complétées. Droit
    /// `conformite.valider`. Atomique : une fiche à compléter ou à revoir refuse toute la validation. Fige, par
    /// fiche, les empreintes des facettes ; clôt les écarts ouverts. Renvoie le nombre de fiches validées.
    pub async fn valider_registre(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<usize, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteValider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ctx = self.contexte_rgpd(&mut tx, etab).await?;
        if ctx.traitements.is_empty() {
            return Err(EntDbError::ConflitDossier(
                "le registre n'est pas initialisé",
            ));
        }
        // Une fiche déjà validée dont une facette a changé doit d'abord être revue : détecter avant.
        let versions = versions_courantes(&mut tx, etab).await?;
        for t in ctx
            .traitements
            .iter()
            .filter(|t| !t.hors_scolia && t.etat == "valide")
        {
            if let (Some(v), Some(e)) = (versions.get(&t.id), ctx.entrees.get(t.code.as_str())) {
                if let Some(b) = TRAITEMENTS_BASE.iter().find(|b| b.code == t.code) {
                    if !ecarts_entre(&v.facettes, &generer_fiche(b, e).facettes).is_empty() {
                        return Err(EntDbError::ConflitDossierDetail(format!(
                            "la fiche « {} » a changé depuis sa validation : détecter les écarts puis la compléter de nouveau",
                            t.code
                        )));
                    }
                }
            }
        }
        let mut non_pretes = Vec::new();
        let mut a_valider = Vec::new();
        for t in &ctx.traitements {
            let base = TRAITEMENTS_BASE.iter().find(|b| b.code == t.code);
            if let Some(b) = base {
                if let Some(e) = ctx.entrees.get(b.code) {
                    if !generer_fiche(b, e).actif {
                        continue; // module désactivé : le traitement n'existe pas encore
                    }
                }
            }
            match t.etat.as_str() {
                "complete" => a_valider.push(t),
                "valide" => {}
                _ => non_pretes.push(t.code.clone()),
            }
        }
        if !non_pretes.is_empty() {
            return Err(EntDbError::ConflitDossierDetail(format!(
                "fiches à compléter ou à revoir avant validation : {}",
                non_pretes.join(", ")
            )));
        }
        for t in &a_valider {
            let facettes = match TRAITEMENTS_BASE.iter().find(|b| b.code == t.code) {
                Some(b) => generer_fiche(b, ctx.entrees.get(b.code).expect("entrées")).facettes,
                // Traitement hors ScolIA : aucune facette calculable, empreintes constantes.
                None => Facettes {
                    destinataires: empreinte(["local".into()]),
                    donnees: empreinte(["local".into()]),
                    soustraitants: empreinte(["local".into()]),
                    durees: empreinte(["local".into()]),
                    activation: empreinte(["local".into()]),
                },
            };
            let version: i32 = sqlx::query_scalar(
                "SELECT COALESCE(max(version), 0) + 1 FROM rgp_traitement_version WHERE traitement_id = $1",
            )
            .bind(t.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO rgp_traitement_version (id, etablissement_id, traitement_id, version, empreinte_destinataires,
                    empreinte_donnees, empreinte_soustraitants, empreinte_durees, empreinte_activation, valide_par_compte_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(t.id)
            .bind(version)
            .bind(&facettes.destinataires)
            .bind(&facettes.donnees)
            .bind(&facettes.soustraitants)
            .bind(&facettes.durees)
            .bind(&facettes.activation)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query("UPDATE rgp_traitement SET etat = 'valide', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(t.id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            sqlx::query(
                "UPDATE rgp_ecart SET traite_le = now(), traite_par_compte_id = $3
                  WHERE etablissement_id = $1 AND traitement_id = $2 AND traite_le IS NULL",
            )
            .bind(etab)
            .bind(t.id)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let n = a_valider.len();
        tx.commit().await?;
        Ok(n)
    }

    /// Détecte les écarts (RGP-108, R-RG10) : toute fiche **validée** dont une facette calculée diffère de sa
    /// dernière version validée passe « à revoir » et reçoit un écart. Idempotent. Droit `conformite.lire`.
    /// Renvoie les écarts ouverts (code du traitement, nature).
    pub async fn detecter_ecarts(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<(String, String)>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let ctx = self.contexte_rgpd(&mut tx, etab).await?;
        let versions = versions_courantes(&mut tx, etab).await?;
        for t in ctx
            .traitements
            .iter()
            .filter(|t| !t.hors_scolia && t.etat == "valide")
        {
            let (Some(v), Some(b), Some(e)) = (
                versions.get(&t.id),
                TRAITEMENTS_BASE.iter().find(|b| b.code == t.code),
                ctx.entrees.get(t.code.as_str()),
            ) else {
                continue;
            };
            let natures = ecarts_entre(&v.facettes, &generer_fiche(b, e).facettes);
            if natures.is_empty() {
                continue;
            }
            for n in natures {
                sqlx::query(
                    "INSERT INTO rgp_ecart (id, etablissement_id, traitement_id, nature) VALUES ($1, $2, $3, $4)
                     ON CONFLICT DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(t.id)
                .bind(n)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            sqlx::query("UPDATE rgp_traitement SET etat = 'a_revoir', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(t.id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let rows = sqlx::query(
            "SELECT t.code, e.nature FROM rgp_ecart e JOIN rgp_traitement t
               ON t.etablissement_id = e.etablissement_id AND t.id = e.traitement_id
              WHERE e.etablissement_id = $1 AND e.traite_le IS NULL ORDER BY t.code, e.nature",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("code"), r.get("nature")))
            .collect())
    }

    /// Marque l'état d'un document de conformité (RGP-120 et suivants). Droit `conformite.gerer`. Un document
    /// « sans objet » exige sa raison.
    pub async fn marquer_document(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        code: &str,
        etat: &str,
        raison: Option<&str>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        if !DOCUMENTS.iter().any(|(c, _)| *c == code) {
            return Err(invalide("document inconnu"));
        }
        if !matches!(
            etat,
            "manquant" | "brouillon" | "genere" | "importe" | "valide" | "sans_objet"
        ) {
            return Err(invalide("état de document inconnu"));
        }
        let raison = raison.map(str::trim).filter(|r| !r.is_empty());
        if (etat == "sans_objet") != raison.is_some() {
            return Err(invalide(
                "la raison est exigée pour « sans objet » et réservée à cet état",
            ));
        }
        if raison.is_some_and(|r| {
            r.chars().count() < 3
                || r.chars().count() > 500
                || r.chars().any(|c| c.is_control() && c != '\n')
        }) {
            return Err(invalide("raison invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO rgp_document (id, etablissement_id, code, etat, sans_objet_raison, modifie_par_compte_id)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (etablissement_id, code) DO UPDATE
                SET etat = EXCLUDED.etat, sans_objet_raison = EXCLUDED.sans_objet_raison,
                    modifie_par_compte_id = EXCLUDED.modifie_par_compte_id, modifie_le = now()",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(code)
        .bind(etat)
        .bind(raison)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(())
    }

    /// États des documents de conformité (`manquant` si jamais renseigné). Droit `conformite.lire`.
    pub async fn documents_conformite(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<(String, String, String)>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        documents_tx(&mut tx, etab).await
    }
}

pub(crate) async fn documents_tx(
    c: &mut PgConnection,
    etab: Uuid,
) -> Result<Vec<(String, String, String)>, EntDbError> {
    let rows = sqlx::query("SELECT code, etat FROM rgp_document WHERE etablissement_id = $1")
        .bind(etab)
        .fetch_all(&mut *c)
        .await
        .map_err(db)?;
    let etats: BTreeMap<String, String> = rows
        .iter()
        .map(|r| (r.get("code"), r.get("etat")))
        .collect();
    Ok(DOCUMENTS
        .iter()
        .map(|(code, libelle)| {
            (
                code.to_string(),
                libelle.to_string(),
                etats
                    .get(*code)
                    .cloned()
                    .unwrap_or_else(|| "manquant".into()),
            )
        })
        .collect())
}

pub(crate) struct VersionCourante {
    pub version: i32,
    pub valide_le: DateTime<Utc>,
    pub facettes: Facettes,
}

pub(crate) async fn versions_courantes(
    c: &mut PgConnection,
    etab: Uuid,
) -> Result<BTreeMap<Uuid, VersionCourante>, EntDbError> {
    let rows = sqlx::query(
        "SELECT DISTINCT ON (traitement_id) traitement_id, version, valide_le, empreinte_destinataires, empreinte_donnees,
                empreinte_soustraitants, empreinte_durees, empreinte_activation
           FROM rgp_traitement_version WHERE etablissement_id = $1 ORDER BY traitement_id, version DESC",
    )
    .bind(etab)
    .fetch_all(&mut *c)
    .await
    .map_err(db)?;
    Ok(rows
        .iter()
        .map(|r| {
            (
                r.get::<Uuid, _>("traitement_id"),
                VersionCourante {
                    version: r.get("version"),
                    valide_le: r.get("valide_le"),
                    facettes: Facettes {
                        destinataires: r.get("empreinte_destinataires"),
                        donnees: r.get("empreinte_donnees"),
                        soustraitants: r.get("empreinte_soustraitants"),
                        durees: r.get("empreinte_durees"),
                        activation: r.get("empreinte_activation"),
                    },
                },
            )
        })
        .collect())
}

pub(crate) async fn ecarts_ouverts_par_traitement(
    c: &mut PgConnection,
    etab: Uuid,
) -> Result<BTreeMap<Uuid, Vec<String>>, EntDbError> {
    let rows = sqlx::query("SELECT traitement_id, nature FROM rgp_ecart WHERE etablissement_id = $1 AND traite_le IS NULL ORDER BY nature")
        .bind(etab)
        .fetch_all(&mut *c)
        .await
        .map_err(db)?;
    let mut m: BTreeMap<Uuid, Vec<String>> = BTreeMap::new();
    for r in rows {
        m.entry(r.get("traitement_id"))
            .or_default()
            .push(r.get("nature"));
    }
    Ok(m)
}

fn assembler_registre(
    ctx: &Contexte,
    versions: &BTreeMap<Uuid, VersionCourante>,
    ecarts: &BTreeMap<Uuid, Vec<String>>,
) -> Registre {
    let mut fiches = Vec::new();
    for b in TRAITEMENTS_BASE {
        let e = ctx.entrees.get(b.code).cloned().unwrap_or_default();
        let g = generer_fiche(b, &e);
        let t = ctx.traitements.iter().find(|t| t.code == b.code);
        let v = t.and_then(|t| versions.get(&t.id));
        fiches.push(FicheRegistre {
            etat: t
                .map(|t| t.etat.clone())
                .unwrap_or_else(|| "non_initialise".into()),
            base_legale: t
                .map(|t| t.base_legale.clone())
                .unwrap_or_else(|| "mission_interet_public".into()),
            base_legale_motif: t.and_then(|t| t.base_legale_motif.clone()),
            responsable: t
                .map(|t| t.responsable.clone())
                .unwrap_or_else(|| "etablissement".into()),
            responsable_libelle: t.and_then(|t| t.responsable_libelle.clone()),
            version_validee: v.map(|v| v.version),
            valide_le: v.map(|v| v.valide_le),
            ecarts_ouverts: t
                .and_then(|t| ecarts.get(&t.id).cloned())
                .unwrap_or_default(),
            fiche: g,
        });
    }
    let locaux: Vec<TraitementLocal> = ctx
        .traitements
        .iter()
        .filter(|t| t.hors_scolia)
        .map(|t| TraitementLocal {
            code: t.code.clone(),
            libelle: t.libelle.clone(),
            finalite: t.finalite.clone(),
            base_legale: t.base_legale.clone(),
            etat: t.etat.clone(),
        })
        .collect();
    let actives = || fiches.iter().filter(|f| f.fiche.actif);
    let nb_valides = actives().filter(|f| f.etat == "valide").count();
    let nb_a_revoir = actives().filter(|f| f.etat == "a_revoir").count();
    let nb_a_completer = actives()
        .filter(|f| {
            matches!(
                f.etat.as_str(),
                "non_initialise" | "a_completer" | "complete"
            )
        })
        .count();
    let total = actives().count();
    let synthese = format!(
        "{nb_valides} fiche(s) validée(s) sur {total}, {nb_a_completer} à compléter ou à valider, {nb_a_revoir} à revoir. \
         Le registre décrit les mesures de la plateforme et les choix de l'établissement ; il ne vaut pas attestation de conformité. \
         Durées de conservation : {}.",
        if ctx.politique.nb_validees == ctx.politique.regles.len() {
            "toutes validées".to_string()
        } else {
            format!("{} catégorie(s) non validée(s)", ctx.politique.regles.len() - ctx.politique.nb_validees)
        }
    );
    Registre {
        fiches,
        traitements_hors_scolia: locaux,
        nb_valides,
        nb_a_completer,
        nb_a_revoir,
        synthese,
    }
}

/// Registre en lecture pour les modules voisins (mentions, points ouverts) : même contexte, sans contrôle de droit.
pub(crate) struct VueRegistre {
    pub registre: Registre,
    pub politique: Politique,
}

impl PgEnt {
    /// Registre et politique, lus dans la transaction de l'appelant. **Sans contrôle de droit** : l'appelant
    /// (mentions, points ouverts, page « Vos données ») a déjà vérifié ce qui doit l'être.
    pub(crate) async fn vue_registre_tx(
        &self,
        c: &mut PgConnection,
        etab: Uuid,
    ) -> Result<VueRegistre, EntDbError> {
        let ctx = self.contexte_rgpd(c, etab).await?;
        let versions = versions_courantes(c, etab).await?;
        let ecarts = ecarts_ouverts_par_traitement(c, etab).await?;
        let registre = assembler_registre(&ctx, &versions, &ecarts);
        Ok(VueRegistre {
            registre,
            politique: ctx.politique,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tout domaine du registre des tables appartient à un seul traitement de base.
    #[test]
    fn chaque_domaine_est_couvert_une_fois() {
        let mut vus: BTreeMap<&str, &str> = BTreeMap::new();
        for b in TRAITEMENTS_BASE {
            for d in b.domaines {
                assert!(
                    vus.insert(d, b.code).is_none(),
                    "domaine « {d} » dans deux traitements"
                );
            }
        }
        let domaines: BTreeSet<&str> = REGISTRE.iter().map(|t| t.domaine).collect();
        let manquants: Vec<_> = domaines.iter().filter(|d| !vus.contains_key(*d)).collect();
        assert!(
            manquants.is_empty(),
            "domaines sans traitement de base : {manquants:?}"
        );
        for d in vus.keys() {
            assert!(
                domaines.contains(d),
                "domaine inconnu au registre des tables : {d}"
            );
        }
        let codes: BTreeSet<_> = TRAITEMENTS_BASE.iter().map(|b| b.code).collect();
        assert_eq!(codes.len(), TRAITEMENTS_BASE.len());
    }

    #[test]
    fn la_generation_est_deterministe_et_voit_les_roles_reels() {
        let b = TRAITEMENTS_BASE
            .iter()
            .find(|b| b.code == "finances_familles")
            .unwrap();
        let a = generer_fiche(
            b,
            &EntreesFiche {
                module_actif: true,
                ..Default::default()
            },
        );
        let a2 = generer_fiche(
            b,
            &EntreesFiche {
                module_actif: true,
                ..Default::default()
            },
        );
        assert_eq!(a.facettes, a2.facettes);
        // Un rôle de l'établissement qui lit les finances change les destinataires, rien d'autre.
        let mut e = EntreesFiche {
            module_actif: true,
            ..Default::default()
        };
        e.roles_etablissement.insert("Régisseur".into());
        let c = generer_fiche(b, &e);
        let ecarts = ecarts_entre(&a.facettes, &c.facettes);
        assert_eq!(ecarts, vec!["destinataire_ajoute"]);
        assert!(c
            .destinataires_internes
            .iter()
            .any(|d| d.contains("Régisseur")));
        // La direction lit les finances ; un professeur non.
        assert!(a
            .destinataires_internes
            .iter()
            .any(|d| d == "rôle direction"));
        assert!(!a.destinataires_internes.iter().any(|d| d == "rôle prof"));
        // Aucune mention de conformité.
        assert!(!a.transferts_hors_ue.to_lowercase().contains("conforme"));
    }

    #[test]
    fn la_sante_depend_du_module() {
        let b = TRAITEMENTS_BASE.iter().find(|b| b.code == "sante").unwrap();
        let off = generer_fiche(b, &EntreesFiche::default());
        let on = generer_fiche(
            b,
            &EntreesFiche {
                module_actif: true,
                ..Default::default()
            },
        );
        assert!(!off.actif && on.actif);
        assert_eq!(
            ecarts_entre(&off.facettes, &on.facettes),
            vec!["module_active"]
        );
        assert!(on.categories_donnees.iter().any(|c| c.contains("santé")));
    }

    #[test]
    fn un_sous_traitant_ne_concerne_que_les_categories_qu_il_traite() {
        let fin = TRAITEMENTS_BASE
            .iter()
            .find(|b| b.code == "finances_familles")
            .unwrap();
        let sante = TRAITEMENTS_BASE.iter().find(|b| b.code == "sante").unwrap();
        let st = SousTraitantFiche {
            nom: "Hébergeur".into(),
            service: "Base de données".into(),
            pays: "France".into(),
            categories: vec!["fin.piece".into()],
        };
        let e = EntreesFiche {
            soustraitants: vec![st],
            module_actif: true,
            ..Default::default()
        };
        assert_eq!(generer_fiche(fin, &e).soustraitants.len(), 1);
        assert!(generer_fiche(sante, &e).soustraitants.is_empty());
    }
}
