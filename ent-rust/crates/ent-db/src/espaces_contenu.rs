//! Socle 32, lots ET0 et ET1 — dossiers, éléments, dépôt direct à taille signée, versions, téléchargement, corbeille
//! (`32_BUREAUTIQUE.md` §10, §15, §17.2 ; ETR-112, 114, 115, 118, 120, 122 à 125, 160 à 164, 274 ; R-ET2 à R-ET6, R-ET9,
//! R-ET21, R-ET25 ; ND32-01, 02, 05, 06, 08, 09, 10, 14).
//!
//! * **Jamais d'écrasement** (R-ET3, ND32-02) : un envoi sous un nom déjà pris est **proposé** (« nouvelle version de X » ou
//!   « nouveau fichier X (2) »), le choix est explicite ; une version est un objet immuable ; le retour arrière **crée** une version.
//! * **Taille signée, quota réservé** (R-ET21, ND32-01) : la demande de dépôt réserve le quota dans sa transaction puis émet
//!   l'URL d'envoi **à la taille signée** ; la finalisation relit la **taille réelle** chez le fournisseur et compare ; une
//!   taille différente rejette l'envoi, efface l'objet et libère la réservation ; l'empreinte est calculée par le serveur.
//! * **Analyse fermée par défaut** (CET-14, CET-15) : un fichier déposé est `analyse` ; il n'est visible que de son auteur
//!   (« en cours d'analyse », sans téléchargement) tant que l'antivirus n'a pas rendu un verdict sain. Une analyse indisponible
//!   laisse le fichier invisible.
//! * **Arborescence en base** (R-ET4, ND32-09, ND32-10) : déplacer ou renommer ne touche jamais l'objet ; les clés d'objet
//!   sont opaques (`{établissement}/espaces/{uuid}`) ; aucun chemin n'est interprété.
//! * **Liens courts** (R-ET9, ND32-06) : le téléchargement est un jeton de 60 s lié au compte, émis après contrôle des droits.
//! * **Corbeille 30 jours** (S-3205) : mise à la corbeille par l'auteur ou l'animateur ; un dossier se supprime après aperçu et
//!   confirmation ; l'effacement définitif épargne les versions figées par un autre socle (R-ET6).

#![allow(clippy::type_complexity)]

use crate::espaces::*;
use crate::espaces_quota::{quota_confirmer, quota_liberer, quota_rendre, quota_reserver, reservation_ouverte};
use crate::espaces_service::*;
use crate::notifications::db;
use crate::objets::{enregistrer_objet, hors_runtime};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use ent_object::presign::{now_unix, Disposition, LinkSpec, UploadSpec};
use ent_object::upload::{controler_nom, PolitiqueFichiers, TaillesMax, UsageFichier};
use ent_object::{ObjectInfo, ObjectStore};
use serde_json::json;
use sqlx::{PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

// ───────────────────────── Lignes et accès ─────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct DossierLigne {
    pub id: Uuid,
    pub espace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub nom: String,
    pub auteur: Uuid,
    pub supprime: bool,
    pub corbeille_lot: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub(crate) struct ElementLigne {
    pub id: Uuid,
    pub espace_id: Uuid,
    pub dossier_id: Option<Uuid>,
    pub nom: String,
    pub etat: String,
    pub auteur: Uuid,
    pub version_courante: Option<Uuid>,
    pub corbeille_lot: Option<Uuid>,
}

/// Ce que je peux sur un élément ou un dossier : mon rôle d'espace et/ou un partage nominatif actif.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Acces {
    pub role: Option<RoleEspace>,
    pub partage: Option<DroitPartage>,
}

impl Acces {
    pub fn lit(&self) -> bool {
        self.role.is_some() || self.partage.is_some()
    }

    /// Écrire dans un dossier ou déposer une version : contributeur d'espace, ou partage en écriture.
    pub fn ecrit(&self) -> bool {
        self.role.is_some_and(|r| r.peut_contribuer()) || self.partage == Some(DroitPartage::Ecriture)
    }

    pub fn anime(&self) -> bool {
        self.role.is_some_and(|r| r.est_animateur())
    }
}

pub(crate) async fn dossier_ligne(conn: &mut PgConnection, etab: Uuid, id: Uuid, verrou: bool) -> Result<Option<DossierLigne>, EntDbError> {
    let sql = format!(
        "SELECT id, espace_id, parent_id, nom, cree_par_personne_id, supprime_le IS NOT NULL, corbeille_lot_id
           FROM ent_espace_dossier WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    let r = sqlx::query(&sql).bind(etab).bind(id).fetch_optional(&mut *conn).await.map_err(db)?;
    Ok(r.map(|r| DossierLigne {
        id: r.get(0),
        espace_id: r.get(1),
        parent_id: r.get(2),
        nom: r.get(3),
        auteur: r.get(4),
        supprime: r.get(5),
        corbeille_lot: r.get(6),
    }))
}

pub(crate) async fn element_ligne(conn: &mut PgConnection, etab: Uuid, id: Uuid, verrou: bool) -> Result<Option<ElementLigne>, EntDbError> {
    let sql = format!(
        "SELECT id, espace_id, dossier_id, nom, nature, etat, auteur_personne_id, version_courante_id, corbeille_lot_id
           FROM ent_espace_element WHERE etablissement_id = $1 AND id = $2{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    let r = sqlx::query(&sql).bind(etab).bind(id).fetch_optional(&mut *conn).await.map_err(db)?;
    Ok(r.map(|r| ElementLigne {
        id: r.get(0),
        espace_id: r.get(1),
        dossier_id: r.get(2),
        nom: r.get(3),
        etat: r.get(5),
        auteur: r.get(6),
        version_courante: r.get(7),
        corbeille_lot: r.get(8),
    }))
}

/// Meilleur droit de partage actif de la personne sur un élément ou sur un dossier de la chaîne d'ancêtres
/// (un partage de dossier couvre ses sous-dossiers et leurs éléments ; R-ET13 : sur l'identifiant, jamais sur un chemin).
pub(crate) async fn partage_sur(
    conn: &mut PgConnection,
    etab: Uuid,
    element: Option<Uuid>,
    dossier: Option<Uuid>,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<Option<DroitPartage>, EntDbError> {
    let droits: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE anc AS (
            SELECT d.id, d.parent_id, 1 AS n FROM ent_espace_dossier d
             WHERE d.etablissement_id = $1 AND d.id = $2 AND d.supprime_le IS NULL
            UNION ALL
            SELECT p.id, p.parent_id, anc.n + 1 FROM ent_espace_dossier p
              JOIN anc ON p.id = anc.parent_id
             WHERE p.etablissement_id = $1 AND p.supprime_le IS NULL AND anc.n < 16)
         SELECT s.droit FROM ent_espace_partage s
          WHERE s.etablissement_id = $1 AND s.destinataire_personne_id = $3 AND s.retire_le IS NULL
            AND s.debut <= $4 AND (s.fin IS NULL OR s.fin >= $4)
            AND ((s.element_id IS NOT NULL AND s.element_id = $5) OR s.dossier_id IN (SELECT id FROM anc))",
    )
    .bind(etab)
    .bind(dossier)
    .bind(personne)
    .bind(jour)
    .bind(element)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut meilleur = None;
    for d in droits {
        match DroitPartage::depuis_code(&d) {
            Some(DroitPartage::Ecriture) => return Ok(Some(DroitPartage::Ecriture)),
            Some(DroitPartage::Lecture) => meilleur = Some(DroitPartage::Lecture),
            None => {}
        }
    }
    Ok(meilleur)
}

/// Accès à un élément : lecture par rôle d'espace ou partage ; un fichier non analysé, en quarantaine ou en corbeille
/// n'est visible que de son auteur et des animateurs (jamais téléchargeable : le contrôle des octets est fait ailleurs).
/// Tout refus d'accès répond « introuvable ».
pub(crate) async fn acces_element(
    conn: &mut PgConnection,
    etab: Uuid,
    element: Uuid,
    moi: Moi,
    jour: NaiveDate,
    verrou: bool,
    inclure_corbeille: bool,
) -> Result<(ElementLigne, EspaceLigne, Acces), EntDbError> {
    let el = element_ligne(conn, etab, element, verrou).await?.ok_or_else(introuvable)?;
    let esp = ligne_espace(conn, etab, el.espace_id, false).await?.ok_or_else(introuvable)?;
    let role = role_dans_espace(conn, etab, &esp, moi.personne, jour).await?;
    let partage = partage_sur(conn, etab, Some(el.id), el.dossier_id, moi.personne, jour).await?;
    let acces = Acces { role, partage };
    if !acces.lit() {
        return Err(introuvable());
    }
    if el.etat == "efface" || (el.etat == "corbeille" && !inclure_corbeille) {
        return Err(introuvable());
    }
    let voit_tout = acces.anime() || el.auteur == moi.personne;
    if el.etat != "disponible" && !voit_tout {
        return Err(introuvable());
    }
    // Un élément en corbeille ne se manipule que par l'auteur ou l'animateur.
    if el.etat == "corbeille" && !voit_tout {
        return Err(introuvable());
    }
    Ok((el, esp, acces))
}

/// Accès à un dossier vivant : lecture par rôle ou partage sur lui ou un de ses ancêtres.
pub(crate) async fn acces_dossier(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
    moi: Moi,
    jour: NaiveDate,
    verrou: bool,
) -> Result<(DossierLigne, EspaceLigne, Acces), EntDbError> {
    let d = dossier_ligne(conn, etab, dossier, verrou).await?.ok_or_else(introuvable)?;
    if d.supprime {
        return Err(introuvable());
    }
    let esp = ligne_espace(conn, etab, d.espace_id, false).await?.ok_or_else(introuvable)?;
    let role = role_dans_espace(conn, etab, &esp, moi.personne, jour).await?;
    let partage = partage_sur(conn, etab, None, Some(d.id), moi.personne, jour).await?;
    let acces = Acces { role, partage };
    if !acces.lit() {
        return Err(introuvable());
    }
    Ok((d, esp, acces))
}

/// Profondeur d'un dossier (un dossier à la racine : 1).
async fn profondeur(conn: &mut PgConnection, etab: Uuid, dossier: Option<Uuid>) -> Result<usize, EntDbError> {
    let Some(d) = dossier else { return Ok(0) };
    let n: Option<i32> = sqlx::query_scalar(
        "WITH RECURSIVE anc AS (
            SELECT id, parent_id, 1 AS n FROM ent_espace_dossier WHERE etablissement_id = $1 AND id = $2
            UNION ALL
            SELECT p.id, p.parent_id, anc.n + 1 FROM ent_espace_dossier p JOIN anc ON p.id = anc.parent_id
             WHERE p.etablissement_id = $1 AND anc.n < 64)
         SELECT max(n) FROM anc",
    )
    .bind(etab)
    .bind(d)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(n.unwrap_or(0).max(0) as usize)
}

/// Hauteur du sous-arbre d'un dossier, lui compris (un dossier sans enfant : 1) ; dossiers vivants seulement.
async fn hauteur(conn: &mut PgConnection, etab: Uuid, dossier: Uuid) -> Result<usize, EntDbError> {
    let n: Option<i32> = sqlx::query_scalar(
        "WITH RECURSIVE sub AS (
            SELECT id, 1 AS n FROM ent_espace_dossier WHERE etablissement_id = $1 AND id = $2
            UNION ALL
            SELECT d.id, sub.n + 1 FROM ent_espace_dossier d JOIN sub ON d.parent_id = sub.id
             WHERE d.etablissement_id = $1 AND d.supprime_le IS NULL AND sub.n < 64)
         SELECT max(n) FROM sub",
    )
    .bind(etab)
    .bind(dossier)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(n.unwrap_or(1).max(1) as usize)
}

/// Dossier et tous ses descendants (vivants).
async fn sous_arbre(conn: &mut PgConnection, etab: Uuid, dossier: Uuid) -> Result<Vec<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "WITH RECURSIVE sub AS (
            SELECT id, 1 AS n FROM ent_espace_dossier WHERE etablissement_id = $1 AND id = $2
            UNION ALL
            SELECT d.id, sub.n + 1 FROM ent_espace_dossier d JOIN sub ON d.parent_id = sub.id
             WHERE d.etablissement_id = $1 AND d.supprime_le IS NULL AND sub.n < 64)
         SELECT id FROM sub",
    )
    .bind(etab)
    .bind(dossier)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)
}

async fn chemin_de(conn: &mut PgConnection, etab: Uuid, dossier: Option<Uuid>) -> Result<Vec<DossierVue>, EntDbError> {
    let Some(d) = dossier else { return Ok(Vec::new()) };
    let l = sqlx::query(
        "WITH RECURSIVE anc AS (
            SELECT id, parent_id, nom, 1 AS n FROM ent_espace_dossier WHERE etablissement_id = $1 AND id = $2
            UNION ALL
            SELECT p.id, p.parent_id, p.nom, anc.n + 1 FROM ent_espace_dossier p JOIN anc ON p.id = anc.parent_id
             WHERE p.etablissement_id = $1 AND anc.n < 64)
         SELECT id, parent_id, nom FROM anc ORDER BY n DESC",
    )
    .bind(etab)
    .bind(d)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(l.iter().map(|r| DossierVue { id: r.get(0), parent_id: r.get(1), nom: r.get(2), nb_elements: 0 }).collect())
}

/// Noms normalisés des éléments vivants d'un dossier (ou de la racine) : unicité et proposition « (2) ».
async fn noms_elements(conn: &mut PgConnection, etab: Uuid, espace: Uuid, dossier: Option<Uuid>) -> Result<BTreeMap<String, Uuid>, EntDbError> {
    let l = sqlx::query(
        "SELECT nom_normalise, id FROM ent_espace_element
          WHERE etablissement_id = $1 AND espace_id = $2 AND dossier_id IS NOT DISTINCT FROM $3
            AND etat IN ('analyse', 'disponible', 'masque')",
    )
    .bind(etab)
    .bind(espace)
    .bind(dossier)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(l.iter().map(|r| (r.get::<String, _>(0), r.get::<Uuid, _>(1))).collect())
}

async fn noms_dossiers(conn: &mut PgConnection, etab: Uuid, espace: Uuid, parent: Option<Uuid>) -> Result<BTreeSet<String>, EntDbError> {
    let l: Vec<String> = sqlx::query_scalar(
        "SELECT nom_normalise FROM ent_espace_dossier
          WHERE etablissement_id = $1 AND espace_id = $2 AND parent_id IS NOT DISTINCT FROM $3 AND supprime_le IS NULL",
    )
    .bind(etab)
    .bind(espace)
    .bind(parent)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(l.into_iter().collect())
}

async fn mon_acces_espace(
    conn: &mut PgConnection,
    etab: Uuid,
    espace_id: Uuid,
    moi: Moi,
    jour: NaiveDate,
    dossier: Option<Uuid>,
) -> Result<(EspaceLigne, Acces), EntDbError> {
    let e = ligne_espace(conn, etab, espace_id, false).await?.ok_or_else(introuvable)?;
    let role = role_dans_espace(conn, etab, &e, moi.personne, jour).await?;
    let partage = match dossier {
        Some(d) => partage_sur(conn, etab, None, Some(d), moi.personne, jour).await?,
        None => None,
    };
    let a = Acces { role, partage };
    if !a.lit() {
        return Err(introuvable());
    }
    Ok((e, a))
}

// ───────────────────────── Service : dossiers, contenu ─────────────────────────

impl PgEnt {
    /// Contenu d'un dossier (ou de la racine d'un espace) : sous-dossiers et éléments visibles de moi. Un fichier non
    /// analysé n'apparaît qu'à son auteur et aux animateurs. **Aucune écriture** (ND32-11, ND32-15).
    pub async fn espaces_parcourir(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        dossier_id: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<Contenu, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (e, acces) = if let Some(d) = dossier_id {
            let dl = dossier_ligne(&mut tx, etab, d, false).await?.ok_or_else(introuvable)?;
            if dl.supprime || dl.espace_id != espace_id {
                return Err(introuvable());
            }
            mon_acces_espace(&mut tx, etab, espace_id, moi, jour, Some(d)).await?
        } else {
            // La racine d'un espace ne se parcourt que comme membre ; un destinataire de partage passe par « Partagés avec moi ».
            let (e, a) = mon_acces_espace(&mut tx, etab, espace_id, moi, jour, None).await?;
            (e, a)
        };
        let voit_tout = acces.anime();
        let chemin = chemin_de(&mut tx, etab, dossier_id).await?;
        let dossiers = sqlx::query(
            "SELECT d.id, d.nom, d.parent_id,
                    (SELECT count(*) FROM ent_espace_element el
                      WHERE el.etablissement_id = d.etablissement_id AND el.dossier_id = d.id AND el.etat = 'disponible')
               FROM ent_espace_dossier d
              WHERE d.etablissement_id = $1 AND d.espace_id = $2 AND d.parent_id IS NOT DISTINCT FROM $3 AND d.supprime_le IS NULL
              ORDER BY lower(d.nom)",
        )
        .bind(etab)
        .bind(espace_id)
        .bind(dossier_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let elements = sqlx::query(
            "SELECT el.id, el.nom, el.dossier_id, el.nature, el.etat, btrim(COALESCE(p.prenom || ' ', '') || p.nom),
                    el.auteur_personne_id = $5, v.taille, v.type_reel, v.numero, el.modifie_le
               FROM ent_espace_element el
               JOIN ent_personne p ON p.etablissement_id = el.etablissement_id AND p.id = el.auteur_personne_id
               LEFT JOIN LATERAL (
                    SELECT v.taille, v.type_reel, v.numero FROM ent_espace_version v
                     WHERE v.etablissement_id = el.etablissement_id AND v.element_id = el.id AND v.elaguee_le IS NULL
                     ORDER BY (v.id = el.version_courante_id) DESC, v.numero DESC LIMIT 1) v ON true
              WHERE el.etablissement_id = $1 AND el.espace_id = $2 AND el.dossier_id IS NOT DISTINCT FROM $3
                AND el.etat IN ('analyse', 'disponible', 'quarantaine', 'masque')
                AND ($4 OR el.etat = 'disponible' OR (el.auteur_personne_id = $5 AND el.etat IN ('analyse', 'quarantaine')))
              ORDER BY lower(el.nom)",
        )
        .bind(etab)
        .bind(espace_id)
        .bind(dossier_id)
        .bind(voit_tout)
        .bind(moi.personne)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(Contenu {
            espace_id,
            dossier_id,
            chemin,
            dossiers: dossiers
                .iter()
                .map(|r| DossierVue { id: r.get(0), nom: r.get(1), parent_id: r.get(2), nb_elements: r.get(3) })
                .collect(),
            elements: elements
                .iter()
                .map(|r| ElementVue {
                    id: r.get(0),
                    nom: r.get(1),
                    dossier_id: r.get(2),
                    nature: r.get(3),
                    etat: r.get(4),
                    auteur_nom: r.get(5),
                    auteur_moi: r.get(6),
                    taille: r.get(7),
                    type_reel: r.get(8),
                    version_numero: r.get(9),
                    modifie_le: r.get(10),
                })
                .collect(),
            role: acces.role,
            peut_ecrire: e.etat == "ouvert" && acces.ecrit(),
        })
    }

    /// Crée un dossier (ETR-112, NX32-03) : contributeur ; nom ≤ 120 caractères, profondeur ≤ 8 (R-ET25) ; un nom déjà
    /// pris est refusé (la proposition « (2) » est dans le message d'erreur du conflit).
    pub async fn espaces_dossier_creer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        parent_id: Option<Uuid>,
        nom: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<DossierVue, EntDbError> {
        let propre = nom_propre(nom, NOM_MAX).map_err(invalide)?;
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        if let Some(p) = parent_id {
            let pl = dossier_ligne(&mut tx, etab, p, false).await?.ok_or_else(introuvable)?;
            if pl.supprime || pl.espace_id != espace_id {
                return Err(introuvable());
            }
        }
        let (e, acces) = mon_acces_espace(&mut tx, etab, espace_id, moi, jour, parent_id).await?;
        exiger_ouvert(&e)?;
        if !acces.ecrit() {
            return Err(interdit("Vous ne pouvez pas créer de dossier ici."));
        }
        if profondeur(&mut tx, etab, parent_id).await? + 1 > PROFONDEUR_MAX {
            return Err(conflit("Les dossiers sont limités à huit niveaux d'imbrication."));
        }
        let normalise = nom_normalise(&propre);
        if noms_dossiers(&mut tx, etab, espace_id, parent_id).await?.contains(&normalise) {
            return Err(conflit("Un dossier de ce nom existe déjà à cet endroit."));
        }
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_espace_dossier
                (id, etablissement_id, espace_id, parent_id, nom, nom_normalise, cree_par_personne_id, cree_le, modifie_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)",
        )
        .bind(id)
        .bind(etab)
        .bind(espace_id)
        .bind(parent_id)
        .bind(&propre)
        .bind(&normalise)
        .bind(moi.personne)
        .bind(maintenant)
        .execute(&mut *tx)
        .await;
        if let Err(e) = r {
            return Err(if est_unicite(&e) { conflit("Un dossier de ce nom existe déjà à cet endroit.") } else { db(e) });
        }
        toucher_activite(&mut tx, etab, espace_id, maintenant).await?;
        journaliser(&mut tx, etab, Some(espace_id), None, Some(moi.personne), "dossier.cree", None, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(DossierVue { id, nom: propre, parent_id, nb_elements: 0 })
    }

    /// Renomme un dossier (ETR-120) : animateur, ou auteur du dossier s'il contribue.
    pub async fn espaces_dossier_renommer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        dossier_id: Uuid,
        nom: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let propre = nom_propre(nom, NOM_MAX).map_err(invalide)?;
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (d, e, acces) = acces_dossier(&mut tx, etab, dossier_id, moi, jour, true).await?;
        exiger_ouvert(&e)?;
        if !(acces.anime() || (d.auteur == moi.personne && acces.role.is_some_and(|r| r.peut_contribuer()))) {
            return Err(interdit("Seul l'animateur ou l'auteur du dossier peut le renommer."));
        }
        let normalise = nom_normalise(&propre);
        let mut pris = noms_dossiers(&mut tx, etab, d.espace_id, d.parent_id).await?;
        pris.remove(&nom_normalise(&d.nom));
        if pris.contains(&normalise) {
            return Err(conflit("Un dossier de ce nom existe déjà à cet endroit."));
        }
        sqlx::query(
            "UPDATE ent_espace_dossier SET nom = $3, nom_normalise = $4, modifie_le = $5 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(dossier_id)
        .bind(&propre)
        .bind(&normalise)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        toucher_activite(&mut tx, etab, d.espace_id, maintenant).await?;
        journaliser(&mut tx, etab, Some(d.espace_id), None, Some(moi.personne), "dossier.renomme", None, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Déplace un dossier dans le même espace (ETR-120, NX32-07) : jamais dans lui-même ni dans un descendant, profondeur
    /// totale ≤ 8, nom libre à destination. L'arborescence est en base : **aucun objet ne bouge** (R-ET4, ND32-09).
    pub async fn espaces_dossier_deplacer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        dossier_id: Uuid,
        vers_parent: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (d, e, acces) = acces_dossier(&mut tx, etab, dossier_id, moi, jour, true).await?;
        exiger_ouvert(&e)?;
        if !(acces.anime() || (d.auteur == moi.personne && acces.role.is_some_and(|r| r.peut_contribuer()))) {
            return Err(interdit("Seul l'animateur ou l'auteur du dossier peut le déplacer."));
        }
        if let Some(cible) = vers_parent {
            let cl = dossier_ligne(&mut tx, etab, cible, false).await?.ok_or_else(introuvable)?;
            if cl.supprime || cl.espace_id != d.espace_id {
                return Err(introuvable());
            }
            // L'animateur ou le contributeur doit pouvoir écrire dans la destination.
            let a2 = Acces { role: acces.role, partage: partage_sur(&mut tx, etab, None, Some(cible), moi.personne, jour).await? };
            if !a2.ecrit() {
                return Err(interdit("Vous ne pouvez pas écrire dans le dossier de destination."));
            }
            if sous_arbre(&mut tx, etab, dossier_id).await?.contains(&cible) {
                return Err(conflit("Un dossier ne se déplace pas dans lui-même ni dans un de ses sous-dossiers."));
            }
        }
        if profondeur(&mut tx, etab, vers_parent).await? + hauteur(&mut tx, etab, dossier_id).await? > PROFONDEUR_MAX {
            return Err(conflit("Les dossiers sont limités à huit niveaux d'imbrication."));
        }
        let normalise = nom_normalise(&d.nom);
        if d.parent_id != vers_parent && noms_dossiers(&mut tx, etab, d.espace_id, vers_parent).await?.contains(&normalise) {
            return Err(conflit("Un dossier de ce nom existe déjà à destination."));
        }
        let r = sqlx::query(
            "UPDATE ent_espace_dossier SET parent_id = $3, modifie_le = $4 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(dossier_id)
        .bind(vers_parent)
        .bind(maintenant)
        .execute(&mut *tx)
        .await;
        if let Err(e) = r {
            return Err(if est_unicite(&e) { conflit("Un dossier de ce nom existe déjà à destination.") } else { db(e) });
        }
        toucher_activite(&mut tx, etab, d.espace_id, maintenant).await?;
        journaliser(&mut tx, etab, Some(d.espace_id), None, Some(moi.personne), "dossier.deplace", None, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Renomme un élément (ETR-120) : l'animateur, ou l'auteur s'il contribue. Nom pris : refus (proposition « (2) »).
    pub async fn espaces_element_renommer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Uuid,
        nom: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let propre = nom_propre(nom, NOM_FICHIER_MAX).map_err(invalide)?;
        controler_nom(&propre).map_err(EntDbError::Fichier)?;
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (el, e, acces) = acces_element(&mut tx, etab, element_id, moi, jour, true, false).await?;
        exiger_ouvert(&e)?;
        if !(acces.anime() || (el.auteur == moi.personne && acces.role.is_some_and(|r| r.peut_contribuer()))) {
            return Err(interdit("Seul l'animateur ou l'auteur peut renommer ce fichier."));
        }
        let normalise = nom_normalise(&propre);
        let mut noms = noms_elements(&mut tx, etab, el.espace_id, el.dossier_id).await?;
        noms.remove(&nom_normalise(&el.nom));
        if noms.contains_key(&normalise) {
            return Err(conflit("Un fichier de ce nom existe déjà dans ce dossier."));
        }
        let r = sqlx::query(
            "UPDATE ent_espace_element SET nom = $3, nom_normalise = $4, modifie_le = $5 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(element_id)
        .bind(&propre)
        .bind(&normalise)
        .bind(maintenant)
        .execute(&mut *tx)
        .await;
        if let Err(e) = r {
            return Err(if est_unicite(&e) { conflit("Un fichier de ce nom existe déjà dans ce dossier.") } else { db(e) });
        }
        toucher_activite(&mut tx, etab, el.espace_id, maintenant).await?;
        journaliser(&mut tx, etab, Some(el.espace_id), Some(element_id), Some(moi.personne), "element.renomme", None, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Déplace un élément dans le même espace (ETR-120, NX32-07) : animateur, ou auteur s'il contribue. Nom pris à
    /// destination : refus. Aucun objet ne bouge ; un partage de l'élément le suit (il porte l'identifiant, ND32-08).
    pub async fn espaces_element_deplacer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Uuid,
        vers_dossier: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (el, e, acces) = acces_element(&mut tx, etab, element_id, moi, jour, true, false).await?;
        exiger_ouvert(&e)?;
        if !(acces.anime() || (el.auteur == moi.personne && acces.role.is_some_and(|r| r.peut_contribuer()))) {
            return Err(interdit("Seul l'animateur ou l'auteur peut déplacer ce fichier."));
        }
        if let Some(cible) = vers_dossier {
            let cl = dossier_ligne(&mut tx, etab, cible, false).await?.ok_or_else(introuvable)?;
            if cl.supprime || cl.espace_id != el.espace_id {
                return Err(introuvable());
            }
            let a2 = Acces { role: acces.role, partage: partage_sur(&mut tx, etab, None, Some(cible), moi.personne, jour).await? };
            if !a2.ecrit() {
                return Err(interdit("Vous ne pouvez pas écrire dans le dossier de destination."));
            }
        }
        let normalise = nom_normalise(&el.nom);
        let noms = noms_elements(&mut tx, etab, el.espace_id, vers_dossier).await?;
        if el.dossier_id != vers_dossier {
            if noms.contains_key(&normalise) {
                return Err(conflit("Un fichier de ce nom existe déjà à destination."));
            }
            if noms.len() as i64 >= ELEMENTS_PAR_DOSSIER_MAX {
                return Err(conflit("Ce dossier contient déjà le nombre maximal de fichiers."));
            }
        }
        let r = sqlx::query(
            "UPDATE ent_espace_element SET dossier_id = $3, modifie_le = $4 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(element_id)
        .bind(vers_dossier)
        .bind(maintenant)
        .execute(&mut *tx)
        .await;
        if let Err(e) = r {
            return Err(if est_unicite(&e) { conflit("Un fichier de ce nom existe déjà à destination.") } else { db(e) });
        }
        toucher_activite(&mut tx, etab, el.espace_id, maintenant).await?;
        journaliser(&mut tx, etab, Some(el.espace_id), Some(element_id), Some(moi.personne), "element.deplace", None, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───────────────────────── Dépôt direct, taille signée ─────────────────────────

    /// Demande de dépôt (ETR-114, NX32-04, ETR-161) : contrôle les droits, le nom, la taille annoncée et le quota, **réserve
    /// le quota** dans la transaction, puis émet l'URL d'envoi **à la taille signée**. Sous un nom déjà pris, la réponse
    /// est « nom déjà pris » avec la proposition : ni écrasement ni choix silencieux.
    pub async fn espaces_demander_depot(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        d: DemandeDepot,
        maintenant: DateTime<Utc>,
    ) -> Result<ReponseDepot, EntDbError> {
        if d.taille <= 0 || d.taille > PLAFOND_FICHIER_OCTETS {
            return Err(invalide("La taille du fichier est nulle ou dépasse le plafond de 64 Mio."));
        }
        if d.taille as usize > self.politique_fichiers().taille_max(UsageFichier::Espace) {
            return Err(invalide("Ce fichier dépasse la taille maximale autorisée."));
        }
        let mut propre = nom_propre(&d.nom, NOM_FICHIER_MAX).map_err(invalide)?;
        controler_nom(&propre).map_err(EntDbError::Fichier)?;
        if d.mime.as_deref().is_some_and(|m| m.len() > 127 || m.chars().any(|c| c.is_control())) {
            return Err(invalide("Type de fichier annoncé invalide."));
        }
        let commentaire = d.commentaire.as_deref().map(str::trim).filter(|c| !c.is_empty()).map(str::to_string);
        if commentaire.as_ref().is_some_and(|c| c.chars().count() > 300 || c.chars().any(|ch| ch.is_control())) {
            return Err(invalide("Le commentaire est trop long ou invalide."));
        }
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        if let Some(dos) = d.dossier_id {
            let dl = dossier_ligne(&mut tx, etab, dos, false).await?.ok_or_else(introuvable)?;
            if dl.supprime || dl.espace_id != espace_id {
                return Err(introuvable());
            }
        }
        let (e, acces) = mon_acces_espace(&mut tx, etab, espace_id, moi, jour, d.dossier_id).await?;
        exiger_ouvert(&e)?;
        let noms = noms_elements(&mut tx, etab, espace_id, d.dossier_id).await?;
        let normalise = nom_normalise(&propre);
        let mut element_cible: Option<Uuid> = None;
        if let Some(existant) = noms.get(&normalise).copied() {
            match d.si_existe {
                None => {
                    if !acces.ecrit() {
                        return Err(interdit("Vous ne pouvez pas déposer ici."));
                    }
                    let pris: BTreeSet<String> = noms.keys().cloned().collect();
                    tx.rollback().await.map_err(db)?;
                    return Ok(ReponseDepot::NomDejaPris { element_id: existant, nom_propose: nom_propose(&propre, &pris) });
                }
                Some(ChoixSiExiste::NouveauFichier) => {
                    let pris: BTreeSet<String> = noms.keys().cloned().collect();
                    propre = nom_propose(&propre, &pris);
                }
                Some(ChoixSiExiste::NouvelleVersion) => {
                    // Nouvelle version d'un élément existant : droit d'écriture sur lui (espace ou partage).
                    let (el, _, a_el) = acces_element(&mut tx, etab, existant, moi, jour, true, false).await?;
                    if !a_el.ecrit() {
                        return Err(interdit("Vous ne pouvez pas déposer de version de ce fichier."));
                    }
                    if el.etat == "quarantaine" {
                        return Err(conflit("Ce fichier est en quarantaine."));
                    }
                    element_cible = Some(existant);
                }
            }
        } else {
            if !acces.ecrit() {
                return Err(interdit("Vous ne pouvez pas déposer ici."));
            }
            if d.si_existe == Some(ChoixSiExiste::NouvelleVersion) {
                return Err(conflit("Aucun fichier de ce nom n'existe : rien à versionner."));
            }
        }
        // Nouvel élément (nom libre ou « (2) ») : il faut pouvoir écrire dans le dossier, quel que soit le chemin pris.
        if element_cible.is_none() && !acces.ecrit() {
            return Err(interdit("Vous ne pouvez pas déposer ici."));
        }
        if element_cible.is_none() && noms.len() as i64 >= ELEMENTS_PAR_DOSSIER_MAX {
            return Err(conflit("Ce dossier contient déjà le nombre maximal de fichiers."));
        }
        let autorise = self
            .reserver_envoi(&mut tx, etab, user, moi, espace_id, d.dossier_id, element_cible, propre, d.taille, d.mime.as_deref(), commentaire.as_deref(), maintenant)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(ReponseDepot::Autorise(autorise))
    }

    /// Demande de dépôt d'une **nouvelle version** d'un élément par son identifiant (ETR-161) : droit d'écriture sur
    /// l'élément (rôle d'espace, ou partage en écriture : un destinataire n'a pas besoin d'être membre de l'espace). Même
    /// réservation de quota et même URL à taille signée ; jamais d'écrasement.
    pub async fn espaces_demander_version(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Uuid,
        taille: i64,
        mime: Option<String>,
        commentaire: Option<String>,
        maintenant: DateTime<Utc>,
    ) -> Result<DepotAutorise, EntDbError> {
        if taille <= 0 || taille > PLAFOND_FICHIER_OCTETS || taille as usize > self.politique_fichiers().taille_max(UsageFichier::Espace) {
            return Err(invalide("La taille du fichier est nulle ou dépasse le plafond de 64 Mio."));
        }
        if mime.as_deref().is_some_and(|m| m.len() > 127 || m.chars().any(|c| c.is_control())) {
            return Err(invalide("Type de fichier annoncé invalide."));
        }
        let commentaire = commentaire.as_deref().map(str::trim).filter(|c| !c.is_empty()).map(str::to_string);
        if commentaire.as_ref().is_some_and(|c| c.chars().count() > 300 || c.chars().any(|ch| ch.is_control())) {
            return Err(invalide("Le commentaire est trop long ou invalide."));
        }
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (el, e, acces) = acces_element(&mut tx, etab, element_id, moi, jour, true, false).await?;
        exiger_ouvert(&e)?;
        if !acces.ecrit() {
            return Err(interdit("Vous ne pouvez pas déposer de version de ce fichier."));
        }
        if !matches!(el.etat.as_str(), "disponible" | "analyse") {
            return Err(conflit("Ce fichier n'est pas disponible."));
        }
        let autorise = self
            .reserver_envoi(&mut tx, etab, user, moi, el.espace_id, el.dossier_id, Some(el.id), el.nom.clone(), taille, mime.as_deref(), commentaire.as_deref(), maintenant)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(autorise)
    }

    /// Réserve le quota, note l'intention d'envoi et émet l'URL d'envoi à taille signée (dans la transaction de l'appelant).
    #[allow(clippy::too_many_arguments)]
    async fn reserver_envoi(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        moi: Moi,
        espace_id: Uuid,
        dossier_id: Option<Uuid>,
        element_cible: Option<Uuid>,
        nom: String,
        taille: i64,
        mime: Option<&str>,
        commentaire: Option<&str>,
        maintenant: DateTime<Utc>,
    ) -> Result<DepotAutorise, EntDbError> {
        // Clé opaque : aucun nom de fichier (E14, ND32-10).
        let cle = ObjectStore::key_for(etab, "espaces", Uuid::new_v4(), "bin");
        let (reservation_id, expire_le) = quota_reserver(tx, etab, espace_id, taille, &cle, user, maintenant).await?;
        sqlx::query(
            "INSERT INTO ent_espace_envoi
                (reservation_id, etablissement_id, espace_id, dossier_id, element_id, nom, mime_declare, commentaire, demandeur_personne_id, cree_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(reservation_id)
        .bind(etab)
        .bind(espace_id)
        .bind(dossier_id)
        .bind(element_cible)
        .bind(&nom)
        .bind(mime)
        .bind(commentaire)
        .bind(moi.personne)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // L'URL ne vaut que pour cette taille (signée) et 15 minutes : si l'émission est impossible, tout est annulé.
        let envoi = self
            .presigner()
            .issue_upload(&UploadSpec { key: cle, user_id: user, etablissement_id: etab, taille: taille as u64 }, maintenant.timestamp())
            .map_err(|_| conflit("L'envoi de fichiers est momentanément indisponible."))?;
        Ok(DepotAutorise {
            reservation_id,
            url: envoi.url,
            methode: envoi.methode.to_string(),
            en_tetes: envoi.en_tetes,
            expire_le,
            nom,
        })
    }

    /// Récepteur de l'envoi **en mode local** (labo, sans S3) : équivalent du magasin qui reçoit l'envoi direct. Il vérifie que
    /// la clé correspond à une réservation ouverte du demandeur, non expirée, que le corps a **exactement la taille signée**, puis
    /// écrit l'objet sous la clé réservée. En production (S3), cette route n'existe pas : le fournisseur impose la taille signée.
    pub async fn espaces_recevoir_envoi_local(
        &self,
        etab: Uuid,
        user: Uuid,
        cle: &str,
        taille_signee: u64,
        octets: Vec<u8>,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        if octets.len() as u64 != taille_signee {
            return Err(invalide("La taille du corps ne correspond pas à la taille signée."));
        }
        let ok: Option<i32> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let r = sqlx::query_scalar(
                "SELECT 1 FROM ent_espace_reservation
                  WHERE etablissement_id = $1 AND cle_objet = $2 AND demandeur_user_id = $3 AND octets = $4
                    AND confirmee_le IS NULL AND liberee_le IS NULL AND expire_le > $5",
            )
            .bind(etab)
            .bind(cle)
            .bind(user)
            .bind(taille_signee as i64)
            .bind(maintenant)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            r
        };
        if ok.is_none() {
            return Err(introuvable());
        }
        self.objet_put(cle, &octets).await?;
        Ok(())
    }

    /// Annule un envoi annoncé (le navigateur renonce) : libère le quota et efface l'éventuel objet.
    pub async fn espaces_annuler_depot(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        reservation_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        // Une réservation qui n'est pas la mienne, ou n'existe pas dans cet établissement : « introuvable » (jamais un 2xx
        // « idempotent » sur une cible absente). Une réservation à moi déjà confirmée ou libérée : rien à faire, succès.
        let a_moi: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM ent_espace_reservation WHERE etablissement_id = $1 AND id = $2 AND demandeur_user_id = $3",
        )
        .bind(etab)
        .bind(reservation_id)
        .bind(moi.user)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if a_moi.is_none() {
            return Err(introuvable());
        }
        let Some(r) = reservation_ouverte(&mut tx, etab, reservation_id).await? else {
            return Ok(());
        };
        quota_liberer(&mut tx, etab, reservation_id, "annulee", maintenant).await?;
        tx.commit().await.map_err(db)?;
        self.effacer_objet_sans_registre(&r.cle_objet).await;
        Ok(())
    }

    /// Efface physiquement un objet jamais enregistré (envoi rejeté, annulé ou expiré) : au mieux, l'échec est journalisé.
    pub(crate) async fn effacer_objet_sans_registre(&self, cle: &str) {
        if let Err(e) = self.objet_supprimer(cle).await {
            tracing::warn!(erreur = %e, "espaces : effacement d'un objet d'envoi rejeté impossible (à rattraper par la réconciliation)");
        }
    }

    /// Finalisation d'un dépôt (ETR-114) : **réconciliation** de la taille réelle (relue chez le fournisseur) avec la taille
    /// signée, empreinte calculée par le serveur, type réel par signature, liste blanche ; puis, dans une transaction, le
    /// quota passe de réservé à confirmé et l'élément (ou la version) est créé **en cours d'analyse**. Toute anomalie
    /// rejette l'envoi : objet effacé, quota libéré, rien d'écrit.
    pub async fn espaces_finaliser_depot(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        reservation_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<ResultatDepot, EntDbError> {
        // 1. Lecture de la réservation et de l'intention d'envoi (sans verrou).
        let (moi, envoi, rsv) = {
            let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
            let a_moi: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM ent_espace_reservation WHERE etablissement_id = $1 AND id = $2 AND demandeur_user_id = $3",
            )
            .bind(etab)
            .bind(reservation_id)
            .bind(moi.user)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if a_moi.is_none() {
                return Err(introuvable());
            }
            let rsv = reservation_ouverte(&mut tx, etab, reservation_id).await?.ok_or_else(|| conflit("Cet envoi a expiré ou a déjà été finalisé."))?;
            let l = sqlx::query(
                "SELECT espace_id, dossier_id, element_id, nom, mime_declare, commentaire
                   FROM ent_espace_envoi WHERE etablissement_id = $1 AND reservation_id = $2",
            )
            .bind(etab)
            .bind(reservation_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            (
                moi,
                (
                    l.get::<Uuid, _>(0),
                    l.get::<Option<Uuid>, _>(1),
                    l.get::<Option<Uuid>, _>(2),
                    l.get::<String, _>(3),
                    l.get::<Option<String>, _>(4),
                    l.get::<Option<String>, _>(5),
                ),
                rsv,
            )
        };
        let (espace_id, dossier_id, element_cible, nom, mime_declare, commentaire) = envoi;
        // Une réservation échue n'est plus honorée : l'URL signée est expirée, l'objet éventuel est effacé.
        if maintenant >= rsv.expire_le {
            self.rejeter_depot(etab, reservation_id, &rsv.cle_objet, espace_id, moi.personne, "expiree", "depot.expire", maintenant).await?;
            return Err(conflit("Cet envoi a expiré : recommencez."));
        }
        // 2. Taille réelle chez le fournisseur (sans lire le contenu) : comparée à la taille signée.
        let cle = rsv.cle_objet.clone();
        let tete = {
            let c = cle.clone();
            hors_runtime(self.objects(), move |s| s.tete(&c)).await?
        };
        let Some(tete) = tete else {
            return Err(conflit("Le fichier n'a pas été reçu : renvoyez-le."));
        };
        if tete.taille != rsv.octets as u64 {
            tracing::error!(attendu = rsv.octets, recu = tete.taille, "espaces : taille reçue différente de la taille signée (envoi rejeté)");
            self.rejeter_depot(etab, reservation_id, &cle, espace_id, moi.personne, "taille_differente", "depot.taille_differente", maintenant).await?;
            return Err(conflit("La taille reçue ne correspond pas à celle annoncée : envoi rejeté."));
        }
        // 3. Contenu : empreinte calculée par le serveur ; forme (signature, liste blanche, macros) sans antivirus
        //    (l'analyse est un second temps, fermé par défaut).
        let octets = self.objet_get(&cle).await?;
        if octets.len() as u64 != tete.taille {
            self.rejeter_depot(etab, reservation_id, &cle, espace_id, moi.personne, "taille_differente", "depot.taille_differente", maintenant).await?;
            return Err(conflit("La taille reçue ne correspond pas à celle annoncée : envoi rejeté."));
        }
        let sha256 = ent_object::sha256_hex(&octets);
        let forme = PolitiqueFichiers::nouvelle(TaillesMax { espace: self.politique_fichiers().taille_max(UsageFichier::Espace), ..TaillesMax::default() });
        let valide = match forme.valider(UsageFichier::Espace, &nom, mime_declare.as_deref(), &octets) {
            Ok(v) => v,
            Err(e) => {
                self.rejeter_depot(etab, reservation_id, &cle, espace_id, moi.personne, "refusee", "depot.refuse", maintenant).await?;
                return Err(EntDbError::Fichier(e));
            }
        };
        // 4. Écriture : registre d'objets, quota, élément et version, dans une seule transaction.
        let info = ObjectInfo { key: cle.clone(), taille: tete.taille, sha256 };
        let resultat = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let r = self
                .creer_version_deposee(&mut tx, etab, moi, espace_id, dossier_id, element_cible, &valide.nom, valide.mime, commentaire.as_deref(), reservation_id, &info, maintenant)
                .await;
            match r {
                Ok(res) => {
                    tx.commit().await.map_err(db)?;
                    Ok(res)
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    Err(e)
                }
            }
        };
        match resultat {
            Ok(r) => Ok(r),
            Err(e) => {
                // Échec métier (nom pris entre-temps, droit retiré, quota…) : l'envoi est rejeté proprement.
                self.rejeter_depot(etab, reservation_id, &cle, espace_id, moi.personne, "refusee", "depot.refuse", maintenant).await?;
                Err(e)
            }
        }
    }

    /// Rejette un envoi : réservation libérée, objet effacé, ligne de journal (code seulement).
    async fn rejeter_depot(
        &self,
        etab: Uuid,
        reservation: Uuid,
        cle: &str,
        espace: Uuid,
        personne: Uuid,
        motif: &str,
        action: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        quota_liberer(&mut tx, etab, reservation, motif, maintenant).await?;
        journaliser(&mut tx, etab, Some(espace), None, Some(personne), action, None, maintenant).await?;
        tx.commit().await.map_err(db)?;
        self.effacer_objet_sans_registre(cle).await;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn creer_version_deposee(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        moi: Moi,
        espace_id: Uuid,
        dossier_id: Option<Uuid>,
        element_cible: Option<Uuid>,
        nom_final: &str,
        mime: &str,
        commentaire: Option<&str>,
        reservation: Uuid,
        info: &ObjectInfo,
        maintenant: DateTime<Utc>,
    ) -> Result<ResultatDepot, EntDbError> {
        let jour = jour_a(conn, etab, maintenant).await?;
        // Droits relus au moment de l'écriture (un rôle retiré entre-temps n'écrit plus).
        let esp = ligne_espace(conn, etab, espace_id, false).await?.ok_or_else(introuvable)?;
        exiger_ouvert(&esp)?;
        let role = role_dans_espace(conn, etab, &esp, moi.personne, jour).await?;
        // Quota : la réservation devient une occupation confirmée (taille réelle = taille signée, vérifiée).
        quota_confirmer(conn, etab, reservation, espace_id, info.taille as i64, maintenant).await?;
        enregistrer_objet(&mut *conn, etab, info, Some(mime)).await?;
        let objet_id: Uuid = sqlx::query_scalar("SELECT id FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2")
            .bind(etab)
            .bind(&info.key)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
        let (element_id, numero) = if let Some(cible) = element_cible {
            let el = element_ligne(conn, etab, cible, true).await?.ok_or_else(introuvable)?;
            if el.espace_id != espace_id || !matches!(el.etat.as_str(), "analyse" | "disponible" | "masque") {
                return Err(conflit("Ce fichier n'est plus disponible."));
            }
            let partage = partage_sur(conn, etab, Some(el.id), el.dossier_id, moi.personne, jour).await?;
            if !(Acces { role, partage }).ecrit() {
                return Err(interdit("Vous ne pouvez plus déposer de version de ce fichier."));
            }
            let n: i32 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(numero), 0) + 1 FROM ent_espace_version WHERE etablissement_id = $1 AND element_id = $2",
            )
            .bind(etab)
            .bind(cible)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
            (cible, n)
        } else {
            if let Some(dos) = dossier_id {
                let dl = dossier_ligne(conn, etab, dos, true).await?.ok_or_else(introuvable)?;
                if dl.supprime || dl.espace_id != espace_id {
                    return Err(introuvable());
                }
            }
            let partage = match dossier_id {
                Some(d) => partage_sur(conn, etab, None, Some(d), moi.personne, jour).await?,
                None => None,
            };
            if !(Acces { role, partage }).ecrit() {
                return Err(interdit("Vous ne pouvez plus déposer ici."));
            }
            let noms = noms_elements(conn, etab, espace_id, dossier_id).await?;
            let normalise = nom_normalise(nom_final);
            if noms.contains_key(&normalise) {
                return Err(conflit("Un fichier de ce nom existe déjà dans ce dossier."));
            }
            if noms.len() as i64 >= ELEMENTS_PAR_DOSSIER_MAX {
                return Err(conflit("Ce dossier contient déjà le nombre maximal de fichiers."));
            }
            let id = Uuid::new_v4();
            let r = sqlx::query(
                "INSERT INTO ent_espace_element
                    (id, etablissement_id, espace_id, dossier_id, nature, nom, nom_normalise, auteur_personne_id, etat, cree_le, modifie_le)
                 VALUES ($1, $2, $3, $4, 'fichier', $5, $6, $7, 'analyse', $8, $8)",
            )
            .bind(id)
            .bind(etab)
            .bind(espace_id)
            .bind(dossier_id)
            .bind(nom_final)
            .bind(&normalise)
            .bind(moi.personne)
            .bind(maintenant)
            .execute(&mut *conn)
            .await;
            if let Err(e) = r {
                return Err(if est_unicite(&e) { conflit("Un fichier de ce nom existe déjà dans ce dossier.") } else { db(e) });
            }
            (id, 1)
        };
        let version_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_espace_version
                (id, etablissement_id, element_id, numero, objet_id, auteur_personne_id, commentaire, type_reel, taille, origine, etat, cree_le)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'depot', 'analyse', $10)",
        )
        .bind(version_id)
        .bind(etab)
        .bind(element_id)
        .bind(numero)
        .bind(objet_id)
        .bind(moi.personne)
        .bind(commentaire)
        .bind(mime)
        .bind(info.taille as i64)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE ent_espace_envoi SET version_id = $3 WHERE etablissement_id = $1 AND reservation_id = $2")
            .bind(etab)
            .bind(reservation)
            .bind(version_id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE ent_espace_element SET modifie_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(element_id)
            .bind(maintenant)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        toucher_activite(conn, etab, espace_id, maintenant).await?;
        journaliser(conn, etab, Some(espace_id), Some(element_id), Some(moi.personne), "depot.recu", Some(if numero == 1 { "premiere" } else { "version" }), maintenant).await?;
        Ok(ResultatDepot { element_id, version_id, numero, etat: "analyse".into() })
    }

    // ───────────────────────── Téléchargement, versions ─────────────────────────

    /// Lien de téléchargement court (ETR-118, ETR-164) : jeton de 60 s lié au compte, émis **après** contrôle des droits.
    /// Une version non analysée ou en quarantaine n'est jamais téléchargeable. Journalisé pour un espace avec élèves.
    pub async fn espaces_lien_telechargement(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Uuid,
        numero: Option<i32>,
        maintenant: DateTime<Utc>,
    ) -> Result<LienTelechargement, EntDbError> {
        self.lien_fichier(etab, user, role, element_id, numero, maintenant, Disposition::Attachment).await
    }

    /// **Aperçu minimal** (ET1) : même lien court, servi `inline` pour les seuls types sûrs (PDF, images, texte) ; tout autre type
    /// reste forcé en téléchargement par le noyau (`effective_disposition`). L'aperçu riche (bureautique, pages, vignettes) est le
    /// lot ET4 (moteur de conversion isolé, lot ET5a).
    pub async fn espaces_lien_apercu(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Uuid,
        numero: Option<i32>,
        maintenant: DateTime<Utc>,
    ) -> Result<LienTelechargement, EntDbError> {
        self.lien_fichier(etab, user, role, element_id, numero, maintenant, Disposition::Inline).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn lien_fichier(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Uuid,
        numero: Option<i32>,
        maintenant: DateTime<Utc>,
        disposition: Disposition,
    ) -> Result<LienTelechargement, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (el, esp, _) = acces_element(&mut tx, etab, element_id, moi, jour, false, false).await?;
        let v = sqlx::query(
            "SELECT o.cle_objet, v.type_reel FROM ent_espace_version v
               JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
              WHERE v.etablissement_id = $1 AND v.element_id = $2 AND v.etat = 'disponible' AND v.elaguee_le IS NULL
                AND o.supprime_le IS NULL
                AND ((($3::int IS NULL) AND v.id = $4) OR v.numero = $3)",
        )
        .bind(etab)
        .bind(element_id)
        .bind(numero)
        .bind(el.version_courante)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let (cle, mime): (String, String) = (v.get(0), v.get(1));
        let lien = self
            .presigner()
            .issue(
                &LinkSpec {
                    key: cle,
                    user_id: user,
                    etablissement_id: etab,
                    disposition,
                    filename: el.nom.clone(),
                    mime,
                },
                maintenant.timestamp(),
            )
            .map_err(|_| conflit("Le téléchargement est momentanément indisponible."))?;
        if espace_avec_eleves(&esp.type_espace) {
            journaliser(&mut tx, etab, Some(esp.id), Some(el.id), Some(moi.personne), "element.telecharge", None, maintenant).await?;
        }
        tx.commit().await.map_err(db)?;
        let _ = now_unix;
        Ok(LienTelechargement {
            url: lien.url,
            expire_le: DateTime::from_timestamp(lien.expires_at, 0).unwrap_or(maintenant + Duration::seconds(60)),
            nom: el.nom,
        })
    }

    /// Versions d'un élément (ETR-160) : auteur, date, commentaire, taille. Un non-auteur non-animateur ne voit que les
    /// versions disponibles.
    pub async fn espaces_versions(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<VersionVue>, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (el, _, acces) = acces_element(&mut tx, etab, element_id, moi, jour, false, true).await?;
        let voit_tout = acces.anime() || el.auteur == moi.personne;
        let l = sqlx::query(
            "SELECT v.id, v.numero, btrim(COALESCE(p.prenom || ' ', '') || p.nom), v.cree_le, v.commentaire, v.taille,
                    v.type_reel, v.origine, v.etat, v.restaure_de_numero
               FROM ent_espace_version v
               JOIN ent_personne p ON p.etablissement_id = v.etablissement_id AND p.id = v.auteur_personne_id
              WHERE v.etablissement_id = $1 AND v.element_id = $2 AND v.elaguee_le IS NULL
                AND ($3 OR v.etat = 'disponible')
              ORDER BY v.numero DESC",
        )
        .bind(etab)
        .bind(element_id)
        .bind(voit_tout)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l.iter()
            .map(|r| VersionVue {
                id: r.get(0),
                numero: r.get(1),
                auteur_nom: r.get(2),
                cree_le: r.get(3),
                commentaire: r.get(4),
                taille: r.get(5),
                type_reel: r.get(6),
                origine: r.get(7),
                etat: r.get(8),
                courante: Some(r.get::<Uuid, _>(0)) == el.version_courante,
                restaure_de_numero: r.get(9),
            })
            .collect())
    }

    /// Revient à une version antérieure (ETR-162) : **crée** une version (copie de l'objet, quota réservé et confirmé),
    /// l'ancienne reste. Auteur ou animateur.
    pub async fn espaces_version_restaurer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Uuid,
        numero: i32,
        maintenant: DateTime<Utc>,
    ) -> Result<ResultatDepot, EntDbError> {
        // 1. Droits, version source, réservation du quota (transaction courte).
        let (moi, espace_id, cle_source, mime, taille, cle_nouvelle, reservation) = {
            let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
            let jour = jour_a(&mut tx, etab, maintenant).await?;
            let (el, e, acces) = acces_element(&mut tx, etab, element_id, moi, jour, false, false).await?;
            exiger_ouvert(&e)?;
            if !(acces.anime() || (el.auteur == moi.personne && acces.role.is_some_and(|r| r.peut_contribuer()))) {
                return Err(interdit("Seul l'animateur ou l'auteur peut revenir à une version antérieure."));
            }
            let s = sqlx::query(
                "SELECT o.cle_objet, v.type_reel, v.taille FROM ent_espace_version v
                   JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
                  WHERE v.etablissement_id = $1 AND v.element_id = $2 AND v.numero = $3 AND v.etat = 'disponible'
                    AND v.elaguee_le IS NULL AND o.supprime_le IS NULL",
            )
            .bind(etab)
            .bind(element_id)
            .bind(numero)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
            let cle_nouvelle = ObjectStore::key_for(etab, "espaces", Uuid::new_v4(), "bin");
            let (reservation, _) = quota_reserver(&mut tx, etab, el.espace_id, s.get::<i64, _>(2), &cle_nouvelle, user, maintenant).await?;
            tx.commit().await.map_err(db)?;
            (moi, el.espace_id, s.get::<String, _>(0), s.get::<String, _>(1), s.get::<i64, _>(2), cle_nouvelle, reservation)
        };
        // 2. Copie de l'objet (contrôlée contre le registre), sous une clé neuve.
        let copie: Result<ObjectInfo, EntDbError> = async {
            let octets = self.lire_objet(etab, &cle_source).await?;
            self.objet_put(&cle_nouvelle, &octets).await
        }
        .await;
        let info = match copie {
            Ok(i) => i,
            Err(e) => {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                quota_liberer(&mut tx, etab, reservation, "refusee", maintenant).await?;
                tx.commit().await.map_err(db)?;
                self.effacer_objet_sans_registre(&cle_nouvelle).await;
                return Err(e);
            }
        };
        // 3. Écriture (contenu déjà analysé : la version naît disponible).
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Result<ResultatDepot, EntDbError> = async {
            quota_confirmer(&mut tx, etab, reservation, espace_id, taille, maintenant).await?;
            enregistrer_objet(&mut *tx, etab, &info, Some(&mime)).await?;
            let objet_id: Uuid = sqlx::query_scalar("SELECT id FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2")
                .bind(etab)
                .bind(&info.key)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            let el = element_ligne(&mut tx, etab, element_id, true).await?.ok_or_else(introuvable)?;
            if el.etat != "disponible" {
                return Err(conflit("Ce fichier n'est plus disponible."));
            }
            let n: i32 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(numero), 0) + 1 FROM ent_espace_version WHERE etablissement_id = $1 AND element_id = $2",
            )
            .bind(etab)
            .bind(element_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let vid = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_espace_version
                    (id, etablissement_id, element_id, numero, objet_id, auteur_personne_id, type_reel, taille, origine,
                     restaure_de_numero, etat, cree_le)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'restauration', $9, 'disponible', $10)",
            )
            .bind(vid)
            .bind(etab)
            .bind(element_id)
            .bind(n)
            .bind(objet_id)
            .bind(moi.personne)
            .bind(&mime)
            .bind(taille)
            .bind(numero)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_espace_element SET version_courante_id = $3, modifie_le = $4 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(element_id)
            .bind(vid)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            toucher_activite(&mut tx, etab, espace_id, maintenant).await?;
            journaliser(&mut tx, etab, Some(espace_id), Some(element_id), Some(moi.personne), "version.restauree", None, maintenant).await?;
            publier_evenement(&mut tx, etab, "espaces.version_creee", vid, Some(user), json!({ "espace_id": espace_id, "element_id": element_id, "numero": n })).await?;
            Ok(ResultatDepot { element_id, version_id: vid, numero: n, etat: "disponible".into() })
        }
        .await;
        match r {
            Ok(res) => {
                tx.commit().await.map_err(db)?;
                Ok(res)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                let mut t2 = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                quota_liberer(&mut t2, etab, reservation, "refusee", maintenant).await?;
                t2.commit().await.map_err(db)?;
                self.effacer_objet_sans_registre(&cle_nouvelle).await;
                Err(e)
            }
        }
    }

    // ───────────────────────── Corbeille ─────────────────────────

    /// Met un élément à la corbeille (ETR-122, NX32-08) : l'auteur ou l'animateur. 30 jours puis effacement.
    pub async fn espaces_element_corbeille(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        element_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (el, e, acces) = acces_element(&mut tx, etab, element_id, moi, jour, true, false).await?;
        exiger_ouvert(&e)?;
        if !(acces.anime() || (el.auteur == moi.personne && acces.role.is_some())) {
            return Err(interdit("Seul l'auteur ou l'animateur peut supprimer ce fichier."));
        }
        mettre_en_corbeille(&mut tx, etab, &[el.id], moi.personne, Uuid::new_v4(), maintenant).await?;
        toucher_activite(&mut tx, etab, el.espace_id, maintenant).await?;
        journaliser(&mut tx, etab, Some(el.espace_id), Some(el.id), Some(moi.personne), "element.corbeille", None, maintenant).await?;
        publier_evenement(&mut tx, etab, "espaces.element_supprime", el.id, Some(user), json!({ "espace_id": el.espace_id, "element_id": el.id })).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Met un dossier entier à la corbeille (ETR-123, NX32-09) : **aperçu d'abord** (nombre d'éléments, éléments des autres
    /// listés), puis confirmation. L'animateur, ou l'auteur du dossier si tout ce qu'il contient est de lui (ETR-274 / R-ET40 :
    /// en ET1 il n'y a pas de productions d'élèves ; la garde « éléments d'autrui » est déjà là).
    pub async fn espaces_dossier_corbeille(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        dossier_id: Uuid,
        confirmer: bool,
        maintenant: DateTime<Utc>,
    ) -> Result<ApercuCorbeilleDossier, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (d, e, acces) = acces_dossier(&mut tx, etab, dossier_id, moi, jour, true).await?;
        exiger_ouvert(&e)?;
        let est_auteur = d.auteur == moi.personne && acces.role.is_some();
        if !(acces.anime() || est_auteur) {
            return Err(interdit("Seul l'animateur ou l'auteur du dossier peut le supprimer."));
        }
        let dossiers = sous_arbre(&mut tx, etab, dossier_id).await?;
        let elements = sqlx::query(
            "SELECT id, nom, auteur_personne_id FROM ent_espace_element
              WHERE etablissement_id = $1 AND dossier_id = ANY($2) AND etat IN ('analyse', 'disponible', 'quarantaine', 'masque')
              ORDER BY lower(nom)",
        )
        .bind(etab)
        .bind(&dossiers)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let d_autrui: Vec<String> = elements.iter().filter(|r| r.get::<Uuid, _>(2) != moi.personne).map(|r| r.get::<String, _>(1)).collect();
        let apercu = ApercuCorbeilleDossier {
            nb_dossiers: dossiers.len() as i64,
            nb_elements: elements.len() as i64,
            elements_d_autrui: d_autrui.clone(),
            fait: false,
        };
        if !confirmer {
            tx.rollback().await.map_err(db)?;
            return Ok(apercu);
        }
        if !acces.anime() && !d_autrui.is_empty() {
            return Err(interdit("Ce dossier contient des fichiers d'autres personnes : seul un animateur peut le supprimer."));
        }
        let lot = Uuid::new_v4();
        let ids: Vec<Uuid> = elements.iter().map(|r| r.get::<Uuid, _>(0)).collect();
        mettre_en_corbeille(&mut tx, etab, &ids, moi.personne, lot, maintenant).await?;
        sqlx::query(
            "UPDATE ent_espace_dossier SET supprime_le = $3, supprime_par_personne_id = $4, corbeille_lot_id = $5, modifie_le = $3
              WHERE etablissement_id = $1 AND id = ANY($2) AND supprime_le IS NULL",
        )
        .bind(etab)
        .bind(&dossiers)
        .bind(maintenant)
        .bind(moi.personne)
        .bind(lot)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        toucher_activite(&mut tx, etab, d.espace_id, maintenant).await?;
        journaliser(&mut tx, etab, Some(d.espace_id), None, Some(moi.personne), "dossier.corbeille", None, maintenant).await?;
        publier_evenement(&mut tx, etab, "espaces.element_supprime", dossier_id, Some(user), json!({ "espace_id": d.espace_id, "dossier_id": dossier_id, "nb_elements": ids.len() })).await?;
        tx.commit().await.map_err(db)?;
        Ok(ApercuCorbeilleDossier { fait: true, ..apercu })
    }

    /// Contenu de la corbeille d'un espace : l'animateur voit tout, les autres ce qu'ils ont supprimé.
    pub async fn espaces_corbeille_lister(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<LigneCorbeille>, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (_, r) = exiger_membre(&mut tx, etab, espace_id, moi.personne, jour, false).await?;
        let anime = r.est_animateur();
        let mut out = Vec::new();
        let els = sqlx::query(
            "SELECT el.id, el.nom, el.corbeille_le, btrim(COALESCE(p.prenom || ' ', '') || p.nom),
                    EXISTS (SELECT 1 FROM ent_espace_version_figee f JOIN ent_espace_version v
                              ON v.etablissement_id = f.etablissement_id AND v.id = f.version_id
                             WHERE v.etablissement_id = el.etablissement_id AND v.element_id = el.id
                               AND (f.jusqu_au IS NULL OR f.jusqu_au >= $4))
               FROM ent_espace_element el
               JOIN ent_personne p ON p.etablissement_id = el.etablissement_id AND p.id = el.auteur_personne_id
              WHERE el.etablissement_id = $1 AND el.espace_id = $2 AND el.etat = 'corbeille'
                AND ($3 OR el.auteur_personne_id = $5)
              ORDER BY el.corbeille_le DESC",
        )
        .bind(etab)
        .bind(espace_id)
        .bind(anime)
        .bind(jour)
        .bind(moi.personne)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for l in &els {
            let le: DateTime<Utc> = l.get(2);
            out.push(LigneCorbeille {
                genre: "element".into(),
                id: l.get(0),
                nom: l.get(1),
                supprime_le: le,
                auteur_nom: l.get(3),
                purge_le: le + Duration::days(CORBEILLE_JOURS),
                figee: l.get(4),
            });
        }
        let dos = sqlx::query(
            "SELECT d.id, d.nom, d.supprime_le, btrim(COALESCE(p.prenom || ' ', '') || p.nom)
               FROM ent_espace_dossier d
               JOIN ent_personne p ON p.etablissement_id = d.etablissement_id AND p.id = d.cree_par_personne_id
              WHERE d.etablissement_id = $1 AND d.espace_id = $2 AND d.supprime_le IS NOT NULL
                AND ($3 OR d.cree_par_personne_id = $4)
                AND NOT EXISTS (SELECT 1 FROM ent_espace_dossier p2
                                 WHERE p2.etablissement_id = d.etablissement_id AND p2.id = d.parent_id
                                   AND p2.corbeille_lot_id = d.corbeille_lot_id)
              ORDER BY d.supprime_le DESC",
        )
        .bind(etab)
        .bind(espace_id)
        .bind(anime)
        .bind(moi.personne)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for l in &dos {
            let le: DateTime<Utc> = l.get(2);
            out.push(LigneCorbeille {
                genre: "dossier".into(),
                id: l.get(0),
                nom: l.get(1),
                supprime_le: le,
                auteur_nom: l.get(3),
                purge_le: le + Duration::days(CORBEILLE_JOURS),
                figee: false,
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Restaure un élément ou un dossier de la corbeille (ETR-124) : auteur ou animateur ; un nom devenu pris reçoit
    /// « (2) » ; un dossier parent supprimé entre-temps renvoie à la racine de l'espace.
    pub async fn espaces_restaurer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        genre: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        match genre {
            "element" => {
                let (el, e, acces) = acces_element(&mut tx, etab, id, moi, jour, true, true).await?;
                exiger_ouvert(&e)?;
                if el.etat != "corbeille" {
                    return Err(conflit("Ce fichier n'est pas dans la corbeille."));
                }
                if !(acces.anime() || el.auteur == moi.personne) {
                    return Err(introuvable());
                }
                // Supprimé avec son dossier : on restaure le dossier (le contenu suit).
                if let Some(dos) = el.dossier_id {
                    let dl = dossier_ligne(&mut tx, etab, dos, false).await?;
                    if dl.as_ref().is_some_and(|d| d.supprime && d.corbeille_lot == el.corbeille_lot) {
                        return Err(conflit("Ce fichier a été supprimé avec son dossier : restaurez le dossier."));
                    }
                }
                restaurer_elements(&mut tx, etab, &[el.id], maintenant).await?;
                journaliser(&mut tx, etab, Some(el.espace_id), Some(el.id), Some(moi.personne), "element.restaure", None, maintenant).await?;
                publier_evenement(&mut tx, etab, "espaces.element_restaure", el.id, Some(user), json!({ "espace_id": el.espace_id, "element_id": el.id })).await?;
                toucher_activite(&mut tx, etab, el.espace_id, maintenant).await?;
            }
            "dossier" => {
                let d = dossier_ligne(&mut tx, etab, id, true).await?.ok_or_else(introuvable)?;
                let esp = ligne_espace(&mut tx, etab, d.espace_id, false).await?.ok_or_else(introuvable)?;
                let role_e = role_dans_espace(&mut tx, etab, &esp, moi.personne, jour).await?.ok_or_else(introuvable)?;
                exiger_ouvert(&esp)?;
                if !d.supprime {
                    return Err(conflit("Ce dossier n'est pas dans la corbeille."));
                }
                if !(role_e.est_animateur() || d.auteur == moi.personne) {
                    return Err(introuvable());
                }
                let lot = d.corbeille_lot.ok_or_else(|| conflit("Ce dossier n'est pas dans la corbeille."))?;
                restaurer_dossiers_du_lot(&mut tx, etab, &d, lot, maintenant).await?;
                let ids: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM ent_espace_element WHERE etablissement_id = $1 AND corbeille_lot_id = $2 AND etat = 'corbeille'",
                )
                .bind(etab)
                .bind(lot)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                restaurer_elements(&mut tx, etab, &ids, maintenant).await?;
                journaliser(&mut tx, etab, Some(d.espace_id), None, Some(moi.personne), "dossier.restaure", None, maintenant).await?;
                toucher_activite(&mut tx, etab, d.espace_id, maintenant).await?;
            }
            _ => return Err(invalide("Genre inconnu.")),
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Vide la corbeille (ETR-125) : effacement définitif anticipé, par l'animateur (ou le propriétaire de son espace
    /// personnel). Les éléments dont une version est figée par un autre socle restent (R-ET6). Renvoie `(effacés, gardés)`.
    pub async fn espaces_vider_corbeille(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(usize, usize), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (e, r) = exiger_membre(&mut tx, etab, espace_id, moi.personne, jour, false).await?;
        exiger_role(r, RoleEspace::Animateur)?;
        exiger_ouvert(&e)?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_espace_element WHERE etablissement_id = $1 AND espace_id = $2 AND etat = 'corbeille'",
        )
        .bind(etab)
        .bind(espace_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut cles = Vec::new();
        let (mut effaces, mut gardes) = (0usize, 0usize);
        for id in ids {
            match purger_element(&mut tx, etab, id, jour, maintenant).await? {
                Some(k) => {
                    cles.extend(k);
                    effaces += 1;
                    journaliser(&mut tx, etab, Some(espace_id), Some(id), Some(moi.personne), "element.efface", Some("anticipe"), maintenant).await?;
                    publier_evenement(&mut tx, etab, "espaces.element_efface", id, Some(user), json!({ "espace_id": espace_id, "element_id": id })).await?;
                }
                None => gardes += 1,
            }
        }
        tx.commit().await.map_err(db)?;
        for c in cles {
            self.effacer_objet_sans_registre(&c).await;
        }
        Ok((effaces, gardes))
    }
}

// ───────────────────────── Corbeille et purge (interne) ─────────────────────────

pub(crate) async fn mettre_en_corbeille(
    conn: &mut PgConnection,
    etab: Uuid,
    elements: &[Uuid],
    par: Uuid,
    lot: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_espace_element
            SET etat_avant_corbeille = etat, etat = 'corbeille', corbeille_le = $3, corbeille_par_personne_id = $4,
                corbeille_lot_id = $5, modifie_le = $3
          WHERE etablissement_id = $1 AND id = ANY($2) AND etat IN ('analyse', 'disponible', 'quarantaine', 'masque')",
    )
    .bind(etab)
    .bind(elements)
    .bind(maintenant)
    .bind(par)
    .bind(lot)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn restaurer_dossiers_du_lot(
    conn: &mut PgConnection,
    etab: Uuid,
    racine: &DossierLigne,
    lot: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    // Le dossier racine du lot retrouve son parent s'il est vivant, sinon la racine de l'espace.
    let parent_vivant: bool = match racine.parent_id {
        Some(p) => sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_espace_dossier WHERE etablissement_id = $1 AND id = $2 AND supprime_le IS NULL)",
        )
        .bind(etab)
        .bind(p)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?,
        None => true,
    };
    let parent = if parent_vivant { racine.parent_id } else { None };
    let mut nom = racine.nom.clone();
    let pris = noms_dossiers(conn, etab, racine.espace_id, parent).await?;
    if pris.contains(&nom_normalise(&nom)) {
        nom = nom_propose(&nom, &pris);
    }
    sqlx::query(
        "UPDATE ent_espace_dossier SET parent_id = $3, nom = $4, nom_normalise = $5, supprime_le = NULL,
                supprime_par_personne_id = NULL, corbeille_lot_id = NULL, modifie_le = $6
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(racine.id)
    .bind(parent)
    .bind(&nom)
    .bind(nom_normalise(&nom))
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_espace_dossier SET supprime_le = NULL, supprime_par_personne_id = NULL, corbeille_lot_id = NULL, modifie_le = $4
          WHERE etablissement_id = $1 AND corbeille_lot_id = $2 AND id <> $3",
    )
    .bind(etab)
    .bind(lot)
    .bind(racine.id)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Restaure des éléments : état d'avant, nom libre (sinon « (2) »), dossier vivant (sinon la racine).
async fn restaurer_elements(conn: &mut PgConnection, etab: Uuid, ids: &[Uuid], maintenant: DateTime<Utc>) -> Result<(), EntDbError> {
    for id in ids {
        let Some(el) = element_ligne(conn, etab, *id, true).await? else { continue };
        if el.etat != "corbeille" {
            continue;
        }
        let dossier_vivant = match el.dossier_id {
            Some(d) => dossier_ligne(conn, etab, d, false).await?.is_some_and(|dl| !dl.supprime),
            None => true,
        };
        let dossier = if dossier_vivant { el.dossier_id } else { None };
        let noms = noms_elements(conn, etab, el.espace_id, dossier).await?;
        let mut nom = el.nom.clone();
        if noms.contains_key(&nom_normalise(&nom)) {
            let pris: BTreeSet<String> = noms.keys().cloned().collect();
            nom = nom_propose(&nom, &pris);
        }
        sqlx::query(
            "UPDATE ent_espace_element
                SET etat = COALESCE(etat_avant_corbeille, 'disponible'), etat_avant_corbeille = NULL, corbeille_le = NULL,
                    corbeille_par_personne_id = NULL, corbeille_lot_id = NULL, dossier_id = $3, nom = $4, nom_normalise = $5,
                    modifie_le = $6
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'corbeille'",
        )
        .bind(etab)
        .bind(id)
        .bind(dossier)
        .bind(&nom)
        .bind(nom_normalise(&nom))
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

/// Efface définitivement un élément de la corbeille : objets marqués supprimés au registre, quota rendu, état `efface`.
/// Renvoie les clés d'objet à effacer **physiquement après validation** ; `None` si une version est figée par un autre
/// socle (R-ET6) : l'élément reste.
pub(crate) async fn purger_element(
    conn: &mut PgConnection,
    etab: Uuid,
    element: Uuid,
    jour: NaiveDate,
    maintenant: DateTime<Utc>,
) -> Result<Option<Vec<String>>, EntDbError> {
    let el = element_ligne(conn, etab, element, true).await?.ok_or_else(introuvable)?;
    if el.etat != "corbeille" {
        return Ok(None);
    }
    let figee: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_espace_version_figee f
                          JOIN ent_espace_version v ON v.etablissement_id = f.etablissement_id AND v.id = f.version_id
                         WHERE v.etablissement_id = $1 AND v.element_id = $2 AND (f.jusqu_au IS NULL OR f.jusqu_au >= $3))",
    )
    .bind(etab)
    .bind(element)
    .bind(jour)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if figee {
        return Ok(None);
    }
    let versions = sqlx::query(
        "SELECT o.id, o.cle_objet, v.taille FROM ent_espace_version v
           JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
          WHERE v.etablissement_id = $1 AND v.element_id = $2 AND o.supprime_le IS NULL",
    )
    .bind(etab)
    .bind(element)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut cles = Vec::new();
    let mut rendus = 0i64;
    for v in &versions {
        sqlx::query("UPDATE ent_objet SET supprime_le = $3 WHERE etablissement_id = $1 AND id = $2 AND supprime_le IS NULL")
            .bind(etab)
            .bind(v.get::<Uuid, _>(0))
            .bind(maintenant)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        cles.push(v.get::<String, _>(1));
        rendus += v.get::<i64, _>(2);
    }
    sqlx::query("UPDATE ent_espace_element SET etat = 'efface', modifie_le = $3 WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(element)
        .bind(maintenant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    if rendus > 0 {
        quota_rendre(conn, etab, el.espace_id, rendus, maintenant).await?;
    }
    Ok(Some(cles))
}
