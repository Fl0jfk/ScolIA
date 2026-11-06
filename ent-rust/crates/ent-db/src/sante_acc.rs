//! Santé (socle 2, lot S2-11) : PAI complet et signatures, contraintes publiées sans
//! donnée médicale, passages à l'infirmerie, fiche d'urgence, registre des protocoles.
//!
//! **Minimisation (Q7 HDS en attente du DPO).**
//! - Les événements de ce module ne portent JAMAIS de donnée de santé : ni motif, ni
//!   soins, ni diagnostic, ni code de contrainte (identifiants et états seulement).
//! - Une contrainte publiée est un code d'un catalogue fermé ([`CATALOGUE_CONTRAINTES`]) :
//!   la base ne reçoit aucun texte libre pour les destinataires hors santé.
//! - Textes cliniques (PAI, notes de soins) chiffrés (socle 1, D3) ; tables sous RLS
//!   (`TenantTx`, migrations 100 et 215).
//! - Consultations des données de santé journalisées (`tracer_acces_sensible`).
//!
//! **Droits** (moteur D-017) : `sante.lire` / `sante.saisir` pour le personnel de santé,
//! `contraintes.lire` (professeurs, portée « mes classes »), `sante.urgence_lire`
//! (fiche d'urgence minimale), `repas.*` et `sorties.*` pour les audiences
//! « restauration » et « sorties ».

use crate::outbox::Publier;
use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::vie_scolaire::{db, introuvable, invalide};
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use domain_eleve::{
    plan_signer_pai_famille, plan_upsert_pai, today_utc, DomainError, Pai, SignerPaiFamille,
    UpsertPai,
};
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use sqlx::PgConnection;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Types d'événements du lot (constantes locales, comme `types_vs`). Leurs charges
/// utiles ne contiennent que des identifiants, des états et des dates techniques.
pub mod types_sante {
    pub const PASSAGE: &str = "infirmerie.passage";
    pub const PAI: &str = "infirmerie.pai";
    pub const PAI_SIGNE: &str = "infirmerie.pai_signe";
    pub const FAMILLE_INFORMEE: &str = "infirmerie.famille_informee";
    pub const RENVOI_DOMICILE: &str = "infirmerie.renvoi_domicile";
    pub const CONTRAINTE_PUBLIEE: &str = "sante.contrainte_publiee";
    pub const CONTRAINTE_RETIREE: &str = "sante.contrainte_retiree";
    pub const PROTOCOLE_MIS_A_JOUR: &str = "sante.protocole_mis_a_jour";
}

// ---------------------------------------------------------------------------
// Catalogues fermés (aucun texte libre côté destinataires)
// ---------------------------------------------------------------------------

/// Destinataires d'une contrainte publiée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Audience {
    Restauration,
    Enseignants,
    Sorties,
}

impl Audience {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Restauration => "restauration",
            Self::Enseignants => "enseignants",
            Self::Sorties => "sorties",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "restauration" => Some(Self::Restauration),
            "enseignants" => Some(Self::Enseignants),
            "sorties" => Some(Self::Sorties),
            _ => None,
        }
    }
}

/// Entrée du catalogue des contraintes : le libellé est **le seul texte** publié.
pub struct EntreeContrainte {
    pub code: &'static str,
    pub libelle: &'static str,
    pub audiences: &'static [Audience],
}

use Audience::{Enseignants as ENS, Restauration as RST, Sorties as SOR};

/// Contraintes publiables (SAN-06). La contrainte, jamais le diagnostic : aucun
/// libellé ne nomme une maladie, un traitement ou un symptôme (test de minimisation).
pub const CATALOGUE_CONTRAINTES: &[EntreeContrainte] = &[
    EntreeContrainte { code: "pai_alimentaire", libelle: "Projet d'accueil alimentaire : repas adapté", audiences: &[RST] },
    EntreeContrainte { code: "repas_sans_arachide", libelle: "Repas sans arachide", audiences: &[RST] },
    EntreeContrainte { code: "repas_sans_fruits_a_coque", libelle: "Repas sans fruits à coque", audiences: &[RST] },
    EntreeContrainte { code: "repas_sans_lait", libelle: "Repas sans lait ni produits laitiers", audiences: &[RST] },
    EntreeContrainte { code: "repas_sans_oeuf", libelle: "Repas sans œuf", audiences: &[RST] },
    EntreeContrainte { code: "repas_sans_gluten", libelle: "Repas sans gluten", audiences: &[RST] },
    EntreeContrainte { code: "repas_sans_poisson", libelle: "Repas sans poisson ni fruits de mer", audiences: &[RST] },
    EntreeContrainte { code: "repas_panier", libelle: "Repas fourni par la famille", audiences: &[RST] },
    EntreeContrainte { code: "trousse_urgence", libelle: "Trousse d'urgence à emporter", audiences: &[ENS, SOR] },
    EntreeContrainte { code: "ne_pas_courir", libelle: "Ne pas courir", audiences: &[ENS, SOR] },
    EntreeContrainte { code: "pas_effort_intense", libelle: "Pas d'effort physique intense", audiences: &[ENS, SOR] },
    EntreeContrainte { code: "acces_eau_libre", libelle: "Accès libre à l'eau", audiences: &[ENS, SOR] },
    EntreeContrainte { code: "acces_toilettes_libre", libelle: "Accès libre aux toilettes", audiences: &[ENS, SOR] },
    EntreeContrainte { code: "pauses_autorisees", libelle: "Pauses autorisées", audiences: &[ENS] },
    EntreeContrainte { code: "eviter_chaleur", libelle: "Éviter l'exposition prolongée à la chaleur", audiences: &[ENS, SOR] },
    EntreeContrainte { code: "prevenir_infirmerie", libelle: "Prévenir l'infirmerie au moindre doute", audiences: &[ENS, SOR] },
    EntreeContrainte { code: "ascenseur", libelle: "Usage de l'ascenseur", audiences: &[ENS, SOR] },
];

pub fn contrainte_du_catalogue(code: &str) -> Option<&'static EntreeContrainte> {
    CATALOGUE_CONTRAINTES.iter().find(|e| e.code == code)
}

/// Libellé d'un code (le code lui-même s'il n'est plus au catalogue : jamais de texte libre).
fn libelle_contrainte(code: &str) -> String {
    contrainte_du_catalogue(code)
        .map(|e| e.libelle.to_string())
        .unwrap_or_else(|| code.to_string())
}

// ---------------------------------------------------------------------------
// PAI complet
// ---------------------------------------------------------------------------

/// Champs du PAI complet absents du PAI historique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtrasPai {
    /// Échéance de la révision annuelle.
    pub date_revision: Option<NaiveDate>,
    pub trousse_urgence: bool,
    /// Lieu de rangement (« infirmerie », « vie scolaire »…) : non médical.
    pub trousse_lieu: String,
    pub trousse_peremption: Option<NaiveDate>,
}

impl ExtrasPai {
    /// Valeurs à la création d'un PAI sans extras : révision un an après le début.
    pub fn par_defaut(date_debut: NaiveDate) -> Self {
        Self {
            date_revision: Some(un_an_apres(date_debut)),
            trousse_urgence: false,
            trousse_lieu: String::new(),
            trousse_peremption: None,
        }
    }

    pub(crate) fn valider(&self, date_debut: NaiveDate) -> Result<(), EntDbError> {
        if self.trousse_lieu.chars().count() > 80
            || self.trousse_lieu.chars().any(char::is_control)
        {
            return Err(invalide());
        }
        if let Some(r) = self.date_revision {
            if r < date_debut {
                return Err(invalide());
            }
        }
        if self.trousse_urgence && self.trousse_lieu.trim().is_empty() {
            return Err(EntDbError::SaisieInvalide(
                "le lieu de la trousse d'urgence est obligatoire",
            ));
        }
        Ok(())
    }
}

fn un_an_apres(d: NaiveDate) -> NaiveDate {
    d.with_year(d.year() + 1)
        .unwrap_or_else(|| d + chrono::Duration::days(365))
}

/// Parties qui signent un PAI (les quatre premières sont requises).
pub const PARTIES_REQUISES: [&str; 4] = ["famille", "medecin", "chef_etablissement", "infirmier"];
pub const PARTIES_PAI: [&str; 5] = [
    "famille",
    "medecin",
    "chef_etablissement",
    "infirmier",
    "collectivite",
];

#[derive(Debug, Clone, Serialize)]
pub struct SignaturePai {
    pub partie: String,
    pub signataire_nom: String,
    pub signe_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaiComplet {
    pub pai: Pai,
    pub version: i32,
    pub date_revision: Option<NaiveDate>,
    /// Révision échue ou à moins de 30 jours.
    pub revision_due: bool,
    pub trousse_urgence: bool,
    pub trousse_lieu: String,
    pub trousse_peremption: Option<NaiveDate>,
    pub trousse_perimee: bool,
    /// Signatures de la **version courante** uniquement.
    pub signatures: Vec<SignaturePai>,
    pub parties_manquantes: Vec<String>,
    pub complet: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct ExtrasRow {
    version: i32,
    date_revision: Option<NaiveDate>,
    trousse_urgence: bool,
    trousse_lieu: String,
    trousse_peremption: Option<NaiveDate>,
}

/// PAI à réviser (liste de l'infirmerie).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PaiARevoir {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub date_revision: NaiveDate,
    /// Négatif : échue depuis ce nombre de jours.
    pub jours_restants: i32,
}

// ---------------------------------------------------------------------------
// Contraintes publiées
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ContraintePubliee {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub audiences: Vec<String>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    /// `pai` | `pap` | `pps` | `manuel` : visible du seul personnel de santé.
    pub source: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ContrainteRow {
    id: Uuid,
    eleve_id: Uuid,
    code: String,
    audiences: Vec<String>,
    source: String,
    date_debut: NaiveDate,
    date_fin: Option<NaiveDate>,
}

impl ContrainteRow {
    fn vue(self, avec_source: bool) -> ContraintePubliee {
        ContraintePubliee {
            id: self.id,
            eleve_id: self.eleve_id,
            libelle: libelle_contrainte(&self.code),
            code: self.code,
            audiences: self.audiences,
            date_debut: self.date_debut,
            date_fin: self.date_fin,
            source: avec_source.then_some(self.source),
        }
    }
}

/// Contraintes d'un élève pour une liste (restauration, sorties, enseignants).
#[derive(Debug, Clone, Serialize)]
pub struct ContraintesEleve {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub contraintes: Vec<ContraintePubliee>,
}

#[derive(Debug, Clone)]
pub struct PublierContrainte {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    pub audiences: Vec<Audience>,
    pub date_fin: Option<NaiveDate>,
}

// ---------------------------------------------------------------------------
// Passages
// ---------------------------------------------------------------------------

pub const ISSUES: [&str; 6] = [
    "non_renseignee",
    "retour_classe",
    "repos_infirmerie",
    "famille_appelee",
    "renvoi_domicile",
    "secours_appeles",
];

/// Issues qui déclenchent l'information automatique de la famille (règle par défaut [H],
/// à valider par l'établissement) ; un accident aussi.
fn issue_informe_famille(issue: &str) -> bool {
    matches!(issue, "famille_appelee" | "renvoi_domicile" | "secours_appeles")
}

fn doit_informer(motif: &str, issue: &str) -> bool {
    motif == "accident" || issue_informe_famille(issue)
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PassageSante {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub jour: NaiveDate,
    pub heure_arrivee: Option<NaiveTime>,
    pub heure_sortie: Option<NaiveTime>,
    pub motif: String,
    /// Note clinique : jamais exposée à la famille.
    pub note_soins: String,
    pub soins: String,
    pub issue: String,
    pub visible_famille: bool,
    pub famille_information: String,
    pub famille_informee_at: Option<DateTime<Utc>>,
    pub recupere_par_nom: Option<String>,
    pub created_at: DateTime<Utc>,
}

const COLONNES_PASSAGE: &str = "p.id, p.eleve_id, e.nom, e.prenom, e.classe, p.jour, \
    p.heure_arrivee, p.heure_sortie, p.motif, p.note_soins, p.soins, p.issue, p.visible_famille, \
    p.famille_information, p.famille_informee_at, p.recupere_par_nom, p.created_at";

#[derive(Debug, Clone)]
pub struct NouveauPassage {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub heure_arrivee: Option<NaiveTime>,
    pub motif: String,
    pub note_soins: String,
    pub soins: String,
    pub visible_famille: bool,
    pub issue: String,
    /// Informer la famille même si la règle ne l'impose pas.
    pub informer_famille: bool,
}

#[derive(Debug, Clone)]
pub struct ClorePassage {
    pub etablissement_id: Uuid,
    pub passage_id: Uuid,
    pub heure_sortie: Option<NaiveTime>,
    pub issue: String,
    pub soins: Option<String>,
    /// Renvoi au domicile : personne (responsable actif ou contact habilité) venue chercher l'élève.
    pub recupere_par_personne_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Fiche d'urgence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct PersonneUrgence {
    pub personne_id: Uuid,
    pub nom: String,
    pub relation: String,
    pub telephone: Option<String>,
    pub telephone_2: Option<String>,
    pub peut_recuperer: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FicheUrgence {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub date_naissance: Option<NaiveDate>,
    /// `minimale` (encadrant : contacts + contraintes) ou `complete` (personnel de santé).
    pub niveau: String,
    pub genere_le: DateTime<Utc>,
    pub responsables: Vec<PersonneUrgence>,
    pub contacts: Vec<PersonneUrgence>,
    pub contraintes: Vec<ContraintePubliee>,
    pub trousse_urgence: bool,
    pub trousse_lieu: String,
    /// Conduite à tenir générique (niveau minimal et complet).
    pub consigne_generale: String,
    /// Niveau complet uniquement : protocole du PAI et médecin référent.
    pub protocole_urgence: Option<String>,
    pub medecin_nom: Option<String>,
    pub urgence_tel: Option<String>,
}

pub const CONSIGNE_URGENCE: &str = "En cas d'urgence vitale : appeler le 15 (SAMU) ou le 112, \
    puis prévenir sans délai l'établissement et la famille.";

// ---------------------------------------------------------------------------
// Protocoles (registre de l'établissement)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProtocoleSante {
    pub id: Uuid,
    pub titre: String,
    pub categorie: String,
    pub contenu: String,
    pub version: i32,
    pub actif: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct EnregistrerProtocole {
    pub etablissement_id: Uuid,
    pub id: Option<Uuid>,
    pub titre: String,
    pub categorie: String,
    pub contenu: String,
    pub actif: bool,
}

// ---------------------------------------------------------------------------
// Outils
// ---------------------------------------------------------------------------

fn texte(v: &str, max: usize) -> Result<String, EntDbError> {
    let t = v.trim();
    if t.chars().count() > max || t.chars().any(|c| c.is_control() && c != '\n') {
        return Err(invalide());
    }
    Ok(t.to_string())
}

#[allow(clippy::too_many_arguments)]
async fn evenement(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate_id: Uuid,
    eleve_id: Option<Uuid>,
    payload: serde_json::Value,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Infirmerie,
        aggregate_id,
        eleve_id,
        payload,
        actor_user_id: Some(acteur.to_string()),
    })
    .await?;
    Ok(())
}

async fn tenant(pool: &sqlx::PgPool, etab: Uuid) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(pool, etab).await.map_err(db)
}

impl PgEnt {
    async fn est_personnel_sante(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        action: Action,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        self.staff_eleve(etab, acteur, role, action, eleve_id).await
    }

    async fn exiger_sante(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        action: Action,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_eleve_existe(etab, eleve_id).await?;
        if self.est_personnel_sante(etab, acteur, role, action, eleve_id).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    // -----------------------------------------------------------------------
    // PAI (SAN-05)
    // -----------------------------------------------------------------------

    /// Écrit le PAI (création ou mise à jour) : PAI, version, contraintes dérivées et
    /// événement dans **une** transaction. Toute écriture invalide les signatures.
    pub async fn enregistrer_pai_for_actor(
        &self,
        role: &str,
        cmd: UpsertPai,
        extras: Option<ExtrasPai>,
    ) -> Result<Pai, EntDbError> {
        let (etab, acteur, eleve_id, actif) =
            (cmd.etablissement_id, cmd.actor_user_id, cmd.eleve_id, cmd.actif);
        self.exiger_sante(etab, acteur, role, Action::SanteSaisir, eleve_id)
            .await?;
        let planned = plan_upsert_pai(cmd)?;
        if let Some(e) = &extras {
            e.valider(planned.date_debut)?;
        }
        let repo = self.infirmerie_repo();
        let mut tx = tenant(self.pool(), etab).await?;
        let pai = repo.upsert_pai_tx(&mut tx, &planned, extras.as_ref()).await?;
        let (version,): (i32,) =
            sqlx::query_as("SELECT version FROM ent_pai WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(pai.id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        self.synchroniser_contraintes_pai(&mut tx, &pai, acteur).await?;
        // Minimisation : ni motif, ni mesures, ni protocole dans l'événement.
        evenement(
            &mut tx,
            etab,
            types_sante::PAI,
            pai.id,
            Some(eleve_id),
            json!({ "actif": pai.actif, "version": version, "date_debut": pai.date_debut.to_string() }),
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        // Aligne le drapeau de la fiche santé (comportement historique).
        if let Some(mut fiche) = repo.get_fiche(etab, eleve_id).await? {
            if fiche.pai_actif != actif {
                fiche.pai_actif = actif;
                fiche.updated_by_user_id = acteur;
                fiche.updated_at = Utc::now();
                repo.upsert_fiche(&fiche).await?;
            }
        }
        Ok(pai)
    }

    /// La trousse d'urgence du PAI se publie (et se retire) automatiquement ;
    /// un PAI inactif retire toutes les contraintes qu'il a dérivées.
    async fn synchroniser_contraintes_pai(
        &self,
        conn: &mut PgConnection,
        pai: &Pai,
        acteur: Uuid,
    ) -> Result<(), EntDbError> {
        let (trousse,): (bool,) = sqlx::query_as(
            "SELECT trousse_urgence FROM ent_pai WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(pai.etablissement_id)
        .bind(pai.id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if !pai.actif {
            retirer_contraintes_source(conn, pai, None, acteur).await?;
        } else if trousse {
            publier_contrainte_tx(
                conn,
                pai.etablissement_id,
                pai.eleve_id,
                "trousse_urgence",
                &[Audience::Enseignants, Audience::Sorties],
                "pai",
                Some(pai.id),
                pai.date_debut.max(today_utc()),
                pai.date_fin,
                acteur,
            )
            .await?;
        } else {
            retirer_contraintes_source(conn, pai, Some("trousse_urgence"), acteur).await?;
        }
        Ok(())
    }

    /// Signature d'une partie du PAI (version courante). La famille signe depuis son
    /// compte ; médecin et collectivité sont consignés par le personnel de santé ;
    /// la direction signe pour le chef d'établissement.
    pub async fn signer_pai_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        partie: &str,
        signataire_nom: Option<&str>,
    ) -> Result<PaiComplet, EntDbError> {
        if !PARTIES_PAI.contains(&partie) {
            return Err(invalide());
        }
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let nom: String;
        if partie == "famille" {
            if parsed != Role::Parent
                || self.lien_parent_eleve(etab, acteur, eleve_id).await?.is_none()
            {
                return Err(EntDbError::Forbidden);
            }
            let n: Option<(Option<String>,)> =
                sqlx::query_as("SELECT display_name FROM ent_user WHERE id = $1")
                    .bind(acteur)
                    .fetch_optional(self.pool())
                    .await
                    .map_err(db)?;
            nom = n
                .and_then(|(d,)| d)
                .filter(|d| !d.trim().is_empty())
                .unwrap_or_else(|| "Responsable légal".into());
        } else {
            self.exiger_sante(etab, acteur, role, Action::SanteSaisir, eleve_id)
                .await?;
            if partie == "chef_etablissement" && !matches!(parsed, Role::Direction | Role::Admin) {
                return Err(EntDbError::Forbidden);
            }
            nom = texte(signataire_nom.unwrap_or(""), 120)?;
            if nom.is_empty() {
                return Err(invalide());
            }
        }
        let repo = self.infirmerie_repo();
        let mut tx = tenant(self.pool(), etab).await?;
        let pai = repo
            .get_pai_tx(&mut tx, etab, eleve_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !pai.actif {
            return Err(DomainError::PaiInactif.into());
        }
        let (version,): (i32,) =
            sqlx::query_as("SELECT version FROM ent_pai WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(pai.id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        let signature_id = Uuid::new_v4();
        let ins = sqlx::query(
            "INSERT INTO ent_pai_signature
                (id, etablissement_id, eleve_id, pai_id, version, partie, signataire_nom,
                 signataire_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT (pai_id, version, partie) DO NOTHING",
        )
        .bind(signature_id)
        .bind(etab)
        .bind(eleve_id)
        .bind(pai.id)
        .bind(version)
        .bind(partie)
        .bind(&nom)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if ins.rows_affected() == 0 {
            return Err(DomainError::PaiDejaSigne.into());
        }
        if partie == "famille" {
            // Colonnes historiques lues par l'ancienne vue : même signature.
            let signe = plan_signer_pai_famille(
                &pai,
                &SignerPaiFamille {
                    etablissement_id: etab,
                    eleve_id,
                    actor_user_id: acteur,
                },
            );
            if let Ok(s) = signe {
                sqlx::query(
                    "UPDATE ent_pai SET signe_famille_at = $3, signe_famille_user_id = $4
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(pai.id)
                .bind(s.signe_famille_at)
                .bind(s.signe_famille_user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        evenement(
            &mut tx,
            etab,
            types_sante::PAI_SIGNE,
            pai.id,
            Some(eleve_id),
            json!({ "partie": partie, "version": version }),
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.pai_complet_for_actor(etab, acteur, role, eleve_id)
            .await?
            .ok_or_else(introuvable)
    }

    /// PAI et ses compléments, avec les contrôles de lecture du PAI historique
    /// (personnel de santé ; responsable lié ; élève lui-même) et la trace de consultation.
    pub async fn pai_complet_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<PaiComplet>, EntDbError> {
        let Some(pai) = self.get_pai_for_actor(etab, acteur, role, eleve_id).await? else {
            return Ok(None);
        };
        let mut tx = tenant(self.pool(), etab).await?;
        let ex: ExtrasRow = sqlx::query_as(
            "SELECT version, date_revision, trousse_urgence, trousse_lieu, trousse_peremption
               FROM ent_pai WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(pai.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let lignes: Vec<(String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT partie, signataire_nom, signe_at FROM ent_pai_signature
              WHERE etablissement_id = $1 AND pai_id = $2 AND version = $3
              ORDER BY signe_at",
        )
        .bind(etab)
        .bind(pai.id)
        .bind(ex.version)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let signatures: Vec<SignaturePai> = lignes
            .into_iter()
            .map(|(partie, signataire_nom, signe_at)| SignaturePai {
                partie,
                signataire_nom,
                signe_at,
            })
            .collect();
        let manquantes: Vec<String> = PARTIES_REQUISES
            .iter()
            .filter(|p| !signatures.iter().any(|s| s.partie == **p))
            .map(|p| p.to_string())
            .collect();
        let aujourdhui = today_utc();
        Ok(Some(PaiComplet {
            version: ex.version,
            revision_due: ex
                .date_revision
                .is_some_and(|d| d <= aujourdhui + chrono::Duration::days(30)),
            date_revision: ex.date_revision,
            trousse_urgence: ex.trousse_urgence,
            trousse_lieu: ex.trousse_lieu,
            trousse_perimee: ex.trousse_peremption.is_some_and(|d| d < aujourdhui),
            trousse_peremption: ex.trousse_peremption,
            complet: manquantes.is_empty(),
            parties_manquantes: manquantes,
            signatures,
            pai,
        }))
    }

    /// PAI dont la révision est échue ou proche (`horizon_jours`), élèves couverts
    /// par la portée de `sante.lire`.
    pub async fn pai_a_reviser_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        horizon_jours: i32,
    ) -> Result<Vec<PaiARevoir>, EntDbError> {
        if !self.a_action_quelconque(etab, acteur, role, Action::SanteLire).await? {
            return Err(EntDbError::Forbidden);
        }
        let autorises = self.eleves_autorises(etab, acteur, role, Action::SanteLire).await?;
        let horizon = horizon_jours.clamp(0, 365);
        let mut tx = tenant(self.pool(), etab).await?;
        let mut liste: Vec<PaiARevoir> = sqlx::query_as(
            "SELECT p.eleve_id, e.nom, e.prenom, e.classe, p.date_revision,
                    (p.date_revision - CURRENT_DATE)::int AS jours_restants
               FROM ent_pai p JOIN ent_eleve e ON e.id = p.eleve_id AND e.etablissement_id = p.etablissement_id
              WHERE p.etablissement_id = $1 AND p.actif AND p.date_revision IS NOT NULL
                AND p.date_revision <= CURRENT_DATE + $2::int
              ORDER BY p.date_revision, e.nom, e.prenom",
        )
        .bind(etab)
        .bind(horizon)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        if let Some(ens) = autorises {
            liste.retain(|p| ens.contains(&p.eleve_id));
        }
        self.tracer_acces_sensible(
            etab,
            acteur,
            role,
            None,
            "consultation",
            "pai_a_reviser",
            "Consultation de la liste des PAI à réviser",
        )
        .await;
        Ok(liste)
    }

    // -----------------------------------------------------------------------
    // Contraintes publiées (SAN-06)
    // -----------------------------------------------------------------------

    /// Publie (ou met à jour) une contrainte du catalogue vers des audiences.
    pub async fn publier_contrainte_for_actor(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: PublierContrainte,
    ) -> Result<ContraintePubliee, EntDbError> {
        let etab = cmd.etablissement_id;
        self.exiger_sante(etab, acteur, role, Action::SanteSaisir, cmd.eleve_id)
            .await?;
        let entree = contrainte_du_catalogue(&cmd.code).ok_or(EntDbError::SaisieInvalide(
            "contrainte hors catalogue",
        ))?;
        if cmd.audiences.is_empty()
            || cmd.audiences.iter().any(|a| !entree.audiences.contains(a))
        {
            return Err(EntDbError::SaisieInvalide(
                "audience non prévue pour cette contrainte",
            ));
        }
        let debut = today_utc();
        if cmd.date_fin.is_some_and(|f| f < debut) {
            return Err(invalide());
        }
        let mut tx = tenant(self.pool(), etab).await?;
        let id = publier_contrainte_tx(
            &mut tx,
            etab,
            cmd.eleve_id,
            &cmd.code,
            &cmd.audiences,
            "manuel",
            None,
            debut,
            cmd.date_fin,
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ContraintePubliee {
            id,
            eleve_id: cmd.eleve_id,
            libelle: entree.libelle.to_string(),
            code: cmd.code,
            audiences: cmd.audiences.iter().map(|a| a.as_str().to_string()).collect(),
            date_debut: debut,
            date_fin: cmd.date_fin,
            source: Some("manuel".into()),
        })
    }

    pub async fn retirer_contrainte_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        contrainte_id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = tenant(self.pool(), etab).await?;
        let ligne: Option<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_contrainte_publiee
              WHERE etablissement_id = $1 AND id = $2 AND actif",
        )
        .bind(etab)
        .bind(contrainte_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (eleve_id,) = ligne.ok_or_else(introuvable)?;
        if !self
            .est_personnel_sante(etab, acteur, role, Action::SanteSaisir, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        sqlx::query(
            "UPDATE ent_contrainte_publiee SET actif = false, retire_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(contrainte_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        evenement(
            &mut tx,
            etab,
            types_sante::CONTRAINTE_RETIREE,
            contrainte_id,
            Some(eleve_id),
            json!({ "contrainte_id": contrainte_id }),
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Droit de lire les contraintes d'une audience (hors personnel de santé).
    async fn audience_autorisee(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        audience: Audience,
    ) -> Result<bool, EntDbError> {
        Ok(match audience {
            Audience::Restauration => {
                self.a_action_etablissement(etab, acteur, role, Action::RepasGerer).await?
                    || self.a_action_etablissement(etab, acteur, role, Action::RepasScanner).await?
            }
            Audience::Sorties => {
                self.a_action_etablissement(etab, acteur, role, Action::SortiesGerer).await?
                    || self.a_action_etablissement(etab, acteur, role, Action::SortiesControler).await?
            }
            Audience::Enseignants => {
                self.a_action_quelconque(etab, acteur, role, Action::ContraintesLire).await?
            }
        })
    }

    /// Contraintes publiées d'un élève. Le personnel de santé voit tout (avec la
    /// source) ; les autres ne voient que les contraintes de **leur** audience.
    pub async fn contraintes_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        audience: Option<Audience>,
    ) -> Result<Vec<ContraintePubliee>, EntDbError> {
        self.exiger_eleve_existe(etab, eleve_id).await?;
        let sante = self
            .est_personnel_sante(etab, acteur, role, Action::SanteLire, eleve_id)
            .await?;
        let filtre: Option<Audience> = if sante {
            audience
        } else {
            let a = audience.ok_or(EntDbError::Forbidden)?;
            if !self.audience_autorisee(etab, acteur, role, a).await? {
                return Err(EntDbError::Forbidden);
            }
            if a == Audience::Enseignants
                && !self
                    .autoriser_eleve(etab, acteur, role, Action::ContraintesLire, eleve_id)
                    .await?
            {
                return Err(EntDbError::Forbidden);
            }
            Some(a)
        };
        let mut tx = tenant(self.pool(), etab).await?;
        let rows: Vec<ContrainteRow> = sqlx::query_as(
            "SELECT id, eleve_id, code, audiences, source, date_debut, date_fin
               FROM ent_contrainte_publiee
              WHERE etablissement_id = $1 AND eleve_id = $2 AND actif
                AND date_debut <= CURRENT_DATE AND (date_fin IS NULL OR date_fin >= CURRENT_DATE)
                AND ($3::text IS NULL OR $3 = ANY(audiences))
              ORDER BY code",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(filtre.map(Audience::as_str))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows.into_iter().map(|r| r.vue(sante)).collect())
    }

    /// Liste des élèves ayant une contrainte pour l'audience (cantine, sorties,
    /// professeurs : ceux dont l'acteur a la charge).
    pub async fn contraintes_liste_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        audience: Audience,
    ) -> Result<Vec<ContraintesEleve>, EntDbError> {
        if !self.audience_autorisee(etab, acteur, role, audience).await? {
            return Err(EntDbError::Forbidden);
        }
        let limite = if audience == Audience::Enseignants {
            self.eleves_autorises(etab, acteur, role, Action::ContraintesLire).await?
        } else {
            None
        };
        let mut tx = tenant(self.pool(), etab).await?;
        let rows: Vec<(String, String, Option<String>, ContrainteLigne)> = {
            let l: Vec<ContrainteLigne> = sqlx::query_as(
                "SELECT c.id, c.eleve_id, c.code, c.audiences, c.source, c.date_debut, c.date_fin,
                        e.nom, e.prenom, e.classe
                   FROM ent_contrainte_publiee c
                   JOIN ent_eleve e ON e.id = c.eleve_id AND e.etablissement_id = c.etablissement_id
                  WHERE c.etablissement_id = $1 AND c.actif AND $2 = ANY(c.audiences)
                    AND c.date_debut <= CURRENT_DATE
                    AND (c.date_fin IS NULL OR c.date_fin >= CURRENT_DATE)
                  ORDER BY e.nom, e.prenom, c.code",
            )
            .bind(etab)
            .bind(audience.as_str())
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            l.into_iter()
                .map(|x| (x.nom.clone(), x.prenom.clone(), x.classe.clone(), x))
                .collect()
        };
        tx.commit().await.map_err(db)?;
        let mut par_eleve: BTreeMap<(String, String, Uuid), ContraintesEleve> = BTreeMap::new();
        for (nom, prenom, classe, l) in rows {
            if limite.as_ref().is_some_and(|ens| !ens.contains(&l.eleve_id)) {
                continue;
            }
            let eleve_id = l.eleve_id;
            let entree = par_eleve
                .entry((nom.clone(), prenom.clone(), eleve_id))
                .or_insert_with(|| ContraintesEleve {
                    eleve_id,
                    nom,
                    prenom,
                    classe,
                    contraintes: Vec::new(),
                });
            // La source (PAI…) n'est jamais communiquée à ces audiences.
            entree.contraintes.push(
                ContrainteRow {
                    id: l.id,
                    eleve_id: l.eleve_id,
                    code: l.code,
                    audiences: l.audiences,
                    source: l.source,
                    date_debut: l.date_debut,
                    date_fin: l.date_fin,
                }
                .vue(false),
            );
        }
        Ok(par_eleve.into_values().collect())
    }

    // -----------------------------------------------------------------------
    // Passages à l'infirmerie (SAN-02, SAN-03, SAN-04)
    // -----------------------------------------------------------------------

    pub async fn enregistrer_passage_for_actor(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: NouveauPassage,
    ) -> Result<PassageSante, EntDbError> {
        let etab = cmd.etablissement_id;
        self.exiger_sante(etab, acteur, role, Action::SanteSaisir, cmd.eleve_id)
            .await?;
        if domain_eleve::PassageInfirmerieMotif::parse(&cmd.motif).is_none()
            || !ISSUES.contains(&cmd.issue.as_str())
            || cmd.issue == "renvoi_domicile"
        {
            // Le renvoi au domicile se consigne à la clôture, avec la personne qui récupère.
            return Err(invalide());
        }
        let note = texte(&cmd.note_soins, 4000)?;
        let soins = texte(&cmd.soins, 4000)?;
        if note.is_empty() && soins.is_empty() {
            return Err(EntDbError::Domain(DomainError::PassageInfirmerieInvalide));
        }
        let id = Uuid::new_v4();
        let champs = self.infirmerie_repo().champs();
        let note_s = champs.sceller("ent_infirmerie_passage", "note_soins", id, &note)?;
        let soins_s = champs.sceller("ent_infirmerie_passage", "soins", id, &soins)?;
        let informer = cmd.informer_famille || doit_informer(&cmd.motif, &cmd.issue);
        let mut tx = tenant(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO ent_infirmerie_passage
                (id, etablissement_id, eleve_id, jour, motif, note_soins, soins, visible_famille,
                 created_by_user_id, heure_arrivee, issue, famille_information)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'non')",
        )
        .bind(id)
        .bind(etab)
        .bind(cmd.eleve_id)
        .bind(cmd.jour)
        .bind(&cmd.motif)
        .bind(&note_s)
        .bind(&soins_s)
        .bind(cmd.visible_famille)
        .bind(acteur)
        .bind(cmd.heure_arrivee)
        .bind(&cmd.issue)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        evenement(
            &mut tx,
            etab,
            types_sante::PASSAGE,
            id,
            Some(cmd.eleve_id),
            json!({ "jour": cmd.jour.to_string(), "visible_famille": cmd.visible_famille }),
            acteur,
        )
        .await?;
        if informer {
            informer_famille_tx(&mut tx, etab, id, cmd.eleve_id, cmd.jour, acteur).await?;
        }
        tx.commit().await.map_err(db)?;
        self.lire_passage(etab, id).await
    }

    /// Clôture d'un passage : heure de sortie, issue, soins complémentaires. Le renvoi
    /// au domicile exige la personne qui récupère l'élève (responsable actif ou contact
    /// habilité à récupérer).
    pub async fn cloturer_passage_for_actor(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: ClorePassage,
    ) -> Result<PassageSante, EntDbError> {
        let etab = cmd.etablissement_id;
        let courant = self.lire_passage(etab, cmd.passage_id).await?;
        self.exiger_sante(etab, acteur, role, Action::SanteSaisir, courant.eleve_id)
            .await?;
        if !ISSUES.contains(&cmd.issue.as_str()) || cmd.issue == "non_renseignee" {
            return Err(invalide());
        }
        let mut recupere_nom: Option<String> = None;
        if cmd.issue == "renvoi_domicile" {
            let pid = cmd.recupere_par_personne_id.ok_or(EntDbError::SaisieInvalide(
                "la personne qui récupère l'élève est obligatoire",
            ))?;
            recupere_nom = Some(
                self.personne_habilitee_a_recuperer(etab, courant.eleve_id, pid)
                    .await?
                    .ok_or(EntDbError::SaisieInvalide(
                        "personne non habilitée à récupérer l'élève",
                    ))?,
            );
        }
        let champs = self.infirmerie_repo().champs();
        let soins_s = match cmd.soins.as_deref() {
            Some(s) => Some(champs.sceller(
                "ent_infirmerie_passage",
                "soins",
                courant.id,
                &texte(s, 4000)?,
            )?),
            None => None,
        };
        let mut tx = tenant(self.pool(), etab).await?;
        sqlx::query(
            "UPDATE ent_infirmerie_passage SET
                heure_sortie = COALESCE($3, heure_sortie), issue = $4,
                soins = COALESCE($5, soins), recupere_par_nom = $6, recupere_par_personne_id = $7,
                updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(courant.id)
        .bind(cmd.heure_sortie)
        .bind(&cmd.issue)
        .bind(soins_s)
        .bind(&recupere_nom)
        .bind(cmd.recupere_par_personne_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if cmd.issue == "renvoi_domicile" {
            evenement(
                &mut tx,
                etab,
                types_sante::RENVOI_DOMICILE,
                courant.id,
                Some(courant.eleve_id),
                json!({ "passage_id": courant.id, "jour": courant.jour.to_string() }),
                acteur,
            )
            .await?;
        }
        if courant.famille_information != "informee"
            && doit_informer(&courant.motif, &cmd.issue)
        {
            informer_famille_tx(&mut tx, etab, courant.id, courant.eleve_id, courant.jour, acteur)
                .await?;
        }
        tx.commit().await.map_err(db)?;
        self.lire_passage(etab, courant.id).await
    }

    /// SAN-03 : informer la famille (notification sans détail médical), à la demande.
    pub async fn informer_famille_passage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        passage_id: Uuid,
    ) -> Result<PassageSante, EntDbError> {
        let p = self.lire_passage(etab, passage_id).await?;
        self.exiger_sante(etab, acteur, role, Action::SanteSaisir, p.eleve_id)
            .await?;
        if p.famille_information == "informee" {
            return Ok(p);
        }
        let mut tx = tenant(self.pool(), etab).await?;
        informer_famille_tx(&mut tx, etab, p.id, p.eleve_id, p.jour, acteur).await?;
        tx.commit().await.map_err(db)?;
        self.lire_passage(etab, passage_id).await
    }

    /// Nom de la personne si elle peut récupérer l'élève : responsable au lien actif
    /// ou contact d'urgence validé « peut récupérer ».
    async fn personne_habilitee_a_recuperer(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
        personne_id: Uuid,
    ) -> Result<Option<String>, EntDbError> {
        for r in self.responsables_bruts(etab, eleve_id).await? {
            if r.actif && r.personne.id == personne_id {
                return Ok(Some(r.personne.libelle()));
            }
        }
        for c in self.contacts_bruts(etab, eleve_id).await? {
            if c.peut_recuperer && c.statut == "valide" && c.personne.id == personne_id {
                return Ok(Some(c.personne.libelle()));
            }
        }
        Ok(None)
    }

    async fn lire_passage(&self, etab: Uuid, id: Uuid) -> Result<PassageSante, EntDbError> {
        let mut tx = tenant(self.pool(), etab).await?;
        let row: Option<PassageSante> = sqlx::query_as(&format!(
            "SELECT {COLONNES_PASSAGE} FROM ent_infirmerie_passage p
               JOIN ent_eleve e ON e.id = p.eleve_id AND e.etablissement_id = p.etablissement_id
              WHERE p.etablissement_id = $1 AND p.id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut p = row.ok_or_else(introuvable)?;
        self.ouvrir_passage(&mut p)?;
        Ok(p)
    }

    fn ouvrir_passage(&self, p: &mut PassageSante) -> Result<(), EntDbError> {
        let c = self.infirmerie_repo().champs();
        p.note_soins = c.ouvrir("ent_infirmerie_passage", "note_soins", p.id, &p.note_soins)?;
        p.soins = c.ouvrir("ent_infirmerie_passage", "soins", p.id, &p.soins)?;
        Ok(())
    }

    /// Tableau du jour (ou d'un élève) : passages couverts par `sante.lire`. La famille
    /// ne voit que les passages marqués visibles, sans note ni soins.
    pub async fn passages_sante_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Option<Uuid>,
        jour: Option<NaiveDate>,
    ) -> Result<Vec<PassageSante>, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let personnel = match eleve_id {
            Some(e) => self.est_personnel_sante(etab, acteur, role, Action::SanteLire, e).await?,
            None => self.a_action_quelconque(etab, acteur, role, Action::SanteLire).await?,
        };
        if !personnel {
            // Famille / élève : uniquement les passages d'un élève précis qui les concerne.
            let e = eleve_id.ok_or(EntDbError::Forbidden)?;
            let autorise = match parsed {
                Role::Parent => self.lien_parent_eleve(etab, acteur, e).await?.is_some(),
                Role::Eleve => self.eleve_id_for_compte(etab, acteur).await? == Some(e),
                _ => false,
            };
            if !autorise {
                return Err(EntDbError::Forbidden);
            }
        }
        let mut tx = tenant(self.pool(), etab).await?;
        let mut rows: Vec<PassageSante> = sqlx::query_as(&format!(
            "SELECT {COLONNES_PASSAGE} FROM ent_infirmerie_passage p
               JOIN ent_eleve e ON e.id = p.eleve_id AND e.etablissement_id = p.etablissement_id
              WHERE p.etablissement_id = $1
                AND ($2::uuid IS NULL OR p.eleve_id = $2)
                AND ($3::date IS NULL OR p.jour = $3)
              ORDER BY p.jour DESC, p.created_at DESC LIMIT 200"
        ))
        .bind(etab)
        .bind(eleve_id)
        .bind(if eleve_id.is_none() { Some(jour.unwrap_or_else(today_utc)) } else { jour })
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        if personnel {
            if eleve_id.is_none() {
                if let Some(ens) =
                    self.eleves_autorises(etab, acteur, role, Action::SanteLire).await?
                {
                    rows.retain(|p| ens.contains(&p.eleve_id));
                }
            }
            for p in rows.iter_mut() {
                self.ouvrir_passage(p)?;
            }
            self.tracer_acces_sensible(
                etab,
                acteur,
                role,
                eleve_id,
                "consultation",
                "passages_infirmerie",
                "Consultation des passages à l'infirmerie",
            )
            .await;
        } else {
            rows.retain(|p| p.visible_famille);
            for p in rows.iter_mut() {
                p.note_soins.clear();
                p.soins.clear();
                p.recupere_par_nom = None;
            }
        }
        Ok(rows)
    }

    // -----------------------------------------------------------------------
    // Fiche d'urgence
    // -----------------------------------------------------------------------

    /// Fiche d'urgence imprimable. Niveau « complet » : personnel de santé
    /// (`sante.lire`). Niveau « minimal » : encadrants (`sante.urgence_lire`) — contacts,
    /// contraintes publiées, trousse ; ni diagnostic, ni traitement, ni protocole.
    /// Chaque consultation est journalisée.
    pub async fn fiche_urgence_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<FicheUrgence, EntDbError> {
        self.exiger_eleve_existe(etab, eleve_id).await?;
        let complet = self
            .est_personnel_sante(etab, acteur, role, Action::SanteLire, eleve_id)
            .await?;
        if !complet
            && !self
                .staff_eleve(etab, acteur, role, Action::SanteUrgenceLire, eleve_id)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = tenant(self.pool(), etab).await?;
        let (nom, prenom, classe, naissance): (String, String, Option<String>, Option<NaiveDate>) =
            sqlx::query_as(
                "SELECT nom, prenom, classe, date_naissance FROM ent_eleve
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(eleve_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let trousse: Option<(bool, String)> = sqlx::query_as(
            "SELECT trousse_urgence, trousse_lieu FROM ent_pai
              WHERE etablissement_id = $1 AND eleve_id = $2 AND actif",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let contraintes: Vec<ContrainteRow> = sqlx::query_as(
            "SELECT id, eleve_id, code, audiences, source, date_debut, date_fin
               FROM ent_contrainte_publiee
              WHERE etablissement_id = $1 AND eleve_id = $2 AND actif
                AND date_debut <= CURRENT_DATE AND (date_fin IS NULL OR date_fin >= CURRENT_DATE)
              ORDER BY code",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let responsables = self
            .responsables_bruts(etab, eleve_id)
            .await?
            .into_iter()
            .filter(|r| r.actif)
            .map(|r| PersonneUrgence {
                personne_id: r.personne.id,
                nom: r.personne.libelle(),
                relation: r.lien,
                telephone: r.personne.telephone,
                telephone_2: r.personne.telephone_2,
                peut_recuperer: true,
            })
            .collect();
        let contacts = self
            .contacts_bruts(etab, eleve_id)
            .await?
            .into_iter()
            .filter(|c| c.statut == "valide")
            .map(|c| PersonneUrgence {
                personne_id: c.personne.id,
                nom: c.personne.libelle(),
                relation: c.relation,
                telephone: c.personne.telephone,
                telephone_2: c.personne.telephone_2,
                peut_recuperer: c.peut_recuperer,
            })
            .collect();
        let (mut protocole, mut medecin, mut tel) = (None, None, None);
        if complet {
            if let Some(p) = self.infirmerie_repo().get_pai(etab, eleve_id).await? {
                if p.actif && !p.protocole_urgence.is_empty() {
                    protocole = Some(p.protocole_urgence);
                }
            }
            if let Some(f) = self.infirmerie_repo().get_fiche(etab, eleve_id).await? {
                medecin = f.medecin_nom;
                tel = f.urgence_tel;
            }
        }
        self.tracer_acces_sensible(
            etab,
            acteur,
            role,
            Some(eleve_id),
            "consultation",
            "fiche_urgence",
            if complet {
                "Consultation de la fiche d'urgence (complète)"
            } else {
                "Consultation de la fiche d'urgence (minimale)"
            },
        )
        .await;
        let (trousse_ok, trousse_lieu) = trousse.unwrap_or((false, String::new()));
        Ok(FicheUrgence {
            eleve_id,
            nom,
            prenom,
            classe,
            date_naissance: naissance,
            niveau: if complet { "complete" } else { "minimale" }.into(),
            genere_le: Utc::now(),
            responsables,
            contacts,
            contraintes: contraintes.into_iter().map(|c| c.vue(false)).collect(),
            trousse_urgence: trousse_ok,
            trousse_lieu,
            consigne_generale: CONSIGNE_URGENCE.into(),
            protocole_urgence: protocole,
            medecin_nom: medecin,
            urgence_tel: tel,
        })
    }

    // -----------------------------------------------------------------------
    // Protocoles d'urgence (SAN-08)
    // -----------------------------------------------------------------------

    pub async fn protocoles_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<ProtocoleSante>, EntDbError> {
        if !self.a_action_etablissement(etab, acteur, role, Action::SanteLire).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = tenant(self.pool(), etab).await?;
        let l = sqlx::query_as(
            "SELECT id, titre, categorie, contenu, version, actif, updated_at
               FROM ent_sante_protocole WHERE etablissement_id = $1 ORDER BY actif DESC, titre",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l)
    }

    pub async fn enregistrer_protocole_for_actor(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: EnregistrerProtocole,
    ) -> Result<ProtocoleSante, EntDbError> {
        let etab = cmd.etablissement_id;
        self.exiger_action_etab(etab, acteur, role, Action::SanteSaisir).await?;
        let titre = texte(&cmd.titre, 120)?;
        let contenu = texte(&cmd.contenu, 8000)?;
        if titre.is_empty() || !["urgence", "traitement", "autre"].contains(&cmd.categorie.as_str())
        {
            return Err(invalide());
        }
        let mut tx = tenant(self.pool(), etab).await?;
        let id = cmd.id.unwrap_or_else(Uuid::new_v4);
        let p: ProtocoleSante = sqlx::query_as(
            "INSERT INTO ent_sante_protocole
                (id, etablissement_id, titre, categorie, contenu, actif, mis_a_jour_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (id) DO UPDATE SET
                titre = EXCLUDED.titre, categorie = EXCLUDED.categorie,
                contenu = EXCLUDED.contenu, actif = EXCLUDED.actif,
                version = ent_sante_protocole.version + 1,
                mis_a_jour_par_user_id = EXCLUDED.mis_a_jour_par_user_id, updated_at = now()
              WHERE ent_sante_protocole.etablissement_id = EXCLUDED.etablissement_id
             RETURNING id, titre, categorie, contenu, version, actif, updated_at",
        )
        .bind(id)
        .bind(etab)
        .bind(&titre)
        .bind(&cmd.categorie)
        .bind(&contenu)
        .bind(cmd.actif)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(d) if d.is_unique_violation() => {
                EntDbError::ConflitDossier("un protocole porte déjà ce titre")
            }
            _ => EntDbError::Domain(map_sqlx("sante_protocole", e)),
        })?;
        evenement(
            &mut tx,
            etab,
            types_sante::PROTOCOLE_MIS_A_JOUR,
            p.id,
            None,
            json!({ "protocole_id": p.id, "version": p.version }),
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ContrainteLigne {
    id: Uuid,
    eleve_id: Uuid,
    code: String,
    audiences: Vec<String>,
    source: String,
    date_debut: NaiveDate,
    date_fin: Option<NaiveDate>,
    nom: String,
    prenom: String,
    classe: Option<String>,
}

#[allow(clippy::too_many_arguments)]
async fn publier_contrainte_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    code: &str,
    audiences: &[Audience],
    source: &str,
    source_id: Option<Uuid>,
    date_debut: NaiveDate,
    date_fin: Option<NaiveDate>,
    acteur: Uuid,
) -> Result<Uuid, EntDbError> {
    let auds: Vec<String> = audiences.iter().map(|a| a.as_str().to_string()).collect();
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO ent_contrainte_publiee
            (id, etablissement_id, eleve_id, code, audiences, source, source_id, date_debut,
             date_fin, publie_par_user_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
         ON CONFLICT (etablissement_id, eleve_id, code) WHERE actif DO UPDATE SET
            audiences = EXCLUDED.audiences, source = EXCLUDED.source,
            source_id = EXCLUDED.source_id, date_debut = EXCLUDED.date_debut,
            date_fin = EXCLUDED.date_fin, publie_par_user_id = EXCLUDED.publie_par_user_id
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(eleve_id)
    .bind(code)
    .bind(&auds)
    .bind(source)
    .bind(source_id)
    .bind(date_debut)
    .bind(date_fin)
    .bind(acteur)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    // Ni code ni libellé dans l'événement : les destinataires relisent la contrainte
    // avec leurs droits.
    evenement(
        conn,
        etab,
        types_sante::CONTRAINTE_PUBLIEE,
        id,
        Some(eleve_id),
        json!({ "contrainte_id": id, "audiences": auds }),
        acteur,
    )
    .await?;
    Ok(id)
}

async fn retirer_contraintes_source(
    conn: &mut PgConnection,
    pai: &Pai,
    code: Option<&str>,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    let retirees: Vec<(Uuid,)> = sqlx::query_as(
        "UPDATE ent_contrainte_publiee SET actif = false, retire_at = now()
          WHERE etablissement_id = $1 AND eleve_id = $2 AND source = 'pai' AND source_id = $3
            AND actif AND ($4::text IS NULL OR code = $4)
         RETURNING id",
    )
    .bind(pai.etablissement_id)
    .bind(pai.eleve_id)
    .bind(pai.id)
    .bind(code)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (id,) in retirees {
        evenement(
            conn,
            pai.etablissement_id,
            types_sante::CONTRAINTE_RETIREE,
            id,
            Some(pai.eleve_id),
            json!({ "contrainte_id": id }),
            acteur,
        )
        .await?;
    }
    Ok(())
}

/// Marque la famille informée et publie `infirmerie.famille_informee` (la notification
/// est produite par le consommateur d'événements ; elle ne contient aucun détail médical).
async fn informer_famille_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    passage_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_infirmerie_passage SET famille_information = 'informee',
            famille_informee_at = now(), famille_informee_par = $3, updated_at = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(passage_id)
    .bind(acteur)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    evenement(
        conn,
        etab,
        types_sante::FAMILLE_INFORMEE,
        passage_id,
        Some(eleve_id),
        json!({ "passage_id": passage_id, "jour": jour.to_string() }),
        acteur,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimisation : aucun libellé du catalogue ne nomme une pathologie, un
    /// traitement ou un symptôme ; la contrainte, jamais le diagnostic.
    #[test]
    fn catalogue_sans_vocabulaire_medical() {
        const INTERDITS: &[&str] = &[
            "asthme", "diabète", "diabete", "épilep", "epilep", "anaphyl", "allergi", "crise",
            "insuline", "ventoline", "traitement", "médicament", "medicament", "diagnostic",
            "maladie", "handicap", "trouble", "syndrome", "cardiaque", "cancer", "autisme",
            "dys", "tdah", "ordonnance", "épipen", "epipen",
        ];
        let mut vus = std::collections::HashSet::new();
        for e in CATALOGUE_CONTRAINTES {
            assert!(vus.insert(e.code), "code en double : {}", e.code);
            assert!(!e.audiences.is_empty());
            let l = e.libelle.to_lowercase();
            let c = e.code.to_lowercase();
            for m in INTERDITS {
                assert!(!l.contains(m), "libellé « {} » contient « {m} »", e.libelle);
                assert!(!c.contains(m), "code « {} » contient « {m} »", e.code);
            }
        }
    }

    #[test]
    fn regle_information_famille() {
        assert!(doit_informer("accident", "retour_classe"));
        assert!(doit_informer("soins", "famille_appelee"));
        assert!(doit_informer("malaise", "renvoi_domicile"));
        assert!(!doit_informer("soins", "retour_classe"));
        assert!(!doit_informer("malaise", "non_renseignee"));
    }

    #[test]
    fn revision_un_an_apres() {
        let d = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        assert_eq!(un_an_apres(d), NaiveDate::from_ymd_opt(2027, 9, 1).unwrap());
        let bissextile = NaiveDate::from_ymd_opt(2028, 2, 29).unwrap();
        assert_eq!(un_an_apres(bissextile), NaiveDate::from_ymd_opt(2029, 2, 28).unwrap_or(bissextile + chrono::Duration::days(365)));
    }

    #[test]
    fn extras_exigent_un_lieu_pour_la_trousse() {
        let d = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let mut e = ExtrasPai::par_defaut(d);
        assert!(e.valider(d).is_ok());
        e.trousse_urgence = true;
        assert!(e.valider(d).is_err());
        e.trousse_lieu = "infirmerie".into();
        assert!(e.valider(d).is_ok());
        e.date_revision = Some(d - chrono::Duration::days(1));
        assert!(e.valider(d).is_err());
    }
}
