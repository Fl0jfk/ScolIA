//! Socle 7, lot D6 : **service de la brique de signature**, partie 1 — réglages, lancement d'un circuit, gestion
//! (annuler, prolonger, remplacer, refuser, expirer, nouvelle version), lectures (mes documents à signer, parapheur,
//! suivi, pièces de preuve). Les modes de signature (passkey, code, guichet, papier) et la finalisation sont dans
//! `documents_signature_signer.rs`.
//!
//! Règles, toutes appliquées ici (les plus graves redoublées par la base : migration 3705) :
//! * **droits côté serveur** : `documents.signatures.gerer` pour lancer, suivre, annuler, valider un papier ;
//!   `documents.signatures.constater` pour le guichet ; un signataire n'agit que sur **sa** ligne ; réglages par
//!   `documents.modeles.parametrer` ;
//! * **cloisonnement** : tout passe par `TenantTx` (RLS) et chaque requête filtre `etablissement_id` ;
//! * **immuabilité** : un circuit porte sur UNE version ; une nouvelle version (ou l'annulation du document) clôt le
//!   circuit en cours (`obsolete` / `annule`) dans la transaction du changement ; un nouveau circuit demande de
//!   nouvelles signatures ;
//! * **un mineur ne signe pas seul** : un élève mineur ne peut figurer que **avec** ses titulaires (RP1-10) ;
//! * événements `documents.*` **sans titre ni contenu**.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_auth::Action;
use serde_json::json;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::documents_natures::{est_majeur, titulaires_destinataires};
use crate::documents_repo::{journaliser, publier, texte};
use crate::documents_signature::*;
use crate::ref_commun::{db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

pub const EVT_SIGNATURE_DEMANDEE: &str = "documents.signature_demandee";
pub const EVT_SIGNE: &str = "documents.signe";
pub const EVT_SIGNATURE_REFUSEE: &str = "documents.signature_refusee";
pub const EVT_CIRCUIT_CLOS: &str = "documents.circuit_clos";
pub const EVT_CIRCUIT_COMPLET: &str = "documents.circuit_complet";

pub(crate) fn refus(nature: RefusMetier, msg: &'static str) -> EntDbError {
    EntDbError::Refus(nature, msg)
}

pub(crate) fn interdit() -> EntDbError {
    EntDbError::Forbidden
}

// ───────────────────────── Types publiés ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoixPar {
    /// L'émetteur fixe le mode (un seul).
    Emetteur,
    /// Le signataire choisit parmi les modes autorisés (ex. le tuteur de stage).
    Signataire,
}

impl ChoixPar {
    pub fn code(self) -> &'static str {
        match self {
            Self::Emetteur => "emetteur",
            Self::Signataire => "signataire",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Externe {
    pub nom: String,
    pub organisation: Option<String>,
    /// Mobile qui reçoit le code à usage unique (scellé en base).
    pub telephone: String,
}

#[derive(Debug, Clone)]
pub struct SignataireDemande {
    pub role: RoleSignataire,
    pub personne_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub eleve_id: Option<Uuid>,
    pub externe: Option<Externe>,
    /// `None` = le menu de la nature (si le signataire choisit) ou son mode par défaut (si l'émetteur choisit).
    pub modes: Option<Vec<ModeSignature>>,
    pub choix_par: ChoixPar,
    pub requis: bool,
}

impl SignataireDemande {
    fn vide(role: RoleSignataire) -> Self {
        Self {
            role,
            personne_id: None,
            agent_id: None,
            eleve_id: None,
            externe: None,
            modes: None,
            choix_par: ChoixPar::Signataire,
            requis: true,
        }
    }
    pub fn personne(role: RoleSignataire, personne_id: Uuid) -> Self {
        Self { personne_id: Some(personne_id), ..Self::vide(role) }
    }
    pub fn agent(role: RoleSignataire, agent_id: Uuid) -> Self {
        Self { agent_id: Some(agent_id), ..Self::vide(role) }
    }
    pub fn eleve(eleve_id: Uuid) -> Self {
        Self { eleve_id: Some(eleve_id), ..Self::vide(RoleSignataire::Eleve) }
    }
    pub fn externe(e: Externe) -> Self {
        Self { externe: Some(e), ..Self::vide(RoleSignataire::Externe) }
    }
}

#[derive(Debug, Clone)]
pub struct DemandeCircuit {
    pub document_id: Uuid,
    pub ordre: Option<Ordre>,
    pub echeance: Option<NaiveDate>,
    /// Ajoute les titulaires de l'élève sujet (autorité parentale, lien actif non suspendu) et, selon la nature,
    /// l'élève cosignataire ; un élève majeur signe seul.
    pub titulaires_de_l_eleve: bool,
    pub signataires: Vec<SignataireDemande>,
}

#[derive(Debug, Clone)]
pub struct CircuitLance {
    pub circuit_id: Uuid,
    pub signataires: Vec<Uuid>,
    pub horodater: bool,
    /// Jetons de lien des signataires externes, **rendus une seule fois** (jamais stockés), même ordre que `externes`.
    pub liens_externes: Vec<(Uuid, String)>,
}

#[derive(Debug, Clone)]
pub struct VueSignataire {
    pub id: Uuid,
    pub rang: i16,
    pub role: String,
    pub libelle_role: String,
    pub nom: String,
    pub etat: String,
    pub requis: bool,
    pub choix_par: String,
    pub modes: Vec<String>,
    pub niveau_obtenu: Option<String>,
    pub methode: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VueCircuit {
    pub id: Uuid,
    pub document_id: Uuid,
    pub version_id: Uuid,
    pub nature_code: String,
    pub etat: String,
    pub ordre: String,
    pub regle_titulaires: String,
    pub echeance: Option<NaiveDate>,
    pub horodater: bool,
    pub complet: bool,
    pub pdf_signe_sha256: Option<String>,
    pub preuve_sha256: Option<String>,
    pub signataires: Vec<VueSignataire>,
}

#[derive(Debug, Clone)]
pub struct LigneASigner {
    pub signataire_id: Uuid,
    pub circuit_id: Uuid,
    pub document_id: Uuid,
    pub nature_code: String,
    pub titre: String,
    pub role: String,
    pub echeance: Option<NaiveDate>,
    pub modes: Vec<String>,
    pub choix_par: String,
    /// Formule imprimée si la signature est donnée par délégation (parapheur).
    pub formule_delegation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VueReglage {
    pub nature_code: String,
    pub menu: Vec<ChoixMenu>,
    pub horodatage_actif: bool,
    pub service_horodatage: Option<(ModeHorodatage, bool)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceCircuit {
    PdfSigne,
    Preuve,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleveHorodatage {
    pub mutualise_obtenus: i64,
    pub propre_obtenus: i64,
    pub en_attente: i64,
    pub echecs: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RapportVerification {
    pub circuit_id: Uuid,
    pub nb_signatures: usize,
    pub chaine_valide: bool,
    pub rupture: Option<(i32, &'static str)>,
    pub empreinte_version_conforme: bool,
    pub liens_conformes: bool,
    pub tete_chaine: Option<String>,
}

// ───────────────────────── Accès internes ─────────────────────────

pub(crate) struct LigneCircuit {
    pub id: Uuid,
    pub document_id: Uuid,
    pub version_id: Uuid,
    pub nature_code: String,
    pub ordre: String,
    pub regle: String,
    pub echeance: Option<NaiveDate>,
    pub horodater: bool,
    pub etat: String,
}

pub(crate) async fn charger_circuit(
    conn: &mut PgConnection,
    etab: Uuid,
    circuit_id: Uuid,
    verrou: bool,
) -> Result<LigneCircuit, EntDbError> {
    let sql = format!(
        "SELECT id, document_id, version_id, nature_code, ordre, regle_titulaires, echeance, horodater, etat
           FROM ent_doc_circuit WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    let r = sqlx::query(&sql)
        .bind(etab)
        .bind(circuit_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
    Ok(LigneCircuit {
        id: r.get("id"),
        document_id: r.get("document_id"),
        version_id: r.get("version_id"),
        nature_code: r.get("nature_code"),
        ordre: r.get("ordre"),
        regle: r.get("regle_titulaires"),
        echeance: r.get("echeance"),
        horodater: r.get("horodater"),
        etat: r.get("etat"),
    })
}

pub(crate) async fn jour_etab(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1)")
        .bind(etab)
        .fetch_one(conn)
        .await
        .map_err(db)?;
    Ok(j)
}

/// Réglage du menu d'une nature par l'établissement (absent = menu de la nature).
pub(crate) async fn reglage_menu(
    conn: &mut PgConnection,
    etab: Uuid,
    nature: &str,
) -> Result<Option<ReglageMenu>, EntDbError> {
    let lignes: Vec<(String, bool)> = sqlx::query_as(
        "SELECT mode, defaut FROM ent_doc_signature_menu
          WHERE etablissement_id = $1 AND nature_code = $2 ORDER BY rang",
    )
    .bind(etab)
    .bind(nature)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    if lignes.is_empty() {
        return Ok(None);
    }
    let mut modes = Vec::new();
    let mut defaut = None;
    for (m, d) in lignes {
        let m = ModeSignature::parse(&m).ok_or_else(invalide)?;
        if d {
            defaut = Some(m);
        }
        modes.push(m);
    }
    Ok(defaut.map(|defaut| ReglageMenu { modes, defaut }))
}

pub(crate) async fn horodatage_actif(
    conn: &mut PgConnection,
    etab: Uuid,
    n: &NatureSignature,
) -> Result<bool, EntDbError> {
    let r: Option<(bool,)> = sqlx::query_as(
        "SELECT actif FROM ent_doc_horodatage_nature WHERE etablissement_id = $1 AND nature_code = $2",
    )
    .bind(etab)
    .bind(n.code)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.map(|x| x.0).unwrap_or(n.horodatage_defaut))
}

/// Compte (utilisateur) d'un signataire, ou `None` (personne sans compte, externe).
pub(crate) async fn compte_du_signataire(
    conn: &mut PgConnection,
    etab: Uuid,
    signataire_id: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    let r: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT COALESCE(p.user_id, ap.user_id, ec.user_id)
           FROM ent_doc_signataire s
           LEFT JOIN ent_personne p ON p.etablissement_id = s.etablissement_id AND p.id = s.personne_id
           LEFT JOIN ent_agent ag ON ag.etablissement_id = s.etablissement_id AND ag.id = s.agent_id
           LEFT JOIN ent_personne ap ON ap.etablissement_id = ag.etablissement_id AND ap.id = ag.personne_id
           LEFT JOIN ent_eleve_compte ec ON ec.etablissement_id = s.etablissement_id AND ec.eleve_id = s.eleve_id
          WHERE s.etablissement_id = $1 AND s.id = $2",
    )
    .bind(etab)
    .bind(signataire_id)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.and_then(|x| x.0))
}

async fn nom_signataire(conn: &mut PgConnection, etab: Uuid, signataire_id: Uuid) -> Result<String, EntDbError> {
    let r: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT COALESCE(s.externe_nom,
                         NULLIF(btrim(COALESCE(p.prenom, '') || ' ' || p.nom), ''),
                         NULLIF(btrim(COALESCE(ap.prenom, '') || ' ' || ap.nom), ''),
                         NULLIF(btrim(COALESCE(e.prenom, '') || ' ' || e.nom), ''))
           FROM ent_doc_signataire s
           LEFT JOIN ent_personne p ON p.etablissement_id = s.etablissement_id AND p.id = s.personne_id
           LEFT JOIN ent_agent ag ON ag.etablissement_id = s.etablissement_id AND ag.id = s.agent_id
           LEFT JOIN ent_personne ap ON ap.etablissement_id = ag.etablissement_id AND ap.id = ag.personne_id
           LEFT JOIN ent_eleve e ON e.etablissement_id = s.etablissement_id AND e.id = s.eleve_id
          WHERE s.etablissement_id = $1 AND s.id = $2",
    )
    .bind(etab)
    .bind(signataire_id)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.and_then(|x| x.0).unwrap_or_else(|| "—".to_string()))
}

pub(crate) async fn nom_du_signataire(
    conn: &mut PgConnection,
    etab: Uuid,
    signataire_id: Uuid,
) -> Result<String, EntDbError> {
    nom_signataire(conn, etab, signataire_id).await
}

async fn modes_du_signataire(
    conn: &mut PgConnection,
    etab: Uuid,
    signataire_id: Uuid,
) -> Result<Vec<String>, EntDbError> {
    let l: Vec<(String,)> = sqlx::query_as(
        "SELECT mode FROM ent_doc_signataire_mode WHERE etablissement_id = $1 AND signataire_id = $2 ORDER BY mode",
    )
    .bind(etab)
    .bind(signataire_id)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(l.into_iter().map(|x| x.0).collect())
}

/// Une famille accuse ou signe tant que son lien est actif : titulaire encore en droit à la date du jour.
pub(crate) async fn titulaire_en_droit(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    eleve_id: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien l
                         WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.personne_id = $3
                           AND l.autorite_parentale AND l.suspendu_le IS NULL
                           AND (l.date_debut IS NULL OR l.date_debut <= $4)
                           AND (l.date_fin IS NULL OR l.date_fin >= $4))",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(personne_id)
    .bind(jour)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(ok)
}

/// Pose les signataires « invités » : tous en parallèle, le premier rang en séquentiel.
pub(crate) async fn inviter_suivants(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    circuit: &LigneCircuit,
) -> Result<Vec<Uuid>, EntDbError> {
    let attendus: Vec<(Uuid, i16)> = sqlx::query_as(
        "SELECT id, rang FROM ent_doc_signataire
          WHERE etablissement_id = $1 AND circuit_id = $2 AND etat = 'attendu' ORDER BY rang",
    )
    .bind(a.etablissement_id)
    .bind(circuit.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut invites = Vec::new();
    if circuit.ordre == "parallele" {
        for (id, _) in attendus {
            invites.push(id);
        }
    } else {
        // Séquentiel : on invite le prochain seulement si plus personne de rang inférieur n'est en cours.
        let (en_cours,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_doc_signataire
              WHERE etablissement_id = $1 AND circuit_id = $2 AND etat IN ('invite', 'papier_depose')
                AND (requis OR role = 'titulaire')",
        )
        .bind(a.etablissement_id)
        .bind(circuit.id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if en_cours == 0 {
            if let Some((id, _)) = attendus.first() {
                invites.push(*id);
            }
        }
    }
    for id in &invites {
        sqlx::query(
            "UPDATE ent_doc_signataire SET etat = 'invite' WHERE etablissement_id = $1 AND id = $2 AND etat = 'attendu'",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(invites)
}

/// Clôt un circuit ouvert (motif obligatoire pour refus, annulation, obsolescence) et retire les signataires restants.
pub(crate) async fn clore_circuit(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    circuit: &LigneCircuit,
    nouvel_etat: &str,
    motif: &str,
    eleve_id: Option<Uuid>,
) -> Result<MetierEventLie, EntDbError> {
    sqlx::query(
        "UPDATE ent_doc_circuit SET etat = $3, clos_le = now(), clos_motif = $4
          WHERE etablissement_id = $1 AND id = $2 AND etat = 'ouvert'",
    )
    .bind(a.etablissement_id)
    .bind(circuit.id)
    .bind(nouvel_etat)
    .bind(motif)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_doc_signataire SET etat = 'retire'
          WHERE etablissement_id = $1 AND circuit_id = $2 AND etat IN ('attendu', 'invite', 'papier_depose')",
    )
    .bind(a.etablissement_id)
    .bind(circuit.id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journaliser(
        conn,
        a,
        Some(circuit.document_id),
        None,
        "circuit_clos",
        (Some("ouvert"), Some(nouvel_etat)),
        Some(&circuit.id.to_string()),
        Some(motif),
    )
    .await?;
    publier(
        conn,
        a,
        EVT_CIRCUIT_CLOS,
        circuit.document_id,
        eleve_id,
        json!({ "circuit_id": circuit.id, "etat": nouvel_etat, "nature": circuit.nature_code }),
    )
    .await
}

pub(crate) type MetierEventLie = ent_events::MetierEvent;

/// Appelé dans la transaction d'un changement du document : une **nouvelle version** rend le circuit ouvert
/// `obsolete` (D27), l'**annulation** du document le clôt `annule`. Le motif porte le fait.
pub(crate) async fn clore_circuits_du_document_tx(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    document_id: Uuid,
    nouvel_etat: &str,
    motif: &str,
    eleve_id: Option<Uuid>,
) -> Result<Vec<MetierEventLie>, EntDbError> {
    let ids: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_doc_circuit WHERE etablissement_id = $1 AND document_id = $2 AND etat = 'ouvert' FOR UPDATE",
    )
    .bind(a.etablissement_id)
    .bind(document_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut evts = Vec::new();
    for (id,) in ids {
        let c = charger_circuit(conn, a.etablissement_id, id, false).await?;
        evts.push(clore_circuit(conn, a, &c, nouvel_etat, motif, eleve_id).await?);
    }
    Ok(evts)
}

// ───────────────────────── Service ─────────────────────────

impl PgEnt {
    // ─────────────── Réglages ───────────────

    /// Règle le menu des modes (1 à 4) et/ou l'horodatage d'une nature, **à l'intérieur de ce que la nature admet**.
    pub async fn parametrer_signature_nature_for_actor(
        &self,
        a: &Acteur<'_>,
        nature_code: &str,
        menu_modes: Option<ReglageMenu>,
        horodatage: Option<bool>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsModelesParametrer).await?;
        let n = nature_signature(nature_code)
            .ok_or_else(|| refus(RefusMetier::Invalide, "nature sans circuit de signature"))?;
        if let Some(r) = &menu_modes {
            valider_reglage(n, r).map_err(|e| match e {
                ErreurMode::Bloque(m) => refus(RefusMetier::Invalide, m),
                ErreurMode::NombreInvalide => refus(RefusMetier::Invalide, "le menu compte de 1 à 4 modes"),
                ErreurMode::Doublon => refus(RefusMetier::Invalide, "mode en double"),
                _ => refus(RefusMetier::Invalide, "le mode par défaut doit figurer au menu"),
            })?;
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        if let Some(r) = &menu_modes {
            sqlx::query("DELETE FROM ent_doc_signature_menu WHERE etablissement_id = $1 AND nature_code = $2")
                .bind(a.etablissement_id)
                .bind(nature_code)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            for (i, m) in r.modes.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO ent_doc_signature_menu (etablissement_id, nature_code, mode, rang, defaut, maj_par)
                     VALUES ($1,$2,$3,$4,$5,$6)",
                )
                .bind(a.etablissement_id)
                .bind(nature_code)
                .bind(m.code())
                .bind(i as i16 + 1)
                .bind(*m == r.defaut)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            journaliser(&mut tx, a, None, None, "signature_menu_modifie", (None, None), Some(nature_code), None).await?;
        }
        if let Some(actif) = horodatage {
            sqlx::query(
                "INSERT INTO ent_doc_horodatage_nature (etablissement_id, nature_code, actif, maj_par)
                 VALUES ($1,$2,$3,$4)
                 ON CONFLICT (etablissement_id, nature_code) DO UPDATE SET actif = EXCLUDED.actif, maj_par = EXCLUDED.maj_par, maj_le = now()",
            )
            .bind(a.etablissement_id)
            .bind(nature_code)
            .bind(actif)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            journaliser(
                &mut tx,
                a,
                None,
                None,
                "horodatage_nature_modifie",
                (None, Some(if actif { "actif" } else { "inactif" })),
                Some(nature_code),
                None,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Réglage actuel d'une nature : menu avec avertissements, horodatage, service d'horodatage.
    pub async fn reglage_signature_for_actor(&self, a: &Acteur<'_>, nature_code: &str) -> Result<VueReglage, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsModelesParametrer).await?;
        let n = nature_signature(nature_code)
            .ok_or_else(|| refus(RefusMetier::Invalide, "nature sans circuit de signature"))?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let r = reglage_menu(&mut tx, a.etablissement_id, nature_code).await?;
        let actif = horodatage_actif(&mut tx, a.etablissement_id, n).await?;
        let svc: Option<(String, bool)> = sqlx::query_as(
            "SELECT mode, actif FROM ent_doc_horodatage_service WHERE etablissement_id = $1",
        )
        .bind(a.etablissement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(VueReglage {
            nature_code: nature_code.to_string(),
            menu: menu(n, r.as_ref()),
            horodatage_actif: actif,
            service_horodatage: svc.and_then(|(m, a)| ModeHorodatage::parse(&m).map(|m| (m, a))),
        })
    }

    /// Branche (ou débranche) le service d'horodatage RFC 3161 de l'établissement. **Désactivé par défaut.**
    /// Mode « propre » : adresse HTTPS, identifiant et secret (scellé) ; mode « mutualisé » : le compte de ScolIA.
    pub async fn parametrer_service_horodatage_for_actor(
        &self,
        a: &Acteur<'_>,
        mode: ModeHorodatage,
        actif: bool,
        propre: Option<ConfigServiceHorodatage>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsModelesParametrer).await?;
        let (url, identifiant, secret) = match (mode, propre) {
            (ModeHorodatage::Mutualise, _) => (None, None, None),
            (ModeHorodatage::Propre, Some(c)) => {
                if !c.url.starts_with("https://") || c.url.len() > 300 {
                    return Err(refus(RefusMetier::Invalide, "adresse du service d'horodatage : HTTPS exigé"));
                }
                (Some(c.url), c.identifiant, c.secret)
            }
            (ModeHorodatage::Propre, None) => {
                return Err(refus(RefusMetier::Invalide, "adresse du service d'horodatage manquante"))
            }
        };
        let scelle = match &secret {
            Some(s) if !s.is_empty() => Some(
                ChampsSensibles::depuis_env()
                    .sceller("ent_doc_horodatage_service", "secret_scelle", a.etablissement_id, s)
                    .map_err(EntDbError::Domain)?,
            ),
            _ => None,
        };
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_doc_horodatage_service (etablissement_id, mode, actif, url, identifiant, secret_scelle, maj_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (etablissement_id) DO UPDATE
               SET mode = EXCLUDED.mode, actif = EXCLUDED.actif, url = EXCLUDED.url, identifiant = EXCLUDED.identifiant,
                   secret_scelle = EXCLUDED.secret_scelle, maj_par = EXCLUDED.maj_par, maj_le = now()",
        )
        .bind(a.etablissement_id)
        .bind(mode.code())
        .bind(actif)
        .bind(url)
        .bind(identifiant)
        .bind(scelle)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            a,
            None,
            None,
            "service_horodatage_modifie",
            (None, Some(if actif { "actif" } else { "inactif" })),
            Some(mode.code()),
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// **Relevé de consommation** d'horodatage de l'établissement, sur une période (jours de l'établissement,
    /// bornes incluses).
    pub async fn releve_horodatage_for_actor(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<ReleveHorodatage, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsModelesParametrer).await?;
        if au < du {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let r = sqlx::query(
            "SELECT count(*) FILTER (WHERE etat = 'obtenu' AND mode = 'mutualise') AS mutu,
                    count(*) FILTER (WHERE etat = 'obtenu' AND mode = 'propre') AS propre,
                    count(*) FILTER (WHERE etat = 'en_attente') AS attente,
                    count(*) FILTER (WHERE etat = 'echec') AS echec
               FROM ent_doc_horodatage
              WHERE etablissement_id = $1
                AND ent_aujourdhui_etab(etablissement_id, demande_le) BETWEEN $2 AND $3",
        )
        .bind(a.etablissement_id)
        .bind(du)
        .bind(au)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(ReleveHorodatage {
            mutualise_obtenus: r.get("mutu"),
            propre_obtenus: r.get("propre"),
            en_attente: r.get("attente"),
            echecs: r.get("echec"),
        })
    }

    // ─────────────── Lancement ───────────────

    /// **Lance un circuit** sur la version courante d'un document émis (DCT-59). Droit `documents.signatures.gerer`.
    pub async fn lancer_circuit_for_actor(&self, a: &Acteur<'_>, d: DemandeCircuit) -> Result<CircuitLance, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsSignaturesGerer).await?;
        self.lancer_circuit_service(a, d).await
    }

    /// Entrée **des modules** (voyages, stages, inscriptions, discipline, RH…) : ils ont déjà contrôlé leur propre
    /// droit métier ; la brique contrôle le reste (document émis, nature déclarée, menu, signataires, mineurs).
    /// Ne s'appelle jamais depuis une route sans contrôle de droit propre du module.
    pub async fn lancer_circuit_service(&self, a: &Acteur<'_>, d: DemandeCircuit) -> Result<CircuitLance, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        let l = r_doc(&mut tx, a.etablissement_id, d.document_id).await?;
        let n = nature_signature(&l.nature_code)
            .ok_or_else(|| refus(RefusMetier::Invalide, "cette nature de document ne se signe pas"))?;
        if l.etat != "emis" || l.version_id.is_none() {
            return Err(refus(RefusMetier::Conflit, "seul un document émis se signe"));
        }
        let version_id = l.version_id.ok_or_else(introuvable)?;
        let (deja,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_doc_circuit WHERE etablissement_id = $1 AND version_id = $2 AND etat = 'ouvert'",
        )
        .bind(a.etablissement_id)
        .bind(version_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja > 0 {
            return Err(refus(RefusMetier::Conflit, "un circuit est déjà ouvert sur cette version"));
        }
        let reglage = reglage_menu(&mut tx, a.etablissement_id, n.code).await?;
        let menu_eff = menu(n, reglage.as_ref());
        let defaut = menu_eff.iter().find(|c| c.defaut).map(|c| c.mode).ok_or_else(invalide)?;
        let horodater = horodatage_actif(&mut tx, a.etablissement_id, n).await?;

        // Signataires : ceux de l'émetteur, puis ceux que la brique calcule.
        let mut tous: Vec<SignataireDemande> = Vec::new();
        let mut eleve_majeur = false;
        if d.titulaires_de_l_eleve {
            let eleve = l.eleve_id.ok_or_else(|| refus(RefusMetier::Invalide, "ce document n'a pas d'élève"))?;
            let naissance: Option<(Option<NaiveDate>,)> =
                sqlx::query_as("SELECT date_naissance FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                    .bind(a.etablissement_id)
                    .bind(eleve)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
            eleve_majeur = naissance.and_then(|x| x.0).is_some_and(|nn| est_majeur(nn, jour));
            if eleve_majeur {
                tous.push(SignataireDemande::eleve(eleve));
            } else {
                if n.regle != RegleAccord::Aucun {
                    let titulaires = titulaires_destinataires(&mut tx, a.etablissement_id, eleve, jour).await?;
                    if titulaires.is_empty() {
                        return Err(refus(RefusMetier::Conflit, "aucun titulaire en droit de signer pour cet élève"));
                    }
                    for t in titulaires {
                        if let Some(p) = t.personne_id {
                            tous.push(SignataireDemande::personne(RoleSignataire::Titulaire, p));
                        }
                    }
                }
                if n.eleve_cosigne {
                    tous.push(SignataireDemande::eleve(eleve));
                }
            }
        }
        tous.extend(d.signataires.iter().cloned());
        if tous.is_empty() || tous.len() > 20 {
            return Err(refus(RefusMetier::Invalide, "un circuit compte de 1 à 20 signataires"));
        }

        // Un mineur ne signe jamais seul : s'il figure au circuit, au moins un titulaire doit figurer aussi (RP1-10).
        for s in &tous {
            if s.role == RoleSignataire::Eleve {
                let eleve = s.eleve_id.ok_or_else(invalide)?;
                let naissance: Option<(Option<NaiveDate>,)> =
                    sqlx::query_as("SELECT date_naissance FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                        .bind(a.etablissement_id)
                        .bind(eleve)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(db)?;
                let majeur = naissance.and_then(|x| x.0).is_some_and(|nn| est_majeur(nn, jour));
                let a_titulaire = tous.iter().any(|t| t.role == RoleSignataire::Titulaire);
                if !majeur && !a_titulaire {
                    return Err(refus(RefusMetier::Conflit, "un élève mineur ne signe pas seul : ses titulaires signent avec lui"));
                }
            }
        }
        let _ = eleve_majeur;

        let ordre = d.ordre.unwrap_or(n.ordre);
        let echeance = d.echeance.unwrap_or_else(|| jour + Duration::days(n.echeance_jours));
        if echeance < jour {
            return Err(refus(RefusMetier::Invalide, "échéance passée"));
        }
        let circuit_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_doc_circuit (id, etablissement_id, document_id, version_id, nature_code, ordre, regle_titulaires,
                                          eleve_cosigne, echeance, horodater, lance_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(circuit_id)
        .bind(a.etablissement_id)
        .bind(d.document_id)
        .bind(version_id)
        .bind(n.code)
        .bind(ordre.code())
        .bind(n.regle.code())
        .bind(n.eleve_cosigne)
        .bind(echeance)
        .bind(horodater)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;

        let coffre = ChampsSensibles::depuis_env();
        let mut ids = Vec::new();
        let mut liens = Vec::new();
        for (i, s) in tous.iter().enumerate() {
            // Un seul porteur d'identité, cohérent avec le rôle.
            let porteurs = [s.personne_id.is_some(), s.agent_id.is_some(), s.eleve_id.is_some(), s.externe.is_some()];
            if porteurs.iter().filter(|x| **x).count() != 1 {
                return Err(refus(RefusMetier::Invalide, "un signataire est une personne, un agent, un élève ou un externe"));
            }
            let modes: Vec<ModeSignature> = match (&s.modes, s.choix_par) {
                (Some(m), _) => m.clone(),
                (None, ChoixPar::Emetteur) => vec![defaut],
                (None, ChoixPar::Signataire) => menu_eff
                    .iter()
                    .map(|c| c.mode)
                    // Un externe n'a pas de compte : jamais de passkey par défaut.
                    .filter(|m| s.role != RoleSignataire::Externe || *m != ModeSignature::Passkey)
                    .collect(),
            };
            if modes.is_empty() || modes.len() > 4 {
                return Err(refus(RefusMetier::Invalide, "1 à 4 modes par signataire"));
            }
            if s.choix_par == ChoixPar::Emetteur && modes.len() != 1 {
                return Err(refus(RefusMetier::Invalide, "quand l'émetteur choisit, un seul mode"));
            }
            for (j, m) in modes.iter().enumerate() {
                if modes[..j].contains(m) {
                    return Err(refus(RefusMetier::Invalide, "mode en double"));
                }
                evaluer_mode(n, &menu_eff, *m).map_err(|e| match e {
                    ErreurMode::Bloque(msg) => refus(RefusMetier::Conflit, msg),
                    _ => refus(RefusMetier::Invalide, "mode hors du menu de cette nature"),
                })?;
            }
            if s.role == RoleSignataire::Externe && modes.contains(&ModeSignature::Passkey) && s.personne_id.is_none() {
                return Err(refus(RefusMetier::Invalide, "un externe sans compte ne signe pas par passkey"));
            }
            if s.role == RoleSignataire::Etablissement || s.role == RoleSignataire::Agent {
                let agent = s.agent_id.ok_or_else(|| refus(RefusMetier::Invalide, "agent attendu"))?;
                let (ok,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_agent WHERE etablissement_id = $1 AND id = $2)",
                )
                .bind(a.etablissement_id)
                .bind(agent)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if !ok {
                    return Err(introuvable());
                }
            }
            let sid = Uuid::new_v4();
            let (externe_nom, externe_org, contact) = match &s.externe {
                Some(e) => {
                    let nom = texte(&e.nom, 2, 120)?;
                    let tel = e.telephone.trim();
                    if tel.chars().filter(char::is_ascii_digit).count() < 8 {
                        return Err(refus(RefusMetier::Invalide, "mobile de l'externe invalide"));
                    }
                    let scelle = coffre
                        .sceller("ent_doc_signataire", "externe_contact_scelle", sid, tel)
                        .map_err(EntDbError::Domain)?;
                    (Some(nom), e.organisation.clone(), Some(scelle))
                }
                None => (None, None, None),
            };
            // Jeton du lien d'un externe : aléatoire, stocké seulement haché.
            let (jeton_sha, expire) = if s.role == RoleSignataire::Externe {
                let jeton = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
                let h = sha256_hex(jeton.as_bytes());
                liens.push((sid, jeton));
                let exp: DateTime<Utc> = Utc::now() + Duration::days((echeance - jour).num_days().max(1) + 1);
                (Some(h), Some(exp))
            } else {
                (None, None)
            };
            sqlx::query(
                "INSERT INTO ent_doc_signataire
                    (id, etablissement_id, circuit_id, rang, role, libelle_role, personne_id, agent_id, eleve_id,
                     externe_nom, externe_organisation, externe_contact_scelle, choix_par, requis, lien_jeton_sha256, lien_expire_le)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
            )
            .bind(sid)
            .bind(a.etablissement_id)
            .bind(circuit_id)
            .bind(i as i16 + 1)
            .bind(s.role.code())
            .bind(s.role.libelle())
            .bind(s.personne_id)
            .bind(s.agent_id)
            .bind(s.eleve_id)
            .bind(externe_nom)
            .bind(externe_org)
            .bind(contact)
            .bind(s.choix_par.code())
            .bind(s.requis)
            .bind(jeton_sha)
            .bind(expire)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            for m in &modes {
                sqlx::query(
                    "INSERT INTO ent_doc_signataire_mode (etablissement_id, signataire_id, mode) VALUES ($1,$2,$3)",
                )
                .bind(a.etablissement_id)
                .bind(sid)
                .bind(m.code())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
            ids.push(sid);
        }

        let circuit = charger_circuit(&mut tx, a.etablissement_id, circuit_id, false).await?;
        let invites = inviter_suivants(&mut tx, a, &circuit).await?;
        journaliser(
            &mut tx,
            a,
            Some(d.document_id),
            None,
            "circuit_lance",
            (None, Some("ouvert")),
            Some(&circuit_id.to_string()),
            None,
        )
        .await?;
        let mut evts = Vec::new();
        for sid in invites {
            evts.push(
                publier(
                    &mut tx,
                    a,
                    EVT_SIGNATURE_DEMANDEE,
                    d.document_id,
                    l.eleve_id,
                    json!({ "circuit_id": circuit_id, "signataire_id": sid, "nature": n.code }),
                )
                .await?,
            );
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evts);
        Ok(CircuitLance { circuit_id, signataires: ids, horodater, liens_externes: liens })
    }

    // ─────────────── Gestion ───────────────

    /// Annule un circuit ouvert (DCT-73) : motif obligatoire, signataires retirés.
    pub async fn annuler_circuit_for_actor(&self, a: &Acteur<'_>, circuit_id: Uuid, motif: &str) -> Result<(), EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsSignaturesGerer).await?;
        let motif = texte(motif, 3, 500)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let c = charger_circuit(&mut tx, a.etablissement_id, circuit_id, true).await?;
        if c.etat != "ouvert" {
            return Err(refus(RefusMetier::Conflit, "circuit déjà clos"));
        }
        let ev = clore_circuit(&mut tx, a, &c, "annule", &motif, None).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    /// Prolonge l'échéance d'un circuit ouvert (DCT-75).
    pub async fn prolonger_circuit_for_actor(
        &self,
        a: &Acteur<'_>,
        circuit_id: Uuid,
        nouvelle_echeance: NaiveDate,
    ) -> Result<(), EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsSignaturesGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        let c = charger_circuit(&mut tx, a.etablissement_id, circuit_id, true).await?;
        if c.etat != "ouvert" {
            return Err(refus(RefusMetier::Conflit, "circuit déjà clos"));
        }
        if nouvelle_echeance < jour || c.echeance.is_some_and(|e| nouvelle_echeance <= e) {
            return Err(refus(RefusMetier::Invalide, "la nouvelle échéance doit être future et postérieure à l'ancienne"));
        }
        sqlx::query("UPDATE ent_doc_circuit SET echeance = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(circuit_id)
            .bind(nouvelle_echeance)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journaliser(
            &mut tx,
            a,
            Some(c.document_id),
            None,
            "circuit_prolonge",
            (None, None),
            Some(&circuit_id.to_string()),
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Remplace un signataire non encore signé (DCT-74) : motif obligatoire, jamais par le signataire lui-même.
    pub async fn remplacer_signataire_for_actor(
        &self,
        a: &Acteur<'_>,
        signataire_id: Uuid,
        nouveau: SignataireDemande,
        motif: &str,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsSignaturesGerer).await?;
        let motif = texte(motif, 3, 500)?;
        if nouveau.role == RoleSignataire::Externe {
            return Err(refus(RefusMetier::Invalide, "remplacement d'un externe : lancer un nouveau circuit"));
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let ancien: Option<(Uuid, i16, String, String, String, bool, String)> = sqlx::query_as(
            "SELECT circuit_id, rang, role, libelle_role, etat, requis, choix_par
               FROM ent_doc_signataire WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(signataire_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (circuit_id, rang, role, libelle, etat, requis, choix) = ancien.ok_or_else(introuvable)?;
        let c = charger_circuit(&mut tx, a.etablissement_id, circuit_id, true).await?;
        if c.etat != "ouvert" || !matches!(etat.as_str(), "attendu" | "invite" | "papier_depose") {
            return Err(refus(RefusMetier::Conflit, "ce signataire ne peut plus être remplacé"));
        }
        if compte_du_signataire(&mut tx, a.etablissement_id, signataire_id).await? == Some(a.user_id) {
            return Err(interdit());
        }
        if nouveau.role.code() != role {
            return Err(refus(RefusMetier::Invalide, "le remplaçant garde le rôle"));
        }
        let modes = modes_du_signataire(&mut tx, a.etablissement_id, signataire_id).await?;
        sqlx::query("UPDATE ent_doc_signataire SET etat = 'remplace' WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(signataire_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let nid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_doc_signataire
                (id, etablissement_id, circuit_id, rang, role, libelle_role, personne_id, agent_id, eleve_id, choix_par, requis, etat)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(nid)
        .bind(a.etablissement_id)
        .bind(circuit_id)
        .bind(rang)
        .bind(&role)
        .bind(&libelle)
        .bind(nouveau.personne_id)
        .bind(nouveau.agent_id)
        .bind(nouveau.eleve_id)
        .bind(&choix)
        .bind(requis)
        .bind(&etat)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for m in modes {
            sqlx::query("INSERT INTO ent_doc_signataire_mode (etablissement_id, signataire_id, mode) VALUES ($1,$2,$3)")
                .bind(a.etablissement_id)
                .bind(nid)
                .bind(m)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        journaliser(
            &mut tx,
            a,
            Some(c.document_id),
            None,
            "signataire_remplace",
            (Some(&etat), Some(&etat)),
            Some(&nid.to_string()),
            Some(&motif),
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            EVT_SIGNATURE_DEMANDEE,
            c.document_id,
            None,
            json!({ "circuit_id": circuit_id, "signataire_id": nid, "nature": c.nature_code }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(nid)
    }

    /// **Refus** d'un signataire sur sa propre ligne (DCT-68). Le circuit passe `refuse` si ce signataire est requis ou
    /// si la règle des titulaires ne peut plus être satisfaite.
    pub async fn refuser_signature_for_actor(
        &self,
        a: &Acteur<'_>,
        signataire_id: Uuid,
        motif: Option<&str>,
    ) -> Result<bool, EntDbError> {
        let motif = match motif.map(str::trim).filter(|m| !m.is_empty()) {
            Some(m) => Some(texte(m, 3, 500)?),
            None => None,
        };
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let s: Option<(Uuid, String, String, bool)> = sqlx::query_as(
            "SELECT circuit_id, role, etat, requis FROM ent_doc_signataire
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(signataire_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (circuit_id, role, etat, requis) = s.ok_or_else(introuvable)?;
        if compte_du_signataire(&mut tx, a.etablissement_id, signataire_id).await? != Some(a.user_id) {
            return Err(interdit());
        }
        let c = charger_circuit(&mut tx, a.etablissement_id, circuit_id, true).await?;
        if c.etat != "ouvert" || etat != "invite" {
            return Err(refus(RefusMetier::Conflit, "ce document n'attend plus votre signature"));
        }
        sqlx::query(
            "UPDATE ent_doc_signataire SET etat = 'refuse', refus_motif = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(signataire_id)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Le circuit est-il encore satisfiable ?
        let bloquant = if role == "titulaire" {
            match c.regle.as_str() {
                "tous" => true,
                _ => {
                    let (reste,): (i64,) = sqlx::query_as(
                        "SELECT count(*) FROM ent_doc_signataire
                          WHERE etablissement_id = $1 AND circuit_id = $2 AND role = 'titulaire'
                            AND etat IN ('attendu', 'invite', 'papier_depose', 'signe')",
                    )
                    .bind(a.etablissement_id)
                    .bind(circuit_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                    reste == 0
                }
            }
        } else {
            requis
        };
        journaliser(
            &mut tx,
            a,
            Some(c.document_id),
            None,
            "signature_refusee",
            (Some("invite"), Some("refuse")),
            Some(&signataire_id.to_string()),
            motif.as_deref(),
        )
        .await?;
        let mut evts = vec![
            publier(
                &mut tx,
                a,
                EVT_SIGNATURE_REFUSEE,
                c.document_id,
                None,
                json!({ "circuit_id": circuit_id, "signataire_id": signataire_id, "nature": c.nature_code }),
            )
            .await?,
        ];
        if bloquant {
            evts.push(clore_circuit(&mut tx, a, &c, "refuse", "refus d'un signataire requis", None).await?);
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evts);
        Ok(bloquant)
    }

    /// Expire les circuits ouverts dont l'échéance est strictement antérieure à `jour` (tâche de fond ; le jour est
    /// passé par l'appelant, jamais lu ici). Retourne le nombre de circuits expirés.
    pub async fn expirer_circuits_echus(&self, etab: Uuid, jour: NaiveDate) -> Result<usize, EntDbError> {
        let a = Acteur { etablissement_id: etab, user_id: Uuid::nil(), role: "systeme" };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_doc_circuit WHERE etablissement_id = $1 AND etat = 'ouvert' AND echeance < $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut evts = Vec::new();
        for (id,) in &ids {
            let c = charger_circuit(&mut tx, etab, *id, false).await?;
            evts.push(clore_circuit(&mut tx, &a, &c, "expire", "échéance dépassée", None).await?);
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evts);
        Ok(ids.len())
    }

    // ─────────────── Lectures ───────────────

    /// Un circuit et ses signataires : le personnel (`gerer`) ou l'un de ses signataires.
    pub async fn circuit_for_actor(&self, a: &Acteur<'_>, circuit_id: Uuid) -> Result<VueCircuit, EntDbError> {
        let gere = self.exiger_action_documents(a, Action::DocumentsSignaturesGerer).await.is_ok();
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let c = charger_circuit(&mut tx, a.etablissement_id, circuit_id, false).await?;
        if !gere && !est_signataire_du_circuit(&mut tx, a, circuit_id).await? {
            return Err(introuvable());
        }
        let v = vue_circuit(&mut tx, a.etablissement_id, &c).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Suivi des circuits d'un document (DCT-62).
    pub async fn circuits_du_document_for_actor(&self, a: &Acteur<'_>, document_id: Uuid) -> Result<Vec<VueCircuit>, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsSignaturesGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_doc_circuit WHERE etablissement_id = $1 AND document_id = $2 ORDER BY lance_le DESC",
        )
        .bind(a.etablissement_id)
        .bind(document_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (id,) in ids {
            let c = charger_circuit(&mut tx, a.etablissement_id, id, false).await?;
            out.push(vue_circuit(&mut tx, a.etablissement_id, &c).await?);
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// **Mes documents à signer** (DCT-81) : famille, élève, agent ; échéance d'abord. Le droit du titulaire est relu au
    /// jour (lien actif) ; le **parapheur** (signataires de l'établissement) en est une variante.
    pub async fn mes_documents_a_signer_for_actor(&self, a: &Acteur<'_>) -> Result<Vec<LigneASigner>, EntDbError> {
        self.lignes_a_signer(a, false).await
    }

    /// **Parapheur** de la direction (DCT, §9.6) : ce qui attend la signature de l'établissement pour l'agent connecté,
    /// trié par échéance, avec la formule de délégation. Signature **à l'unité** (le lot est au lot D7).
    pub async fn parapheur_for_actor(&self, a: &Acteur<'_>) -> Result<Vec<LigneASigner>, EntDbError> {
        self.lignes_a_signer(a, true).await
    }

    async fn lignes_a_signer(&self, a: &Acteur<'_>, parapheur: bool) -> Result<Vec<LigneASigner>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        let lignes = sqlx::query(
            "SELECT s.id AS sid, s.circuit_id, s.role, s.choix_par, s.personne_id, d.eleve_id, d.id AS did,
                    c.nature_code, d.titre, d.titre_neutre, c.echeance
               FROM ent_doc_signataire s
               JOIN ent_doc_circuit c ON c.etablissement_id = s.etablissement_id AND c.id = s.circuit_id
               JOIN ent_doc_document d ON d.etablissement_id = c.etablissement_id AND d.id = c.document_id
               LEFT JOIN ent_personne p ON p.etablissement_id = s.etablissement_id AND p.id = s.personne_id
               LEFT JOIN ent_agent ag ON ag.etablissement_id = s.etablissement_id AND ag.id = s.agent_id
               LEFT JOIN ent_personne ap ON ap.etablissement_id = ag.etablissement_id AND ap.id = ag.personne_id
               LEFT JOIN ent_eleve_compte ec ON ec.etablissement_id = s.etablissement_id AND ec.eleve_id = s.eleve_id
              WHERE s.etablissement_id = $1 AND c.etat = 'ouvert' AND s.etat = 'invite'
                AND COALESCE(p.user_id, ap.user_id, ec.user_id) = $2
                AND ((s.role IN ('agent', 'etablissement')) = $3)
              ORDER BY c.echeance NULLS LAST, c.lance_le",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .bind(parapheur)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for r in lignes {
            let sid: Uuid = r.get("sid");
            let role: String = r.get("role");
            let personne: Option<Uuid> = r.get("personne_id");
            let eleve: Option<Uuid> = r.get("eleve_id");
            if role == "titulaire" {
                match (personne, eleve) {
                    (Some(p), Some(e)) if titulaire_en_droit(&mut tx, a.etablissement_id, p, e, jour).await? => {}
                    _ => continue,
                }
            }
            let nature: String = r.get("nature_code");
            let formule = if role == "etablissement" {
                let pouvoir = self.peut_signer_compte(a.etablissement_id, a.user_id, &nature).await?;
                if !pouvoir.autorise {
                    continue;
                }
                pouvoir.formule
            } else {
                None
            };
            let titre: String = if parapheur { r.get("titre") } else { r.get("titre_neutre") };
            out.push(LigneASigner {
                signataire_id: sid,
                circuit_id: r.get("circuit_id"),
                document_id: r.get("did"),
                nature_code: nature,
                titre,
                role,
                echeance: r.get("echeance"),
                modes: modes_du_signataire(&mut tx, a.etablissement_id, sid).await?,
                choix_par: r.get("choix_par"),
                formule_delegation: formule,
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }
}

struct LigneDoc {
    nature_code: String,
    eleve_id: Option<Uuid>,
    etat: String,
    version_id: Option<Uuid>,
}

async fn r_doc(conn: &mut PgConnection, etab: Uuid, document_id: Uuid) -> Result<LigneDoc, EntDbError> {
    let r = sqlx::query(
        "SELECT d.nature_code, d.eleve_id, d.etat, v.id AS vid
           FROM ent_doc_document d
           LEFT JOIN ent_doc_version v ON v.etablissement_id = d.etablissement_id AND v.document_id = d.id
                                      AND v.numero_version = d.version_courante
          WHERE d.etablissement_id = $1 AND d.id = $2 FOR UPDATE OF d",
    )
    .bind(etab)
    .bind(document_id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    Ok(LigneDoc {
        nature_code: r.get("nature_code"),
        eleve_id: r.get("eleve_id"),
        etat: r.get("etat"),
        version_id: r.get("vid"),
    })
}

pub(crate) async fn est_signataire_du_circuit(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    circuit_id: Uuid,
) -> Result<bool, EntDbError> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM ent_doc_signataire s
              LEFT JOIN ent_personne p ON p.etablissement_id = s.etablissement_id AND p.id = s.personne_id
              LEFT JOIN ent_agent ag ON ag.etablissement_id = s.etablissement_id AND ag.id = s.agent_id
              LEFT JOIN ent_personne ap ON ap.etablissement_id = ag.etablissement_id AND ap.id = ag.personne_id
              LEFT JOIN ent_eleve_compte ec ON ec.etablissement_id = s.etablissement_id AND ec.eleve_id = s.eleve_id
             WHERE s.etablissement_id = $1 AND s.circuit_id = $2
               AND COALESCE(p.user_id, ap.user_id, ec.user_id) = $3)",
    )
    .bind(a.etablissement_id)
    .bind(circuit_id)
    .bind(a.user_id)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(ok)
}

async fn vue_circuit(conn: &mut PgConnection, etab: Uuid, c: &LigneCircuit) -> Result<VueCircuit, EntDbError> {
    let r = sqlx::query(
        "SELECT pdf_signe_sha256, preuve_sha256 FROM ent_doc_circuit WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(c.id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let lignes = sqlx::query(
        "SELECT s.id, s.rang, s.role, s.libelle_role, s.etat, s.requis, s.choix_par, g.niveau, g.methode
           FROM ent_doc_signataire s
           LEFT JOIN ent_doc_signature g ON g.etablissement_id = s.etablissement_id AND g.signataire_id = s.id
          WHERE s.etablissement_id = $1 AND s.circuit_id = $2 ORDER BY s.rang, s.cree_le",
    )
    .bind(etab)
    .bind(c.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut sig = Vec::new();
    for l in lignes {
        let id: Uuid = l.get("id");
        sig.push(VueSignataire {
            id,
            rang: l.get("rang"),
            role: l.get("role"),
            libelle_role: l.get("libelle_role"),
            nom: nom_signataire(conn, etab, id).await?,
            etat: l.get("etat"),
            requis: l.get("requis"),
            choix_par: l.get("choix_par"),
            modes: modes_du_signataire(conn, etab, id).await?,
            niveau_obtenu: l.get("niveau"),
            methode: l.get("methode"),
        });
    }
    Ok(VueCircuit {
        id: c.id,
        document_id: c.document_id,
        version_id: c.version_id,
        nature_code: c.nature_code.clone(),
        etat: c.etat.clone(),
        ordre: c.ordre.clone(),
        regle_titulaires: c.regle.clone(),
        echeance: c.echeance,
        horodater: c.horodater,
        complet: c.etat == "complet",
        pdf_signe_sha256: r.get("pdf_signe_sha256"),
        preuve_sha256: r.get("preuve_sha256"),
        signataires: sig,
    })
}
