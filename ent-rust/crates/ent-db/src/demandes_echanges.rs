//! Socle N1 (demandes), lot N1-2 : **échanges, pièces et abonnés** (`N1_DEMANDES.md` §12).
//!
//! * Le fil d'une demande a trois visibilités : **au demandeur** (et à son déposant, à ses co-demandeurs), **interne**
//!   (membres de la file), **de consultation** (un tiers consulté et la file). Un message envoyé ne se modifie jamais :
//!   une correction est un nouveau message ; seule une note interne se **masque**, par son auteur, dans les 10 minutes.
//! * Les pièces sont des **références au socle 7** : un `document_id`, un rôle, une visibilité — jamais un nom de
//!   fichier, un chemin ni un octet (NJ-83, D-036). Le socle 7 n'étant pas encore versé, tout passe par le port
//!   [`PortDocuments`] ; son adaptateur « source absente » **refuse tout** (§30.2 : pas de repli vers un stockage direct).
//! * **NJ-80 étendu** : chaque lecture et chaque écriture du fil, des pièces et des abonnés passe par la garde
//!   d'accès unique ([`charger_pour_acces`]) : sans lien avec la demande, 404 neutre, rien n'est écrit.
//! * **NJ-83** : la lecture d'une pièce se contrôle **par N1** (membre de la file, demandeur pour ce qui lui est
//!   visible, consulté pour ce qui lui est partagé) avant tout jeton du socle 7 ; une pièce d'une autre file n'est
//!   jamais lisible.
//! * Le texte d'une nature C2+ est **scellé** (champs chiffrés) comme celui de la demande (R-DM16).
//! * Conversion message ↔ demande (§37.3) : le socle 16 expose [`PortMessagerie`] ; N1 ne dépend d'aucune table du 16.

use crate::champs_chiffres::ChampsSensibles;
use crate::demandes::{
    ajouter_fait, charger_pour_acces, publier_enveloppe, Acces, EnveloppeNeutre, FaitNouveau, Lien, QuotasDemandes,
    RefusDemande, SourcesDemandes, Statut,
};
use crate::service::PgEnt;
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Utc};
use ent_auth::Role;
use ent_events::types;
use serde::Serialize;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

/// Longueur maximale d'un message en clair (les textes scellés sont plus longs en base).
pub const TEXTE_MESSAGE_MAX: usize = 8000;
/// Pièces non retirées par demande (NXN-01 : 12 au plus).
pub const PIECES_MAX: i64 = 12;
/// Délai pendant lequel l'auteur d'une note interne peut la masquer (faute de frappe, §12.1).
pub const MASQUAGE_MINUTES: i64 = 10;
/// Natures sur lesquelles un « +1 » (appui) a un sens : un signalement partagé (DEM-114).
pub const NATURES_APPUYABLES: &[&str] = &["maintenance.intervention"];

// ───────────────────────── Ports (sources externes) ─────────────────────────

/// Ce que N1 demande au **socle 7** (`07_DOCUMENTS.md` §17). Instantané sans accès base, comme
/// [`SourcesDemandes`] : l'adaptateur réel arrive avec le socle 7.
pub trait PortDocuments: Send + Sync {
    /// Le document existe, a passé les contrôles du socle 7 (type réel, antivirus, taille) et a été **déposé par
    /// cette personne** : on ne rattache jamais le document d'un autre (rattachement forcé = fuite, NJ-83).
    fn document_depose_par(&self, etab: Uuid, document_id: Uuid, personne_id: Uuid) -> bool;
    /// Jeton de lecture **court, lié à la session** (socle 7 §17.3), émis seulement après le contrôle de droit de N1.
    fn jeton_lecture(&self, etab: Uuid, document_id: Uuid, user_id: Uuid) -> Option<String>;
}

/// Socle 7 absent : aucune pièce n'est acceptée ni lue (fail-closed, jamais de stockage direct de secours).
pub struct DocumentsAbsents;

impl PortDocuments for DocumentsAbsents {
    fn document_depose_par(&self, _: Uuid, _: Uuid, _: Uuid) -> bool {
        false
    }
    fn jeton_lecture(&self, _: Uuid, _: Uuid, _: Uuid) -> Option<String> {
        None
    }
}

/// Ce que N1 demande au **socle 16** (messagerie) pour la conversion message ↔ demande (§37.3, §37.9). Le 16 n'est
/// pas modifié : il expose ce port, N1 ne lit aucune de ses tables.
pub trait PortMessagerie: Send + Sync {
    /// La personne peut-elle lire ce fil et le convertir en demande (matrice du 16) ?
    fn peut_convertir_fil(&self, etab: Uuid, personne_id: Uuid, fil_id: Uuid) -> bool;
    /// Ouvre (ou retrouve) la conversation qui reprend le contexte d'une demande close `transmise_conversation`.
    fn conversation_depuis_demande(&self, etab: Uuid, personne_id: Uuid, demande_id: Uuid) -> Option<Uuid>;
}

/// Socle 16 absent : aucune conversion (fail-closed).
pub struct MessagerieAbsente;

impl PortMessagerie for MessagerieAbsente {
    fn peut_convertir_fil(&self, _: Uuid, _: Uuid, _: Uuid) -> bool {
        false
    }
    fn conversation_depuis_demande(&self, _: Uuid, _: Uuid, _: Uuid) -> Option<Uuid> {
        None
    }
}

// ───────────────────────── Vocabulaire ─────────────────────────

/// Le fil dans lequel on écrit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fil {
    /// Visible du demandeur (et de son déposant, de ses co-demandeurs, de la copie interne).
    Demandeur,
    /// Note interne : membres de la file seulement. **Jamais** visible du demandeur (NJ-82).
    Interne,
    /// Échange avec un tiers consulté (identifiant de la consultation).
    Consultation(Uuid),
}

impl Fil {
    fn code(self) -> &'static str {
        match self {
            Self::Demandeur => "demandeur",
            Self::Interne => "interne",
            Self::Consultation(_) => "consultation",
        }
    }
}

/// Rôle d'une pièce (§12.2). Les rôles que chaque camp peut poser sont contrôlés à l'écriture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RolePiece {
    Justificatif,
    Annexe,
    Reponse,
    PhotoLieu,
    FichierAImprimer,
    Accord,
}

impl RolePiece {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Justificatif => "justificatif",
            Self::Annexe => "annexe",
            Self::Reponse => "reponse",
            Self::PhotoLieu => "photo_lieu",
            Self::FichierAImprimer => "fichier_a_imprimer",
            Self::Accord => "accord",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "justificatif" => Self::Justificatif,
            "annexe" => Self::Annexe,
            "reponse" => Self::Reponse,
            "photo_lieu" => Self::PhotoLieu,
            "fichier_a_imprimer" => Self::FichierAImprimer,
            "accord" => Self::Accord,
            _ => return None,
        })
    }
    /// Rôles qu'un demandeur (ou co-demandeur) peut poser ; le service pose les autres.
    fn cote_demandeur(self) -> bool {
        matches!(self, Self::Justificatif | Self::Annexe | Self::PhotoLieu | Self::FichierAImprimer)
    }
    fn cote_service(self) -> bool {
        matches!(self, Self::Reponse | Self::Annexe | Self::Accord)
    }
}

/// Une pièce à rattacher : la **référence** du document au socle 7 et son rôle. Aucun nom de fichier.
#[derive(Debug, Clone, Copy)]
pub struct PieceAJoindre {
    pub document_id: Uuid,
    pub role: RolePiece,
    /// Pièce posée seule par le service : adressée au demandeur (vrai) ou interne (faux). Ignoré ailleurs.
    pub au_demandeur: bool,
}

/// Un message à écrire.
#[derive(Debug, Clone)]
pub struct NouveauMessage {
    pub fil: Fil,
    pub texte: String,
    pub pieces: Vec<PieceAJoindre>,
}

/// Ce que renvoie l'écriture d'un message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageEnvoye {
    pub id: Uuid,
    pub statut: String,
    pub version: i32,
}

/// Un message tel que **cet acteur** le lit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MessageVu {
    pub id: Uuid,
    pub le: DateTime<Utc>,
    /// « Vous », « Secrétariat — Paul D. », « Paul D. » (côté service), « Demandeur » : jamais une adresse.
    pub auteur: String,
    pub du_service: bool,
    pub fil: &'static str,
    pub genre: String,
    pub texte: Option<String>,
    pub masque: bool,
    pub pieces: Vec<PieceVue>,
}

/// Une pièce telle que cet acteur la voit : identifiant de pièce de N1, rôle, état. Jamais l'identifiant du document
/// du socle 7 (il ne sort que par un jeton, après contrôle).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PieceVue {
    pub id: Uuid,
    pub role: String,
    pub au_demandeur: bool,
    pub deposee_par_moi: bool,
    pub retiree: bool,
    pub le: DateTime<Utc>,
}

/// Lecture d'une pièce : le jeton court du socle 7, jamais l'identifiant du document ni son nom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LecturePiece {
    pub piece_id: Uuid,
    pub role: String,
    pub jeton: String,
}

/// Qualité d'un abonné (§12.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualiteAbonne {
    CoDemandeur,
    CopieInterne,
}

impl QualiteAbonne {
    fn code(self) -> &'static str {
        match self {
            Self::CoDemandeur => "co_demandeur",
            Self::CopieInterne => "copie_interne",
        }
    }
}

// ───────────────────────── Aides internes ─────────────────────────

/// « Paul D. » : prénom et initiale du nom (§37.1). Sans prénom : l'initiale seule ; sans rien : « un agent ».
pub(crate) async fn prenom_initiale(conn: &mut PgConnection, etab: Uuid, personne_id: Uuid) -> Result<String, RefusDemande> {
    let r: Option<(Option<String>, String)> =
        sqlx::query_as("SELECT prenom, nom FROM ent_personne WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(personne_id)
            .fetch_optional(conn)
            .await?;
    Ok(match r {
        None => "un agent".to_string(),
        Some((prenom, nom)) => {
            let initiale = nom.trim().chars().next().map(|c| c.to_uppercase().collect::<String>());
            match (prenom.as_deref().map(str::trim).filter(|p| !p.is_empty()), initiale) {
                (Some(p), Some(i)) => format!("{p} {i}."),
                (Some(p), None) => p.to_string(),
                (None, Some(i)) => format!("{i}."),
                (None, None) => "un agent".to_string(),
            }
        }
    })
}

/// Ce que le demandeur lit du service : le rôle de la file, ou « l'établissement » quand la file ou la nature est
/// confidentielle (le nom d'une cellule ne se dévoile pas, §37.1). Le second terme : le prénom est-il montré ?
pub(crate) struct ServicePublic {
    pub(crate) service: String,
    pub(crate) role: Option<String>,
    pub(crate) nominatif: bool,
}

pub(crate) async fn service_public(
    conn: &mut PgConnection,
    etab: Uuid,
    file_id: Uuid,
    confidentialite: &str,
) -> Result<ServicePublic, RefusDemande> {
    let r: Option<(String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT libelle, role_traitant, type, affichage_traitant FROM ent_dem_file WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(file_id)
    .fetch_optional(conn)
    .await?;
    let confidentiel = !matches!(confidentialite, "C0" | "C1");
    Ok(match r {
        Some((_, _, type_, _)) if confidentiel || type_ == "confidentielle" => {
            ServicePublic { service: "l'établissement".into(), role: None, nominatif: false }
        }
        Some((libelle, role, _, affichage)) => {
            let nominatif = affichage == "prenom_initiale";
            ServicePublic { service: libelle, role, nominatif }
        }
        None => ServicePublic { service: "l'établissement".into(), role: None, nominatif: false },
    })
}

impl ServicePublic {
    /// « le secrétariat des élèves », sinon « le service {libellé} ».
    pub(crate) fn qui(&self) -> String {
        if self.service == "l'établissement" {
            return self.service.clone();
        }
        self.role.clone().unwrap_or_else(|| format!("le service {}", self.service))
    }
}

fn statut_de(ligne_statut: &str) -> Result<Statut, RefusDemande> {
    Statut::parse(ligne_statut).ok_or(RefusDemande::Indisponible)
}

fn sceller_message(conf: &str, id: Uuid, texte: &str) -> Result<(String, bool), RefusDemande> {
    if matches!(conf, "C0" | "C1") {
        return Ok((texte.to_string(), false));
    }
    ChampsSensibles::depuis_env()
        .sceller("ent_dem_message", "texte", id, texte)
        .map(|t| (t, true))
        .map_err(|_| RefusDemande::Indisponible)
}

fn ouvrir_message(id: Uuid, stocke: &str, scelle: bool) -> Result<String, RefusDemande> {
    if !scelle {
        return Ok(stocke.to_string());
    }
    ChampsSensibles::depuis_env().ouvrir("ent_dem_message", "texte", id, stocke).map_err(|_| RefusDemande::Indisponible)
}

/// Valide un texte de message : non vide, ≤ 8000 caractères, sans caractère nul.
pub(crate) fn valider_texte_message(t: &str) -> Result<String, RefusDemande> {
    let t = t.trim();
    if t.is_empty() {
        return Err(RefusDemande::Invalide("Le message est vide."));
    }
    if t.chars().count() > TEXTE_MESSAGE_MAX || t.contains('\0') {
        return Err(RefusDemande::Invalide("Le message est trop long (8000 caractères au plus)."));
    }
    Ok(t.to_string())
}

/// Le responsable d'une file : membre dont la règle porte la qualité « responsable ». Sert aux gestes réservés au
/// responsable (réaffecter, déléguer pour autrui…). Recalculé au jour métier, jamais copié.
pub(crate) async fn est_responsable_de(
    conn: &mut PgConnection,
    etab: Uuid,
    file_id: Uuid,
    personne: Uuid,
) -> Result<bool, RefusDemande> {
    let jour = crate::demandes_files::jour_metier(conn, etab).await?;
    let g = crate::demandes_files::charger_graphe(conn, etab, jour).await?;
    Ok(g.membres.get(&file_id).is_some_and(|ms| ms.iter().any(|m| m.personne_id == personne && m.responsable)))
}

/// Écrit un message (cœur commun : écriture, correction, note de délégation). `corrige` = message corrigé.
pub(crate) struct Ecriture<'a> {
    pub(crate) fil: Fil,
    pub(crate) texte: &'a str,
    pub(crate) pieces: &'a [PieceAJoindre],
    pub(crate) corrige: Option<Uuid>,
    /// Demande de complément : le traitant attend une réponse (met la demande « en attente du demandeur »).
    pub(crate) complement: bool,
}

impl PgEnt {
    /// Écrit un message dans le fil d'une demande. Dans l'ordre : débit (NJ-72), garde d'accès unique (NJ-80), droit
    /// d'écrire dans CE fil selon le lien, état de la demande, pièces rattachées au socle 7, écriture, fait chaîné,
    /// événement à enveloppe neutre. Une réponse du demandeur à une demande de complément remet la demande au
    /// traitement et reprend le décompte du délai (R-DM20).
    #[allow(clippy::too_many_arguments)]
    pub async fn ecrire_message_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        docs: &dyn PortDocuments,
        id: Uuid,
        m: NouveauMessage,
    ) -> Result<MessageEnvoye, RefusDemande> {
        self.limiter("demandes.echange", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        let e = Ecriture { fil: m.fil, texte: &m.texte, pieces: &m.pieces, corrige: None, complement: false };
        let r = ecrire_tx(&mut tx, etab, user_id, docs, id, &acces, e).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Demande de complément (DEM-180) : le traitant liste ce qui manque ; la demande passe « en attente du
    /// demandeur », le décompte du délai est suspendu, le demandeur voit « on attend votre réponse ».
    #[allow(clippy::too_many_arguments)]
    pub async fn demander_complement_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        texte: &str,
    ) -> Result<MessageEnvoye, RefusDemande> {
        self.limiter("demandes.echange", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        if acces.ligne.version != version_attendue {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        let e = Ecriture { fil: Fil::Demandeur, texte, pieces: &[], corrige: None, complement: true };
        let r = ecrire_tx(&mut tx, etab, user_id, &DocumentsAbsents, id, &acces, e).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Corrige un message : **nouveau** message « Correction », dans le même fil, par le même auteur (CDM-21). Le
    /// message d'origine n'est jamais modifié.
    #[allow(clippy::too_many_arguments)]
    pub async fn corriger_message_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        message_id: Uuid,
        texte: &str,
    ) -> Result<MessageEnvoye, RefusDemande> {
        self.limiter("demandes.echange", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (demande_id, auteur, visibilite, consultation): (Uuid, Uuid, String, Option<Uuid>) = sqlx::query_as(
            "SELECT demande_id, auteur_personne_id, visibilite, consultation_id
               FROM ent_dem_message WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(message_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RefusDemande::Introuvable)?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, demande_id, true).await?;
        if auteur != acces.moi {
            return Err(RefusDemande::Interdit);
        }
        let fil = match (visibilite.as_str(), consultation) {
            ("demandeur", _) => Fil::Demandeur,
            ("interne", _) => Fil::Interne,
            (_, Some(c)) => Fil::Consultation(c),
            _ => return Err(RefusDemande::Indisponible),
        };
        let e = Ecriture { fil, texte, pieces: &[], corrige: Some(message_id), complement: false };
        let r = ecrire_tx(&mut tx, etab, user_id, &DocumentsAbsents, demande_id, &acces, e).await?;
        tx.commit().await?;
        Ok(r)
    }

    /// Masque **sa** note interne dans les 10 minutes (DEM-202, faute de frappe). Le message reste en base, le fait
    /// reste au journal ; il disparaît des lectures. Jamais un message visible du demandeur.
    pub async fn masquer_message_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        message_id: Uuid,
    ) -> Result<(), RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (demande_id, auteur, visibilite, masque, recent): (Uuid, Uuid, String, Option<DateTime<Utc>>, bool) =
            sqlx::query_as(
                "SELECT demande_id, auteur_personne_id, visibilite, masque_le,
                        (now() - cree_le) <= make_interval(mins => $3::int)
                   FROM ent_dem_message WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(message_id)
            .bind(MASQUAGE_MINUTES as i32)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(RefusDemande::Introuvable)?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, demande_id, true).await?;
        if auteur != acces.moi || visibilite != "interne" || acces.lien != Lien::Traitant {
            return Err(RefusDemande::Interdit);
        }
        if masque.is_some() {
            return Err(RefusDemande::Conflit("Cette note est déjà masquée."));
        }
        if !recent {
            return Err(RefusDemande::Conflit("Le délai pour masquer cette note est dépassé (10 minutes)."));
        }
        sqlx::query("UPDATE ent_dem_message SET masque_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(message_id)
            .execute(&mut *tx)
            .await?;
        let mut f = FaitNouveau::nouveau(demande_id, "message_masque");
        f.auteur_personne = Some(acces.moi);
        f.ref_id = Some(message_id);
        ajouter_fait(&mut tx, etab, f).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Le fil d'une demande **tel que cet acteur le lit** : le demandeur (déposant, co-demandeur, copie interne) ne
    /// voit jamais une note interne ni un échange de consultation ; un consulté ne voit que sa consultation ; un
    /// appui ne voit rien. Un message masqué n'a plus de texte. Lecture C2+ par le service : tracée (R-DM16).
    pub async fn messages_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
    ) -> Result<Vec<MessageVu>, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, false).await?;
        let filtre = match acces.lien {
            Lien::Traitant => "TRUE".to_string(),
            Lien::Demandeur | Lien::Deposant | Lien::CoDemandeur | Lien::CopieInterne => "m.visibilite = 'demandeur'".to_string(),
            Lien::Consulte(_) => "m.visibilite = 'consultation' AND c.consulte_agent_id = $3".to_string(),
            Lien::Appui => return Err(RefusDemande::Introuvable),
        };
        let sql = format!(
            "SELECT m.id, m.cree_le, m.auteur_personne_id, m.visibilite, m.genre, m.texte, m.texte_scelle, m.masque_le
               FROM ent_dem_message m
               LEFT JOIN ent_dem_consultation c ON c.etablissement_id = m.etablissement_id AND c.id = m.consultation_id
              WHERE m.etablissement_id = $1 AND m.demande_id = $2 AND {filtre}
              ORDER BY m.cree_le, m.id"
        );
        let mut q = sqlx::query_as::<_, LigneMessage>(&sql).bind(etab).bind(id);
        if matches!(acces.lien, Lien::Consulte(_)) {
            q = q.bind(acces.agent);
        }
        let lignes: Vec<LigneMessage> = q.fetch_all(&mut *tx).await?;
        let sp = service_public(&mut tx, etab, acces.ligne.file_id, &acces.ligne.confidentialite).await?;
        let pieces = pieces_visibles(&mut tx, etab, id, &acces).await?;
        let mut out = Vec::with_capacity(lignes.len());
        for l in lignes {
            let du_service = match l.visibilite.as_str() {
                "demandeur" => est_du_service(&mut tx, etab, id, l.auteur_personne_id, &acces).await?,
                _ => true,
            };
            let auteur = if l.auteur_personne_id == acces.moi {
                "Vous".to_string()
            } else if acces.lien == Lien::Traitant {
                if du_service {
                    prenom_initiale(&mut tx, etab, l.auteur_personne_id).await?
                } else {
                    "Demandeur".to_string()
                }
            } else if du_service {
                // Côté demandeur : la signature du service, avec le prénom si la file l'autorise (SN1-02).
                if sp.nominatif {
                    format!("{} — {}", sp.qui(), prenom_initiale(&mut tx, etab, l.auteur_personne_id).await?)
                } else {
                    sp.qui()
                }
            } else {
                "Le demandeur".to_string()
            };
            let masque = l.masque_le.is_some();
            let texte = if masque { None } else { Some(ouvrir_message(l.id, &l.texte, l.texte_scelle)?) };
            let pieces_du_message: Vec<PieceVue> =
                pieces.iter().filter(|(mid, _)| *mid == Some(l.id)).map(|(_, p)| p.clone()).collect();
            out.push(MessageVu {
                id: l.id,
                le: l.cree_le,
                auteur,
                du_service,
                fil: match l.visibilite.as_str() {
                    "interne" => "interne",
                    "consultation" => "consultation",
                    _ => "demandeur",
                },
                genre: l.genre,
                texte,
                masque,
                pieces: pieces_du_message,
            });
        }
        if acces.lien == Lien::Traitant && !matches!(acces.ligne.confidentialite.as_str(), "C0" | "C1") && !out.is_empty() {
            let mut f = FaitNouveau::nouveau(id, "lecture_confidentielle");
            f.auteur_personne = Some(acces.moi);
            f.motif = Some("fil");
            ajouter_fait(&mut tx, etab, f).await?;
        }
        tx.commit().await?;
        Ok(out)
    }

    // ───────────── Pièces ─────────────

    /// Rattache une pièce (référence du socle 7) à la demande, hors message. Le demandeur pose un justificatif, une
    /// annexe, une photo du lieu, un fichier à imprimer ; le service pose une réponse, une annexe, un accord.
    #[allow(clippy::too_many_arguments)]
    pub async fn joindre_piece_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        docs: &dyn PortDocuments,
        id: Uuid,
        piece: PieceAJoindre,
    ) -> Result<Uuid, RefusDemande> {
        self.limiter("demandes.echange", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        if statut_de(&acces.ligne.statut)?.est_final() {
            return Err(RefusDemande::Conflit("Cette demande est terminée."));
        }
        let cote_service = match acces.lien {
            Lien::Traitant => true,
            Lien::Demandeur | Lien::Deposant | Lien::CoDemandeur => false,
            _ => return Err(RefusDemande::Interdit),
        };
        let au_demandeur = if cote_service { piece.au_demandeur } else { true };
        let pid = attacher_piece(&mut tx, etab, docs, id, None, &acces, piece, au_demandeur, cote_service).await?;
        tx.commit().await?;
        Ok(pid)
    }

    /// Retire une pièce **qu'on a déposée** (DEM-212) : le demandeur tant que personne n'a pris la demande (ou
    /// qu'on attend sa réponse), le service à tout moment avant la clôture. La trace et le document du socle 7
    /// restent (le 7 applique sa propre règle).
    pub async fn retirer_piece_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        piece_id: Uuid,
    ) -> Result<(), RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (demande_id, depose_par, retiree): (Uuid, Uuid, Option<DateTime<Utc>>) = sqlx::query_as(
            "SELECT demande_id, depose_par_personne_id, retire_le FROM ent_dem_piece WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(piece_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RefusDemande::Introuvable)?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, demande_id, true).await?;
        // Une pièce qu'on ne peut pas lire n'existe pas pour soi (404), puis seul son déposant la retire.
        if !piece_lisible(&mut tx, etab, piece_id, &acces).await? {
            return Err(RefusDemande::Introuvable);
        }
        if depose_par != acces.moi {
            return Err(RefusDemande::Interdit);
        }
        if retiree.is_some() {
            return Err(RefusDemande::Conflit("Cette pièce est déjà retirée."));
        }
        let statut = statut_de(&acces.ligne.statut)?;
        if statut.est_final() {
            return Err(RefusDemande::Conflit("Cette demande est terminée."));
        }
        if acces.lien != Lien::Traitant
            && acces.ligne.attribuee_a_agent_id.is_some()
            && statut != Statut::EnAttenteDemandeur
        {
            return Err(RefusDemande::Conflit("Cette demande est déjà prise en charge : écrivez au service."));
        }
        sqlx::query(
            "UPDATE ent_dem_piece SET retire_le = now(), retire_par_personne_id = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(piece_id)
        .bind(acces.moi)
        .execute(&mut *tx)
        .await?;
        let mut f = FaitNouveau::nouveau(demande_id, "piece_retiree");
        f.auteur_personne = Some(acces.moi);
        f.ref_id = Some(piece_id);
        ajouter_fait(&mut tx, etab, f).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Les pièces d'une demande avec leur rôle (DEM-211), **selon le lien** : le demandeur voit ce qui lui est
    /// visible, le service tout, un consulté ce qui lui est partagé, un appui rien (NJ-83).
    pub async fn pieces_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
    ) -> Result<Vec<PieceVue>, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, false).await?;
        if acces.lien == Lien::Appui {
            return Err(RefusDemande::Introuvable);
        }
        let v = pieces_visibles(&mut tx, etab, id, &acces).await?;
        tx.commit().await?;
        Ok(v.into_iter().map(|(_, p)| p).collect())
    }

    /// Lit une pièce (DEM-205, NJ-83) : le contrôle de droit est celui de **N1**, **avant** tout jeton du socle 7 ;
    /// une pièce qui n'est pas visible de cet acteur n'existe pas pour lui (404). Lecture C2+ : tracée.
    pub async fn lire_piece_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        docs: &dyn PortDocuments,
        piece_id: Uuid,
    ) -> Result<LecturePiece, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (demande_id, document_id, role_piece, retiree): (Uuid, Uuid, String, Option<DateTime<Utc>>) = sqlx::query_as(
            "SELECT demande_id, document_id, role, retire_le FROM ent_dem_piece WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(piece_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RefusDemande::Introuvable)?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, demande_id, false).await?;
        if retiree.is_some() || !piece_lisible(&mut tx, etab, piece_id, &acces).await? {
            return Err(RefusDemande::Introuvable);
        }
        let jeton = docs.jeton_lecture(etab, document_id, user_id).ok_or(RefusDemande::Indisponible)?;
        if !matches!(acces.ligne.confidentialite.as_str(), "C0" | "C1") {
            let mut f = FaitNouveau::nouveau(demande_id, "piece_lue_confidentielle");
            f.auteur_personne = Some(acces.moi);
            f.ref_id = Some(piece_id);
            ajouter_fait(&mut tx, etab, f).await?;
        }
        tx.commit().await?;
        Ok(LecturePiece { piece_id, role: role_piece, jeton })
    }

    // ───────────── Abonnés ─────────────

    /// Ajoute un abonné. **Co-demandeur** (DEM-115) : le demandeur, responsable légal, désigne l'autre responsable
    /// du **même élève** ; jamais pour un élève (le canal de l'élève reste invisible des parents), jamais un
    /// responsable sans lien actif (interdiction : refusé, R-DM17). **Copie interne** (DEM-208) : un membre de la file
    /// tient un collègue informé ; jamais sur une demande C2+.
    pub async fn ajouter_abonne_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        personne_id: Uuid,
        qualite: QualiteAbonne,
    ) -> Result<Uuid, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        if statut_de(&acces.ligne.statut)?.est_final() {
            return Err(RefusDemande::Conflit("Cette demande est terminée."));
        }
        // Le droit d'abord (on connaît la demande : « interdit »), les contrôles sur la personne ensuite.
        match qualite {
            QualiteAbonne::CoDemandeur if acces.lien != Lien::Demandeur || role != Role::Parent => return Err(RefusDemande::Interdit),
            QualiteAbonne::CopieInterne if acces.lien != Lien::Traitant => return Err(RefusDemande::Interdit),
            _ => {}
        }
        let cible: Option<(Option<Uuid>, bool)> = sqlx::query_as(
            "SELECT p.user_id, EXISTS (SELECT 1 FROM ent_agent a WHERE a.etablissement_id = p.etablissement_id
                                        AND a.personne_id = p.id AND a.fusionne_dans_agent_id IS NULL)
               FROM ent_personne p WHERE p.etablissement_id = $1 AND p.id = $2 AND p.fusionnee_dans IS NULL",
        )
        .bind(etab)
        .bind(personne_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (cible_user, cible_agent) = cible.ok_or(RefusDemande::Invalide("Cette personne est introuvable."))?;
        if Some(personne_id) == acces.ligne.demandeur_personne_id {
            return Err(RefusDemande::Invalide("Cette personne est déjà le demandeur."));
        }
        match qualite {
            QualiteAbonne::CoDemandeur => {
                if acces.lien != Lien::Demandeur || role != Role::Parent {
                    return Err(RefusDemande::Interdit);
                }
                let (Some(eleve), Some(cible_user)) = (acces.ligne.eleve_id, cible_user) else {
                    return Err(RefusDemande::Invalide("Cette demande ne concerne pas un élève."));
                };
                // Lien actif du socle 4 : un responsable sous interdiction, suspendu ou sans lien est refusé.
                if self.lien_parent_eleve(etab, cible_user, eleve).await?.is_none() {
                    return Err(RefusDemande::Invalide("Cette personne ne peut pas être ajoutée à cette demande."));
                }
            }
            QualiteAbonne::CopieInterne => {
                if acces.lien != Lien::Traitant || !cible_agent {
                    return Err(RefusDemande::Interdit);
                }
                // Dans ses droits : jamais une demande confidentielle, jamais la personne visée (R-DM47).
                if !matches!(acces.ligne.confidentialite.as_str(), "C0" | "C1") {
                    return Err(RefusDemande::Invalide("Une demande confidentielle ne se partage pas en copie."));
                }
                let visee: (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_dem_personne_visee
                                     WHERE etablissement_id = $1 AND demande_id = $2 AND personne_id = $3)",
                )
                .bind(etab)
                .bind(id)
                .bind(personne_id)
                .fetch_one(&mut *tx)
                .await?;
                if visee.0 {
                    return Err(RefusDemande::Invalide("Cette personne ne peut pas être ajoutée à cette demande."));
                }
            }
        }
        let aid = abonner(&mut tx, etab, id, personne_id, qualite.code(), acces.moi).await?;
        tx.commit().await?;
        Ok(aid)
    }

    /// « +1 » (DEM-114) : un personnel appuie un signalement partagé (panne). Il reçoit l'avis de clôture et ne lit
    /// rien. Réservé aux natures appuyables et aux demandes non confidentielles ; sans effet sur son propre dépôt.
    pub async fn appuyer_for_actor(&self, etab: Uuid, user_id: Uuid, role: &str, id: Uuid) -> Result<Uuid, RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        if !role.is_staff() {
            return Err(RefusDemande::Introuvable);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = crate::demandes::personne_du_user(&mut tx, etab, user_id).await.map_err(|_| RefusDemande::Introuvable)?;
        let l = crate::demandes::charger_demande(&mut tx, etab, id, false).await?;
        // Une demande qui n'est pas un signalement appuyable n'existe pas pour l'appui (404 neutre).
        if !NATURES_APPUYABLES.contains(&l.nature_code.as_str()) || !matches!(l.confidentialite.as_str(), "C0" | "C1") {
            return Err(RefusDemande::Introuvable);
        }
        if statut_de(&l.statut)?.est_final() {
            return Err(RefusDemande::Conflit("Cette demande est terminée."));
        }
        if l.demandeur_personne_id == Some(moi) || l.deposant_personne_id == Some(moi) {
            return Err(RefusDemande::Conflit("C'est votre propre demande."));
        }
        let aid = abonner(&mut tx, etab, id, moi, "appui", moi).await?;
        tx.commit().await?;
        Ok(aid)
    }

    /// Rattache une conversation du socle 16 à la demande (§37.3, DEM-318) : N1 pose un lien
    /// `issue_de_conversation` et un fait ; **le droit de lire et de convertir le fil est celui du 16**, rendu par le
    /// port. Sans port, rien ne se rattache (fail-closed). Aucune table du 16 n'est lue.
    #[allow(clippy::too_many_arguments)]
    pub async fn rattacher_conversation_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        messagerie: &dyn PortMessagerie,
        id: Uuid,
        fil_id: Uuid,
    ) -> Result<(), RefusDemande> {
        self.limiter("demandes.geste", user_id, QuotasDemandes::depuis_env().geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        if acces.lien != Lien::Traitant {
            return Err(RefusDemande::Interdit);
        }
        if !messagerie.peut_convertir_fil(etab, acces.moi, fil_id) {
            return Err(RefusDemande::Interdit);
        }
        sqlx::query(
            "INSERT INTO ent_dem_lien (id, etablissement_id, demande_id, type, cible_ref, cree_par_personne_id)
             VALUES ($1, $2, $3, 'issue_de_conversation', $4, $5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(fil_id)
        .bind(acces.moi)
        .execute(&mut *tx)
        .await?;
        let mut f = FaitNouveau::nouveau(id, "conversation_rattachee");
        f.auteur_personne = Some(acces.moi);
        f.ref_id = Some(fil_id);
        ajouter_fait(&mut tx, etab, f).await?;
        tx.commit().await?;
        Ok(())
    }
}

// ───────────────────────── Écriture commune ─────────────────────────

#[derive(Debug, FromRow)]
struct LigneMessage {
    id: Uuid,
    cree_le: DateTime<Utc>,
    auteur_personne_id: Uuid,
    visibilite: String,
    genre: String,
    texte: String,
    texte_scelle: bool,
    masque_le: Option<DateTime<Utc>>,
}

/// L'auteur est-il du côté du service (pas le demandeur ni un co-demandeur) ? Un message `demandeur` écrit par une
/// personne qui n'est ni le demandeur, ni le déposant, ni un co-demandeur actif est un message du service.
async fn est_du_service(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    auteur: Uuid,
    acces: &Acces,
) -> Result<bool, RefusDemande> {
    if Some(auteur) == acces.ligne.demandeur_personne_id || Some(auteur) == acces.ligne.deposant_personne_id {
        return Ok(false);
    }
    let co: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_dem_abonne WHERE etablissement_id = $1 AND demande_id = $2
                           AND personne_id = $3 AND qualite = 'co_demandeur')",
    )
    .bind(etab)
    .bind(demande)
    .bind(auteur)
    .fetch_one(conn)
    .await?;
    Ok(!co.0)
}

/// Cœur de l'écriture, dans la transaction de l'appelant (qui détient le verrou de la demande).
pub(crate) async fn ecrire_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
    docs: &dyn PortDocuments,
    id: Uuid,
    acces: &Acces,
    e: Ecriture<'_>,
) -> Result<MessageEnvoye, RefusDemande> {
    let statut = statut_de(&acces.ligne.statut)?;
    if statut.est_final() {
        return Err(RefusDemande::Conflit("Cette demande est terminée."));
    }
    // Qui écrit dans quel fil (le lien a déjà prouvé que la demande est connue de l'acteur : « interdit », pas 404).
    let cote_service = match (acces.lien, e.fil) {
        (Lien::Traitant, Fil::Demandeur | Fil::Interne) => true,
        (Lien::Traitant, Fil::Consultation(c)) => {
            consultation_de_la_demande(conn, etab, id, c).await?;
            true
        }
        (Lien::Demandeur | Lien::Deposant | Lien::CoDemandeur, Fil::Demandeur) => false,
        (Lien::Consulte(_), Fil::Consultation(c)) => {
            let agent = consultation_de_la_demande(conn, etab, id, c).await?;
            if agent != acces.agent {
                return Err(RefusDemande::Interdit);
            }
            true
        }
        _ => return Err(RefusDemande::Interdit),
    };
    if e.complement && !(cote_service && e.fil == Fil::Demandeur && acces.lien == Lien::Traitant) {
        return Err(RefusDemande::Interdit);
    }
    // Une demande de complément se fait par celui qui a la main : le porteur de la prise ou, sans prise, un membre.
    if e.complement
        && acces.ligne.attribuee_a_agent_id.is_some()
        && acces.ligne.attribuee_a_agent_id != acces.agent
        && !est_responsable_de(conn, etab, acces.ligne.file_id, acces.moi).await?
    {
        return Err(RefusDemande::Interdit);
    }
    if e.complement && statut == Statut::EnAttenteDemandeur {
        return Err(RefusDemande::Conflit("Un complément est déjà demandé."));
    }
    let texte = valider_texte_message(e.texte)?;
    if e.pieces.len() as i64 > PIECES_MAX {
        return Err(RefusDemande::Invalide("Trop de pièces jointes (12 au plus)."));
    }
    let mid = Uuid::new_v4();
    let (stocke, scelle) = sceller_message(&acces.ligne.confidentialite, mid, &texte)?;
    // Réponse du demandeur à un complément attendu (§12.1) : la demande reprend, le décompte repart (R-DM20).
    let reprise = !cote_service && statut == Statut::EnAttenteDemandeur;
    let genre = if e.corrige.is_some() {
        "correction"
    } else if e.complement {
        "complement_demande"
    } else if reprise {
        "complement_recu"
    } else {
        "echange"
    };
    let (consultation_id, visibilite) = match e.fil {
        Fil::Consultation(c) => (Some(c), "consultation"),
        autre => (None, autre.code()),
    };
    sqlx::query(
        "INSERT INTO ent_dem_message (id, etablissement_id, demande_id, auteur_personne_id, visibilite, genre,
                consultation_id, texte, texte_scelle, corrige_message_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(mid)
    .bind(etab)
    .bind(id)
    .bind(acces.moi)
    .bind(visibilite)
    .bind(genre)
    .bind(consultation_id)
    .bind(&stocke)
    .bind(scelle)
    .bind(e.corrige)
    .execute(&mut *conn)
    .await?;
    for p in e.pieces {
        let au_demandeur = e.fil == Fil::Demandeur;
        attacher_piece(conn, etab, docs, id, Some(mid), acces, *p, au_demandeur, cote_service).await?;
    }

    let (statut_apres, version) = if e.complement || reprise {
        let apres = if e.complement {
            Statut::EnAttenteDemandeur
        } else if acces.ligne.attribuee_a_agent_id.is_some() {
            Statut::Prise
        } else {
            Statut::Deposee
        };
        if e.complement {
            // Le décompte du délai de traitement est suspendu tant que le demandeur n'a pas répondu.
            sqlx::query(
                "UPDATE ent_dem_echeance SET suspendue_depuis = now()
                  WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'traitement'
                    AND traitee_le IS NULL AND suspendue_depuis IS NULL",
            )
            .bind(etab)
            .bind(id)
            .execute(&mut *conn)
            .await?;
        } else {
            sqlx::query(
                "UPDATE ent_dem_echeance SET due_le = due_le + (now() - suspendue_depuis), suspendue_depuis = NULL
                  WHERE etablissement_id = $1 AND demande_id = $2 AND suspendue_depuis IS NOT NULL AND traitee_le IS NULL",
            )
            .bind(etab)
            .bind(id)
            .execute(&mut *conn)
            .await?;
        }
        let n = sqlx::query(
            "UPDATE ent_dem_demande SET statut = $4, version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(acces.ligne.version)
        .bind(apres.code())
        .execute(&mut *conn)
        .await?
        .rows_affected();
        if n != 1 {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        let mut f = FaitNouveau::nouveau(id, if e.complement { "complement_demande" } else { "complement_recu" });
        f.auteur_personne = Some(acces.moi);
        f.statut_avant = Some(&acces.ligne.statut);
        f.statut_apres = Some(apres.code());
        f.ref_id = Some(mid);
        ajouter_fait(conn, etab, f).await?;
        (apres, acces.ligne.version + 1)
    } else {
        (statut, acces.ligne.version)
    };
    let mut f = FaitNouveau::nouveau(id, "message_envoye");
    f.auteur_personne = Some(acces.moi);
    f.motif = Some(visibilite);
    f.ref_id = Some(mid);
    ajouter_fait(conn, etab, f).await?;

    // Événement à enveloppe neutre : jamais le texte (NJ-86). Les notes internes ne notifient personne.
    let evenement = match (e.fil, cote_service) {
        (Fil::Demandeur, true) => Some("echange_au_demandeur"),
        (Fil::Demandeur, false) => Some("echange_au_service"),
        (Fil::Consultation(_), _) => Some("echange_consultation"),
        (Fil::Interne, _) => None,
    };
    if let Some(ev) = evenement {
        let env = EnveloppeNeutre::nouvelle(id, &acces.ligne.numero, &acces.ligne.nature_code, ev);
        publier_enveloppe(conn, etab, types::DEMANDE_ECHANGE, &env, Some(user_id)).await?;
    }
    Ok(MessageEnvoye { id: mid, statut: statut_apres.code().to_string(), version })
}

/// La consultation existe, appartient à CETTE demande ; rend l'agent consulté. Sinon 404 neutre.
async fn consultation_de_la_demande(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    consultation: Uuid,
) -> Result<Option<Uuid>, RefusDemande> {
    let r: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT consulte_agent_id FROM ent_dem_consultation WHERE etablissement_id = $1 AND demande_id = $2 AND id = $3",
    )
    .bind(etab)
    .bind(demande)
    .bind(consultation)
    .fetch_optional(conn)
    .await?;
    r.map(|(a,)| a).ok_or(RefusDemande::Introuvable)
}

/// Rattache une pièce : rôle permis pour ce camp, document déposé **par l'acteur** (socle 7), 12 pièces au plus.
#[allow(clippy::too_many_arguments)]
async fn attacher_piece(
    conn: &mut PgConnection,
    etab: Uuid,
    docs: &dyn PortDocuments,
    demande: Uuid,
    message: Option<Uuid>,
    acces: &Acces,
    p: PieceAJoindre,
    au_demandeur: bool,
    cote_service: bool,
) -> Result<Uuid, RefusDemande> {
    let permis = if cote_service { p.role.cote_service() } else { p.role.cote_demandeur() };
    if !permis {
        return Err(RefusDemande::Invalide("Ce type de pièce n'est pas possible ici."));
    }
    // Le document est celui de l'acteur : on ne rattache jamais le document d'un autre (NJ-83).
    if !docs.document_depose_par(etab, p.document_id, acces.moi) {
        return Err(RefusDemande::Invalide("Cette pièce est introuvable ou n'a pas été acceptée."));
    }
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_dem_piece WHERE etablissement_id = $1 AND demande_id = $2 AND retire_le IS NULL",
    )
    .bind(etab)
    .bind(demande)
    .fetch_one(&mut *conn)
    .await?;
    if n >= PIECES_MAX {
        return Err(RefusDemande::Invalide("Trop de pièces jointes (12 au plus)."));
    }
    let pid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_dem_piece (id, etablissement_id, demande_id, message_id, document_id, role, visibilite,
                depose_par_personne_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(pid)
    .bind(etab)
    .bind(demande)
    .bind(message)
    .bind(p.document_id)
    .bind(p.role.code())
    .bind(if au_demandeur { "demandeur" } else { "interne" })
    .bind(acces.moi)
    .execute(&mut *conn)
    .await?;
    let mut f = FaitNouveau::nouveau(demande, "piece_jointe");
    f.auteur_personne = Some(acces.moi);
    f.motif = Some(p.role.code());
    f.ref_id = Some(pid);
    ajouter_fait(conn, etab, f).await?;
    Ok(pid)
}

async fn abonner(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    personne: Uuid,
    qualite: &str,
    par: Uuid,
) -> Result<Uuid, RefusDemande> {
    let aid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_dem_abonne (id, etablissement_id, demande_id, personne_id, qualite, du, ajoute_par_personne_id)
         VALUES ($1, $2, $3, $4, $5, ent_aujourdhui_etab($2, now()), $6)",
    )
    .bind(aid)
    .bind(etab)
    .bind(demande)
    .bind(personne)
    .bind(qualite)
    .bind(par)
    .execute(&mut *conn)
    .await?;
    let mut f = FaitNouveau::nouveau(demande, "abonne_ajoute");
    f.auteur_personne = Some(par);
    f.motif = Some(qualite);
    f.ref_id = Some(aid);
    ajouter_fait(conn, etab, f).await?;
    Ok(aid)
}

// ───────────────────────── Visibilité des pièces ─────────────────────────

#[derive(Debug, FromRow)]
struct LignePiece {
    id: Uuid,
    message_id: Option<Uuid>,
    role: String,
    visibilite: String,
    depose_par_personne_id: Uuid,
    retire_le: Option<DateTime<Utc>>,
    cree_le: DateTime<Utc>,
}

/// Les pièces de la demande que **cet acteur** peut voir, avec le message auquel elles sont jointes. Une seule règle,
/// utilisée par la liste, la lecture et le retrait (NJ-83) : service = tout ; demandeur, déposant, co-demandeur,
/// copie interne = ce qui est « au demandeur » ; consulté = ce qui est partagé avec lui ; appui = rien.
async fn pieces_visibles(
    conn: &mut PgConnection,
    etab: Uuid,
    demande: Uuid,
    acces: &Acces,
) -> Result<Vec<(Option<Uuid>, PieceVue)>, RefusDemande> {
    let filtre = match acces.lien {
        Lien::Traitant => "TRUE",
        Lien::Demandeur | Lien::Deposant | Lien::CoDemandeur | Lien::CopieInterne => "p.visibilite = 'demandeur'",
        Lien::Consulte(_) => {
            "EXISTS (SELECT 1 FROM ent_dem_consultation_partage s
                       JOIN ent_dem_consultation c ON c.etablissement_id = s.etablissement_id AND c.id = s.consultation_id
                      WHERE s.etablissement_id = p.etablissement_id AND s.piece_id = p.id
                        AND c.consulte_agent_id = $3)"
        }
        Lien::Appui => return Ok(Vec::new()),
    };
    let sql = format!(
        "SELECT p.id, p.message_id, p.role, p.visibilite, p.depose_par_personne_id, p.retire_le, p.cree_le
           FROM ent_dem_piece p WHERE p.etablissement_id = $1 AND p.demande_id = $2 AND {filtre}
          ORDER BY p.cree_le, p.id"
    );
    let mut q = sqlx::query_as::<_, LignePiece>(&sql).bind(etab).bind(demande);
    if matches!(acces.lien, Lien::Consulte(_)) {
        q = q.bind(acces.agent);
    }
    let l: Vec<LignePiece> = q.fetch_all(conn).await?;
    Ok(l.into_iter()
        .map(|p| {
            (
                p.message_id,
                PieceVue {
                    id: p.id,
                    role: p.role,
                    au_demandeur: p.visibilite == "demandeur",
                    deposee_par_moi: p.depose_par_personne_id == acces.moi,
                    retiree: p.retire_le.is_some(),
                    le: p.cree_le,
                },
            )
        })
        .collect())
}

/// Cette pièce précise est-elle visible de cet acteur ? (même règle que [`pieces_visibles`])
async fn piece_lisible(conn: &mut PgConnection, etab: Uuid, piece: Uuid, acces: &Acces) -> Result<bool, RefusDemande> {
    let (demande,): (Uuid,) = sqlx::query_as("SELECT demande_id FROM ent_dem_piece WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(piece)
        .fetch_one(&mut *conn)
        .await?;
    Ok(pieces_visibles(conn, etab, demande, acces).await?.iter().any(|(_, p)| p.id == piece))
}
