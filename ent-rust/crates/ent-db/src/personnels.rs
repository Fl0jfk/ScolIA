//! Personnels, lot P1 (socle 6 ; `docs/socles/06_PERSONNELS.md` §6, §8.1-8.3, §10.1, §14.2).
//!
//! **Agent** = une personne du socle 4 (`ent_personne`) en tant qu'elle travaille ici ; le
//! professeur qui est aussi parent a UNE personne (un lien de responsabilité ET un agent).
//! **Engagement** = une période continue de présence à un titre donné ; plusieurs engagements
//! simultanés sont permis s'ils diffèrent par l'employeur ou la nature (R-P1) ; une disponibilité
//! ou un congé long est une **suspension** de l'engagement (R-P4), jamais sa clôture.
//!
//! | Port / commande                          | Lecture / écriture | Droit                                   |
//! |------------------------------------------|--------------------|-----------------------------------------|
//! | [`agent_du_compte_tx`]                   | lecture à une date | aucun (port interne : session, casquette) |
//! | [`annuaire_tx`] / `annuaire_for_actor`   | lecture à une date | `personnels.annuaire`                   |
//! | `creer_agent_for_actor` (PRS-01)         | écriture           | `personnels.gerer`                      |
//! | `modifier_identite_agent_for_actor` (02) | écriture           | `personnels.gerer` ou `personnes.gerer` |
//! | `ajouter_engagement_for_actor` (03)      | écriture           | `personnels.gerer`                      |
//! | `prolonger_`, `modifier_`, `terminer_`, `corriger_engagement_for_actor` (04) | écriture | `personnels.gerer`      |
//! | `suspendre_`, `reprendre_engagement_for_actor` (147) | écriture | `personnels.gerer`                  |
//!
//! Chaque commande est une transaction `TenantTx` (RLS) = fait + journal + événement. Journal et
//! événements portent des identifiants, des codes, des dates et des noms de champs ; **jamais** de
//! nom, de coordonnée ni de motif détaillé.

use chrono::{Datelike, Duration, NaiveDate};
use domain_eleve::DomainError;
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::personnels::*;
use serde_json::{json, Value};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::dossier_personnes::{
    aujourdhui_tx, correspondances_tx, creer_personne_tx, DeclarationResponsable, DroitsDeclares,
    PersonneSaisie, ReserveDeclaree, SourceLien,
};
use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

// ---------------------------------------------------------------------------------------------
// Listes fermées (vérifiées aussi par les contraintes de la base)
// ---------------------------------------------------------------------------------------------

pub const NATURES_ENGAGEMENT: &[&str] = &[
    "titulaire",
    "stagiaire",
    "contractuel",
    "vacataire",
    "maitre_contractuel",
    "maitre_agree",
    "delegue_auxiliaire",
    "aed",
    "aesh",
    "assistant_langue",
    "salarie_organisme",
    "agent_collectivite",
    "mis_a_disposition",
    "intervenant",
    "benevole",
    "service_civique",
    "apprenti",
    "stagiaire_ecole",
    "a_preciser",
];
/// Natures à durée limitée : date de fin (au plus un an) et référent obligatoires.
pub const NATURES_LIMITEES: &[&str] = &[
    "intervenant",
    "benevole",
    "stagiaire_ecole",
    "service_civique",
];
pub const EMPLOYEURS: &[&str] = &[
    "etat",
    "collectivite",
    "organisme_gestion",
    "association",
    "autre_etablissement",
    "aucun",
    "inconnu",
];
pub const MOTIFS_ARRIVEE: &[&str] = &[
    "rentree",
    "nomination",
    "mutation",
    "remplacement",
    "creation_poste",
    "renouvellement",
    "autre",
];
/// Motifs de fin, complétés par R-P13 : toujours codés, jamais de texte.
pub const MOTIFS_FIN: &[&str] = &[
    "fin_contrat",
    "mutation",
    "retraite",
    "demission",
    "deces",
    "fin_mission",
    "autre",
    "fin_periode_essai",
    "rupture_conventionnelle",
    "licenciement",
    "abandon_de_poste",
    "detachement",
    "radiation",
];
pub const MOTIFS_SUSPENSION_ENGAGEMENT: &[&str] = &[
    "disponibilite",
    "detachement",
    "conge_parental",
    "conge_long",
    "formation",
    "autre",
];
pub const MOTIFS_RESTREINTE: &[&str] = &["menace", "protection_judiciaire", "autre"];
pub const SOURCES_AGENT: &[&str] = &[
    "saisie",
    "import_sts",
    "import_aaf",
    "import_csv",
    "reprise",
];
pub const CODES_QUALIFICATION: &[&str] = &[
    "secourisme",
    "encadrement_sortie",
    "permis_b",
    "permis_transport",
    "langue",
    "habilitation_electrique",
    "surveillance_baignade",
    "autre",
    "sst",
    "ssiap",
    "caces",
    "haccp",
    "bafa",
    "bnssa",
];
pub const FAMILLES_FONCTION: &[&str] = &[
    "direction",
    "enseignement",
    "education",
    "administration",
    "sante_social",
    "accompagnement",
    "service_agents",
    "technique",
    "intervention",
    "autre",
];
pub const LIENS_URGENCE: &[&str] = &["conjoint", "parent", "enfant", "proche", "autre"];
pub const SYSTEMES_IDENTIFIANT: &[&str] = &[
    "sts",
    "aaf_jointure",
    "guichet_agents",
    "gar",
    "edt_logiciel",
    "ancien_logiciel",
    "annuaire_entreprise",
];

/// Pause de notifications : au plus un an (au-delà, l'agent suspend son accès, PRS-37).
pub(crate) const PAUSE_MAX_JOURS: i64 = 366;

/// nom, prénom, civilité, e-mail, téléphone, 2ᵉ téléphone, adresse, fiche absorbante.
type LignePersonne = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<Uuid>,
);

// ---------------------------------------------------------------------------------------------
// Erreurs
// ---------------------------------------------------------------------------------------------

pub(crate) fn invalide(m: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Invalide, m)
}
pub(crate) fn conflit(m: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Conflit, m)
}
pub(crate) fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

pub(crate) fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("personnels", err))
}

/// Écriture : une violation d'exclusion ou d'unicité devient un conflit lisible (409), pas une panne.
pub(crate) fn ecriture(err: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &err {
        let code = d.code().map(|c| c.to_string()).unwrap_or_default();
        if code == "23P01" || code == "23505" {
            let message = match d.constraint().unwrap_or_default() {
                "ent_engagement_sans_chevauchement" => {
                    "Un engagement de même employeur et de même nature couvre déjà cette période."
                }
                "ent_suspension_sans_chevauchement" => {
                    "Une suspension existe déjà sur cette période."
                }
                "ent_agent_fonction_une_principale" => "Une seule fonction principale à la fois.",
                "ent_agent_fonction_sans_doublon" => {
                    "Cette fonction est déjà attribuée sur la période."
                }
                "ent_agent_discipline_une_principale" => {
                    "Une seule discipline principale à la fois."
                }
                "ent_agent_contact_un_par_rang" => "Ce rang de contact d'urgence est déjà utilisé.",
                "ent_agent_personne_uq" => "Cette personne est déjà agent de l'établissement.",
                "ent_agent_identifiant_actif_uq" => "Cet identifiant désigne déjà un agent.",
                "ent_fonction_code_uq" => "Ce code de fonction existe déjà.",
                _ => "Cette écriture entre en conflit avec une donnée existante.",
            };
            return conflit(message);
        }
        if code == "23503" {
            return EntDbError::Domain(DomainError::ReferenceInconnue);
        }
        if code == "23514" || code == "23502" {
            return invalide("Valeur refusée par les règles de l'établissement.");
        }
    }
    db(err)
}

// ---------------------------------------------------------------------------------------------
// Journal et événements (dans la transaction de la commande)
// ---------------------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub(crate) async fn journal_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    action: &str,
    libelle: &str,
    payload: Value,
    actor: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                         payload, visible_famille, actor_user_id, created_at)
         VALUES ($1,$2,NULL,'personnels',$3,$4,$5,false,$6, now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(action)
    .bind(libelle)
    .bind(payload)
    .bind(actor)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

pub(crate) async fn publier_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    agent_id: Uuid,
    payload: Value,
    actor: Uuid,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Agent,
        aggregate_id: agent_id,
        eleve_id: None,
        payload,
        actor_user_id: Some(actor.to_string()),
    })
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Lignes lues
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct AgentLigne {
    pub id: Uuid,
    pub personne_id: Uuid,
    pub matricule_interne: Option<String>,
    pub identite_restreinte: bool,
    pub identite_restreinte_motif: Option<String>,
    pub personnel_mineur: bool,
    pub photo_visible_collegues: bool,
    pub photo_visible_eleves: bool,
    pub photo_visible_familles: bool,
    pub notifications_en_pause_jusqu_au: Option<NaiveDate>,
    pub identite_a_separer: bool,
    pub source: String,
    pub fusionne_dans_agent_id: Option<Uuid>,
}

const AGENT_COLONNES: &str = "a.id, a.personne_id, a.matricule_interne, a.identite_restreinte,
    a.identite_restreinte_motif, a.personnel_mineur, a.photo_visible_collegues, a.photo_visible_eleves,
    a.photo_visible_familles, a.notifications_en_pause_jusqu_au, a.identite_a_separer, a.source,
    a.fusionne_dans_agent_id";

/// Agent de l'établissement (verrouillé si `verrou`) ; 404 uniforme s'il est d'un autre établissement.
pub(crate) async fn agent_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
    verrou: bool,
) -> Result<AgentLigne, EntDbError> {
    let sql = format!(
        "SELECT {AGENT_COLONNES} FROM ent_agent a
          WHERE a.etablissement_id = $1 AND a.id = $2 {}",
        if verrou { "FOR UPDATE OF a" } else { "" }
    );
    sqlx::query_as::<_, AgentLigne>(&sql)
        .bind(etab)
        .bind(agent_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

/// L'agent (non fusionné) d'un compte dans l'établissement, s'il en a un.
pub(crate) async fn agent_du_user_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT a.id FROM ent_agent a
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND p.user_id = $2 AND a.fusionne_dans_agent_id IS NULL",
    )
    .bind(etab)
    .bind(user_id)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(row.map(|(id,)| id))
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct EngagementLigne {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub du: NaiveDate,
    pub au: Option<NaiveDate>,
    pub nature: String,
    pub employeur_type: String,
    pub employeur_organisme_id: Option<Uuid>,
    pub quotite: Option<f64>,
    pub jours_travailles: Option<Vec<i16>>,
    pub heures_dues: Option<f64>,
    pub motif_arrivee: Option<String>,
    pub motif_fin: Option<String>,
    pub referent_agent_id: Option<Uuid>,
    pub remplace_agent_id: Option<Uuid>,
    pub installe_le: Option<NaiveDate>,
}

const ENGAGEMENT_COLONNES: &str = "e.id, e.agent_id, e.du, e.au, e.nature, e.employeur_type,
    e.employeur_organisme_id, e.quotite_pct::float8 AS quotite, e.jours_travailles,
    e.heures_dues_hebdo::float8 AS heures_dues, e.motif_arrivee, e.motif_fin, e.referent_agent_id,
    e.remplace_agent_id, e.installe_le";

pub(crate) async fn engagement_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    engagement_id: Uuid,
    verrou: bool,
) -> Result<EngagementLigne, EntDbError> {
    let sql = format!(
        "SELECT {ENGAGEMENT_COLONNES} FROM ent_agent_engagement e
          WHERE e.etablissement_id = $1 AND e.id = $2 AND e.annule_le IS NULL {}",
        if verrou { "FOR UPDATE" } else { "" }
    );
    sqlx::query_as::<_, EngagementLigne>(&sql)
        .bind(etab)
        .bind(engagement_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

/// Engagements non annulés d'un agent, avec suspensions et alertes de qualité, à la date `jour`.
pub(crate) async fn engagements_agent_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
    jour: NaiveDate,
    avec_motifs: bool,
) -> Result<Vec<EngagementDto>, EntDbError> {
    let sql = format!(
        "SELECT {ENGAGEMENT_COLONNES} FROM ent_agent_engagement e
          WHERE e.etablissement_id = $1 AND e.agent_id = $2 AND e.annule_le IS NULL
          ORDER BY e.du, e.cree_le"
    );
    let lignes: Vec<EngagementLigne> = sqlx::query_as(&sql)
        .bind(etab)
        .bind(agent_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    let mut out = Vec::with_capacity(lignes.len());
    for l in lignes {
        let susp: Vec<(Uuid, NaiveDate, Option<NaiveDate>, String, bool, bool)> = sqlx::query_as(
            "SELECT id, du, au, motif_code, acces_suspendu, levee_le IS NOT NULL
               FROM ent_agent_engagement_suspension
              WHERE etablissement_id = $1 AND engagement_id = $2 AND annule_le IS NULL
              ORDER BY du",
        )
        .bind(etab)
        .bind(l.id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let suspendu = susp
            .iter()
            .any(|(_, du, au, _, _, _)| *du <= jour && au.is_none_or(|a| a >= jour));
        let (meme,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM ent_agent_engagement o
                 WHERE o.etablissement_id = $1 AND o.agent_id = $2 AND o.id <> $3 AND o.annule_le IS NULL
                   AND o.employeur_type = $4
                   AND COALESCE(o.employeur_organisme_id, '00000000-0000-0000-0000-000000000000'::uuid)
                     = COALESCE($5, '00000000-0000-0000-0000-000000000000'::uuid)
                   AND daterange(o.du, o.au, '[]') && daterange($6, $7, '[]'))",
        )
        .bind(etab)
        .bind(agent_id)
        .bind(l.id)
        .bind(&l.employeur_type)
        .bind(l.employeur_organisme_id)
        .bind(l.du)
        .bind(l.au)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        let mut alertes: Vec<String> = Vec::new();
        if l.nature == "a_preciser" {
            alertes.push("nature_a_preciser".into());
        }
        if meme {
            alertes.push("meme_employeur_simultane".into());
        }
        if l.au.is_none()
            && matches!(
                l.nature.as_str(),
                "contractuel" | "vacataire" | "delegue_auxiliaire" | "aed"
            )
        {
            alertes.push("fin_a_saisir".into());
        }
        out.push(EngagementDto {
            id: l.id,
            agent_id: l.agent_id,
            du: l.du,
            au: l.au,
            nature: l.nature,
            employeur_type: l.employeur_type,
            employeur_organisme_id: l.employeur_organisme_id,
            quotite_pct: l.quotite,
            jours_travailles: l.jours_travailles,
            heures_dues_hebdo: l.heures_dues,
            motif_arrivee: l.motif_arrivee,
            motif_fin: l.motif_fin,
            referent_agent_id: l.referent_agent_id,
            remplace_agent_id: l.remplace_agent_id,
            installe_le: l.installe_le,
            suspendu,
            suspensions: susp
                .into_iter()
                .map(|(id, du, au, motif, acces, levee)| SuspensionDto {
                    id,
                    du,
                    au,
                    motif_code: if avec_motifs { Some(motif) } else { None },
                    acces_suspendu: acces,
                    levee,
                })
                .collect(),
            alertes,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Ports : `agent_du_compte` et `annuaire`
// ---------------------------------------------------------------------------------------------

/// Agent actif d'un compte à une date (`06_PERSONNELS.md` §5.3). Appelé par la session (casquette) et
/// par tous les socles qui ont besoin de l'agent d'un utilisateur.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentActif {
    pub agent_id: Uuid,
    pub personne_id: Uuid,
    pub user_id: Uuid,
    /// `actif` ou `suspendu` (disponibilité, congé : l'engagement est courant mais suspendu).
    pub etat: String,
    /// Engagements courants à la date (au moins un).
    pub engagements: Vec<EngagementDto>,
    pub fonction_principale: Option<String>,
}

/// `Some` si le compte a un agent avec au moins un engagement courant à `jour`.
pub async fn agent_du_compte_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
    jour: NaiveDate,
) -> Result<Option<AgentActif>, EntDbError> {
    let Some(agent_id) = agent_du_user_tx(conn, etab, user_id).await? else {
        return Ok(None);
    };
    let ligne = agent_tx(conn, etab, agent_id, false).await?;
    let (etat,): (String,) = sqlx::query_as("SELECT ent_agent_etat($1, $2, $3)")
        .bind(etab)
        .bind(agent_id)
        .bind(jour)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let courants: Vec<EngagementDto> = engagements_agent_tx(conn, etab, agent_id, jour, false)
        .await?
        .into_iter()
        .filter(|e| e.du <= jour && e.au.is_none_or(|a| a >= jour))
        .collect();
    if courants.is_empty() {
        return Ok(None);
    }
    let fonction: Option<(String,)> = sqlx::query_as(
        "SELECT f.code FROM ent_agent_fonction af JOIN ent_fonction f ON f.id = af.fonction_id
          WHERE af.etablissement_id = $1 AND af.agent_id = $2 AND af.principale AND af.annule_le IS NULL
            AND af.du <= $3 AND (af.au IS NULL OR af.au >= $3)",
    )
    .bind(etab)
    .bind(agent_id)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(Some(AgentActif {
        agent_id,
        personne_id: ligne.personne_id,
        user_id,
        etat,
        engagements: courants,
        fonction_principale: fonction.map(|(c,)| c),
    }))
}

/// Qui lit l'annuaire : détermine ce qui est rendu (§13.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NiveauAnnuaire {
    /// Personnel : nom, fonction, e-mail, compte ; agents menacés visibles tels quels des collègues.
    Interne,
    /// Famille : agents actifs seulement, jamais d'e-mail ni de compte ; identité restreinte masquée.
    Famille,
    /// Élève : comme la famille.
    Eleve,
}

#[derive(Debug, Clone, Default)]
pub struct FiltreAnnuaire {
    pub q: Option<String>,
    pub fonction_code: Option<String>,
    /// États rendus ; vide = `actif` seulement. Les familles et élèves ne voient jamais que `actif`.
    pub etats: Vec<String>,
    pub limite: Option<i64>,
}

fn motif_like(q: &str) -> String {
    let mut s = String::from("%");
    for c in q.trim().chars() {
        if matches!(c, '%' | '_' | '\\') {
            s.push('\\');
        }
        s.push(c);
    }
    s.push('%');
    s
}

fn civilite_courte(c: Option<&str>) -> Option<&'static str> {
    match c {
        Some("m") => Some("M."),
        Some("mme") => Some("Mme"),
        _ => None,
    }
}

/// Nom rendu à un lecteur : jamais le nom complet d'un agent à identité restreinte aux familles et
/// aux élèves (civilité et initiale, ou intitulé de fonction), choix de l'agent sinon (§13.3, P35).
pub fn nom_pour_lecteur(
    niveau: NiveauAnnuaire,
    nom: &str,
    prenom: Option<&str>,
    civilite: Option<&str>,
    nom_affiche_familles: Option<&str>,
    restreinte: bool,
    fonction_libelle: Option<&str>,
) -> String {
    match niveau {
        NiveauAnnuaire::Interne => match prenom.map(str::trim).filter(|p| !p.is_empty()) {
            Some(p) => format!("{p} {nom}"),
            None => nom.to_string(),
        },
        NiveauAnnuaire::Famille | NiveauAnnuaire::Eleve => {
            if restreinte {
                let initiale = nom
                    .trim()
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string());
                match (civilite_courte(civilite), initiale) {
                    (Some(c), Some(i)) => format!("{c} {i}."),
                    _ => fonction_libelle
                        .unwrap_or("Un membre de l'équipe")
                        .to_string(),
                }
            } else if let Some(n) = nom_affiche_familles
                .map(str::trim)
                .filter(|n| !n.is_empty())
            {
                n.to_string()
            } else {
                match civilite_courte(civilite) {
                    Some(c) => format!("{c} {nom}"),
                    None => nom.to_string(),
                }
            }
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AnnuaireLigne {
    agent_id: Uuid,
    nom: String,
    prenom: Option<String>,
    civilite: Option<String>,
    email: Option<String>,
    user_id: Option<Uuid>,
    nom_affiche_familles: Option<String>,
    identite_restreinte: bool,
    fonction_code: Option<String>,
    fonction_libelle: Option<String>,
    famille: Option<String>,
    etat: String,
}

/// Annuaire à une date. Aucun contrôle de droit ici (port) : l'appelant a vérifié que le lecteur
/// est habilité à ce niveau (`personnels.annuaire` pour l'interne ; lien actif pour une famille).
pub async fn annuaire_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    niveau: NiveauAnnuaire,
    filtre: &FiltreAnnuaire,
    jour: NaiveDate,
) -> Result<Vec<AgentResumeDto>, EntDbError> {
    let etats: Vec<String> = if niveau == NiveauAnnuaire::Interne && !filtre.etats.is_empty() {
        filtre.etats.clone()
    } else {
        vec!["actif".to_string()]
    };
    for e in &etats {
        if !matches!(e.as_str(), "prevu" | "actif" | "suspendu" | "parti") {
            return Err(invalide("État d'agent inconnu."));
        }
    }
    let q = filtre.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
    let pattern = q.map(motif_like);
    let limite = filtre.limite.unwrap_or(50).clamp(1, 200);
    let lignes: Vec<AnnuaireLigne> = sqlx::query_as(
        "SELECT * FROM (
           SELECT a.id AS agent_id, p.nom, p.prenom, p.civilite, p.email, p.user_id,
                  a.nom_affiche_familles, a.identite_restreinte,
                  f.code AS fonction_code, f.libelle AS fonction_libelle, f.famille,
                  ent_agent_etat(a.etablissement_id, a.id, $2) AS etat, a.personnel_mineur
             FROM ent_agent a
             JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
             LEFT JOIN LATERAL (
                  SELECT fo.code, fo.libelle, fo.famille
                    FROM ent_agent_fonction af JOIN ent_fonction fo ON fo.id = af.fonction_id
                   WHERE af.etablissement_id = a.etablissement_id AND af.agent_id = a.id
                     AND af.principale AND af.annule_le IS NULL
                     AND af.du <= $2 AND (af.au IS NULL OR af.au >= $2)
                   LIMIT 1) f ON true
            WHERE a.etablissement_id = $1 AND a.fusionne_dans_agent_id IS NULL
              AND p.fusionnee_dans IS NULL
              AND ($3::text IS NULL
                   OR p.nom ILIKE $3 OR COALESCE(p.prenom, '') ILIKE $3
                   OR (COALESCE(p.prenom, '') || ' ' || p.nom) ILIKE $3
                   OR ($7 AND COALESCE(p.email, '') ILIKE $3)
                   OR COALESCE(f.libelle, '') ILIKE $3 OR COALESCE(f.code, '') ILIKE $3)
              AND ($4::text IS NULL OR f.code = $4)
              AND ($7 OR NOT a.personnel_mineur)
         ) x
         WHERE x.etat = ANY($5)
         ORDER BY lower(x.nom), lower(COALESCE(x.prenom, ''))
         LIMIT $6",
    )
    .bind(etab)
    .bind(jour)
    .bind(pattern)
    .bind(filtre.fonction_code.as_deref())
    .bind(&etats)
    .bind(limite)
    .bind(niveau == NiveauAnnuaire::Interne)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(lignes
        .into_iter()
        .map(|l| {
            let interne = niveau == NiveauAnnuaire::Interne;
            AgentResumeDto {
                agent_id: l.agent_id,
                nom_affiche: nom_pour_lecteur(
                    niveau,
                    &l.nom,
                    l.prenom.as_deref(),
                    l.civilite.as_deref(),
                    l.nom_affiche_familles.as_deref(),
                    l.identite_restreinte,
                    l.fonction_libelle.as_deref(),
                ),
                fonction_code: l.fonction_code,
                fonction_libelle: l.fonction_libelle,
                famille: l.famille,
                etat: l.etat,
                email: if interne { l.email } else { None },
                user_id: if interne { l.user_id } else { None },
                identite_restreinte: l.identite_restreinte && interne,
            }
        })
        .collect())
}

// ---------------------------------------------------------------------------------------------
// Validation des saisies d'engagement
// ---------------------------------------------------------------------------------------------

struct EngagementValide {
    nature: String,
    employeur_type: String,
    employeur_organisme_id: Option<Uuid>,
    quotite: Option<f64>,
    jours: Option<Vec<i16>>,
    heures: Option<f64>,
    referent: Option<Uuid>,
}

fn valider_corps(
    nature: &str,
    employeur_type: &str,
    employeur_organisme_id: Option<Uuid>,
    quotite: Option<f64>,
    jours: &Option<Vec<i16>>,
    heures: Option<f64>,
    referent: Option<Uuid>,
) -> Result<EngagementValide, EntDbError> {
    if !NATURES_ENGAGEMENT.contains(&nature) {
        return Err(invalide("Nature d'engagement inconnue."));
    }
    if !EMPLOYEURS.contains(&employeur_type) {
        return Err(invalide("Type d'employeur inconnu."));
    }
    if let Some(q) = quotite {
        if !q.is_finite() || q <= 0.0 || q > 100.0 {
            return Err(invalide(
                "La quotité doit être comprise entre 0 (exclu) et 100.",
            ));
        }
    }
    if let Some(h) = heures {
        if !h.is_finite() || !(0.0..=60.0).contains(&h) {
            return Err(invalide(
                "Les heures dues hebdomadaires doivent être comprises entre 0 et 60.",
            ));
        }
    }
    let jours = match jours {
        None => None,
        Some(j) => {
            let mut j = j.clone();
            j.sort_unstable();
            j.dedup();
            if j.is_empty() || j.iter().any(|d| !(1..=6).contains(d)) {
                return Err(invalide("Jours travaillés : de 1 (lundi) à 6 (samedi)."));
            }
            Some(j)
        }
    };
    Ok(EngagementValide {
        nature: nature.to_string(),
        employeur_type: employeur_type.to_string(),
        employeur_organisme_id,
        quotite,
        jours,
        heures,
        referent,
    })
}

/// Bornes de plausibilité d'une date d'engagement (garde contre une saisie aberrante).
fn date_plausible(d: NaiveDate, aujourdhui: NaiveDate) -> bool {
    d.year() >= 1950 && d <= aujourdhui + Duration::days(366 * 5)
}

/// Un an après `du` (inclus) : borne des natures limitées.
fn un_an_apres(du: NaiveDate) -> NaiveDate {
    du.checked_add_months(chrono::Months::new(12))
        .unwrap_or(du + Duration::days(366))
}

async fn exiger_referent(
    conn: &mut PgConnection,
    etab: Uuid,
    referent: Uuid,
    agent_id: Uuid,
) -> Result<(), EntDbError> {
    if referent == agent_id {
        return Err(invalide("Un agent ne peut pas être son propre référent."));
    }
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2
                           AND fusionne_dans_agent_id IS NULL)",
    )
    .bind(etab)
    .bind(referent)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    if ok {
        Ok(())
    } else {
        Err(invalide("Référent inconnu dans cet établissement."))
    }
}

// ---------------------------------------------------------------------------------------------
// Commandes
// ---------------------------------------------------------------------------------------------

/// Résultat de PRS-01.
#[derive(Debug, Clone, PartialEq)]
pub enum ResultatCreationAgent {
    Cree(AgentCreeDto),
    /// Des personnes pourraient être celle que l'on veut créer : à trancher (aucune écriture).
    AConfirmer(Vec<CorrespondanceAgentDto>),
}

fn declaration_de_recherche(s: &PersonneSaisie) -> DeclarationResponsable {
    DeclarationResponsable {
        cle: Uuid::nil(),
        rang: 1,
        nom: s.nom.clone(),
        prenom: s.prenom.clone(),
        civilite: s.civilite.clone(),
        email: s.email.clone(),
        telephone: None,
        adresse: None,
        lien: "autre".into(),
        meme_foyer: false,
        autorite_parentale: false,
        droits: DroitsDeclares {
            notes: false,
            vs: false,
            justifier: false,
            sorties: false,
        },
        payeur: false,
        part_paiement: None,
        reserve: ReserveDeclaree::default(),
    }
}

fn saisie_de(b: &PersonneAgentBody) -> PersonneSaisie {
    PersonneSaisie {
        nom: b.nom.clone(),
        prenom: b.prenom.clone(),
        civilite: b.civilite.clone(),
        email: b.email.clone(),
        telephone: b.telephone.clone(),
        telephone_2: None,
        adresse: b.adresse.clone(),
    }
}

impl PgEnt {
    /// Contrôle serveur d'une action d'établissement (403 uniforme).
    pub(crate) async fn exiger_personnels(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        action: Action,
    ) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etab, actor, role, action)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Port `personnels::agent_du_compte` : l'agent actif d'un compte à une date.
    pub async fn agent_du_compte(
        &self,
        etab: Uuid,
        user_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<AgentActif>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        agent_du_compte_tx(&mut tx, etab, user_id, jour).await
    }

    /// Port `personnels::annuaire` pour un lecteur interne (`personnels.annuaire`), à la date du jour
    /// de l'établissement. Jamais de téléphone.
    pub async fn annuaire_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        filtre: &FiltreAnnuaire,
    ) -> Result<Vec<AgentResumeDto>, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsAnnuaire)
            .await?;
        // Voir les agents parti / prévus / suspendus exige la fiche d'organisation.
        if filtre.etats.iter().any(|e| e != "actif") {
            self.exiger_personnels(etab, actor, role, Action::PersonnelsLire)
                .await?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        annuaire_tx(&mut tx, etab, NiveauAnnuaire::Interne, filtre, jour).await
    }

    /// PRS-01 : créer un agent. Recherche de la personne d'abord (« est-ce M. Martin, père de
    /// Léa ? ») : une personne existante est réutilisée (professeur-parent = une seule personne) ;
    /// une nouvelle identité n'est créée qu'après confirmation si des correspondances existent.
    pub async fn creer_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &CreerAgentBody,
    ) -> Result<ResultatCreationAgent, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let matricule = body
            .matricule_interne
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty());
        if matricule.is_some_and(|m| m.chars().count() > 40) {
            return Err(invalide("Matricule interne : 40 caractères au plus."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let mut avertissements: Vec<String> = Vec::new();
        let personne_id = match (&body.personne_id, &body.personne) {
            (Some(_), Some(_)) | (None, None) => {
                return Err(invalide(
                    "Indiquer une personne existante OU une nouvelle identité.",
                ));
            }
            (Some(pid), None) => {
                let p: Option<(Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
                    "SELECT fusionnee_dans, eleve_id FROM ent_personne
                      WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
                )
                .bind(etab)
                .bind(pid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                let (fusionnee, eleve) = p.ok_or_else(introuvable)?;
                if fusionnee.is_some() {
                    return Err(conflit(
                        "Cette fiche est fusionnée dans une autre : utiliser la fiche conservée.",
                    ));
                }
                if eleve.is_some() {
                    avertissements.push("personne_est_eleve".into());
                }
                let (resp,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_actif
                                     WHERE etablissement_id = $1 AND personne_id = $2)",
                )
                .bind(etab)
                .bind(pid)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if resp {
                    avertissements.push("personne_responsable".into());
                }
                *pid
            }
            (None, Some(nouvelle)) => {
                let saisie = saisie_de(nouvelle).normaliser()?;
                if !body.confirmer_nouvelle {
                    let trouves =
                        correspondances_tx(&mut tx, etab, &declaration_de_recherche(&saisie))
                            .await?;
                    if !trouves.is_empty() {
                        let mut out = Vec::with_capacity(trouves.len());
                        for c in trouves {
                            let (est_agent,): (bool,) = sqlx::query_as(
                                "SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND personne_id = $2)",
                            )
                            .bind(etab)
                            .bind(c.personne_id)
                            .fetch_one(&mut *tx)
                            .await
                            .map_err(db)?;
                            out.push(CorrespondanceAgentDto {
                                personne_id: c.personne_id,
                                nom: c.nom,
                                prenom: c.prenom,
                                email_masque: c.email_masque,
                                raison: c.raison.to_string(),
                                a_un_compte: c.a_un_compte,
                                est_agent,
                            });
                        }
                        return Ok(ResultatCreationAgent::AConfirmer(out));
                    }
                }
                creer_personne_tx(&mut tx, etab, &saisie, SourceLien::Saisie, actor).await?
            }
        };
        let agent_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_agent (id, etablissement_id, personne_id, matricule_interne, personnel_mineur,
                                    source, cree_par_user_id)
             VALUES ($1, $2, $3, $4, $5, 'saisie', $6)",
        )
        .bind(agent_id)
        .bind(etab)
        .bind(personne_id)
        .bind(matricule)
        .bind(body.personnel_mineur)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        if let Some(code) = body.fonction_code.as_deref() {
            let fonction_id = fonction_id_tx(&mut tx, etab, code).await?;
            ajouter_fonction_ligne_tx(
                &mut tx,
                etab,
                agent_id,
                fonction_id,
                true,
                jour,
                None,
                actor,
            )
            .await?;
        }
        journal_tx(
            &mut tx,
            etab,
            "agent_cree",
            "Agent créé",
            json!({ "agent_id": agent_id, "personne_id": personne_id, "source": "saisie", "avertissements": avertissements }),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_CREE,
            agent_id,
            json!({ "agent_id": agent_id, "personne_id": personne_id, "source": "saisie" }),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ResultatCreationAgent::Cree(AgentCreeDto {
            agent_id,
            personne_id,
            avertissements,
        }))
    }

    /// PRS-02 : modifier l'identité d'un agent (par la personne du socle 4). `personnels.gerer` OU
    /// `personnes.gerer` : les deux secrétariats peuvent corriger une faute de frappe ; l'autre domaine
    /// est prévenu par l'événement de la personne. Correction ≠ changement.
    pub async fn modifier_identite_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
        body: &ModifierIdentiteBody,
    ) -> Result<IdentiteAgentDto, EntDbError> {
        if !self
            .a_action_etablissement(etab, actor, role, Action::PersonnelsGerer)
            .await?
            && !self
                .a_action_etablissement(etab, actor, role, Action::PersonnesGerer)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let saisie = saisie_de(body).normaliser()?;
        let personne_id = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            agent_tx(&mut tx, etab, agent_id, false).await?.personne_id
        };
        // Conflit d'intérêts (R-A18) : même règle que pour toute personne du dossier.
        self.exiger_pas_de_conflit_personne(etab, actor, role, personne_id)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let avant: Option<LignePersonne> = sqlx::query_as(
            "SELECT nom, prenom, civilite, email, telephone, telephone_2, adresse, fusionnee_dans
                   FROM ent_personne WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(personne_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (nom, prenom, civilite, email, telephone, telephone_2, adresse, fusionnee) =
            avant.ok_or_else(introuvable)?;
        if fusionnee.is_some() {
            return Err(conflit("Cette fiche est fusionnée dans une autre."));
        }
        let mut champs: Vec<&str> = Vec::new();
        for (n, a, b) in [
            ("nom", Some(nom), Some(saisie.nom.clone())),
            ("prenom", prenom, saisie.prenom.clone()),
            ("civilite", civilite, saisie.civilite.clone()),
            ("email", email, saisie.email.clone()),
            ("telephone", telephone, saisie.telephone.clone()),
            ("adresse", adresse, saisie.adresse.clone()),
        ] {
            if a != b {
                champs.push(n);
            }
        }
        let _ = telephone_2;
        if !champs.is_empty() {
            // Le téléphone et l'adresse d'un agent relèvent de `personnels.personnel.lire` (§13.3) :
            // la commande ne les modifie que pour ceux qui les lisent.
            if (champs.contains(&"telephone") || champs.contains(&"adresse"))
                && !self
                    .a_action_etablissement(etab, actor, role, Action::PersonnelsPersonnelLire)
                    .await?
                && !self
                    .a_action_etablissement(etab, actor, role, Action::PersonnesGerer)
                    .await?
            {
                return Err(EntDbError::Forbidden);
            }
            sqlx::query(
                "UPDATE ent_personne SET nom = $3, prenom = $4, civilite = $5, email = $6,
                        telephone = $7, adresse = $8, updated_at = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(personne_id)
            .bind(&saisie.nom)
            .bind(&saisie.prenom)
            .bind(&saisie.civilite)
            .bind(&saisie.email)
            .bind(&saisie.telephone)
            .bind(&saisie.adresse)
            .execute(&mut *tx)
            .await
            .map_err(ecriture)?;
            // L'identité d'un compte repris était une seule chaîne : une correction du nom la sépare.
            if champs.contains(&"nom") || champs.contains(&"prenom") {
                sqlx::query("UPDATE ent_agent SET identite_a_separer = false WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(agent_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
            }
            let payload =
                json!({ "agent_id": agent_id, "personne_id": personne_id, "champs": champs });
            journal_tx(
                &mut tx,
                etab,
                "agent_identite_modifiee",
                "Identité d'un agent modifiée",
                payload.clone(),
                actor,
            )
            .await?;
            // Événement du socle 4 : l'autre domaine (dossier administratif) est prévenu.
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types::PERSONNE_COORDONNEES_MODIFIEES.to_string(),
                aggregate: AggregateKind::Personne,
                aggregate_id: personne_id,
                eleve_id: None,
                payload,
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        }
        let ident = identite_tx(&mut tx, etab, personne_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(ident)
    }

    /// PRS-03 : ajouter un engagement (arrivée, renouvellement, second employeur).
    pub async fn ajouter_engagement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
        body: &EngagementBody,
    ) -> Result<EngagementDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let v = valider_corps(
            &body.nature,
            &body.employeur_type,
            body.employeur_organisme_id,
            body.quotite_pct,
            &body.jours_travailles,
            body.heures_dues_hebdo,
            body.referent_agent_id,
        )?;
        if let Some(m) = body.motif_arrivee.as_deref() {
            if !MOTIFS_ARRIVEE.contains(&m) {
                return Err(invalide("Motif d'arrivée inconnu."));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let du = body.du.unwrap_or(jour);
        if !date_plausible(du, jour) || body.au.is_some_and(|a| !date_plausible(a, jour)) {
            return Err(invalide("Date d'engagement invraisemblable."));
        }
        if body.au.is_some_and(|a| a < du) {
            return Err(invalide("La fin de l'engagement précède son début."));
        }
        let agent = agent_tx(&mut tx, etab, agent_id, true).await?;
        if agent.fusionne_dans_agent_id.is_some() {
            return Err(conflit("Cet agent a été fusionné dans un autre."));
        }
        if NATURES_LIMITEES.contains(&v.nature.as_str()) {
            let Some(au) = body.au else {
                return Err(invalide("Cette nature d'engagement exige une date de fin."));
            };
            if au > un_an_apres(du) {
                return Err(invalide(
                    "Cette nature d'engagement dure un an au plus (prolongeable).",
                ));
            }
            if v.referent.is_none() {
                return Err(invalide("Cette nature d'engagement exige un référent."));
            }
        }
        if let Some(r) = v.referent {
            exiger_referent(&mut tx, etab, r, agent_id).await?;
        }
        if let Some(r) = body.remplace_agent_id {
            exiger_referent(&mut tx, etab, r, agent_id).await?;
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_agent_engagement (id, etablissement_id, agent_id, du, au, nature, employeur_type,
                    employeur_organisme_id, quotite_pct, jours_travailles, heures_dues_hebdo, motif_arrivee,
                    remplace_agent_id, referent_agent_id, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9::float8::numeric(5,2),$10,$11::float8::numeric(4,2),$12,$13,$14,$15)",
        )
        .bind(id)
        .bind(etab)
        .bind(agent_id)
        .bind(du)
        .bind(body.au)
        .bind(&v.nature)
        .bind(&v.employeur_type)
        .bind(v.employeur_organisme_id)
        .bind(v.quotite)
        .bind(&v.jours)
        .bind(v.heures)
        .bind(&body.motif_arrivee)
        .bind(body.remplace_agent_id)
        .bind(v.referent)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let payload = json!({ "agent_id": agent_id, "engagement_id": id, "nature": v.nature, "du": du, "au": body.au });
        journal_tx(
            &mut tx,
            etab,
            "engagement_ajoute",
            "Engagement ajouté",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_ENGAGEMENT_AJOUTE,
            agent_id,
            payload,
            actor,
        )
        .await?;
        let dto = engagement_dto_tx(&mut tx, etab, agent_id, id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-04 : modifier les champs d'un engagement (quotité, jours, heures, référent, employeur, nature).
    pub async fn modifier_engagement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        engagement_id: Uuid,
        body: &ModifierEngagementBody,
    ) -> Result<EngagementDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let v = valider_corps(
            &body.nature,
            &body.employeur_type,
            body.employeur_organisme_id,
            body.quotite_pct,
            &body.jours_travailles,
            body.heures_dues_hebdo,
            body.referent_agent_id,
        )?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let avant = engagement_tx(&mut tx, etab, engagement_id, true).await?;
        if NATURES_LIMITEES.contains(&v.nature.as_str()) {
            let Some(au) = avant.au else {
                return Err(invalide("Cette nature d'engagement exige une date de fin."));
            };
            if au > un_an_apres(avant.du) {
                return Err(invalide(
                    "Cette nature d'engagement dure un an au plus (prolongeable).",
                ));
            }
            if v.referent.is_none() {
                return Err(invalide("Cette nature d'engagement exige un référent."));
            }
        }
        if let Some(r) = v.referent {
            exiger_referent(&mut tx, etab, r, avant.agent_id).await?;
        }
        if let Some(i) = body.installe_le {
            if i < avant.du || i > jour {
                return Err(invalide(
                    "La prise de fonctions ne précède pas le début et n'est pas dans le futur.",
                ));
            }
        }
        let mut champs: Vec<&str> = Vec::new();
        if avant.nature != v.nature {
            champs.push("nature");
        }
        if avant.employeur_type != v.employeur_type
            || avant.employeur_organisme_id != v.employeur_organisme_id
        {
            champs.push("employeur");
        }
        if avant.quotite != v.quotite {
            champs.push("quotite");
        }
        if avant.jours_travailles != v.jours {
            champs.push("jours_travailles");
        }
        if avant.heures_dues != v.heures {
            champs.push("heures_dues");
        }
        if avant.referent_agent_id != v.referent {
            champs.push("referent");
        }
        if avant.installe_le != body.installe_le {
            champs.push("installe_le");
        }
        if !champs.is_empty() {
            sqlx::query(
                "UPDATE ent_agent_engagement
                    SET nature = $3, employeur_type = $4, employeur_organisme_id = $5,
                        quotite_pct = $6::float8::numeric(5,2), jours_travailles = $7,
                        heures_dues_hebdo = $8::float8::numeric(4,2), referent_agent_id = $9, installe_le = $10
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(engagement_id)
            .bind(&v.nature)
            .bind(&v.employeur_type)
            .bind(v.employeur_organisme_id)
            .bind(v.quotite)
            .bind(&v.jours)
            .bind(v.heures)
            .bind(v.referent)
            .bind(body.installe_le)
            .execute(&mut *tx)
            .await
            .map_err(ecriture)?;
            let payload = json!({ "agent_id": avant.agent_id, "engagement_id": engagement_id,
                                  "nature": "modification", "champs": champs });
            journal_tx(
                &mut tx,
                etab,
                "engagement_modifie",
                "Engagement modifié",
                payload.clone(),
                actor,
            )
            .await?;
            publier_tx(
                &mut tx,
                etab,
                types::PERSONNEL_ENGAGEMENT_MODIFIE,
                avant.agent_id,
                payload,
                actor,
            )
            .await?;
        }
        let dto = engagement_dto_tx(&mut tx, etab, avant.agent_id, engagement_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-04 : prolonger (décaler la fin). Une prolongation saisie après la fin est permise : l'historique
    /// est conservé dans le journal ; la réouverture de l'accès dans les 30 jours relève du lot P2 (R-P9).
    pub async fn prolonger_engagement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        engagement_id: Uuid,
        body: &ProlongerEngagementBody,
    ) -> Result<EngagementDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let avant = engagement_tx(&mut tx, etab, engagement_id, true).await?;
        if let Some(au) = body.au {
            if au < avant.du {
                return Err(invalide("La fin de l'engagement précède son début."));
            }
            if !date_plausible(au, jour) {
                return Err(invalide("Date d'engagement invraisemblable."));
            }
            if NATURES_LIMITEES.contains(&avant.nature.as_str()) && au > un_an_apres(avant.du) {
                return Err(invalide(
                    "Cette nature d'engagement dure un an au plus : ouvrir un nouvel engagement.",
                ));
            }
        } else if NATURES_LIMITEES.contains(&avant.nature.as_str()) {
            return Err(invalide("Cette nature d'engagement exige une date de fin."));
        }
        // Une prolongation efface le motif de fin d'une fin antérieure.
        sqlx::query(
            "UPDATE ent_agent_engagement SET au = $3, motif_fin = CASE WHEN $3::date IS NULL THEN NULL ELSE motif_fin END
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(engagement_id)
        .bind(body.au)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let payload = json!({ "agent_id": avant.agent_id, "engagement_id": engagement_id, "nature": "prolongation",
                              "au_avant": avant.au, "au": body.au });
        journal_tx(
            &mut tx,
            etab,
            "engagement_prolonge",
            "Engagement prolongé",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_ENGAGEMENT_MODIFIE,
            avant.agent_id,
            payload,
            actor,
        )
        .await?;
        let dto = engagement_dto_tx(&mut tx, etab, avant.agent_id, engagement_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-04 / PRS-30 (données) : fixer la fin d'un engagement avec un motif **codé**.
    pub async fn terminer_engagement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        engagement_id: Uuid,
        body: &TerminerEngagementBody,
    ) -> Result<EngagementDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        if !MOTIFS_FIN.contains(&body.motif_fin.as_str()) {
            return Err(invalide("Motif de fin inconnu."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let avant = engagement_tx(&mut tx, etab, engagement_id, true).await?;
        if body.au < avant.du {
            return Err(invalide("La fin de l'engagement précède son début."));
        }
        if !date_plausible(body.au, jour) {
            return Err(invalide("Date d'engagement invraisemblable."));
        }
        sqlx::query(
            "UPDATE ent_agent_engagement SET au = $3, motif_fin = $4 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(engagement_id)
        .bind(body.au)
        .bind(&body.motif_fin)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let payload = json!({ "agent_id": avant.agent_id, "engagement_id": engagement_id, "nature": "fin",
                              "au": body.au, "motif_fin": body.motif_fin });
        journal_tx(
            &mut tx,
            etab,
            "engagement_termine",
            "Fin d'engagement enregistrée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_ENGAGEMENT_MODIFIE,
            avant.agent_id,
            payload,
            actor,
        )
        .await?;
        let dto = engagement_dto_tx(&mut tx, etab, avant.agent_id, engagement_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-04 : corriger une saisie erronée. La ligne est **annulée** (jamais supprimée) : elle sort
    /// des lectures et de l'exclusion de chevauchement, le journal garde la trace.
    pub async fn corriger_engagement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        engagement_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let avant = engagement_tx(&mut tx, etab, engagement_id, true).await?;
        // Les suspensions de la ligne erronée tombent avec elle.
        sqlx::query(
            "UPDATE ent_agent_engagement_suspension SET annule_le = now(), annule_par_user_id = $3
              WHERE etablissement_id = $1 AND engagement_id = $2 AND annule_le IS NULL",
        )
        .bind(etab)
        .bind(engagement_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_agent_engagement SET annule_le = now(), annule_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(engagement_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let payload = json!({ "agent_id": avant.agent_id, "engagement_id": engagement_id, "nature": "correction" });
        journal_tx(
            &mut tx,
            etab,
            "engagement_corrige",
            "Engagement erroné annulé",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_ENGAGEMENT_MODIFIE,
            avant.agent_id,
            payload,
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// PRS-147 (R-P4) : suspendre un engagement (disponibilité, détachement, congé parental) **sans le
    /// clore**. L'accès est suspendu pendant la suspension (sauf choix contraire) ; la levée le rétablit
    /// sans nouvelle invitation. Refusé si cela privait l'établissement de son dernier administrateur.
    pub async fn suspendre_engagement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        engagement_id: Uuid,
        body: &SuspendreEngagementBody,
    ) -> Result<EngagementDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        if !MOTIFS_SUSPENSION_ENGAGEMENT.contains(&body.motif_code.as_str()) {
            return Err(invalide("Motif de suspension inconnu."));
        }
        if body.au.is_some_and(|a| a < body.du) {
            return Err(invalide("La fin de la suspension précède son début."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        if !date_plausible(body.du, jour) || body.au.is_some_and(|a| !date_plausible(a, jour)) {
            return Err(invalide("Date de suspension invraisemblable."));
        }
        let eng = engagement_tx(&mut tx, etab, engagement_id, true).await?;
        if body.du < eng.du
            || eng
                .au
                .is_some_and(|fin| body.du > fin || body.au.is_none_or(|a| a > fin))
        {
            return Err(invalide(
                "La suspension doit tenir dans la période de l'engagement.",
            ));
        }
        if body.acces_suspendu {
            // Garde-fou du dernier administrateur : un administrateur durable dont l'accès serait coupé
            // n'est admis que si un autre administrateur durable reste.
            let admin: Option<(Uuid,)> = sqlx::query_as(
                "SELECT m.user_id FROM ent_agent a
                   JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
                   JOIN ent_membership m ON m.user_id = p.user_id AND m.etablissement_id = a.etablissement_id
                  WHERE a.etablissement_id = $1 AND a.id = $2 AND m.role = 'admin'
                    AND m.etat = 'ouvert' AND m.date_fin IS NULL",
            )
            .bind(etab)
            .bind(eng.agent_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((user,)) = admin {
                if !Self::autre_administrateur_durable_tx(&mut tx, etab, user).await? {
                    return Err(conflit(
                        "Un établissement ne peut pas perdre son dernier administrateur.",
                    ));
                }
            }
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_agent_engagement_suspension (id, etablissement_id, engagement_id, du, au, motif_code,
                    acces_suspendu, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(engagement_id)
        .bind(body.du)
        .bind(body.au)
        .bind(&body.motif_code)
        .bind(body.acces_suspendu)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        // Charge utile sans motif : le motif d'une disponibilité reste dans la table, lisible de la direction.
        let payload = json!({ "agent_id": eng.agent_id, "engagement_id": engagement_id, "suspension_id": id,
                              "du": body.du, "au": body.au, "acces_suspendu": body.acces_suspendu });
        journal_tx(
            &mut tx,
            etab,
            "engagement_suspendu",
            "Engagement suspendu",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_ENGAGEMENT_SUSPENDU,
            eng.agent_id,
            payload,
            actor,
        )
        .await?;
        let dto = engagement_dto_tx(&mut tx, etab, eng.agent_id, engagement_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-147 : lever la suspension. `date_reprise` = premier jour de reprise (la suspension se termine
    /// la veille). Reprise au plus tôt le jour du début : la suspension est alors annulée. Rien à recréer.
    pub async fn reprendre_engagement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        engagement_id: Uuid,
        body: &ReprendreEngagementBody,
    ) -> Result<EngagementDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let eng = engagement_tx(&mut tx, etab, engagement_id, true).await?;
        if !date_plausible(body.date_reprise, jour) {
            return Err(invalide("Date de reprise invraisemblable."));
        }
        // La suspension visée : celle qui couvre la veille de la reprise, ou la dernière commencée.
        let veille = body.date_reprise - Duration::days(1);
        let cible: Option<(Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT id, du FROM ent_agent_engagement_suspension
              WHERE etablissement_id = $1 AND engagement_id = $2 AND annule_le IS NULL AND levee_le IS NULL
                AND (au IS NULL OR au >= $3)
              ORDER BY du DESC LIMIT 1 FOR UPDATE",
        )
        .bind(etab)
        .bind(engagement_id)
        .bind(veille)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (suspension_id, du) = cible
            .ok_or_else(|| conflit("Aucune suspension en cours à lever pour cet engagement."))?;
        if body.date_reprise <= du {
            sqlx::query(
                "UPDATE ent_agent_engagement_suspension SET annule_le = now(), annule_par_user_id = $3
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(suspension_id)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        } else {
            sqlx::query(
                "UPDATE ent_agent_engagement_suspension SET au = $3, levee_le = now(), levee_par_user_id = $4
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(suspension_id)
            .bind(veille)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(ecriture)?;
        }
        let payload = json!({ "agent_id": eng.agent_id, "engagement_id": engagement_id,
                              "suspension_id": suspension_id, "reprise": body.date_reprise });
        journal_tx(
            &mut tx,
            etab,
            "engagement_repris",
            "Suspension d'engagement levée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_ENGAGEMENT_REPRIS,
            eng.agent_id,
            payload,
            actor,
        )
        .await?;
        let dto = engagement_dto_tx(&mut tx, etab, eng.agent_id, engagement_id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// Existe-t-il un autre administrateur « durable » (profil `admin`, accès ouvert, sans fin) ?
    async fn autre_administrateur_durable_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        sauf_user: Uuid,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_membership
                             WHERE etablissement_id = $1 AND user_id <> $2
                               AND role = 'admin' AND etat = 'ouvert' AND date_fin IS NULL
                               AND NOT ent_agent_acces_suspendu(user_id, etablissement_id))",
        )
        .bind(etab)
        .bind(sauf_user)
        .fetch_one(conn)
        .await
        .map_err(db)?;
        Ok(ok)
    }
}

async fn engagement_dto_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
    engagement_id: Uuid,
    jour: NaiveDate,
) -> Result<EngagementDto, EntDbError> {
    engagements_agent_tx(conn, etab, agent_id, jour, true)
        .await?
        .into_iter()
        .find(|e| e.id == engagement_id)
        .ok_or_else(introuvable)
}

pub(crate) async fn identite_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
) -> Result<IdentiteAgentDto, EntDbError> {
    let (nom, prenom, civilite, email): (String, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT nom, prenom, civilite, email FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(personne_id)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(IdentiteAgentDto {
        personne_id,
        nom,
        prenom,
        civilite,
        email,
    })
}

/// Fonction du catalogue (fournie ou de l'établissement) par son code ; l'établissement l'emporte.
pub(crate) async fn fonction_id_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    code: &str,
) -> Result<Uuid, EntDbError> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_fonction
          WHERE code = $2 AND (etablissement_id = $1 OR etablissement_id IS NULL) AND archivee_le IS NULL
          ORDER BY etablissement_id NULLS LAST LIMIT 1",
    )
    .bind(etab)
    .bind(code)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    row.map(|(id,)| id)
        .ok_or_else(|| invalide("Fonction inconnue."))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn ajouter_fonction_ligne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
    fonction_id: Uuid,
    principale: bool,
    du: NaiveDate,
    au: Option<NaiveDate>,
    actor: Uuid,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_agent_fonction (id, etablissement_id, agent_id, fonction_id, principale, du, au, cree_par_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(id)
    .bind(etab)
    .bind(agent_id)
    .bind(fonction_id)
    .bind(principale)
    .bind(du)
    .bind(au)
    .bind(actor)
    .execute(conn)
    .await
    .map_err(ecriture)?;
    Ok(id)
}
