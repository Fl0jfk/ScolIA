//! Cahier de textes, travail à faire / à rendre, copies, agenda et charge (lot S2-6, D7).
//!
//! Actions CDT-01 → CDT-10 de `docs/SOCLE2_DOMAINES.md`.
//!
//! * **Droits** : moteur D-017 (`cahier.lire`, `cahier.ecrire`, `cahier.viser`). La portée
//!   « mes classes » d'un professeur = ses services d'enseignement sur la classe ou le groupe
//!   visé, ou une séance datée qu'il assure (titulaire ou remplaçant). L'élève agit sur
//!   lui-même, un responsable selon son lien ; ni l'un ni l'autre n'a de rôle libre.
//! * **Cloisonnement** : toute lecture et toute écriture portent `etablissement_id` (de la
//!   session) ; une référence d'un autre établissement est introuvable.
//! * **Fichiers** : contrôlés comme tout dépôt (D2, `UsageFichier::PieceCahier` / `Copie`),
//!   stockés dans le magasin d'objets, ouverts par lien signé court après contrôle des droits.
//! * **Événements** : publiés dans la transaction du fait (outbox S2-0).

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::{Action, Portee, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_object::presign::{Disposition, LinkSpec, PresignError};
use ent_object::upload::UsageFichier;
use ent_object::ObjectStore;
use ent_types::cahier::*;
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::outbox::Publier;
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, LienFichier, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

/// Plafond de pièces par travail / contenu de séance / remise.
const MAX_PIECES: i64 = 10;
/// Plus longue période demandée en une fois (jours).
const MAX_PERIODE_JOURS: i64 = 100;
/// Seuils d'alerte de surcharge par défaut (modifiables par appel, bornés).
pub const SEUIL_MINUTES_DEFAUT: i32 = 120;
pub const SEUIL_CONTROLES_DEFAUT: i32 = 2;

fn refus(nature: RefusMetier, message: &'static str) -> EntDbError {
    EntDbError::Refus(nature, message)
}

fn invalide(message: &'static str) -> EntDbError {
    refus(RefusMetier::Invalide, message)
}

fn conflit(message: &'static str) -> EntDbError {
    refus(RefusMetier::Conflit, message)
}

/// Classe **ou** groupe visé par un contenu ou un travail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cible {
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
}

impl Cible {
    fn valider(self) -> Result<Self, EntDbError> {
        if self.classe_id.is_some() == self.groupe_id.is_some() {
            return Err(invalide("indiquez une classe ou un groupe (un seul)"));
        }
        Ok(self)
    }
}

/// Fichier déposé (octets déjà décodés ; contrôle de forme fait ici, D2).
#[derive(Debug, Clone)]
pub struct FichierDepot {
    pub nom: String,
    pub mime_type: String,
    pub contenu: Vec<u8>,
}

/// Où attacher une pièce du professeur.
#[derive(Debug, Clone, Copy)]
pub enum ParentPiece {
    Travail(Uuid),
    Seance(Uuid),
}

#[derive(Debug, Clone)]
pub struct NouveauTravail {
    pub seance_id: Option<Uuid>,
    pub cible: Option<Cible>,
    pub matiere_libelle: Option<String>,
    pub type_travail: String,
    pub titre: String,
    pub consigne: String,
    pub pour_le: NaiveDate,
    pub duree_estimee_min: Option<i32>,
    pub date_limite: Option<DateTime<Utc>>,
    pub acceptation_retard: bool,
    pub formats_acceptes: Vec<String>,
}

/// Vue sous laquelle un acteur consulte l'agenda d'un élève.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VueAgenda {
    Eleve,
    Famille,
    Personnel,
}

// ───────────────────────────── Lignes SQL ─────────────────────────────

/// Fragments `SELECT_*` : le filtre d'établissement (`$1`) est inclus ; chaque appelant
/// complète par `AND …` et lie l'établissement en premier.
const SELECT_TRAVAIL: &str = "SELECT t.id, t.seance_id, t.classe_id, t.groupe_id,
       COALESCE(c.code, g.code, '') AS cible, t.matiere_libelle, t.auteur_user_id,
       ent_cdt_nom_affichage(t.etablissement_id, t.auteur_user_id) AS auteur,
       t.type AS type_travail, t.titre, t.consigne, t.donne_le, t.pour_le, t.duree_estimee_min,
       t.rendu_en_ligne, t.date_limite, t.acceptation_retard, t.formats_acceptes, t.version
  FROM ent_travail t
  LEFT JOIN ent_classe c ON c.id = t.classe_id
  LEFT JOIN ent_groupe g ON g.id = t.groupe_id
 WHERE t.etablissement_id = $1";

/// L'élève `$2` est concerné par le travail `t` (affectation à la classe ou au groupe visé,
/// recoupant la période [donné le, pour le]).
const ELEVE_CONCERNE: &str = "EXISTS (SELECT 1 FROM ent_affectation a
         WHERE a.etablissement_id = t.etablissement_id AND a.eleve_id = $2
           AND (a.classe_id = t.classe_id OR a.groupe_id = t.groupe_id)
           AND a.date_debut <= ent_aujourdhui_etab(t.etablissement_id)
           AND (a.date_fin IS NULL OR a.date_fin >= ent_aujourdhui_etab(t.etablissement_id))
           AND NOT EXISTS (SELECT 1 FROM ent_eleve e
                            WHERE e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
                              AND e.date_sortie IS NOT NULL
                              AND e.date_sortie <= ent_aujourdhui_etab(t.etablissement_id)))";

#[derive(Debug, FromRow)]
struct TravailRow {
    id: Uuid,
    seance_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
    cible: String,
    matiere_libelle: String,
    auteur_user_id: Uuid,
    auteur: String,
    type_travail: String,
    titre: String,
    consigne: String,
    donne_le: NaiveDate,
    pour_le: NaiveDate,
    duree_estimee_min: Option<i32>,
    rendu_en_ligne: bool,
    date_limite: Option<DateTime<Utc>>,
    acceptation_retard: bool,
    formats_acceptes: Vec<String>,
    version: i32,
}

impl TravailRow {
    fn cible(&self) -> Cible {
        Cible {
            classe_id: self.classe_id,
            groupe_id: self.groupe_id,
        }
    }

    fn dto(self, pieces: Vec<PieceCahierDto>) -> TravailDto {
        TravailDto {
            id: self.id,
            seance_id: self.seance_id,
            classe_id: self.classe_id,
            groupe_id: self.groupe_id,
            cible: self.cible,
            matiere_libelle: self.matiere_libelle,
            auteur: self.auteur,
            type_travail: self.type_travail,
            titre: self.titre,
            consigne: self.consigne,
            donne_le: self.donne_le,
            pour_le: self.pour_le,
            duree_estimee_min: self.duree_estimee_min,
            rendu_en_ligne: self.rendu_en_ligne,
            date_limite: self.date_limite,
            acceptation_retard: self.acceptation_retard,
            formats_acceptes: self.formats_acceptes,
            version: self.version,
            pieces,
        }
    }
}

#[derive(Debug, FromRow)]
struct PieceRow {
    id: Uuid,
    genre: String,
    travail_id: Option<Uuid>,
    seance_contenu_id: Option<Uuid>,
    remise_id: Option<Uuid>,
    nom: String,
    mime_type: String,
    taille_octets: i64,
    storage_key: String,
    created_at: DateTime<Utc>,
}

impl PieceRow {
    fn dto(&self) -> PieceCahierDto {
        PieceCahierDto {
            id: self.id,
            genre: self.genre.clone(),
            nom: self.nom.clone(),
            mime_type: self.mime_type.clone(),
            taille_octets: self.taille_octets,
            created_at: self.created_at,
        }
    }
}

const SELECT_PIECE: &str = "SELECT id, genre, travail_id, seance_contenu_id, remise_id, nom,
       mime_type, taille_octets, storage_key, created_at FROM ent_cdt_piece
 WHERE etablissement_id = $1 AND retiree_le IS NULL";

#[derive(Debug, FromRow)]
struct RemiseRow {
    id: Uuid,
    travail_id: Uuid,
    eleve_id: Uuid,
    soumise_le: DateTime<Utc>,
    derniere_le: DateTime<Utc>,
    en_retard: bool,
    commentaire_eleve: String,
    statut: String,
    retour_commentaire: Option<String>,
    retour_le: Option<DateTime<Utc>>,
    note_id: Option<Uuid>,
}

const SELECT_REMISE: &str = "SELECT id, travail_id, eleve_id, soumise_le, derniere_le, en_retard,
       commentaire_eleve, statut, retour_commentaire, retour_le, note_id FROM ent_travail_remise
 WHERE etablissement_id = $1";

/// Vue du responsable habilité (R-C37) : le statut constaté, jamais les dépôts ni le commentaire de l'élève ;
/// le retour (commentaire, copie annotée) seulement une fois rendu et, s'il porte une note, avec le droit
/// « voir les notes ».
fn remise_pour_famille(mut r: RemiseDto, peut_notes: bool) -> RemiseDto {
    let retour_visible = r.statut == "corrige" && (r.note_id.is_none() || peut_notes);
    r.commentaire_eleve = String::new();
    r.pieces.retain(|p| retour_visible && p.genre == "annotation");
    if !retour_visible {
        r.statut = "rendu".into();
        r.retour_commentaire = None;
        r.retour_le = None;
        r.note_id = None;
    }
    r
}

impl RemiseRow {
    fn dto(self, pieces: Vec<PieceCahierDto>) -> RemiseDto {
        RemiseDto {
            id: self.id,
            travail_id: self.travail_id,
            eleve_id: self.eleve_id,
            soumise_le: self.soumise_le,
            derniere_le: self.derniere_le,
            en_retard: self.en_retard,
            commentaire_eleve: self.commentaire_eleve,
            statut: self.statut,
            retour_commentaire: self.retour_commentaire,
            retour_le: self.retour_le,
            note_id: self.note_id,
            pieces,
        }
    }
}

#[derive(Debug, FromRow)]
struct SeanceRow {
    id: Uuid,
    etablissement_id: Uuid,
    jour: NaiveDate,
    heure_debut: chrono::NaiveTime,
    heure_fin: chrono::NaiveTime,
    classe_id: Option<Uuid>,
    classe: String,
    service_id: Option<Uuid>,
    prof_user_id: Uuid,
    remplacant_user_id: Option<Uuid>,
    matiere_libelle: String,
    statut: String,
}

const SELECT_SEANCE: &str = "SELECT id, etablissement_id, jour, heure_debut, heure_fin, classe_id, classe, service_id,
       prof_user_id, remplacant_user_id, matiere_libelle, statut FROM ent_edt_seance
 WHERE etablissement_id = $1";

#[derive(Debug, FromRow)]
struct ContenuRow {
    id: Uuid,
    seance_id: Uuid,
    auteur_user_id: Uuid,
    auteur: String,
    contenu: String,
    ressources: String,
    version: i32,
    updated_at: DateTime<Utc>,
}

const SELECT_CONTENU: &str = "SELECT id, seance_id, auteur_user_id,
       ent_cdt_nom_affichage(etablissement_id, auteur_user_id) AS auteur, contenu, ressources, version,
       updated_at FROM ent_seance_contenu
 WHERE etablissement_id = $1";

impl PgEnt {
    // ───────────────────────────── Droits ─────────────────────────────

    /// La classe ou le groupe existe-t-il dans cet établissement ?
    async fn cible_existe(&self, etab: Uuid, cible: Cible) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2)
                 OR EXISTS (SELECT 1 FROM ent_groupe WHERE etablissement_id = $1 AND id = $3)",
        )
        .bind(etab)
        .bind(cible.classe_id)
        .bind(cible.groupe_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Jour courant à l'heure de l'établissement (C-D13) : toute règle « à la date de l'action »
    /// (portée d'un intervenant, public d'un cours, échéance dans le passé) se calcule sur ce jour,
    /// jamais sur le jour UTC.
    pub(crate) async fn jour_etab(&self, etab: Uuid) -> Result<NaiveDate, EntDbError> {
        let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1)")
            .bind(etab)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?;
        Ok(j)
    }

    /// L'acteur est-il, **à la date de l'action**, intervenant d'un cours dont le public est cette
    /// classe ou ce groupe ? `saisie` : intervention qui saisit effectivement (`droits ∋ cahier`,
    /// suppléance prise en compte, C-D2) ; sinon, lecture (délai de grâce de 7 jours du socle 6).
    async fn intervenant_sur_cible(
        &self,
        a: &Acteur<'_>,
        cible: Cible,
        saisie: bool,
    ) -> Result<bool, EntDbError> {
        let jour = self.jour_etab(a.etablissement_id).await?;
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
                SELECT 1 FROM ent_cours c
                 WHERE c.etablissement_id = $1
                   AND ((c.classe_id IS NOT NULL AND c.classe_id = $3)
                     OR (c.groupe_id IS NOT NULL AND c.groupe_id = $4))
                   AND CASE WHEN $6 THEN ent_cdt_saisie_cours($1, c.id, $2, $5)
                            ELSE ent_prof_sur_cours($1, c.id, $2, $5, false) END)",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .bind(cible.classe_id)
        .bind(cible.groupe_id)
        .bind(jour)
        .bind(saisie)
        .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// L'acteur peut-il exercer `action` sur cette classe ou ce groupe ? (union des portées).
    /// `CahierEcrire` implique `CahierLire`. Le personnel seul porte des droits paramétrables.
    ///
    /// Portée « mes classes » (C-D2) : calculée **à la date de l'action** par les interventions du
    /// socle 6 (`ent_cdt_saisie_cours` pour écrire, `ent_prof_sur_cours` pour lire), jamais par un
    /// service ou une séance passés : un ancien professeur ou un remplaçant d'une heure n'a plus rien.
    pub async fn cahier_peut(
        &self,
        a: &Acteur<'_>,
        action: Action,
        cible: Cible,
    ) -> Result<bool, EntDbError> {
        if !self.cible_existe(a.etablissement_id, cible).await? {
            return Ok(false);
        }
        let droits = self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?;
        let (mut classes, mut groupes) = (false, false);
        let mut designes: Vec<Uuid> = Vec::new();
        for d in droits
            .iter()
            .filter(|d| d.action == action || (action == Action::CahierLire && d.action == Action::CahierEcrire))
        {
            match d.portee {
                Portee::Etablissement => return Ok(true),
                Portee::MesClasses => classes = true,
                Portee::MesGroupes => groupes = true,
                Portee::Designes | Portee::MonPerimetre | Portee::SitesDesignes => {
                    designes.extend(d.cibles.iter().copied())
                }
                Portee::Moi => {}
            }
        }
        if designes
            .iter()
            .any(|id| Some(*id) == cible.classe_id || Some(*id) == cible.groupe_id)
        {
            return Ok(true);
        }
        if classes && self.intervenant_sur_cible(a, cible, action == Action::CahierEcrire).await? {
            return Ok(true);
        }
        if groupes {
            if let Some(g) = cible.groupe_id {
                let (ok,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_groupe_responsable
                       WHERE etablissement_id = $1 AND user_id = $2 AND groupe_id = $3)",
                )
                .bind(a.etablissement_id)
                .bind(a.user_id)
                .bind(g)
                .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
                .await
                .map_err(db)?;
                if ok {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Ancienne route `/cahier-texte` (C-D7) : la classe est désignée par son code ; même contrôle que la
    /// commande `cahier` sur la classe correspondante. Un code inconnu est refusé comme un accès interdit.
    pub(crate) async fn exiger_classe_texte(
        &self,
        a: &Acteur<'_>,
        code: &str,
        action: Action,
    ) -> Result<(), EntDbError> {
        let classe: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_classe WHERE etablissement_id = $1 AND code = $2")
                .bind(a.etablissement_id)
                .bind(code.trim())
                .fetch_optional(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
                .await
                .map_err(db)?;
        let Some((classe_id,)) = classe else {
            return Err(EntDbError::Forbidden);
        };
        let cible = Cible {
            classe_id: Some(classe_id),
            groupe_id: None,
        };
        match self.exiger_cible(a, action, cible).await {
            Err(EntDbError::Forbidden) => {}
            autre => return autre,
        }
        // Repli « classe texte » (socle 6, `ent_repli_texte_actif`) : tant qu'AUCUN cours n'existe pour la classe
        // (établissement sans services d'enseignement), l'ancien périmètre (RF-04) s'applique, avec le droit du moteur.
        let (sans_cours,): (bool,) = sqlx::query_as(
            "SELECT NOT EXISTS (SELECT 1 FROM ent_cours WHERE etablissement_id = $1 AND classe_id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(classe_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        let droit = if action == Action::CahierEcrire {
            self.cahier_droit_ecrire_global(a).await?
        } else {
            self.droits_effectifs(a.etablissement_id, a.user_id, a.role)
                .await?
                .iter()
                .any(|d| matches!(d.action, Action::CahierLire | Action::CahierEcrire))
        };
        if sans_cours && droit {
            return self
                .exiger_classe_dans_perimetre(a.etablissement_id, a.user_id, a.role, code)
                .await;
        }
        Err(EntDbError::Forbidden)
    }

    /// Responsable habilité à suivre la scolarité de l'élève (lien actif et autorité parentale).
    pub(crate) async fn parent_suit_scolarite(&self, a: &Acteur<'_>, eleve_id: Uuid) -> Result<bool, EntDbError> {
        self.parent_lie(a, eleve_id).await
    }

    async fn exiger_cible(
        &self,
        a: &Acteur<'_>,
        action: Action,
        cible: Cible,
    ) -> Result<(), EntDbError> {
        if self.cahier_peut(a, action, cible).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Copies, suivi des remises et retours (C-D2, C-D3) : seul l'**intervenant en saisie à la date**
    /// sur le cours (et le moteur : `cahier.ecrire`) ouvre le droit. Ni la direction ni un autre
    /// professeur de la classe ni un remplaçant d'une séance (lecture sur motif : lot ultérieur).
    async fn exiger_suivi_remises(&self, a: &Acteur<'_>, cible: Cible) -> Result<(), EntDbError> {
        if !self.cahier_droit_ecrire_global(a).await? || !self.intervenant_sur_cible(a, cible, true).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(())
    }

    /// Le compte est-il le remplaçant désigné de cette séance (et d'elle seule) ?
    async fn est_remplacant_de(&self, etab: Uuid, seance_id: Uuid, user: Uuid) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_edt_seance WHERE etablissement_id = $1 AND id = $2
                             AND remplacant_user_id = $3 AND statut <> 'annulee')",
        )
        .bind(etab)
        .bind(seance_id)
        .bind(user)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Écriture sur une séance datée (contenu, pièces de séance) : intervenant en saisie à la date sur
    /// son cours ; remplaçant désigné **de cette séance seulement** ; à défaut de cours (séance « classe
    /// texte » héritée), son titulaire. Renvoie la cible de droit de la séance.
    async fn exiger_ecriture_seance(&self, a: &Acteur<'_>, s: &SeanceRow) -> Result<Cible, EntDbError> {
        let cible = self.cible_de_seance(s).await?;
        if s.remplacant_user_id == Some(a.user_id)
            && s.statut != "annulee"
            && self.cahier_droit_ecrire_global(a).await?
        {
            return Ok(cible);
        }
        if self.cahier_peut(a, Action::CahierEcrire, cible).await? {
            return Ok(cible);
        }
        let (sans_cours,): (bool,) = sqlx::query_as("SELECT ent_cdt_cours_de_seance($1, $2) IS NULL")
            .bind(s.etablissement_id)
            .bind(s.id)
            .fetch_one(&mut *TenantTx::begin(self.pool(), s.etablissement_id).await?)
            .await
            .map_err(db)?;
        if sans_cours && s.prof_user_id == a.user_id && self.cahier_droit_ecrire_global(a).await? {
            return Ok(cible);
        }
        Err(EntDbError::Forbidden)
    }

    /// Élève rattaché au compte de l'acteur (profil `eleve` seulement).
    async fn eleve_du_compte(&self, a: &Acteur<'_>) -> Result<Option<Uuid>, EntDbError> {
        if Role::parse(a.role) != Some(Role::Eleve) {
            return Ok(None);
        }
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        Ok(r.map(|(e,)| e))
    }

    /// Responsable habilité à **suivre la scolarité** de l'élève (C-D4) : lien actif (ni suspendu ni hors
    /// dates, `ent_eleve_lien_actif`) **et** exercice de l'autorité parentale. Tant que le socle 4 n'expose
    /// pas de drapeau « suivi » dédié (responsable informé sans autorité), l'autorité parentale en tient lieu.
    async fn parent_lie(&self, a: &Acteur<'_>, eleve_id: Uuid) -> Result<bool, EntDbError> {
        if Role::parse(a.role) != Some(Role::Parent) {
            return Ok(false);
        }
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_actif l
                JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
               WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.eleve_id = $3 AND m.role = 'parent'
                 AND l.autorite_parentale)",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .bind(eleve_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Le responsable a-t-il aussi le droit « voir les notes » (retour portant une note) ?
    async fn parent_voit_notes(&self, a: &Acteur<'_>, eleve_id: Uuid) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_actif l
               WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.eleve_id = $3
                 AND l.autorite_parentale AND l.peut_voir_notes)",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .bind(eleve_id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Résout l'élève dont l'acteur consulte l'agenda : lui-même, son enfant (lien), ou un élève
    /// de sa portée `cahier.lire` (personnel).
    pub async fn cahier_resoudre_eleve(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
    ) -> Result<(Uuid, VueAgenda), EntDbError> {
        if let Some(propre) = self.eleve_du_compte(a).await? {
            return match eleve_id {
                None => Ok((propre, VueAgenda::Eleve)),
                Some(e) if e == propre => Ok((propre, VueAgenda::Eleve)),
                Some(_) => Err(EntDbError::Forbidden),
            };
        }
        let Some(eleve) = eleve_id else {
            return Err(invalide("indiquez l'élève"));
        };
        match Role::parse(a.role) {
            Some(Role::Parent) => {
                if self.parent_lie(a, eleve).await? {
                    Ok((eleve, VueAgenda::Famille))
                } else {
                    Err(EntDbError::Forbidden)
                }
            }
            Some(r) if r.is_staff() => {
                // Un élève d'un autre établissement n'existe pas pour cet acteur.
                let (existe,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
                )
                .bind(a.etablissement_id)
                .bind(eleve)
                .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
                .await
                .map_err(db)?;
                if existe
                    && self
                    .autoriser_eleve(a.etablissement_id, a.user_id, a.role, Action::CahierLire, eleve)
                    .await?
                {
                    Ok((eleve, VueAgenda::Personnel))
                } else {
                    Err(EntDbError::Forbidden)
                }
            }
            _ => Err(EntDbError::Forbidden),
        }
    }

    // ───────────────────────────── Lectures communes ─────────────────────────────

    async fn charger_travail(&self, etab: Uuid, id: Uuid) -> Result<TravailRow, EntDbError> {
        sqlx::query_as::<_, TravailRow>(&format!(
            "{SELECT_TRAVAIL} AND t.id = $2 AND t.supprime_le IS NULL"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    async fn pieces_de(
        &self,
        etab: Uuid,
        colonne: &'static str,
        ids: &[Uuid],
        genres: &[&str],
    ) -> Result<HashMap<Uuid, Vec<PieceRow>>, EntDbError> {
        let genres: Vec<String> = genres.iter().map(|g| (*g).to_string()).collect();
        let rows: Vec<PieceRow> = sqlx::query_as(&format!(
            "{SELECT_PIECE} AND {colonne} = ANY($2) AND genre = ANY($3)
              ORDER BY created_at, id"
        ))
        .bind(etab)
        .bind(ids)
        .bind(&genres)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let mut out: HashMap<Uuid, Vec<PieceRow>> = HashMap::new();
        for r in rows {
            let parent = match colonne {
                "travail_id" => r.travail_id,
                "seance_contenu_id" => r.seance_contenu_id,
                _ => r.remise_id,
            };
            if let Some(p) = parent {
                out.entry(p).or_default().push(r);
            }
        }
        Ok(out)
    }

    async fn travaux_dto(
        &self,
        etab: Uuid,
        rows: Vec<TravailRow>,
    ) -> Result<Vec<TravailDto>, EntDbError> {
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let mut pieces = self
            .pieces_de(etab, "travail_id", &ids, &["ressource"])
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let p = pieces
                    .remove(&r.id)
                    .unwrap_or_default()
                    .iter()
                    .map(PieceRow::dto)
                    .collect();
                r.dto(p)
            })
            .collect())
    }

    async fn remises_dto(
        &self,
        etab: Uuid,
        rows: Vec<RemiseRow>,
    ) -> Result<Vec<RemiseDto>, EntDbError> {
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let mut pieces = self
            .pieces_de(etab, "remise_id", &ids, &["copie", "annotation"])
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let p = pieces
                    .remove(&r.id)
                    .unwrap_or_default()
                    .iter()
                    .map(PieceRow::dto)
                    .collect();
                r.dto(p)
            })
            .collect())
    }

    /// L'élève fait partie du public du travail **à la date de l'action** (jour de l'établissement) :
    /// affectation active à la classe ou au groupe visé, élève non parti (C-D5). Un élève arrivé après la
    /// donnée du travail le voit (il rattrape) ; un élève parti ou passé dans un autre groupe ne le voit plus.
    async fn eleve_concerne(&self, etab: Uuid, eleve: Uuid, t: &TravailRow) -> Result<bool, EntDbError> {
        let jour = self.jour_etab(etab).await?;
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_affectation a
               JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
               WHERE a.etablissement_id = $1 AND a.eleve_id = $2
                 AND ((a.classe_id IS NOT NULL AND a.classe_id = $3)
                   OR (a.groupe_id IS NOT NULL AND a.groupe_id = $4))
                 AND a.date_debut <= $5 AND (a.date_fin IS NULL OR a.date_fin >= $5)
                 AND (e.date_sortie IS NULL OR e.date_sortie > $5))",
        )
        .bind(etab)
        .bind(eleve)
        .bind(t.classe_id)
        .bind(t.groupe_id)
        .bind(jour)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// Lecture d'un travail par l'acteur : personnel (`cahier.lire` sur la cible) ou élève
    /// concerné / responsable lié. Renvoie la vue qui a autorisé l'accès.
    async fn lire_travail_autorise(
        &self,
        a: &Acteur<'_>,
        t: &TravailRow,
    ) -> Result<VueAgenda, EntDbError> {
        if let Some(e) = self.eleve_du_compte(a).await? {
            return if self.eleve_concerne(a.etablissement_id, e, t).await?
                || self.remise_de(a.etablissement_id, t.id, e).await?.is_some()
            {
                Ok(VueAgenda::Eleve)
            } else {
                Err(EntDbError::Forbidden)
            };
        }
        if Role::parse(a.role) == Some(Role::Parent) {
            let enfants: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT eleve_id FROM ent_eleve_lien_actif WHERE etablissement_id = $1 AND user_id = $2",
            )
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
            .await
            .map_err(db)?;
            for (e,) in enfants {
                if self.parent_lie(a, e).await? && self.eleve_concerne(a.etablissement_id, e, t).await? {
                    return Ok(VueAgenda::Famille);
                }
            }
            return Err(EntDbError::Forbidden);
        }
        self.exiger_cible(a, Action::CahierLire, t.cible()).await?;
        Ok(VueAgenda::Personnel)
    }

    // ───────────────────────────── CDT-01 : séances ─────────────────────────────

    async fn charger_seance(&self, etab: Uuid, id: Uuid) -> Result<SeanceRow, EntDbError> {
        sqlx::query_as::<_, SeanceRow>(&format!("{SELECT_SEANCE} AND id = $2"))
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)
    }

    /// Séance rattachée à une classe : cible de droit (classe de la séance, ou groupe du service).
    async fn cible_de_seance(&self, s: &SeanceRow) -> Result<Cible, EntDbError> {
        if let Some(sid) = s.service_id {
            let g: Option<(Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
                "SELECT groupe_id, classe_id FROM ent_service_enseignement
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(s.etablissement_id)
            .bind(sid)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), s.etablissement_id).await?)
            .await
            .map_err(db)?;
            if let Some((Some(g), _)) = g {
                return Ok(Cible {
                    classe_id: None,
                    groupe_id: Some(g),
                });
            }
        }
        Ok(Cible {
            classe_id: s.classe_id,
            groupe_id: None,
        })
    }

    async fn seance_dto(&self, etab: Uuid, s: SeanceRow) -> Result<SeanceCahierDto, EntDbError> {
        let contenu = self.contenu_de_seance(etab, s.id).await?;
        let (nb,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_travail WHERE etablissement_id = $1 AND seance_id = $2 AND supprime_le IS NULL",
        )
        .bind(etab)
        .bind(s.id)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(SeanceCahierDto {
            seance_id: s.id,
            jour: s.jour,
            heure_debut: s.heure_debut.format("%H:%M:%S").to_string(),
            heure_fin: s.heure_fin.format("%H:%M:%S").to_string(),
            classe_id: s.classe_id,
            classe: s.classe,
            matiere_libelle: s.matiere_libelle,
            statut: s.statut,
            contenu,
            nb_travaux: nb,
        })
    }

    async fn contenu_de_seance(
        &self,
        etab: Uuid,
        seance_id: Uuid,
    ) -> Result<Option<SeanceContenuDto>, EntDbError> {
        let Some(c) = sqlx::query_as::<_, ContenuRow>(&format!(
            "{SELECT_CONTENU} AND seance_id = $2"
        ))
        .bind(etab)
        .bind(seance_id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        else {
            return Ok(None);
        };
        let mut p = self
            .pieces_de(etab, "seance_contenu_id", &[c.id], &["seance"])
            .await?;
        let pieces = p
            .remove(&c.id)
            .unwrap_or_default()
            .iter()
            .map(PieceRow::dto)
            .collect();
        Ok(Some(SeanceContenuDto {
            id: c.id,
            seance_id: c.seance_id,
            auteur: c.auteur,
            contenu: c.contenu,
            ressources: c.ressources,
            version: c.version,
            pieces,
            updated_at: c.updated_at,
        }))
    }

    fn periode(du: NaiveDate, au: NaiveDate) -> Result<(), EntDbError> {
        if au < du || (au - du).num_days() > MAX_PERIODE_JOURS {
            return Err(invalide("période invalide (100 jours au plus)"));
        }
        Ok(())
    }

    /// Séances datées de la période. Sans `classe_id` : celles du professeur connecté (titulaire
    /// ou remplaçant). Avec `classe_id` : celles de la classe (`cahier.lire` sur la classe).
    pub async fn cahier_seances(
        &self,
        a: &Acteur<'_>,
        classe_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceCahierDto>, EntDbError> {
        Self::periode(du, au)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let rows: Vec<SeanceRow> = if let Some(c) = classe_id {
            self.exiger_cible(
                a,
                Action::CahierLire,
                Cible {
                    classe_id: Some(c),
                    groupe_id: None,
                },
            )
            .await?;
            // Une séance de groupe (LV2, option…) n'est pas une séance de la classe : seuls son professeur,
            // son remplaçant et un acteur à portée d'établissement la voient dans la liste de la classe.
            let large = self.cahier_peut_etablissement(a).await?;
            sqlx::query_as(&format!(
                "{SELECT_SEANCE} AND classe_id = $2 AND jour BETWEEN $3 AND $4
                   AND ($6 OR $5 IN (prof_user_id, remplacant_user_id)
                        OR (NOT EXISTS (SELECT 1 FROM ent_edt_creneau cr
                                         WHERE cr.etablissement_id = ent_edt_seance.etablissement_id
                                           AND cr.id = ent_edt_seance.creneau_id AND cr.groupe_id IS NOT NULL)
                            AND NOT EXISTS (SELECT 1 FROM ent_service_enseignement sv
                                             WHERE sv.etablissement_id = ent_edt_seance.etablissement_id
                                               AND sv.id = ent_edt_seance.service_id AND sv.groupe_id IS NOT NULL)))
                  ORDER BY jour, heure_debut"
            ))
            .bind(a.etablissement_id)
            .bind(c)
            .bind(du)
            .bind(au)
            .bind(a.user_id)
            .bind(large)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
        } else {
            let droits = self
                .droits_effectifs(a.etablissement_id, a.user_id, a.role)
                .await?;
            if !droits
                .iter()
                .any(|d| matches!(d.action, Action::CahierEcrire | Action::CahierLire))
            {
                return Err(EntDbError::Forbidden);
            }
            sqlx::query_as(&format!(
                "{SELECT_SEANCE} AND (prof_user_id = $2 OR remplacant_user_id = $2)
                   AND jour BETWEEN $3 AND $4 ORDER BY jour, heure_debut"
            ))
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .bind(du)
            .bind(au)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
        };
        let mut out = Vec::with_capacity(rows.len());
        for s in rows {
            out.push(self.seance_dto(a.etablissement_id, s).await?);
        }
        Ok(out)
    }

    /// CDT-01 : renseigne (ou corrige) le contenu d'une séance datée.
    pub async fn cahier_renseigner_seance(
        &self,
        a: &Acteur<'_>,
        seance_id: Uuid,
        contenu: &str,
        ressources: &str,
    ) -> Result<SeanceContenuDto, EntDbError> {
        let contenu = contenu.trim();
        let ressources = ressources.trim();
        if contenu.is_empty() || contenu.chars().count() > 8000 {
            return Err(invalide("contenu de séance vide ou trop long"));
        }
        if ressources.chars().count() > 4000 {
            return Err(invalide("ressources trop longues"));
        }
        let seance = self.charger_seance(a.etablissement_id, seance_id).await?;
        if seance.statut == "annulee" {
            return Err(conflit("la séance est annulée"));
        }
        let cible = self.exiger_ecriture_seance(a, &seance).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let existant: Option<ContenuRow> = sqlx::query_as(&format!(
            "{SELECT_CONTENU} AND seance_id = $2 FOR UPDATE"
        ))
        .bind(a.etablissement_id)
        .bind(seance_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let id = match existant {
            Some(c) => {
                historiser(
                    &mut tx,
                    a,
                    "seance_contenu",
                    c.id,
                    c.version,
                    "modification",
                    json!({ "contenu": c.contenu, "ressources": c.ressources, "auteur_user_id": c.auteur_user_id }),
                )
                .await?;
                sqlx::query(
                    "UPDATE ent_seance_contenu SET contenu = $3, ressources = $4, version = version + 1,
                            auteur_user_id = $5, updated_at = now()
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(c.id)
                .bind(contenu)
                .bind(ressources)
                .bind(a.user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                c.id
            }
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_seance_contenu (id, etablissement_id, seance_id, auteur_user_id, contenu, ressources)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(id)
                .bind(a.etablissement_id)
                .bind(seance_id)
                .bind(a.user_id)
                .bind(contenu)
                .bind(ressources)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                id
            }
        };
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_SEANCE_RENSEIGNEE,
            id,
            None,
            json!({
                "seance_id": seance_id,
                "classe_id": cible.classe_id,
                "groupe_id": cible.groupe_id,
                "jour": seance.jour.to_string(),
            }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.contenu_de_seance(a.etablissement_id, seance_id)
            .await?
            .ok_or_else(introuvable)
    }

    /// L'acteur lit-il le cahier à portée d'établissement (direction, vie scolaire, administratif) ?
    async fn cahier_peut_etablissement(&self, a: &Acteur<'_>) -> Result<bool, EntDbError> {
        Ok(self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?
            .iter()
            .any(|d| {
                matches!(d.action, Action::CahierLire | Action::CahierEcrire) && d.portee == Portee::Etablissement
            }))
    }

    /// Le profil a-t-il, quelque part, le droit d'écrire dans le cahier ? (garde d'un
    /// titulaire de séance : un rôle sans `cahier.ecrire` ne peut pas écrire même sur sa séance.)
    async fn cahier_droit_ecrire_global(&self, a: &Acteur<'_>) -> Result<bool, EntDbError> {
        Ok(self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?
            .iter()
            .any(|d| d.action == Action::CahierEcrire))
    }

    /// Cahier d'un élève (vue élève / famille / personnel autorisé) : contenus des séances de
    /// sa classe et de ses groupes. Une séance d'un autre groupe n'apparaît pas.
    pub async fn cahier_seances_eleve(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceCahierDto>, EntDbError> {
        Self::periode(du, au)?;
        let (eleve, _) = self.cahier_resoudre_eleve(a, eleve_id).await?;
        let rows: Vec<SeanceRow> = sqlx::query_as(
            "SELECT x.id, x.etablissement_id, x.jour, x.heure_debut, x.heure_fin, x.classe_id, x.classe, x.service_id,
                    x.prof_user_id, x.remplacant_user_id, x.matiere_libelle, x.statut
               FROM ent_edt_seance x
               LEFT JOIN ent_service_enseignement s ON s.id = x.service_id
              WHERE x.etablissement_id = $1 AND x.jour BETWEEN $3 AND $4
                AND EXISTS (SELECT 1 FROM ent_seance_contenu c WHERE c.etablissement_id = x.etablissement_id AND c.seance_id = x.id)
                AND EXISTS (SELECT 1 FROM ent_affectation a
                             WHERE a.etablissement_id = x.etablissement_id AND a.eleve_id = $2
                               AND a.date_debut <= x.jour AND (a.date_fin IS NULL OR a.date_fin >= x.jour)
                               AND ((s.groupe_id IS NULL AND a.classe_id = x.classe_id)
                                 OR (s.groupe_id IS NOT NULL AND a.groupe_id = s.groupe_id)))
                -- C-D5 : un élève parti ou sorti du public ne voit plus le cours (il garde ses remises).
                AND EXISTS (SELECT 1 FROM ent_affectation a2
                              JOIN ent_eleve e2 ON e2.etablissement_id = a2.etablissement_id AND e2.id = a2.eleve_id
                             WHERE a2.etablissement_id = x.etablissement_id AND a2.eleve_id = $2
                               AND a2.date_debut <= ent_aujourdhui_etab(x.etablissement_id)
                               AND (a2.date_fin IS NULL OR a2.date_fin >= ent_aujourdhui_etab(x.etablissement_id))
                               AND (e2.date_sortie IS NULL OR e2.date_sortie > ent_aujourdhui_etab(x.etablissement_id))
                               AND ((s.groupe_id IS NULL AND a2.classe_id = x.classe_id)
                                 OR (s.groupe_id IS NOT NULL AND a2.groupe_id = s.groupe_id)))
              ORDER BY x.jour, x.heure_debut",
        )
        .bind(a.etablissement_id)
        .bind(eleve)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for s in rows {
            out.push(self.seance_dto(a.etablissement_id, s).await?);
        }
        Ok(out)
    }

    // ───────────────────────────── CDT-02 / 03 / 05 : travaux ─────────────────────────────

    fn formats_valides(formats: &[String]) -> Result<Vec<String>, EntDbError> {
        let admis: Vec<&'static str> = UsageFichier::Copie
            .autorises()
            .iter()
            .map(|t| t.extension())
            .collect();
        let mut out: Vec<String> = Vec::new();
        for f in formats {
            let mut f = f.trim().trim_start_matches('.').to_ascii_lowercase();
            if f == "jpeg" {
                f = "jpg".into();
            }
            if !admis.contains(&f.as_str()) {
                return Err(invalide("format de fichier non accepté"));
            }
            if !out.contains(&f) {
                out.push(f);
            }
        }
        Ok(out)
    }

    /// CDT-02 / CDT-05 : donne un travail (à faire, à rendre en ligne, ou contrôle annoncé).
    pub async fn cahier_donner_travail(
        &self,
        a: &Acteur<'_>,
        n: NouveauTravail,
    ) -> Result<TravailCreeDto, EntDbError> {
        let titre = n.titre.trim().to_string();
        let consigne = n.consigne.trim().to_string();
        if titre.is_empty() || titre.chars().count() > 200 {
            return Err(invalide("titre vide ou trop long"));
        }
        if consigne.is_empty() || consigne.chars().count() > 8000 {
            return Err(invalide("consigne vide ou trop longue"));
        }
        if !matches!(n.type_travail.as_str(), "a_faire" | "a_rendre" | "controle") {
            return Err(invalide("type de travail inconnu"));
        }
        if n.duree_estimee_min.is_some_and(|d| !(1..=600).contains(&d)) {
            return Err(invalide("durée estimée invalide (1 à 600 minutes)"));
        }
        let a_rendre = n.type_travail == "a_rendre";
        if !a_rendre && (n.date_limite.is_some() || !n.formats_acceptes.is_empty()) {
            return Err(invalide("date limite et formats ne concernent que le travail à rendre"));
        }
        let formats = Self::formats_valides(&n.formats_acceptes)?;
        let aujourdhui = self.jour_etab(a.etablissement_id).await?;
        if n.pour_le < aujourdhui {
            return Err(invalide("l'échéance est dans le passé"));
        }

        // Séance d'origine : fournit la cible, la matière et la date de départ.
        let (seance, mut cible, mut matiere) = match n.seance_id {
            Some(sid) => {
                let s = self.charger_seance(a.etablissement_id, sid).await?;
                if n.pour_le < s.jour {
                    return Err(invalide("l'échéance précède la séance"));
                }
                let derive = self.cible_de_seance(&s).await?;
                let m = s.matiere_libelle.clone();
                (Some(s), Some(derive), Some(m))
            }
            None => (None, None, None),
        };
        let cible_de_la_seance = cible;
        if let Some(c) = n.cible {
            let c = c.valider()?;
            if let (Some(derive), Some(s)) = (cible, &seance) {
                let coherent = c == derive
                    || match (c.groupe_id, derive.classe_id) {
                        (Some(g), Some(_)) => self.groupe_de_classe(a.etablissement_id, g, s.classe_id).await?,
                        _ => false,
                    };
                if !coherent {
                    return Err(invalide("la cible ne correspond pas à la séance"));
                }
            }
            cible = Some(c);
        }
        let Some(cible) = cible else {
            return Err(invalide("indiquez la classe, le groupe ou la séance"));
        };
        if let Some(m) = n.matiere_libelle.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
            matiere = Some(m.to_string());
        }
        let Some(matiere) = matiere else {
            return Err(invalide("indiquez la matière"));
        };
        if matiere.chars().count() > 120 {
            return Err(invalide("matière trop longue"));
        }
        // Le remplaçant d'une séance peut donner du travail **lié à cette séance** (et à sa cible), rien de plus.
        let borne = match (&seance, cible_de_la_seance) {
            (Some(s), Some(derive)) => {
                derive == cible
                    && s.remplacant_user_id == Some(a.user_id)
                    && s.statut != "annulee"
                    && self.cahier_droit_ecrire_global(a).await?
            }
            _ => false,
        };
        if !borne {
            self.exiger_cible(a, Action::CahierEcrire, cible).await?;
        }

        let donne_le = seance
            .as_ref()
            .map(|s| s.jour.min(aujourdhui))
            .unwrap_or(aujourdhui);
        let date_limite = if a_rendre {
            let limite = match n.date_limite {
                Some(l) => l,
                None => self.fin_de_journee(a.etablissement_id, n.pour_le).await?,
            };
            if limite <= Utc::now() {
                return Err(invalide("la date limite est dans le passé"));
            }
            Some(limite)
        } else {
            None
        };

        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_travail (id, etablissement_id, seance_id, classe_id, groupe_id, matiere_libelle,
                    auteur_user_id, type, titre, consigne, donne_le, pour_le, duree_estimee_min,
                    rendu_en_ligne, date_limite, acceptation_retard, formats_acceptes)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(n.seance_id)
        .bind(cible.classe_id)
        .bind(cible.groupe_id)
        .bind(&matiere)
        .bind(a.user_id)
        .bind(&n.type_travail)
        .bind(&titre)
        .bind(&consigne)
        .bind(donne_le)
        .bind(n.pour_le)
        .bind(n.duree_estimee_min)
        .bind(a_rendre)
        .bind(date_limite)
        .bind(n.acceptation_retard)
        .bind(&formats)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            if a_rendre {
                types::CAHIER_TRAVAIL_A_RENDRE
            } else {
                types::CAHIER_TRAVAIL_DONNE
            },
            id,
            None,
            json!({
                "travail_id": id,
                "classe_id": cible.classe_id,
                "groupe_id": cible.groupe_id,
                "pour_le": n.pour_le.to_string(),
                "type": n.type_travail,
            }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        let travail = self.cahier_travail_dto(a.etablissement_id, id).await?;
        let charge = self
            .charge_jour(a.etablissement_id, cible, n.pour_le, SEUIL_MINUTES_DEFAUT, SEUIL_CONTROLES_DEFAUT)
            .await?;
        Ok(TravailCreeDto { travail, charge })
    }

    async fn groupe_de_classe(
        &self,
        etab: Uuid,
        groupe: Uuid,
        classe: Option<Uuid>,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_groupe WHERE etablissement_id = $1 AND id = $2
                             AND classe_id IS NOT DISTINCT FROM $3 AND $3::uuid IS NOT NULL)",
        )
        .bind(etab)
        .bind(groupe)
        .bind(classe)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(ok)
    }

    async fn cahier_travail_dto(&self, etab: Uuid, id: Uuid) -> Result<TravailDto, EntDbError> {
        let row = self.charger_travail(etab, id).await?;
        Ok(self.travaux_dto(etab, vec![row]).await?.remove(0))
    }

    /// Détail d'un travail pour l'acteur. Un élève qui consulte son propre travail le marque « vu ».
    pub async fn cahier_travail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<AgendaItemDto, EntDbError> {
        let row = self.charger_travail(a.etablissement_id, id).await?;
        let vue = self.lire_travail_autorise(a, &row).await?;
        let eleve = if vue == VueAgenda::Eleve {
            self.eleve_du_compte(a).await?
        } else {
            None
        };
        let (mut fait, mut vu, mut remise) = (false, false, None);
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        if let Some(e) = eleve {
            sqlx::query(
                "INSERT INTO ent_travail_eleve (travail_id, eleve_id, etablissement_id, vu_le)
                 VALUES ($1, $2, $3, now())
                 ON CONFLICT (travail_id, eleve_id) DO UPDATE SET vu_le = COALESCE(ent_travail_eleve.vu_le, now())",
            )
            .bind(id)
            .bind(e)
            .bind(a.etablissement_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let (f,): (bool,) = sqlx::query_as(
                "SELECT fait FROM ent_travail_eleve
                  WHERE travail_id = $1 AND eleve_id = $2 AND etablissement_id = $3",
            )
            .bind(id)
            .bind(e)
            .bind(a.etablissement_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            fait = f;
            vu = true;
            remise = self.remise_de(a.etablissement_id, id, e).await?;
        }
        let travail = self.travaux_dto(a.etablissement_id, vec![row]).await?.remove(0);
        tx.commit().await?;
        Ok(AgendaItemDto {
            travail,
            fait,
            vu,
            remise,
        })
    }

    async fn remise_de(
        &self,
        etab: Uuid,
        travail: Uuid,
        eleve: Uuid,
    ) -> Result<Option<RemiseDto>, EntDbError> {
        let r: Option<RemiseRow> = sqlx::query_as(&format!(
            "{SELECT_REMISE} AND travail_id = $2 AND eleve_id = $3"
        ))
        .bind(etab)
        .bind(travail)
        .bind(eleve)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        match r {
            None => Ok(None),
            Some(r) => Ok(self.remises_dto(etab, vec![r]).await?.pop()),
        }
    }

    /// Travaux d'une classe ou d'un groupe sur une période (personnel, `cahier.lire`).
    pub async fn cahier_travaux_cible(
        &self,
        a: &Acteur<'_>,
        cible: Cible,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<TravailDto>, EntDbError> {
        Self::periode(du, au)?;
        let cible = cible.valider()?;
        self.exiger_cible(a, Action::CahierLire, cible).await?;
        let rows: Vec<TravailRow> = sqlx::query_as(&format!(
            "{SELECT_TRAVAIL} AND t.supprime_le IS NULL
               AND t.pour_le BETWEEN $4 AND $5
               AND ((t.classe_id IS NOT NULL AND t.classe_id = $2) OR (t.groupe_id IS NOT NULL AND t.groupe_id = $3))
             ORDER BY t.pour_le, t.created_at"
        ))
        .bind(a.etablissement_id)
        .bind(cible.classe_id)
        .bind(cible.groupe_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)?;
        self.travaux_dto(a.etablissement_id, rows).await
    }

    /// Droit de modifier : **l'auteur seulement** (C-D10), avec `cahier.ecrire` sur la cible à la date, ou en
    /// tant que remplaçant de la séance à laquelle le travail est lié. La direction ne réécrit jamais le travail
    /// d'un collègue : elle le **retire de la diffusion** (`cahier_retirer_travail`, avec motif et avis).
    async fn exiger_modification(&self, a: &Acteur<'_>, t: &TravailRow) -> Result<(), EntDbError> {
        if t.auteur_user_id != a.user_id {
            return Err(EntDbError::Forbidden);
        }
        if self.cahier_peut(a, Action::CahierEcrire, t.cible()).await? {
            return Ok(());
        }
        if let Some(sid) = t.seance_id {
            if self.cahier_droit_ecrire_global(a).await?
                && self.est_remplacant_de(a.etablissement_id, sid, a.user_id).await?
            {
                return Ok(());
            }
        }
        Err(EntDbError::Forbidden)
    }

    /// C-D10 : la direction (`cahier.viser` à portée d'établissement) retire un travail de la diffusion
    /// (contenu inapproprié). Le travail disparaît des agendas, l'historique garde l'instantané, l'auteur
    /// reçoit un avis avec le motif ; rien n'est réécrit.
    pub async fn cahier_retirer_travail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        let motif = motif.trim();
        if !(3..=500).contains(&motif.chars().count()) {
            return Err(invalide("indiquez le motif du retrait (3 à 500 caractères)"));
        }
        if !self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?
            .iter()
            .any(|d| d.action == Action::CahierViser && d.portee == Portee::Etablissement)
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let t: TravailRow = sqlx::query_as(&format!(
            "{SELECT_TRAVAIL} AND t.id = $2 AND t.supprime_le IS NULL FOR UPDATE OF t"
        ))
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        historiser(
            &mut tx,
            a,
            "travail",
            id,
            t.version,
            "retrait",
            json!({ "titre": t.titre, "consigne": t.consigne, "pour_le": t.pour_le.to_string(),
                    "auteur_user_id": t.auteur_user_id, "motif": motif }),
        )
        .await?;
        sqlx::query(
            "UPDATE ent_travail SET supprime_le = now(), retire_par_user_id = $3, retire_motif = $4,
                    version = version + 1, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(a.user_id)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_TRAVAIL_SUPPRIME,
            id,
            None,
            json!({ "travail_id": id, "retire_par_direction": true }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// CDT-03 : modifie un travail (historique conservé ; événement avec indicateur « déjà vu »).
    pub async fn cahier_modifier_travail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        m: ModifierTravailBody,
    ) -> Result<TravailDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let t: TravailRow = sqlx::query_as(&format!(
            "{SELECT_TRAVAIL} AND t.id = $2 AND t.supprime_le IS NULL FOR UPDATE OF t"
        ))
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        self.exiger_modification(a, &t).await?;

        let titre = match m.titre.as_deref().map(str::trim) {
            Some("") => return Err(invalide("titre vide")),
            Some(x) if x.chars().count() > 200 => return Err(invalide("titre trop long")),
            Some(x) => x.to_string(),
            None => t.titre.clone(),
        };
        let consigne = match m.consigne.as_deref().map(str::trim) {
            Some("") => return Err(invalide("consigne vide")),
            Some(x) if x.chars().count() > 8000 => return Err(invalide("consigne trop longue")),
            Some(x) => x.to_string(),
            None => t.consigne.clone(),
        };
        let pour_le = m.pour_le.unwrap_or(t.pour_le);
        if pour_le != t.pour_le && pour_le < self.jour_etab(a.etablissement_id).await? {
            return Err(invalide("l'échéance est dans le passé"));
        }
        if pour_le < t.donne_le {
            return Err(invalide("l'échéance précède la date de départ"));
        }
        let duree = m.duree_estimee_min.or(t.duree_estimee_min);
        if duree.is_some_and(|d| !(1..=600).contains(&d)) {
            return Err(invalide("durée estimée invalide (1 à 600 minutes)"));
        }
        let mut date_limite = t.date_limite;
        if let Some(l) = m.date_limite {
            if !t.rendu_en_ligne {
                return Err(invalide("pas de date limite sur un travail à faire"));
            }
            if l <= Utc::now() {
                return Err(invalide("la date limite est dans le passé"));
            }
            date_limite = Some(l);
        }
        let acceptation = m.acceptation_retard.unwrap_or(t.acceptation_retard);

        let mut champs: Vec<&str> = Vec::new();
        if titre != t.titre {
            champs.push("titre");
        }
        if consigne != t.consigne {
            champs.push("consigne");
        }
        if pour_le != t.pour_le {
            champs.push("pour_le");
        }
        if duree != t.duree_estimee_min {
            champs.push("duree");
        }
        if date_limite != t.date_limite {
            champs.push("date_limite");
        }
        if acceptation != t.acceptation_retard {
            champs.push("acceptation_retard");
        }
        if champs.is_empty() {
            drop(tx);
            return self.cahier_travail_dto(a.etablissement_id, id).await;
        }
        historiser(
            &mut tx,
            a,
            "travail",
            id,
            t.version,
            "modification",
            json!({
                "titre": t.titre, "consigne": t.consigne, "pour_le": t.pour_le.to_string(),
                "duree_estimee_min": t.duree_estimee_min, "date_limite": t.date_limite,
                "acceptation_retard": t.acceptation_retard,
            }),
        )
        .await?;
        sqlx::query(
            "UPDATE ent_travail SET titre = $3, consigne = $4, pour_le = $5, duree_estimee_min = $6,
                    date_limite = $7, acceptation_retard = $8, version = version + 1, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&titre)
        .bind(&consigne)
        .bind(pour_le)
        .bind(duree)
        .bind(date_limite)
        .bind(acceptation)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let (deja_vu,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_travail_eleve
                 WHERE travail_id = $1 AND etablissement_id = $2 AND vu_le IS NOT NULL)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_TRAVAIL_MODIFIE,
            id,
            None,
            json!({
                "travail_id": id,
                "version": t.version + 1,
                "deja_vu": deja_vu,
                "champs": champs,
            }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.cahier_travail_dto(a.etablissement_id, id).await
    }

    /// CDT-03 : supprime un travail (suppression logique, historique conservé).
    pub async fn cahier_supprimer_travail(&self, a: &Acteur<'_>, id: Uuid) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let t: TravailRow = sqlx::query_as(&format!(
            "{SELECT_TRAVAIL} AND t.id = $2 AND t.supprime_le IS NULL FOR UPDATE OF t"
        ))
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        self.exiger_modification(a, &t).await?;
        historiser(
            &mut tx,
            a,
            "travail",
            id,
            t.version,
            "suppression",
            json!({ "titre": t.titre, "consigne": t.consigne, "pour_le": t.pour_le.to_string() }),
        )
        .await?;
        sqlx::query(
            "UPDATE ent_travail SET supprime_le = now(), version = version + 1, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_TRAVAIL_SUPPRIME,
            id,
            None,
            json!({ "travail_id": id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────────── CDT-04 : pièces ─────────────────────────────

    async fn deposer_fichier(
        &self,
        tx: &mut sqlx::PgConnection,
        a: &Acteur<'_>,
        usage: UsageFichier,
        fichier: &FichierDepot,
        genre: &str,
        colonne: &'static str,
        parent: Uuid,
        formats: &[String],
    ) -> Result<PieceRow, EntDbError> {
        // Verrou de la ligne parent : deux dépôts simultanés ne dépassent pas le plafond de pièces.
        let table_parent = match colonne {
            "travail_id" => "ent_travail",
            "seance_contenu_id" => "ent_seance_contenu",
            _ => "ent_travail_remise",
        };
        sqlx::query(&format!(
            "SELECT 1 FROM {table_parent} WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(a.etablissement_id)
        .bind(parent)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (n,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM ent_cdt_piece
              WHERE etablissement_id = $1 AND {colonne} = $2 AND genre = $3 AND retiree_le IS NULL"
        ))
        .bind(a.etablissement_id)
        .bind(parent)
        .bind(genre)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if n >= MAX_PIECES {
            return Err(conflit("nombre maximal de pièces atteint"));
        }
        // D2 : signature binaire, liste blanche, nom assaini ; le type retenu est celui des octets.
        let valide = self
            .politique_fichiers()
            .valider(usage, &fichier.nom, Some(&fichier.mime_type), &fichier.contenu)
            .map_err(EntDbError::Fichier)?;
        if !formats.is_empty() && !formats.iter().any(|f| f == valide.extension) {
            return Err(invalide("format de fichier non accepté pour ce travail"));
        }
        let id = Uuid::new_v4();
        let key = ObjectStore::key_for(a.etablissement_id, "cahier", id, valide.extension);
        let (col_ressource, col_seance, col_remise) = match colonne {
            "travail_id" => (Some(parent), None, None),
            "seance_contenu_id" => (None, Some(parent), None),
            _ => (None, None, Some(parent)),
        };
        sqlx::query(
            "INSERT INTO ent_cdt_piece (id, etablissement_id, genre, travail_id, seance_contenu_id, remise_id,
                    nom, mime_type, taille_octets, storage_key, created_by_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(genre)
        .bind(col_ressource)
        .bind(col_seance)
        .bind(col_remise)
        .bind(&valide.nom)
        .bind(valide.mime)
        .bind(fichier.contenu.len() as i64)
        .bind(&key)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let info = self.objects().put(&key, &fichier.contenu)?;
        crate::objets::enregistrer_objet(&mut *tx, a.etablissement_id, &info, Some(valide.mime))
            .await?;
        let piece: PieceRow = sqlx::query_as(&format!("{SELECT_PIECE} AND id = $2"))
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        Ok(piece)
    }

    /// CDT-04 : joint une ressource à un travail ou au contenu d'une séance.
    pub async fn cahier_joindre_piece(
        &self,
        a: &Acteur<'_>,
        parent: ParentPiece,
        fichier: FichierDepot,
    ) -> Result<PieceCahierDto, EntDbError> {
        let (genre, colonne, parent_id) = match parent {
            ParentPiece::Travail(id) => {
                let t = self.charger_travail(a.etablissement_id, id).await?;
                self.exiger_modification(a, &t).await?;
                ("ressource", "travail_id", id)
            }
            ParentPiece::Seance(contenu_id) => {
                let (seance_id,): (Uuid,) = sqlx::query_as(
                    "SELECT seance_id FROM ent_seance_contenu WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(contenu_id)
                .fetch_optional(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?;
                let s = self.charger_seance(a.etablissement_id, seance_id).await?;
                self.exiger_ecriture_seance(a, &s).await?;
                ("seance", "seance_contenu_id", contenu_id)
            }
        };
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let piece = self
            .deposer_fichier(&mut tx, a, UsageFichier::PieceCahier, &fichier, genre, colonne, parent_id, &[])
            .await?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_PIECE_JOINTE,
            parent_id,
            None,
            json!({ "piece_id": piece.id, "genre": genre, "taille_octets": piece.taille_octets }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(piece.dto())
    }

    /// C-D12 : retire une pièce (retrait logique : la ligne et l'objet restent, la pièce n'est plus servie ni
    /// comptée dans le plafond). Ressource : auteur du travail ; pièce de séance : qui écrit la séance ;
    /// copie : l'élève avant correction ; annotation : l'intervenant en saisie.
    pub async fn cahier_retirer_piece(&self, a: &Acteur<'_>, piece_id: Uuid) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let p: PieceRow = sqlx::query_as(&format!("{SELECT_PIECE} AND id = $2 FOR UPDATE"))
            .bind(a.etablissement_id)
            .bind(piece_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        match p.genre.as_str() {
            "ressource" => {
                let t = self
                    .charger_travail(a.etablissement_id, p.travail_id.ok_or_else(introuvable)?)
                    .await?;
                self.exiger_modification(a, &t).await?;
            }
            "seance" => {
                let (seance_id,): (Uuid,) = sqlx::query_as(
                    "SELECT seance_id FROM ent_seance_contenu WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(p.seance_contenu_id.ok_or_else(introuvable)?)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                let s = self.charger_seance(a.etablissement_id, seance_id).await?;
                self.exiger_ecriture_seance(a, &s).await?;
            }
            genre => {
                let rid = p.remise_id.ok_or_else(introuvable)?;
                let r: RemiseRow = sqlx::query_as(&format!("{SELECT_REMISE} AND id = $2"))
                    .bind(a.etablissement_id)
                    .bind(rid)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                if genre == "copie" {
                    if self.eleve_du_compte(a).await? != Some(r.eleve_id) {
                        return Err(EntDbError::Forbidden);
                    }
                    if r.statut == "corrige" {
                        return Err(conflit("la copie a déjà été corrigée"));
                    }
                } else {
                    let t = self.charger_travail(a.etablissement_id, r.travail_id).await?;
                    self.exiger_suivi_remises(a, t.cible()).await?;
                }
            }
        }
        sqlx::query(
            "UPDATE ent_cdt_piece SET retiree_le = now(), retiree_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND retiree_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(piece_id)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Lien signé court vers une pièce, après contrôle des droits de lecture.
    ///
    /// * ressource / séance : personnel `cahier.lire` sur la cible, élève concerné, responsable lié ;
    /// * copie / annotation : l'élève auteur, son responsable lié, ou le personnel `cahier.ecrire`
    ///   sur la cible (professeur qui corrige).
    pub async fn cahier_lien_piece(
        &self,
        a: &Acteur<'_>,
        piece_id: Uuid,
    ) -> Result<LienFichier, EntDbError> {
        if !self.presigner().is_enabled() {
            return Err(EntDbError::LiensIndisponibles);
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let p: PieceRow = sqlx::query_as(&format!("{SELECT_PIECE} AND id = $2"))
            .bind(a.etablissement_id)
            .bind(piece_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        match p.genre.as_str() {
            "ressource" => {
                let t = self
                    .charger_travail(a.etablissement_id, p.travail_id.ok_or_else(introuvable)?)
                    .await?;
                self.lire_travail_autorise(a, &t).await?;
            }
            "seance" => {
                let (seance_id,): (Uuid,) = sqlx::query_as(
                    "SELECT seance_id FROM ent_seance_contenu WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(p.seance_contenu_id.ok_or_else(introuvable)?)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                self.lire_seance_autorise(a, seance_id).await?;
            }
            _ => {
                let rid = p.remise_id.ok_or_else(introuvable)?;
                let r: RemiseRow = sqlx::query_as(&format!("{SELECT_REMISE} AND id = $2"))
                    .bind(a.etablissement_id)
                    .bind(rid)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                self.lire_copie_autorise(a, &r, &p.genre).await?;
            }
        }
        let link = self
            .presigner()
            .issue(
                &LinkSpec {
                    key: p.storage_key.clone(),
                    user_id: a.user_id,
                    etablissement_id: a.etablissement_id,
                    disposition: Disposition::Attachment,
                    filename: p.nom.clone(),
                    mime: p.mime_type.clone(),
                },
                ent_object::presign::now_unix(),
            )
            .map_err(|e| match e {
                PresignError::Disabled => EntDbError::LiensIndisponibles,
                _ => EntDbError::FichierAbsent,
            })?;
        if !self.objects().existe(&p.storage_key)? {
            return Err(EntDbError::FichierAbsent);
        }
        self.tracer_acces_sensible(
            a.etablissement_id,
            a.user_id,
            a.role,
            None,
            "telechargement",
            "cahier_piece",
            &format!("Lien court vers la pièce {piece_id} du cahier de textes"),
        )
        .await;
        Ok(LienFichier {
            url: link.url,
            expires_at: link.expires_at,
        })
    }

    async fn lire_seance_autorise(&self, a: &Acteur<'_>, seance_id: Uuid) -> Result<(), EntDbError> {
        let s = self.charger_seance(a.etablissement_id, seance_id).await?;
        let eleve = match self.eleve_du_compte(a).await? {
            Some(e) => Some(e),
            None => None,
        };
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let enfants: Vec<Uuid> = if eleve.is_some() {
            Vec::new()
        } else if Role::parse(a.role) == Some(Role::Parent) {
            let l: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT eleve_id FROM ent_eleve_lien_actif WHERE etablissement_id = $1 AND user_id = $2",
            )
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            l.into_iter().map(|(e,)| e).collect()
        } else {
            return self
                .exiger_cible(a, Action::CahierLire, self.cible_de_seance(&s).await?)
                .await;
        };
        let candidats: Vec<Uuid> = eleve.into_iter().chain(enfants).collect();
        for e in candidats {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_edt_seance x
                   LEFT JOIN ent_service_enseignement s ON s.id = x.service_id
                  WHERE x.etablissement_id = $1 AND x.id = $3
                    AND EXISTS (SELECT 1 FROM ent_affectation a
                                 WHERE a.etablissement_id = x.etablissement_id AND a.eleve_id = $2
                                   AND a.date_debut <= x.jour AND (a.date_fin IS NULL OR a.date_fin >= x.jour)
                                   AND ((s.groupe_id IS NULL AND a.classe_id = x.classe_id)
                                     OR (s.groupe_id IS NOT NULL AND a.groupe_id = s.groupe_id))))",
            )
            .bind(a.etablissement_id)
            .bind(e)
            .bind(seance_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if ok && (Role::parse(a.role) != Some(Role::Parent) || self.parent_lie(a, e).await?) {
                return Ok(());
            }
        }
        Err(EntDbError::Forbidden)
    }

    /// Lecture d'une pièce de remise (R-C13) : l'élève auteur ; le responsable habilité seulement pour la
    /// **copie annotée rendue** (jamais le dépôt de l'élève), et si le retour porte une note, avec le droit « voir
    /// les notes » ; l'intervenant en saisie à la date. Ni la vie scolaire, ni un autre professeur, ni la direction.
    async fn lire_copie_autorise(&self, a: &Acteur<'_>, r: &RemiseRow, genre: &str) -> Result<(), EntDbError> {
        if self.eleve_du_compte(a).await? == Some(r.eleve_id) {
            return Ok(());
        }
        if Role::parse(a.role) == Some(Role::Parent) {
            if genre == "annotation"
                && r.statut == "corrige"
                && self.parent_lie(a, r.eleve_id).await?
                && (r.note_id.is_none() || self.parent_voit_notes(a, r.eleve_id).await?)
            {
                return Ok(());
            }
            return Err(EntDbError::Forbidden);
        }
        let t = self.charger_travail(a.etablissement_id, r.travail_id).await?;
        self.exiger_suivi_remises(a, t.cible()).await
    }

    // ───────────────────────────── CDT-06 / 07 : remises et retours ─────────────────────────────

    /// CDT-06 : l'élève rend (ou complète) son travail. Premier dépôt : fichier obligatoire.
    pub async fn cahier_rendre(
        &self,
        a: &Acteur<'_>,
        travail_id: Uuid,
        commentaire: &str,
        fichier: Option<FichierDepot>,
    ) -> Result<RemiseDto, EntDbError> {
        let Some(eleve) = self.eleve_du_compte(a).await? else {
            return Err(EntDbError::Forbidden);
        };
        let commentaire = commentaire.trim();
        if commentaire.chars().count() > 2000 {
            return Err(invalide("commentaire trop long"));
        }
        let t = self.charger_travail(a.etablissement_id, travail_id).await?;
        if !t.rendu_en_ligne {
            return Err(conflit("ce travail n'est pas à rendre en ligne"));
        }
        if !self.eleve_concerne(a.etablissement_id, eleve, &t).await? {
            return Err(EntDbError::Forbidden);
        }
        let maintenant = Utc::now();
        let en_retard = t.date_limite.is_some_and(|l| maintenant > l);
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let existante: Option<RemiseRow> = sqlx::query_as(&format!(
            "{SELECT_REMISE} AND travail_id = $2 AND eleve_id = $3 FOR UPDATE"
        ))
        .bind(a.etablissement_id)
        .bind(travail_id)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if existante.as_ref().is_some_and(|r| r.statut == "corrige") {
            return Err(conflit("la copie a déjà été corrigée"));
        }
        if en_retard && !t.acceptation_retard {
            return Err(refus(RefusMetier::Interdit, "la date limite est dépassée"));
        }
        if existante.is_none() && fichier.is_none() {
            return Err(invalide("un fichier est attendu pour rendre le travail"));
        }
        // Une seule ligne par (travail, élève), même en cas de dépôts concurrents.
        let (remise_id, cree): (Uuid, bool) = sqlx::query_as(
            "INSERT INTO ent_travail_remise (id, etablissement_id, travail_id, eleve_id, soumise_le,
                    derniere_le, en_retard, commentaire_eleve)
             VALUES ($1, $2, $3, $4, $5, $5, $6, $7)
             ON CONFLICT (travail_id, eleve_id) DO UPDATE
                SET derniere_le = EXCLUDED.derniere_le,
                    en_retard = ent_travail_remise.en_retard OR EXCLUDED.en_retard,
                    commentaire_eleve = CASE WHEN EXCLUDED.commentaire_eleve = ''
                                             THEN ent_travail_remise.commentaire_eleve
                                             ELSE EXCLUDED.commentaire_eleve END
             RETURNING id, (xmax = 0)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(travail_id)
        .bind(eleve)
        .bind(maintenant)
        .bind(en_retard)
        .bind(commentaire)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(f) = &fichier {
            self.deposer_fichier(
                &mut tx,
                a,
                UsageFichier::Copie,
                f,
                "copie",
                "remise_id",
                remise_id,
                &t.formats_acceptes,
            )
            .await?;
        }
        if cree {
            // Un seul événement par remise : un dépôt complémentaire ne renotifie pas.
            publier_cdt(
                &mut tx,
                a,
                types::CAHIER_TRAVAIL_RENDU,
                travail_id,
                Some(eleve),
                json!({
                    "travail_id": travail_id,
                    "remise_id": remise_id,
                    "en_retard": en_retard,
                }),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        self.remise_de(a.etablissement_id, travail_id, eleve)
            .await?
            .ok_or_else(introuvable)
    }

    /// CDT-07 (récupération) : suivi d'un travail à rendre — tous les élèves de la cible et leur remise.
    pub async fn cahier_suivi_travail(
        &self,
        a: &Acteur<'_>,
        travail_id: Uuid,
    ) -> Result<SuiviTravailDto, EntDbError> {
        let t = self.charger_travail(a.etablissement_id, travail_id).await?;
        self.exiger_suivi_remises(a, t.cible()).await?;
        if !t.rendu_en_ligne {
            return Err(conflit("ce travail n'est pas à rendre en ligne"));
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let eleves: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT DISTINCT e.id, e.nom, e.prenom
               FROM ent_affectation a
               JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1
                AND ((a.classe_id IS NOT NULL AND a.classe_id = $2) OR (a.groupe_id IS NOT NULL AND a.groupe_id = $3))
                AND a.date_debut <= $5 AND (a.date_fin IS NULL OR a.date_fin >= $4)
              ORDER BY e.nom, e.prenom, e.id",
        )
        .bind(a.etablissement_id)
        .bind(t.classe_id)
        .bind(t.groupe_id)
        .bind(t.donne_le)
        .bind(t.pour_le)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let remises: Vec<RemiseRow> = sqlx::query_as(&format!(
            "{SELECT_REMISE} AND travail_id = $2"
        ))
        .bind(a.etablissement_id)
        .bind(travail_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut par_eleve: HashMap<Uuid, RemiseDto> = self
            .remises_dto(a.etablissement_id, remises)
            .await?
            .into_iter()
            .map(|r| (r.eleve_id, r))
            .collect();
        let lignes: Vec<LigneSuiviDto> = eleves
            .into_iter()
            .map(|(id, nom, prenom)| LigneSuiviDto {
                eleve_id: id,
                nom,
                prenom,
                remise: par_eleve.remove(&id),
            })
            .collect();
        let nb_rendus = lignes.iter().filter(|l| l.remise.is_some()).count() as i64;
        let nb_en_retard = lignes
            .iter()
            .filter(|l| l.remise.as_ref().is_some_and(|r| r.en_retard))
            .count() as i64;
        let nb_attendus = lignes.len() as i64;
        let travail = self.travaux_dto(a.etablissement_id, vec![t]).await?.remove(0);
        Ok(SuiviTravailDto {
            travail,
            lignes,
            nb_rendus,
            nb_en_retard,
            nb_attendus,
        })
    }

    /// CDT-07 : renvoie la copie corrigée (commentaire, copie annotée facultative, note liée facultative).
    pub async fn cahier_retour(
        &self,
        a: &Acteur<'_>,
        remise_id: Uuid,
        commentaire: &str,
        note_id: Option<Uuid>,
        fichier: Option<FichierDepot>,
    ) -> Result<RemiseDto, EntDbError> {
        let commentaire = commentaire.trim();
        if commentaire.is_empty() || commentaire.chars().count() > 4000 {
            return Err(invalide("commentaire de correction vide ou trop long"));
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let r: RemiseRow = sqlx::query_as(&format!(
            "{SELECT_REMISE} AND id = $2 FOR UPDATE"
        ))
        .bind(a.etablissement_id)
        .bind(remise_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let t = self.charger_travail(a.etablissement_id, r.travail_id).await?;
        self.exiger_suivi_remises(a, t.cible()).await?;
        if let Some(n) = note_id {
            // C-D8 : la note liée existe dans cet établissement, pour cet élève, **de la main du correcteur**
            // et n'est pas déjà portée par la remise d'un autre travail (l'identité du cours viendra avec C1).
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_note n
                                 WHERE n.etablissement_id = $1 AND n.id = $2 AND n.eleve_id = $3
                                   AND n.auteur_user_id = $4
                                   AND NOT EXISTS (SELECT 1 FROM ent_travail_remise o
                                                    WHERE o.etablissement_id = n.etablissement_id
                                                      AND o.note_id = n.id AND o.id <> $5))",
            )
            .bind(a.etablissement_id)
            .bind(n)
            .bind(r.eleve_id)
            .bind(a.user_id)
            .bind(remise_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(invalide("note introuvable pour cet élève"));
            }
        }
        // Un retour corrigé ne s'écrase pas : l'état précédent est gardé à l'historique (C-D8).
        if r.statut == "corrige" {
            historiser(
                &mut tx,
                a,
                "remise",
                remise_id,
                0,
                "retour_modifie",
                json!({ "retour_commentaire": r.retour_commentaire, "retour_le": r.retour_le,
                        "note_id": r.note_id }),
            )
            .await?;
        }
        sqlx::query(
            "UPDATE ent_travail_remise SET statut = 'corrige', retour_commentaire = $3,
                    retour_par_user_id = $4, retour_le = now(), note_id = COALESCE($5, note_id)
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(remise_id)
        .bind(commentaire)
        .bind(a.user_id)
        .bind(note_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(f) = &fichier {
            self.deposer_fichier(
                &mut tx,
                a,
                UsageFichier::Copie,
                f,
                "annotation",
                "remise_id",
                remise_id,
                &[],
            )
            .await?;
        }
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_COPIE_RENDUE,
            r.travail_id,
            Some(r.eleve_id),
            json!({ "travail_id": r.travail_id, "remise_id": remise_id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.remise_de(a.etablissement_id, r.travail_id, r.eleve_id)
            .await?
            .ok_or_else(introuvable)
    }

    // ───────────────────────────── CDT-08 / 09 : agenda ─────────────────────────────

    /// CDT-08 : agenda « à faire » d'un élève (travaux, échéances, contrôles annoncés).
    pub async fn cahier_agenda(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<AgendaItemDto>, EntDbError> {
        Self::periode(du, au)?;
        let (eleve, vue) = self.cahier_resoudre_eleve(a, eleve_id).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let rows: Vec<TravailRow> = sqlx::query_as(&format!(
            "{SELECT_TRAVAIL} AND t.supprime_le IS NULL
               AND t.pour_le BETWEEN $3 AND $4 AND {ELEVE_CONCERNE}
             ORDER BY t.pour_le, t.created_at"
        ))
        .bind(a.etablissement_id)
        .bind(eleve)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // C-D3 : la remise (commentaire, copie, retour) ne sort de l'agenda que pour l'élève, le responsable
        // habilité (version minimisée) et l'intervenant en saisie sur la cible du travail.
        let mut remise_permise: HashMap<Uuid, bool> = HashMap::new();
        if vue == VueAgenda::Personnel {
            let mut par_cible: HashMap<(Option<Uuid>, Option<Uuid>), bool> = HashMap::new();
            for r in &rows {
                let cle = (r.classe_id, r.groupe_id);
                let ok = match par_cible.get(&cle) {
                    Some(ok) => *ok,
                    None => {
                        let ok = self.cahier_droit_ecrire_global(a).await?
                            && self.intervenant_sur_cible(a, r.cible(), true).await?;
                        par_cible.insert(cle, ok);
                        ok
                    }
                };
                remise_permise.insert(r.id, ok);
            }
        }
        let peut_notes = vue == VueAgenda::Famille && self.parent_voit_notes(a, eleve).await?;
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let etats: Vec<(Uuid, bool, bool)> = sqlx::query_as(
            "SELECT travail_id, fait, vu_le IS NOT NULL FROM ent_travail_eleve
              WHERE eleve_id = $1 AND travail_id = ANY($2) AND etablissement_id = $3",
        )
        .bind(eleve)
        .bind(&ids)
        .bind(a.etablissement_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let etats: HashMap<Uuid, (bool, bool)> = etats.into_iter().map(|(t, f, v)| (t, (f, v))).collect();
        let remises: Vec<RemiseRow> = sqlx::query_as(&format!(
            "{SELECT_REMISE} AND eleve_id = $2 AND travail_id = ANY($3)"
        ))
        .bind(a.etablissement_id)
        .bind(eleve)
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut remises: HashMap<Uuid, RemiseDto> = self
            .remises_dto(a.etablissement_id, remises)
            .await?
            .into_iter()
            .map(|r| (r.travail_id, r))
            .collect();
        let travaux = self.travaux_dto(a.etablissement_id, rows).await?;
        Ok(travaux
            .into_iter()
            .map(|t| {
                // « Fait » et « vu » appartiennent à l'élève : jamais montrés à un adulte (R-C14).
                let (fait, vu) = if vue == VueAgenda::Eleve {
                    etats.get(&t.id).copied().unwrap_or((false, false))
                } else {
                    (false, false)
                };
                let remise = remises.remove(&t.id).and_then(|r| match vue {
                    VueAgenda::Eleve => Some(r),
                    VueAgenda::Famille => Some(remise_pour_famille(r, peut_notes)),
                    VueAgenda::Personnel => remise_permise.get(&t.id).copied().unwrap_or(false).then_some(r),
                });
                AgendaItemDto {
                    travail: t,
                    fait,
                    vu,
                    remise,
                }
            })
            .collect())
    }

    /// CDT-09 : l'élève marque un travail comme fait (indicateur personnel).
    pub async fn cahier_marquer_fait(
        &self,
        a: &Acteur<'_>,
        travail_id: Uuid,
        fait: bool,
    ) -> Result<(), EntDbError> {
        let Some(eleve) = self.eleve_du_compte(a).await? else {
            return Err(EntDbError::Forbidden);
        };
        let t = self.charger_travail(a.etablissement_id, travail_id).await?;
        if !self.eleve_concerne(a.etablissement_id, eleve, &t).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        sqlx::query(
            "INSERT INTO ent_travail_eleve (travail_id, eleve_id, etablissement_id, fait, fait_le)
             VALUES ($1, $2, $3, $4, CASE WHEN $4 THEN now() END)
             ON CONFLICT (travail_id, eleve_id) DO UPDATE
                SET fait = EXCLUDED.fait, fait_le = EXCLUDED.fait_le",
        )
        .bind(travail_id)
        .bind(eleve)
        .bind(a.etablissement_id)
        .bind(fait)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(())
    }

    // ───────────────────────────── Charge de travail ─────────────────────────────

    /// Charge d'un seul jour pour une cible (sert aussi l'alerte à la création).
    async fn charge_jour(
        &self,
        etab: Uuid,
        cible: Cible,
        jour: NaiveDate,
        seuil_minutes: i32,
        seuil_controles: i32,
    ) -> Result<ChargeJourDto, EntDbError> {
        let mut jours = self
            .charge_periode(etab, cible, jour, jour, seuil_minutes, seuil_controles)
            .await?;
        Ok(jours.pop().unwrap_or(ChargeJourDto {
            jour,
            minutes: 0,
            nb_travaux: 0,
            nb_controles: 0,
            surcharge: false,
            travaux: Vec::new(),
        }))
    }

    async fn charge_periode(
        &self,
        etab: Uuid,
        cible: Cible,
        du: NaiveDate,
        au: NaiveDate,
        seuil_minutes: i32,
        seuil_controles: i32,
    ) -> Result<Vec<ChargeJourDto>, EntDbError> {
        // Une classe porte aussi la charge des groupes qui lui sont rattachés ; un groupe, la
        // sienne et celle de sa classe (ses élèves cumulent les deux).
        let rows: Vec<(Uuid, NaiveDate, String, String, String, Option<i32>)> = sqlx::query_as(
            "SELECT t.id, t.pour_le, t.matiere_libelle, t.titre, t.type, t.duree_estimee_min
               FROM ent_travail t
              WHERE t.etablissement_id = $1 AND t.supprime_le IS NULL AND t.pour_le BETWEEN $4 AND $5
                AND (t.classe_id = $2
                     OR t.groupe_id = $3
                     OR t.groupe_id IN (SELECT id FROM ent_groupe WHERE etablissement_id = $1 AND classe_id = $2)
                     OR t.classe_id IN (SELECT classe_id FROM ent_groupe WHERE etablissement_id = $1 AND id = $3))
              ORDER BY t.pour_le, t.created_at",
        )
        .bind(etab)
        .bind(cible.classe_id)
        .bind(cible.groupe_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let mut par_jour: std::collections::BTreeMap<NaiveDate, ChargeJourDto> = Default::default();
        for (id, jour, matiere, titre, type_travail, duree) in rows {
            let j = par_jour.entry(jour).or_insert_with(|| ChargeJourDto {
                jour,
                minutes: 0,
                nb_travaux: 0,
                nb_controles: 0,
                surcharge: false,
                travaux: Vec::new(),
            });
            j.minutes += duree.unwrap_or(0);
            j.nb_travaux += 1;
            if type_travail == "controle" {
                j.nb_controles += 1;
            }
            j.travaux.push(ChargeTravailDto {
                travail_id: id,
                matiere_libelle: matiere,
                titre,
                type_travail,
                duree_estimee_min: duree,
            });
        }
        let mut out: Vec<ChargeJourDto> = par_jour.into_values().collect();
        for j in &mut out {
            j.surcharge = j.minutes > seuil_minutes || j.nb_controles >= seuil_controles;
        }
        Ok(out)
    }

    /// Charge de travail d'une classe ou d'un groupe, visible de l'équipe (`cahier.lire`).
    pub async fn cahier_charge(
        &self,
        a: &Acteur<'_>,
        cible: Cible,
        du: NaiveDate,
        au: NaiveDate,
        seuil_minutes: Option<i32>,
        seuil_controles: Option<i32>,
    ) -> Result<ChargeDto, EntDbError> {
        Self::periode(du, au)?;
        let cible = cible.valider()?;
        self.exiger_cible(a, Action::CahierLire, cible).await?;
        let sm = seuil_minutes.unwrap_or(SEUIL_MINUTES_DEFAUT).clamp(15, 600);
        let sc = seuil_controles.unwrap_or(SEUIL_CONTROLES_DEFAUT).clamp(1, 10);
        Ok(ChargeDto {
            seuil_minutes: sm,
            seuil_controles: sc,
            jours: self.charge_periode(a.etablissement_id, cible, du, au, sm, sc).await?,
        })
    }

    // ───────────────────────────── CDT-10 : visa ─────────────────────────────

    /// CDT-10 : vise le cahier d'une classe (et, facultativement, d'un professeur) sur une période.
    pub async fn cahier_viser(
        &self,
        a: &Acteur<'_>,
        v: VisaBody,
    ) -> Result<VisaDto, EntDbError> {
        Self::periode(v.du, v.au)?;
        let commentaire = v.commentaire.trim();
        if commentaire.chars().count() > 2000 {
            return Err(invalide("commentaire trop long"));
        }
        if !self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?
            .iter()
            .any(|d| d.action == Action::CahierViser && d.portee == Portee::Etablissement)
        {
            return Err(EntDbError::Forbidden);
        }
        let cible = Cible {
            classe_id: Some(v.classe_id),
            groupe_id: None,
        };
        if !self.cible_existe(a.etablissement_id, cible).await? {
            return Err(introuvable());
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        if let Some(p) = v.prof_user_id {
            // C-D9 : le professeur visé enseigne bien dans cette classe sur la période du visa.
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2)
                    AND EXISTS (SELECT 1 FROM ent_cours c
                                 WHERE c.etablissement_id = $1 AND c.classe_id = $3
                                   AND (ent_prof_sur_cours($1, c.id, $2, $4, false)
                                     OR ent_prof_sur_cours($1, c.id, $2, $5, false)))",
            )
            .bind(a.etablissement_id)
            .bind(p)
            .bind(v.classe_id)
            .bind(v.du)
            .bind(v.au)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(invalide("ce professeur n'enseigne pas dans cette classe sur la période"));
            }
        }
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_cdt_visa (id, etablissement_id, classe_id, prof_user_id, du, au, commentaire, vise_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(v.classe_id)
        .bind(v.prof_user_id)
        .bind(v.du)
        .bind(v.au)
        .bind(commentaire)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_VISE,
            id,
            None,
            json!({ "classe_id": v.classe_id, "prof_user_id": v.prof_user_id,
                    "du": v.du.to_string(), "au": v.au.to_string() }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        let (vise_le,): (DateTime<Utc>,) =
            sqlx::query_as("SELECT vise_le FROM ent_cdt_visa WHERE etablissement_id = $1 AND id = $2")
                .bind(a.etablissement_id)
                .bind(id)
                .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?)
                .await
                .map_err(db)?;
        Ok(VisaDto {
            id,
            classe_id: v.classe_id,
            prof_user_id: v.prof_user_id,
            du: v.du,
            au: v.au,
            commentaire: commentaire.to_string(),
            vise_par_user_id: a.user_id,
            vise_le,
        })
    }

    /// Visas d'une classe (`cahier.lire` sur la classe).
    ///
    /// C-D9 : le commentaire d'un visa **individuel** n'est lisible que du professeur visé et de la direction ;
    /// les autres collègues de la classe n'en voient que l'existence (commentaire vide).
    pub async fn cahier_visas(&self, a: &Acteur<'_>, classe_id: Uuid) -> Result<Vec<VisaDto>, EntDbError> {
        let direction = self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?
            .iter()
            .any(|d| d.action == Action::CahierViser && d.portee == Portee::Etablissement);
        self.exiger_cible(
            a,
            Action::CahierLire,
            Cible {
                classe_id: Some(classe_id),
                groupe_id: None,
            },
        )
        .await?;
        let rows: Vec<(Uuid, Uuid, Option<Uuid>, NaiveDate, NaiveDate, String, Uuid, DateTime<Utc>)> =
            sqlx::query_as(
                "SELECT id, classe_id, prof_user_id, du, au, commentaire, vise_par_user_id, vise_le
                   FROM ent_cdt_visa WHERE etablissement_id = $1 AND classe_id = $2
                  ORDER BY vise_le DESC LIMIT 100",
            )
            .bind(a.etablissement_id)
            .bind(classe_id)
            .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
            .await
            .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, classe_id, prof, du, au, commentaire, par, le)| VisaDto {
                id,
                classe_id,
                prof_user_id: prof,
                du,
                au,
                commentaire: if direction || prof.is_none() || prof == Some(a.user_id) {
                    commentaire
                } else {
                    String::new()
                },
                vise_par_user_id: par,
                vise_le: le,
            })
            .collect())
    }

    /// Lot C1 : état de la reprise de l'ancien cahier vers le modèle du socle 20 (compteurs avant / après et
    /// quarantaine). Lecture seule, réservée à la direction (`cahier.viser` à portée d'établissement).
    pub async fn cahier_reprise_etat(&self, a: &Acteur<'_>) -> Result<RepriseEtatDto, EntDbError> {
        if !self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?
            .iter()
            .any(|d| d.action == Action::CahierViser && d.portee == Portee::Etablissement)
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let compteurs: Vec<(String, i64, i64, i64)> =
            sqlx::query_as("SELECT objet, anciens, repris, quarantaine FROM ent_cdt_reprise_compteurs($1)")
                .bind(a.etablissement_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
        let quarantaine: Vec<(Uuid, String, Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, objet, ancien_id, raison, detail, cree_le FROM ent_cdt_quarantaine
              WHERE etablissement_id = $1 AND traitee_le IS NULL ORDER BY cree_le, id LIMIT 500",
        )
        .bind(a.etablissement_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(RepriseEtatDto {
            compteurs: compteurs
                .into_iter()
                .map(|(objet, anciens, repris, quarantaine)| CompteurRepriseDto { objet, anciens, repris, quarantaine })
                .collect(),
            quarantaine: quarantaine
                .into_iter()
                .map(|(id, objet, ancien_id, raison, detail, cree_le)| QuarantaineDto {
                    id,
                    objet,
                    ancien_id,
                    raison,
                    detail,
                    cree_le,
                })
                .collect(),
        })
    }

    /// Historique d'un travail (CDT-03) : auteur du travail ou direction.
    pub async fn cahier_historique_travail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<(i32, String, DateTime<Utc>, serde_json::Value)>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let (auteur, classe, groupe): (Uuid, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
            "SELECT auteur_user_id, classe_id, groupe_id FROM ent_travail WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if auteur != a.user_id {
            let direction = self
                .droits_effectifs(a.etablissement_id, a.user_id, a.role)
                .await?
                .iter()
                .any(|d| d.action == Action::CahierViser && d.portee == Portee::Etablissement);
            if !direction {
                self.exiger_suivi_remises(
                    a,
                    Cible {
                        classe_id: classe,
                        groupe_id: groupe,
                    },
                )
                .await?;
            }
        }
        sqlx::query_as(
            "SELECT version, action, created_at, instantane FROM ent_cdt_historique
              WHERE etablissement_id = $1 AND objet = 'travail' AND objet_id = $2 ORDER BY created_at, version",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }
}

impl PgEnt {
    /// Fin de la journée `jour` à l'heure de l'établissement (C-D13) : échéance par défaut d'un travail à
    /// rendre (23 h 59 min 59 s locales, quel que soit le fuseau, La Réunion et Paris comprises).
    async fn fin_de_journee(&self, etab: Uuid, jour: NaiveDate) -> Result<DateTime<Utc>, EntDbError> {
        let (t,): (DateTime<Utc>,) = sqlx::query_as("SELECT ent_cdt_fin_de_journee($1, $2)")
            .bind(etab)
            .bind(jour)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)?;
        Ok(t)
    }
}

async fn historiser(
    tx: &mut sqlx::PgConnection,
    a: &Acteur<'_>,
    objet: &str,
    objet_id: Uuid,
    version: i32,
    action: &str,
    instantane: serde_json::Value,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_cdt_historique (id, etablissement_id, objet, objet_id, version, action, auteur_user_id, instantane)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(Uuid::new_v4())
    .bind(a.etablissement_id)
    .bind(objet)
    .bind(objet_id)
    .bind(version)
    .bind(action)
    .bind(a.user_id)
    .bind(instantane)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Publie un fait du cahier dans la transaction de l'écriture (même commit, outbox S2-0).
async fn publier_cdt(
    tx: &mut sqlx::PgConnection,
    a: &Acteur<'_>,
    type_name: &str,
    aggregate_id: Uuid,
    eleve_id: Option<Uuid>,
    payload: serde_json::Value,
) -> Result<(), EntDbError> {
    let conn: &mut PgConnection = tx;
    conn.publier(MetierEventDraft {
        etablissement_id: a.etablissement_id,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::CahierTexte,
        aggregate_id,
        eleve_id,
        payload,
        actor_user_id: Some(a.user_id.to_string()),
    })
    .await?;
    Ok(())
}
