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

use chrono::{DateTime, Duration, NaiveDate, Utc};
use domain_eleve::today_utc;
use ent_auth::{Action, Portee, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_object::presign::{Disposition, LinkSpec, PresignError};
use ent_object::upload::UsageFichier;
use ent_object::ObjectStore;
use ent_types::cahier::*;
use serde_json::json;
use sqlx::{FromRow, PgConnection, Postgres, Transaction};
use uuid::Uuid;

use crate::outbox::Publier;
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, LienFichier, PgEnt, RefusMetier};

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

const SELECT_TRAVAIL: &str = "SELECT t.id, t.seance_id, t.classe_id, t.groupe_id,
       COALESCE(c.code, g.code, '') AS cible, t.matiere_libelle, t.auteur_user_id,
       t.type AS type_travail, t.titre, t.consigne, t.donne_le, t.pour_le, t.duree_estimee_min,
       t.rendu_en_ligne, t.date_limite, t.acceptation_retard, t.formats_acceptes, t.version
  FROM ent_travail t
  LEFT JOIN ent_classe c ON c.id = t.classe_id
  LEFT JOIN ent_groupe g ON g.id = t.groupe_id";

/// L'élève `$2` est concerné par le travail `t` (affectation à la classe ou au groupe visé,
/// recoupant la période [donné le, pour le]).
const ELEVE_CONCERNE: &str = "EXISTS (SELECT 1 FROM ent_affectation a
         WHERE a.etablissement_id = t.etablissement_id AND a.eleve_id = $2
           AND (a.classe_id = t.classe_id OR a.groupe_id = t.groupe_id)
           AND a.date_debut <= t.pour_le AND (a.date_fin IS NULL OR a.date_fin >= t.donne_le))";

#[derive(Debug, FromRow)]
struct TravailRow {
    id: Uuid,
    seance_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
    cible: String,
    matiere_libelle: String,
    auteur_user_id: Uuid,
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
            auteur_user_id: self.auteur_user_id,
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
       mime_type, taille_octets, storage_key, created_at FROM ent_cdt_piece";

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
       commentaire_eleve, statut, retour_commentaire, retour_le, note_id FROM ent_travail_remise";

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

const SELECT_SEANCE: &str = "SELECT id, jour, heure_debut, heure_fin, classe_id, classe, service_id,
       prof_user_id, remplacant_user_id, matiere_libelle, statut FROM ent_edt_seance";

#[derive(Debug, FromRow)]
struct ContenuRow {
    id: Uuid,
    seance_id: Uuid,
    auteur_user_id: Uuid,
    contenu: String,
    ressources: String,
    version: i32,
    updated_at: DateTime<Utc>,
}

const SELECT_CONTENU: &str = "SELECT id, seance_id, auteur_user_id, contenu, ressources, version,
       updated_at FROM ent_seance_contenu";

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
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(ok)
    }

    /// L'acteur peut-il exercer `action` sur cette classe ou ce groupe ? (union des portées).
    /// `CahierEcrire` implique `CahierLire`. Le personnel seul porte des droits paramétrables.
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
                Portee::Designes => designes.extend(d.cibles.iter().copied()),
            }
        }
        if designes
            .iter()
            .any(|id| Some(*id) == cible.classe_id || Some(*id) == cible.groupe_id)
        {
            return Ok(true);
        }
        if classes {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (
                    SELECT 1 FROM ent_service_enseignement s
                     WHERE s.etablissement_id = $1 AND s.prof_user_id = $2
                       AND ((s.classe_id IS NOT NULL AND s.classe_id = $3)
                         OR (s.groupe_id IS NOT NULL AND s.groupe_id = $4)))
                 OR EXISTS (
                    SELECT 1 FROM ent_edt_seance x
                     WHERE x.etablissement_id = $1 AND $3::uuid IS NOT NULL AND x.classe_id = $3
                       AND (x.prof_user_id = $2 OR x.remplacant_user_id = $2))",
            )
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .bind(cible.classe_id)
            .bind(cible.groupe_id)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if ok {
                return Ok(true);
            }
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
                .fetch_one(self.pool())
                .await
                .map_err(db)?;
                if ok {
                    return Ok(true);
                }
            }
        }
        Ok(false)
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
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(r.map(|(e,)| e))
    }

    async fn parent_lie(&self, a: &Acteur<'_>, eleve_id: Uuid) -> Result<bool, EntDbError> {
        if Role::parse(a.role) != Some(Role::Parent) {
            return Ok(false);
        }
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien l
                JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
               WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.eleve_id = $3 AND m.role = 'parent')",
        )
        .bind(a.etablissement_id)
        .bind(a.user_id)
        .bind(eleve_id)
        .fetch_one(self.pool())
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
                .fetch_one(self.pool())
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
            "{SELECT_TRAVAIL} WHERE t.etablissement_id = $1 AND t.id = $2 AND t.supprime_le IS NULL"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(self.pool())
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
            "{SELECT_PIECE} WHERE etablissement_id = $1 AND {colonne} = ANY($2) AND genre = ANY($3)
              ORDER BY created_at, id"
        ))
        .bind(etab)
        .bind(ids)
        .bind(&genres)
        .fetch_all(self.pool())
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

    async fn eleve_concerne(&self, etab: Uuid, eleve: Uuid, t: &TravailRow) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_affectation a
               WHERE a.etablissement_id = $1 AND a.eleve_id = $2
                 AND ((a.classe_id IS NOT NULL AND a.classe_id = $3)
                   OR (a.groupe_id IS NOT NULL AND a.groupe_id = $4))
                 AND a.date_debut <= $6 AND (a.date_fin IS NULL OR a.date_fin >= $5))",
        )
        .bind(etab)
        .bind(eleve)
        .bind(t.classe_id)
        .bind(t.groupe_id)
        .bind(t.donne_le)
        .bind(t.pour_le)
        .fetch_one(self.pool())
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
            return if self.eleve_concerne(a.etablissement_id, e, t).await? {
                Ok(VueAgenda::Eleve)
            } else {
                Err(EntDbError::Forbidden)
            };
        }
        if Role::parse(a.role) == Some(Role::Parent) {
            let enfants: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT eleve_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND user_id = $2",
            )
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .fetch_all(self.pool())
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
        sqlx::query_as::<_, SeanceRow>(&format!("{SELECT_SEANCE} WHERE etablissement_id = $1 AND id = $2"))
            .bind(etab)
            .bind(id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?
            .ok_or_else(introuvable)
    }

    /// Séance rattachée à une classe : cible de droit (classe de la séance, ou groupe du service).
    async fn cible_de_seance(&self, s: &SeanceRow) -> Result<Cible, EntDbError> {
        if let Some(sid) = s.service_id {
            let g: Option<(Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
                "SELECT groupe_id, classe_id FROM ent_service_enseignement WHERE id = $1",
            )
            .bind(sid)
            .fetch_optional(self.pool())
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
        .fetch_one(self.pool())
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
            "{SELECT_CONTENU} WHERE etablissement_id = $1 AND seance_id = $2"
        ))
        .bind(etab)
        .bind(seance_id)
        .fetch_optional(self.pool())
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
            auteur_user_id: c.auteur_user_id,
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
            sqlx::query_as(&format!(
                "{SELECT_SEANCE} WHERE etablissement_id = $1 AND classe_id = $2 AND jour BETWEEN $3 AND $4
                  ORDER BY jour, heure_debut"
            ))
            .bind(a.etablissement_id)
            .bind(c)
            .bind(du)
            .bind(au)
            .fetch_all(self.pool())
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
                "{SELECT_SEANCE} WHERE etablissement_id = $1 AND (prof_user_id = $2 OR remplacant_user_id = $2)
                   AND jour BETWEEN $3 AND $4 ORDER BY jour, heure_debut"
            ))
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .bind(du)
            .bind(au)
            .fetch_all(self.pool())
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
        let cible = self.cible_de_seance(&seance).await?;
        let titulaire = seance.prof_user_id == a.user_id || seance.remplacant_user_id == Some(a.user_id);
        if !titulaire {
            self.exiger_cible(a, Action::CahierEcrire, cible).await?;
        } else if !self.cahier_droit_ecrire_global(a).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let existant: Option<ContenuRow> = sqlx::query_as(&format!(
            "{SELECT_CONTENU} WHERE etablissement_id = $1 AND seance_id = $2 FOR UPDATE"
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
            "SELECT x.id, x.jour, x.heure_debut, x.heure_fin, x.classe_id, x.classe, x.service_id,
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
              ORDER BY x.jour, x.heure_debut",
        )
        .bind(a.etablissement_id)
        .bind(eleve)
        .bind(du)
        .bind(au)
        .fetch_all(self.pool())
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
        let aujourdhui = today_utc();
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
        self.exiger_cible(a, Action::CahierEcrire, cible).await?;

        let donne_le = seance
            .as_ref()
            .map(|s| s.jour.min(aujourdhui))
            .unwrap_or(aujourdhui);
        let date_limite = if a_rendre {
            let limite = n.date_limite.unwrap_or_else(|| fin_de_journee(n.pour_le));
            if limite <= Utc::now() {
                return Err(invalide("la date limite est dans le passé"));
            }
            Some(limite)
        } else {
            None
        };

        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
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
        .fetch_one(self.pool())
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
        if let Some(e) = eleve {
            sqlx::query(
                "INSERT INTO ent_travail_eleve (travail_id, eleve_id, etablissement_id, vu_le)
                 VALUES ($1, $2, $3, now())
                 ON CONFLICT (travail_id, eleve_id) DO UPDATE SET vu_le = COALESCE(ent_travail_eleve.vu_le, now())",
            )
            .bind(id)
            .bind(e)
            .bind(a.etablissement_id)
            .execute(self.pool())
            .await
            .map_err(db)?;
            let (f,): (bool,) = sqlx::query_as(
                "SELECT fait FROM ent_travail_eleve WHERE travail_id = $1 AND eleve_id = $2",
            )
            .bind(id)
            .bind(e)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            fait = f;
            vu = true;
            remise = self.remise_de(a.etablissement_id, id, e).await?;
        }
        let travail = self.travaux_dto(a.etablissement_id, vec![row]).await?.remove(0);
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
            "{SELECT_REMISE} WHERE etablissement_id = $1 AND travail_id = $2 AND eleve_id = $3"
        ))
        .bind(etab)
        .bind(travail)
        .bind(eleve)
        .fetch_optional(self.pool())
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
            "{SELECT_TRAVAIL} WHERE t.etablissement_id = $1 AND t.supprime_le IS NULL
               AND t.pour_le BETWEEN $4 AND $5
               AND ((t.classe_id IS NOT NULL AND t.classe_id = $2) OR (t.groupe_id IS NOT NULL AND t.groupe_id = $3))
             ORDER BY t.pour_le, t.created_at"
        ))
        .bind(a.etablissement_id)
        .bind(cible.classe_id)
        .bind(cible.groupe_id)
        .bind(du)
        .bind(au)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        self.travaux_dto(a.etablissement_id, rows).await
    }

    /// Droit de modifier : l'auteur (avec `cahier.ecrire` sur la cible) ou la direction (`cahier.viser`).
    async fn exiger_modification(&self, a: &Acteur<'_>, t: &TravailRow) -> Result<(), EntDbError> {
        if t.auteur_user_id == a.user_id {
            return self.exiger_cible(a, Action::CahierEcrire, t.cible()).await;
        }
        let droits = self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?;
        if droits
            .iter()
            .any(|d| d.action == Action::CahierViser && d.portee == Portee::Etablissement)
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// CDT-03 : modifie un travail (historique conservé ; événement avec indicateur « déjà vu »).
    pub async fn cahier_modifier_travail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        m: ModifierTravailBody,
    ) -> Result<TravailDto, EntDbError> {
        let mut tx = self.pool().begin().await.map_err(db)?;
        let t: TravailRow = sqlx::query_as(&format!(
            "{SELECT_TRAVAIL} WHERE t.etablissement_id = $1 AND t.id = $2 AND t.supprime_le IS NULL FOR UPDATE OF t"
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
        if pour_le != t.pour_le && pour_le < today_utc() {
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
            "SELECT EXISTS (SELECT 1 FROM ent_travail_eleve WHERE travail_id = $1 AND vu_le IS NOT NULL)",
        )
        .bind(id)
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
        let mut tx = self.pool().begin().await.map_err(db)?;
        let t: TravailRow = sqlx::query_as(&format!(
            "{SELECT_TRAVAIL} WHERE t.etablissement_id = $1 AND t.id = $2 AND t.supprime_le IS NULL FOR UPDATE OF t"
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
        tx: &mut Transaction<'_, Postgres>,
        a: &Acteur<'_>,
        usage: UsageFichier,
        fichier: &FichierDepot,
        genre: &str,
        colonne: &'static str,
        parent: Uuid,
        formats: &[String],
    ) -> Result<PieceRow, EntDbError> {
        let (n,): (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM ent_cdt_piece WHERE etablissement_id = $1 AND {colonne} = $2 AND genre = $3"
        ))
        .bind(a.etablissement_id)
        .bind(parent)
        .bind(genre)
        .fetch_one(&mut **tx)
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
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        self.objects().put(&key, &fichier.contenu)?;
        let piece: PieceRow = sqlx::query_as(&format!("{SELECT_PIECE} WHERE id = $1"))
            .bind(id)
            .fetch_one(&mut **tx)
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
        let (genre, colonne, parent_id, cible, titulaire) = match parent {
            ParentPiece::Travail(id) => {
                let t = self.charger_travail(a.etablissement_id, id).await?;
                self.exiger_modification(a, &t).await?;
                ("ressource", "travail_id", id, t.cible(), None)
            }
            ParentPiece::Seance(contenu_id) => {
                let (seance_id,): (Uuid,) = sqlx::query_as(
                    "SELECT seance_id FROM ent_seance_contenu WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(contenu_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?;
                let s = self.charger_seance(a.etablissement_id, seance_id).await?;
                let cible = self.cible_de_seance(&s).await?;
                let titulaire = s.prof_user_id == a.user_id || s.remplacant_user_id == Some(a.user_id);
                ("seance", "seance_contenu_id", contenu_id, cible, Some(titulaire))
            }
        };
        if titulaire == Some(true) {
            if !self.cahier_droit_ecrire_global(a).await? {
                return Err(EntDbError::Forbidden);
            }
        } else if matches!(parent, ParentPiece::Seance(_)) {
            self.exiger_cible(a, Action::CahierEcrire, cible).await?;
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
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
        let p: PieceRow = sqlx::query_as(&format!("{SELECT_PIECE} WHERE etablissement_id = $1 AND id = $2"))
            .bind(a.etablissement_id)
            .bind(piece_id)
            .fetch_optional(self.pool())
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
                .fetch_one(self.pool())
                .await
                .map_err(db)?;
                self.lire_seance_autorise(a, seance_id).await?;
            }
            _ => {
                let rid = p.remise_id.ok_or_else(introuvable)?;
                let r: RemiseRow = sqlx::query_as(&format!("{SELECT_REMISE} WHERE etablissement_id = $1 AND id = $2"))
                    .bind(a.etablissement_id)
                    .bind(rid)
                    .fetch_one(self.pool())
                    .await
                    .map_err(db)?;
                self.lire_copie_autorise(a, &r).await?;
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
        if !self.objects().exists(&p.storage_key) {
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
        let enfants: Vec<Uuid> = if eleve.is_some() {
            Vec::new()
        } else if Role::parse(a.role) == Some(Role::Parent) {
            let l: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT eleve_id FROM ent_eleve_lien WHERE etablissement_id = $1 AND user_id = $2",
            )
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .fetch_all(self.pool())
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
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if ok && (Role::parse(a.role) != Some(Role::Parent) || self.parent_lie(a, e).await?) {
                return Ok(());
            }
        }
        Err(EntDbError::Forbidden)
    }

    async fn lire_copie_autorise(&self, a: &Acteur<'_>, r: &RemiseRow) -> Result<(), EntDbError> {
        if self.eleve_du_compte(a).await? == Some(r.eleve_id) {
            return Ok(());
        }
        if self.parent_lie(a, r.eleve_id).await? {
            return Ok(());
        }
        let t = self.charger_travail(a.etablissement_id, r.travail_id).await?;
        self.exiger_cible(a, Action::CahierEcrire, t.cible()).await
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
        let mut tx = self.pool().begin().await.map_err(db)?;
        let existante: Option<RemiseRow> = sqlx::query_as(&format!(
            "{SELECT_REMISE} WHERE etablissement_id = $1 AND travail_id = $2 AND eleve_id = $3 FOR UPDATE"
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
        self.exiger_cible(a, Action::CahierEcrire, t.cible()).await?;
        if !t.rendu_en_ligne {
            return Err(conflit("ce travail n'est pas à rendre en ligne"));
        }
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
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let remises: Vec<RemiseRow> = sqlx::query_as(&format!(
            "{SELECT_REMISE} WHERE etablissement_id = $1 AND travail_id = $2"
        ))
        .bind(a.etablissement_id)
        .bind(travail_id)
        .fetch_all(self.pool())
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
        let mut tx = self.pool().begin().await.map_err(db)?;
        let r: RemiseRow = sqlx::query_as(&format!(
            "{SELECT_REMISE} WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(a.etablissement_id)
        .bind(remise_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let t = self.charger_travail(a.etablissement_id, r.travail_id).await?;
        self.exiger_cible(a, Action::CahierEcrire, t.cible()).await?;
        if let Some(n) = note_id {
            // La note liée doit exister, dans cet établissement, pour cet élève.
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_note WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3)",
            )
            .bind(a.etablissement_id)
            .bind(n)
            .bind(r.eleve_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(invalide("note introuvable pour cet élève"));
            }
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
        let (eleve, _) = self.cahier_resoudre_eleve(a, eleve_id).await?;
        let rows: Vec<TravailRow> = sqlx::query_as(&format!(
            "{SELECT_TRAVAIL} WHERE t.etablissement_id = $1 AND t.supprime_le IS NULL
               AND t.pour_le BETWEEN $3 AND $4 AND {ELEVE_CONCERNE}
             ORDER BY t.pour_le, t.created_at"
        ))
        .bind(a.etablissement_id)
        .bind(eleve)
        .bind(du)
        .bind(au)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let etats: Vec<(Uuid, bool, bool)> = sqlx::query_as(
            "SELECT travail_id, fait, vu_le IS NOT NULL FROM ent_travail_eleve
              WHERE eleve_id = $1 AND travail_id = ANY($2)",
        )
        .bind(eleve)
        .bind(&ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let etats: HashMap<Uuid, (bool, bool)> = etats.into_iter().map(|(t, f, v)| (t, (f, v))).collect();
        let remises: Vec<RemiseRow> = sqlx::query_as(&format!(
            "{SELECT_REMISE} WHERE etablissement_id = $1 AND eleve_id = $2 AND travail_id = ANY($3)"
        ))
        .bind(a.etablissement_id)
        .bind(eleve)
        .bind(&ids)
        .fetch_all(self.pool())
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
                let (fait, vu) = etats.get(&t.id).copied().unwrap_or((false, false));
                let remise = remises.remove(&t.id);
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
        .execute(self.pool())
        .await
        .map_err(db)?;
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
        .fetch_all(self.pool())
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
        if let Some(p) = v.prof_user_id {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2)",
            )
            .bind(a.etablissement_id)
            .bind(p)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
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
            sqlx::query_as("SELECT vise_le FROM ent_cdt_visa WHERE id = $1")
                .bind(id)
                .fetch_one(self.pool())
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
    pub async fn cahier_visas(&self, a: &Acteur<'_>, classe_id: Uuid) -> Result<Vec<VisaDto>, EntDbError> {
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
            .fetch_all(self.pool())
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
                commentaire,
                vise_par_user_id: par,
                vise_le: le,
            })
            .collect())
    }

    /// Historique d'un travail (CDT-03) : auteur du travail ou direction.
    pub async fn cahier_historique_travail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<(i32, String, DateTime<Utc>, serde_json::Value)>, EntDbError> {
        let (auteur, classe, groupe): (Uuid, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
            "SELECT auteur_user_id, classe_id, groupe_id FROM ent_travail WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if auteur != a.user_id {
            self.exiger_cible(
                a,
                Action::CahierEcrire,
                Cible {
                    classe_id: classe,
                    groupe_id: groupe,
                },
            )
            .await?;
        }
        sqlx::query_as(
            "SELECT version, action, created_at, instantane FROM ent_cdt_historique
              WHERE etablissement_id = $1 AND objet = 'travail' AND objet_id = $2 ORDER BY created_at, version",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }
}

/// Fin de la journée `jour` (UTC) : échéance par défaut d'un travail à rendre.
fn fin_de_journee(jour: NaiveDate) -> DateTime<Utc> {
    (jour + Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .map(|d| d.and_utc() - Duration::seconds(1))
        .unwrap_or_else(Utc::now)
}

async fn historiser(
    tx: &mut Transaction<'_, Postgres>,
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
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Publie un fait du cahier dans la transaction de l'écriture (même commit, outbox S2-0).
async fn publier_cdt(
    tx: &mut Transaction<'_, Postgres>,
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
