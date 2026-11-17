//! Socle 32, lot ET1 — espaces, membres, invitations, quota, journal
//! (`32_BUREAUTIQUE.md` §8, §15, §17.1, §17.4, §17.7 ; ETR-106, 107, 110, 111, 150, 151, 152, 153, 154, 155, 260).
//!
//! * **Droit vérifié dans chaque commande** (D-017, ND32-04) : `espaces.utiliser` (puis le rôle d'espace, relu en base),
//!   `espaces.creer`, `espaces.partager`. Un espace dont on n'est pas membre répond **« introuvable »**, jamais « interdit »
//!   (R-ET2 : lire = être membre effectif à la date, ou destinataire d'un partage actif) ; un membre qui n'a pas le rôle
//!   requis reçoit un refus explicite.
//! * **Membres résolus par le cœur** (R-ET12, ND32-03) : un membre est une `personne` d'un **agent actif** de l'établissement
//!   (annuaire du socle 6, à la date) ; un identifiant arbitraire, d'un autre établissement, d'un élève ou d'un parent est
//!   refusé avec le même message.
//! * **Rien n'est écrit par une lecture** (ND32-11) : le badge « invitation non vue » se lève par une commande explicite.
//! * **Avis dans la transaction** (ND32-12) : l'invitation est une notification du socle 8 écrite dans la transaction du fait,
//!   en enveloppe neutre (jamais le nom du fichier ni de l'espace). Pas d'avis pour le créateur.
//! * Aucun lien public (R-ET9), aucun partage vers un élève, aucun élève dans un espace de personnels (R-ET12) : en ET1 les
//!   espaces `personnel` et `libre_personnels` n'ont que des personnels.
//! * Tout l'accès passe par [`TenantTx`] ; l'instant est un paramètre (règle 33 : pas d'heure réelle dans la logique).

#![allow(clippy::type_complexity)]

use crate::espaces::*;
use crate::espaces_quota::occupation_assurer;
use crate::notifications::{db, notifier_personne, EnvoiAPlanifier};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

// ───────────────────────── Erreurs ─────────────────────────

pub(crate) fn introuvable() -> EntDbError {
    EntDbError::Domain(domain_eleve::DomainError::NotFound)
}

pub(crate) fn invalide(m: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Invalide, m)
}

pub(crate) fn conflit(m: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Conflit, m)
}

pub(crate) fn interdit(m: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Interdit, m)
}

/// Violation d'unicité (SQLSTATE 23505).
pub(crate) fn est_unicite(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23505"))
}

// ───────────────────────── Acteur, lignes, rôles ─────────────────────────

/// Le compte qui agit et la personne qu'il représente dans l'établissement.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Moi {
    pub user: Uuid,
    pub personne: Uuid,
}

/// Ligne d'un espace (hors `efface`).
#[derive(Debug, Clone)]
pub(crate) struct EspaceLigne {
    pub id: Uuid,
    pub type_espace: String,
    pub etat: String,
    pub libelle: String,
    pub description: Option<String>,
    pub couleur_code: String,
    pub icone_code: String,
    pub quota_octets: i64,
    pub derniere_activite_le: DateTime<Utc>,
}

const COLONNES_ESPACE: &str =
    "e.id, e.type, e.etat, e.libelle, e.description, e.couleur_code, e.icone_code, e.quota_octets, e.derniere_activite_le";

fn lire_espace_ligne(r: &sqlx::postgres::PgRow) -> EspaceLigne {
    EspaceLigne {
        id: r.get(0),
        type_espace: r.get(1),
        etat: r.get(2),
        libelle: r.get(3),
        description: r.get(4),
        couleur_code: r.get(5),
        icone_code: r.get(6),
        quota_octets: r.get(7),
        derniere_activite_le: r.get(8),
    }
}

/// La personne (fiche de l'établissement) du compte. Un compte sans fiche dans cet établissement n'a pas d'espace.
pub(crate) async fn personne_du_compte(conn: &mut PgConnection, etab: Uuid, user: Uuid) -> Result<Uuid, EntDbError> {
    let p: Option<Uuid> = sqlx::query_scalar(
        "SELECT p.id FROM ent_personne p
          WHERE p.etablissement_id = $1 AND p.user_id = $2 AND p.fusionnee_dans IS NULL",
    )
    .bind(etab)
    .bind(user)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    p.ok_or_else(|| interdit("Votre compte n'a pas de fiche de personnel dans cet établissement : contactez l'administrateur."))
}

/// Jour de l'établissement à l'instant donné (fuseau de l'établissement, jamais l'UTC).
pub(crate) async fn jour_a(conn: &mut PgConnection, etab: Uuid, maintenant: DateTime<Utc>) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(etab)
        .bind(maintenant)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)
}

pub(crate) async fn ligne_espace(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Uuid,
    verrou: bool,
) -> Result<Option<EspaceLigne>, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES_ESPACE} FROM ent_espace e
          WHERE e.etablissement_id = $1 AND e.id = $2 AND e.etat <> 'efface'{}",
        if verrou { " FOR UPDATE OF e" } else { "" }
    );
    let r = sqlx::query(&sql).bind(etab).bind(espace).fetch_optional(&mut *conn).await.map_err(db)?;
    Ok(r.as_ref().map(lire_espace_ligne))
}

/// Mon rôle dans l'espace **à la date** : propriétaire d'un espace personnel (animateur), ou membre ajouté actif.
pub(crate) async fn role_dans_espace(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: &EspaceLigne,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<Option<RoleEspace>, EntDbError> {
    if espace.type_espace == "personnel" {
        let proprio: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM ent_espace_ancrage
              WHERE etablissement_id = $1 AND espace_id = $2 AND type = 'personne' AND personne_id = $3",
        )
        .bind(etab)
        .bind(espace.id)
        .bind(personne)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        return Ok(proprio.map(|_| RoleEspace::Animateur));
    }
    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT role FROM ent_espace_membre_effectif
          WHERE etablissement_id = $1 AND espace_id = $2 AND personne_id = $3
            AND debut <= $4 AND (fin IS NULL OR fin >= $4)",
    )
    .bind(etab)
    .bind(espace.id)
    .bind(personne)
    .bind(jour)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(roles.iter().filter_map(|r| RoleEspace::depuis_code(r)).max())
}

/// L'espace et mon rôle, ou « introuvable » (un non-membre ne sait pas que l'espace existe).
pub(crate) async fn exiger_membre(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Uuid,
    personne: Uuid,
    jour: NaiveDate,
    verrou: bool,
) -> Result<(EspaceLigne, RoleEspace), EntDbError> {
    let e = ligne_espace(conn, etab, espace, verrou).await?.ok_or_else(introuvable)?;
    let r = role_dans_espace(conn, etab, &e, personne, jour).await?.ok_or_else(introuvable)?;
    Ok((e, r))
}

pub(crate) fn exiger_role(role: RoleEspace, min: RoleEspace) -> Result<(), EntDbError> {
    if role >= min {
        Ok(())
    } else {
        Err(interdit("Votre rôle dans cet espace ne permet pas cette action."))
    }
}

/// Un espace archivé est en lecture seule (lot ET2) ; en ET1 tous les espaces sont ouverts.
pub(crate) fn exiger_ouvert(e: &EspaceLigne) -> Result<(), EntDbError> {
    if e.etat == "ouvert" {
        Ok(())
    } else {
        Err(conflit("Cet espace est en lecture seule."))
    }
}

/// L'annuaire du socle 6, à la date : la personne est-elle un **agent actif** de l'établissement ?
/// (R-ET12, ND32-03). Même refus quelle que soit la raison (inexistante, élève, parent, autre établissement, partie).
pub(crate) async fn exiger_collegue_actif(
    conn: &mut PgConnection,
    etab: Uuid,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<(), EntDbError> {
    let ok: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM ent_agent a
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND a.personne_id = $2
            AND a.fusionne_dans_agent_id IS NULL AND p.fusionnee_dans IS NULL
            AND ent_agent_etat(a.etablissement_id, a.id, $3) = 'actif'",
    )
    .bind(etab)
    .bind(personne)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    ok.map(|_| ()).ok_or_else(|| invalide("Cette personne n'est pas un personnel actif de l'établissement."))
}

// ───────────────────────── Journal, événements, avis ─────────────────────────

/// Une ligne du journal en ajout seul : codes courts, jamais un nom de fichier ni un texte libre.
pub(crate) async fn journaliser(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Option<Uuid>,
    element: Option<Uuid>,
    acteur: Option<Uuid>,
    action: &str,
    detail: Option<&str>,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_espace_journal
            (id, etablissement_id, espace_id, element_id, acteur_personne_id, action_code, detail_code, le)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(espace)
    .bind(element)
    .bind(acteur)
    .bind(action)
    .bind(detail)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

pub(crate) async fn toucher_activite(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_espace SET derniere_activite_le = $3, modifie_le = $3
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(espace)
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Événement d'outbox, dans la transaction du fait. Charge utile : identifiants seulement (E14).
pub(crate) async fn publier_evenement(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate_id: Uuid,
    acteur_user: Option<Uuid>,
    payload: serde_json::Value,
) -> Result<Uuid, EntDbError> {
    Ok(conn
        .publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: type_name.to_string(),
            aggregate: AggregateKind::Espace,
            aggregate_id,
            eleve_id: None,
            payload,
            actor_user_id: acteur_user.map(|u| u.to_string()),
        })
        .await?
        .id)
}

/// Avis neutre à une personne (notification du socle 8 écrite dans la transaction du fait). Une personne sans compte
/// n'est pas prévenue : le badge « non vu » la rattrape à sa première connexion.
pub(crate) async fn avertir(
    conn: &mut PgConnection,
    etab: Uuid,
    destinataire: Uuid,
    kind: &'static str,
    titre: &'static str,
    corps: &'static str,
    source_id: Uuid,
    evenement: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let user: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM ent_personne WHERE etablissement_id = $1 AND id = $2 AND user_id IS NOT NULL",
    )
    .bind(etab)
    .bind(destinataire)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some(user_id) = user else { return Ok(()) };
    notifier_personne(
        conn,
        EnvoiAPlanifier {
            etablissement_id: etab,
            user_id,
            kind,
            titre,
            corps,
            eleve_id: None,
            notification_id: None,
            source_type: "systeme",
            source_id,
            cle_dedup: format!("{kind}:{source_id}:{destinataire}"),
            droit_requis: None,
            regroupable: true,
            canaux_voulus: &["email", "push"],
        },
        evenement,
        maintenant,
    )
    .await?;
    Ok(())
}

/// Avis d'invitation (nature `espaces.invitation`, ETR-152) : enveloppe neutre, jamais le nom de l'espace ni du fichier.
pub(crate) async fn inviter(
    conn: &mut PgConnection,
    etab: Uuid,
    destinataire: Uuid,
    source_id: Uuid,
    evenement: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    avertir(
        conn,
        etab,
        destinataire,
        "espaces.invitation",
        "Un espace ou un document est partagé avec vous",
        "Ouvrez « Mes espaces » dans l'ENT pour le consulter.",
        source_id,
        evenement,
        maintenant,
    )
    .await
}

fn description_propre(d: Option<String>) -> Result<Option<String>, EntDbError> {
    match d.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) if s.chars().count() > 500 => Err(invalide("La description est trop longue.")),
        Some(s) if s.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\t')) => {
            Err(invalide("La description contient un caractère interdit."))
        }
        Some(s) => Ok(Some(s)),
    }
}

fn choix_catalogue(v: Option<&str>, catalogue: &[&str], defaut: &'static str, msg: &'static str) -> Result<String, EntDbError> {
    match v {
        None => Ok(defaut.to_string()),
        Some(x) if catalogue.contains(&x) => Ok(x.to_string()),
        Some(_) => Err(invalide(msg)),
    }
}

async fn quota_du_profil(conn: &mut PgConnection, etab: Uuid, type_espace: &str) -> Result<i64, EntDbError> {
    let reglage: Option<i64> = sqlx::query_scalar(
        "SELECT octets FROM ent_espace_quota_profil WHERE etablissement_id = $1 AND profil = $2",
    )
    .bind(etab)
    .bind(profil_quota(type_espace))
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(reglage.unwrap_or_else(|| quota_defaut(type_espace)))
}

async fn resume(
    conn: &mut PgConnection,
    etab: Uuid,
    e: &EspaceLigne,
    role: RoleEspace,
    personne: Uuid,
    jour: NaiveDate,
) -> Result<EspaceResume, EntDbError> {
    let r = sqlx::query(
        "SELECT COALESCE((SELECT octets_confirmes FROM ent_espace_occupation WHERE etablissement_id = $1 AND espace_id = $2), 0),
                COALESCE((SELECT octets_reserves FROM ent_espace_occupation WHERE etablissement_id = $1 AND espace_id = $2), 0),
                (SELECT count(*) FROM ent_espace_membre_ajoute m
                  WHERE m.etablissement_id = $1 AND m.espace_id = $2 AND m.retire_le IS NULL
                    AND m.debut <= $4 AND (m.fin IS NULL OR m.fin >= $4)),
                EXISTS (SELECT 1 FROM ent_espace_membre_ajoute m
                         WHERE m.etablissement_id = $1 AND m.espace_id = $2 AND m.personne_id = $3
                           AND m.retire_le IS NULL AND m.vu_le IS NULL AND m.ajoute_par_personne_id <> $3)",
    )
    .bind(etab)
    .bind(e.id)
    .bind(personne)
    .bind(jour)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let nb: i64 = r.get(2);
    Ok(EspaceResume {
        id: e.id,
        type_espace: e.type_espace.clone(),
        libelle: e.libelle.clone(),
        description: e.description.clone(),
        couleur_code: e.couleur_code.clone(),
        icone_code: e.icone_code.clone(),
        role,
        // L'espace personnel n'a qu'un occupant : son propriétaire.
        nb_membres: if e.type_espace == "personnel" { 1 } else { nb },
        quota_octets: e.quota_octets,
        octets_utilises: r.get(0),
        octets_reserves: r.get(1),
        derniere_activite_le: e.derniere_activite_le,
        invitation_non_vue: r.get(3),
    })
}

// ───────────────────────── Service ─────────────────────────

impl PgEnt {
    /// Début d'une commande : `espaces.utiliser` (D-017) vérifié **avant** d'ouvrir la transaction (le moteur de droits utilise
    /// le pool : le faire connexion prise ferait attendre un second accès au pool, donc un interblocage sous charge), puis la
    /// transaction locataire et la personne du compte.
    pub(crate) async fn espaces_debut(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<(TenantTx<'_>, Moi), EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::EspacesUtiliser).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let personne = personne_du_compte(&mut tx, etab, user).await?;
        Ok((tx, Moi { user, personne }))
    }

    /// Mes espaces (ETR-110, ETR-111) : l'espace personnel s'il existe, puis les espaces libres où je suis membre,
    /// du plus récemment actif au plus ancien. **Aucune écriture** : l'espace personnel s'ouvre par une commande.
    pub async fn espaces_lister(&self, etab: Uuid, user: Uuid, role: &str, maintenant: DateTime<Utc>) -> Result<Vec<EspaceResume>, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let lignes = sqlx::query(&format!(
            "SELECT {COLONNES_ESPACE}
               FROM ent_espace e
              WHERE e.etablissement_id = $1 AND e.etat <> 'efface'
                AND ((e.type = 'personnel' AND EXISTS (
                         SELECT 1 FROM ent_espace_ancrage a
                          WHERE a.etablissement_id = e.etablissement_id AND a.espace_id = e.id
                            AND a.type = 'personne' AND a.personne_id = $2))
                  OR (e.type <> 'personnel' AND EXISTS (
                         SELECT 1 FROM ent_espace_membre_effectif m
                          WHERE m.etablissement_id = e.etablissement_id AND m.espace_id = e.id AND m.personne_id = $2
                            AND m.debut <= $3 AND (m.fin IS NULL OR m.fin >= $3))))
              ORDER BY (e.type = 'personnel') DESC, e.derniere_activite_le DESC, e.libelle"
        ))
        .bind(etab)
        .bind(moi.personne)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for l in &lignes {
            let e = lire_espace_ligne(l);
            if let Some(r) = role_dans_espace(&mut tx, etab, &e, moi.personne, jour).await? {
                out.push(resume(&mut tx, etab, &e, r, moi.personne, jour).await?);
            }
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Ouvre mon espace personnel (idempotent) : une seule fois par personne (index unique d'ancrage).
    pub async fn espaces_ouvrir_mon_espace(&self, etab: Uuid, user: Uuid, role: &str, maintenant: DateTime<Utc>) -> Result<EspaceResume, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let existant: Option<Uuid> = sqlx::query_scalar(
            "SELECT espace_id FROM ent_espace_ancrage
              WHERE etablissement_id = $1 AND type = 'personne' AND personne_id = $2",
        )
        .bind(etab)
        .bind(moi.personne)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let id = if let Some(id) = existant {
            id
        } else {
            let quota = quota_du_profil(&mut tx, etab, "personnel").await?;
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_espace
                    (id, etablissement_id, type, libelle, couleur_code, icone_code, etat, quota_octets,
                     cree_par_personne_id, derniere_activite_le, cree_le, modifie_le)
                 VALUES ($1, $2, 'personnel', 'Mon espace', 'bleu', 'dossier', 'ouvert', $3, $4, $5, $5, $5)",
            )
            .bind(id)
            .bind(etab)
            .bind(quota)
            .bind(moi.personne)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO ent_espace_ancrage (espace_id, etablissement_id, type, personne_id, cree_le)
                 VALUES ($1, $2, 'personne', $3, $4)",
            )
            .bind(id)
            .bind(etab)
            .bind(moi.personne)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            occupation_assurer(&mut tx, etab, id).await?;
            journaliser(&mut tx, etab, Some(id), None, Some(moi.personne), "espace.cree", Some("personnel"), maintenant).await?;
            publier_evenement(&mut tx, etab, "espaces.cree", id, Some(user), json!({ "espace_id": id, "type": "personnel" })).await?;
            id
        };
        let e = ligne_espace(&mut tx, etab, id, false).await?.ok_or_else(introuvable)?;
        let r = resume(&mut tx, etab, &e, RoleEspace::Animateur, moi.personne, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Crée un espace libre entre personnels en un geste (ETR-106, ETR-150 ; reprise de NX32-10) : sans élève, sans
    /// ancrage ; le créateur est animateur. Les collègues sont choisis dans l'annuaire et résolus par le cœur.
    pub async fn espaces_creer_libre(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        n: NouvelEspace,
        maintenant: DateTime<Utc>,
    ) -> Result<EspaceResume, EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::EspacesCreer).await?;
        let libelle = nom_propre(&n.libelle, NOM_MAX).map_err(invalide)?;
        let description = description_propre(n.description)?;
        let couleur = choix_catalogue(n.couleur_code.as_deref(), COULEURS, "bleu", "Couleur inconnue.")?;
        let icone = choix_catalogue(n.icone_code.as_deref(), ICONES, "dossier", "Icône inconnue.")?;
        if n.membres.len() as i64 >= MEMBRES_AJOUTES_MAX {
            return Err(invalide("Trop de membres pour un espace."));
        }
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let crees: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_espace
              WHERE etablissement_id = $1 AND type = 'libre_personnels' AND cree_par_personne_id = $2 AND etat <> 'efface'",
        )
        .bind(etab)
        .bind(moi.personne)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if crees >= ESPACES_LIBRES_PAR_CREATEUR_MAX {
            return Err(conflit("Vous avez atteint le nombre maximal d'espaces libres : supprimez-en un ou transférez-le."));
        }
        // Membres : agents actifs, distincts, différents du créateur.
        let mut vus = std::collections::BTreeSet::new();
        for m in &n.membres {
            if m.personne_id == moi.personne || !vus.insert(m.personne_id) {
                return Err(invalide("Un collègue ne peut être ajouté qu'une fois, et vous êtes déjà membre."));
            }
            exiger_collegue_actif(&mut tx, etab, m.personne_id, jour).await?;
        }
        let quota = quota_du_profil(&mut tx, etab, "libre_personnels").await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_espace
                (id, etablissement_id, type, libelle, description, couleur_code, icone_code, etat, quota_octets,
                 cree_par_personne_id, derniere_activite_le, cree_le, modifie_le)
             VALUES ($1, $2, 'libre_personnels', $3, $4, $5, $6, 'ouvert', $7, $8, $9, $9, $9)",
        )
        .bind(id)
        .bind(etab)
        .bind(&libelle)
        .bind(&description)
        .bind(&couleur)
        .bind(&icone)
        .bind(quota)
        .bind(moi.personne)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        occupation_assurer(&mut tx, etab, id).await?;
        // Le créateur est animateur ; il connaît son espace (déjà « vu »).
        sqlx::query(
            "INSERT INTO ent_espace_membre_ajoute
                (id, etablissement_id, espace_id, personne_id, role, debut, motif_code, ajoute_par_personne_id, vu_le, cree_le)
             VALUES ($1, $2, $3, $4, 'animateur', $5, 'createur', $4, $6, $6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(moi.personne)
        .bind(jour)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, Some(id), None, Some(moi.personne), "espace.cree", Some("libre_personnels"), maintenant).await?;
        publier_evenement(&mut tx, etab, "espaces.cree", id, Some(user), json!({ "espace_id": id, "type": "libre_personnels" })).await?;
        for m in &n.membres {
            ajouter_membre_interne(&mut tx, etab, id, moi, m, jour, maintenant).await?;
        }
        let e = ligne_espace(&mut tx, etab, id, false).await?.ok_or_else(introuvable)?;
        let r = resume(&mut tx, etab, &e, RoleEspace::Animateur, moi.personne, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Renomme, décrit, colore un espace (ETR-107) : animateur seulement ; jamais l'espace personnel (son nom est fixe).
    pub async fn espaces_modifier(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        m: ModifEspace,
        maintenant: DateTime<Utc>,
    ) -> Result<EspaceResume, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (e, r) = exiger_membre(&mut tx, etab, espace_id, moi.personne, jour, true).await?;
        exiger_role(r, RoleEspace::Animateur)?;
        exiger_ouvert(&e)?;
        if e.type_espace == "personnel" {
            return Err(conflit("Votre espace personnel ne se renomme pas."));
        }
        let libelle = match m.libelle {
            Some(l) => nom_propre(&l, NOM_MAX).map_err(invalide)?,
            None => e.libelle.clone(),
        };
        let description = if m.description.is_some() { description_propre(m.description)? } else { e.description.clone() };
        let couleur = choix_catalogue(m.couleur_code.as_deref().or(Some(e.couleur_code.as_str())), COULEURS, "bleu", "Couleur inconnue.")?;
        let icone = choix_catalogue(m.icone_code.as_deref().or(Some(e.icone_code.as_str())), ICONES, "dossier", "Icône inconnue.")?;
        sqlx::query(
            "UPDATE ent_espace SET libelle = $3, description = $4, couleur_code = $5, icone_code = $6, modifie_le = $7
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(espace_id)
        .bind(&libelle)
        .bind(&description)
        .bind(&couleur)
        .bind(&icone)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, Some(espace_id), None, Some(moi.personne), "espace.modifie", None, maintenant).await?;
        let e2 = ligne_espace(&mut tx, etab, espace_id, false).await?.ok_or_else(introuvable)?;
        let res = resume(&mut tx, etab, &e2, r, moi.personne, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(res)
    }

    /// Détail d'un espace et de ses membres (ETR-110) : réservé aux membres (un non-membre : introuvable).
    pub async fn espaces_detail(&self, etab: Uuid, user: Uuid, role: &str, espace_id: Uuid, maintenant: DateTime<Utc>) -> Result<EspaceDetail, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (e, r) = exiger_membre(&mut tx, etab, espace_id, moi.personne, jour, false).await?;
        let espace = resume(&mut tx, etab, &e, r, moi.personne, jour).await?;
        let membres = if e.type_espace == "personnel" {
            Vec::new()
        } else {
            let l = sqlx::query(
                "SELECT m.personne_id, btrim(COALESCE(p.prenom || ' ', '') || p.nom), m.role, m.debut
                   FROM ent_espace_membre_effectif m
                   JOIN ent_personne p ON p.etablissement_id = m.etablissement_id AND p.id = m.personne_id
                  WHERE m.etablissement_id = $1 AND m.espace_id = $2 AND m.debut <= $3 AND (m.fin IS NULL OR m.fin >= $3)
                  ORDER BY (m.role = 'animateur') DESC, lower(p.nom), lower(COALESCE(p.prenom, ''))",
            )
            .bind(etab)
            .bind(espace_id)
            .bind(jour)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            l.iter()
                .filter_map(|r| {
                    RoleEspace::depuis_code(&r.get::<String, _>(2)).map(|role| MembreVue {
                        personne_id: r.get(0),
                        nom_affiche: r.get(1),
                        role,
                        depuis: r.get(3),
                    })
                })
                .collect()
        };
        tx.commit().await.map_err(db)?;
        Ok(EspaceDetail { espace, membres })
    }

    /// Annuaire pour choisir des collègues (ETR-151) : agents actifs, hors moi, par nom ou fonction.
    pub async fn espaces_chercher_collegues(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        q: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<Collegue>, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let motif = {
            let q = q.trim();
            if q.chars().count() < 2 {
                return Err(invalide("Saisissez au moins deux caractères."));
            }
            let echappe: String = q.chars().flat_map(|c| if matches!(c, '%' | '_' | '\\') { vec!['\\', c] } else { vec![c] }).collect();
            format!("%{echappe}%")
        };
        let l = sqlx::query(
            "SELECT p.id, btrim(COALESCE(p.prenom || ' ', '') || p.nom), f.libelle
               FROM ent_agent a
               JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
               LEFT JOIN LATERAL (
                    SELECT fo.libelle FROM ent_agent_fonction af JOIN ent_fonction fo ON fo.id = af.fonction_id
                     WHERE af.etablissement_id = a.etablissement_id AND af.agent_id = a.id AND af.principale
                       AND af.annule_le IS NULL AND af.du <= $3 AND (af.au IS NULL OR af.au >= $3) LIMIT 1) f ON true
              WHERE a.etablissement_id = $1 AND a.fusionne_dans_agent_id IS NULL AND p.fusionnee_dans IS NULL
                AND p.id <> $2 AND ent_agent_etat(a.etablissement_id, a.id, $3) = 'actif'
                AND (p.nom ILIKE $4 OR COALESCE(p.prenom, '') ILIKE $4 OR (COALESCE(p.prenom, '') || ' ' || p.nom) ILIKE $4
                     OR COALESCE(f.libelle, '') ILIKE $4)
              ORDER BY lower(p.nom), lower(COALESCE(p.prenom, ''))
              LIMIT 20",
        )
        .bind(etab)
        .bind(moi.personne)
        .bind(jour)
        .bind(motif)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l.iter().map(|r| Collegue { personne_id: r.get(0), nom_affiche: r.get(1), fonction: r.get(2) }).collect())
    }

    /// Ajoute un membre nominatif à un espace libre (ETR-153) : animateur ; agent actif de l'établissement ; avis neutre.
    pub async fn espaces_membre_ajouter(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        m: NouveauMembre,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (e, r) = exiger_membre(&mut tx, etab, espace_id, moi.personne, jour, true).await?;
        exiger_role(r, RoleEspace::Animateur)?;
        exiger_ouvert(&e)?;
        if e.type_espace != "libre_personnels" {
            return Err(conflit("Les membres de cet espace ne se gèrent pas à la main."));
        }
        exiger_collegue_actif(&mut tx, etab, m.personne_id, jour).await?;
        ajouter_membre_interne(&mut tx, etab, espace_id, moi, &m, jour, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Retire un membre (ETR-153) : animateur ; le dernier animateur ne part pas sans transfert (ETR-155).
    pub async fn espaces_membre_retirer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        personne_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (e, r) = exiger_membre(&mut tx, etab, espace_id, moi.personne, jour, true).await?;
        // Se retirer soi-même passe par « quitter » (ETR-155) ; retirer un autre exige d'animer.
        if personne_id != moi.personne {
            exiger_role(r, RoleEspace::Animateur)?;
        }
        retirer_membre_interne(&mut tx, etab, &e, personne_id, moi.personne, if personne_id == moi.personne { "quitte" } else { "retire" }, jour, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Quitter un espace libre (ETR-155).
    pub async fn espaces_quitter(&self, etab: Uuid, user: Uuid, role: &str, espace_id: Uuid, maintenant: DateTime<Utc>) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (e, _) = exiger_membre(&mut tx, etab, espace_id, moi.personne, jour, true).await?;
        retirer_membre_interne(&mut tx, etab, &e, moi.personne, moi.personne, "quitte", jour, maintenant).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Transfère l'animation d'un espace libre (ETR-154) : la personne choisie devient animatrice (agent actif), je
    /// deviens contributeur. Il reste toujours au moins un animateur.
    pub async fn espaces_transferer_animation(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        vers_personne: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let (e, r) = exiger_membre(&mut tx, etab, espace_id, moi.personne, jour, true).await?;
        exiger_role(r, RoleEspace::Animateur)?;
        exiger_ouvert(&e)?;
        if e.type_espace != "libre_personnels" {
            return Err(conflit("L'animation de cet espace ne se transfère pas à la main."));
        }
        if vers_personne == moi.personne {
            return Err(invalide("Choisissez une autre personne."));
        }
        exiger_collegue_actif(&mut tx, etab, vers_personne, jour).await?;
        // La personne devient animatrice : ajoutée si elle n'est pas membre, promue sinon (remplacement de la ligne).
        let actuel = role_dans_espace(&mut tx, etab, &e, vers_personne, jour).await?;
        if actuel.is_some() {
            changer_role_interne(&mut tx, etab, espace_id, vers_personne, RoleEspace::Animateur, moi.personne, jour, maintenant).await?;
        } else {
            let nv = NouveauMembre { personne_id: vers_personne, role: RoleEspace::Animateur };
            ajouter_membre_interne(&mut tx, etab, espace_id, moi, &nv, jour, maintenant).await?;
        }
        changer_role_interne(&mut tx, etab, espace_id, moi.personne, RoleEspace::Contributeur, moi.personne, jour, maintenant).await?;
        journaliser(&mut tx, etab, Some(espace_id), None, Some(moi.personne), "espace.animation_transferee", None, maintenant).await?;
        publier_evenement(&mut tx, etab, "espaces.animation_transferee", espace_id, Some(user), json!({ "espace_id": espace_id })).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Nombre d'invitations non vues (badge, ETR-152) : membre ajouté par un autre, ou partage reçu. **Lecture seule.**
    pub async fn espaces_invitations_non_vues(&self, etab: Uuid, user: Uuid, role: &str, maintenant: DateTime<Utc>) -> Result<InvitationsNonVues, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let n: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM ent_espace_membre_ajoute m
                      WHERE m.etablissement_id = $1 AND m.personne_id = $2 AND m.retire_le IS NULL AND m.vu_le IS NULL
                        AND m.ajoute_par_personne_id <> $2 AND m.debut <= $3 AND (m.fin IS NULL OR m.fin >= $3))
                  + (SELECT count(*) FROM ent_espace_partage s
                      WHERE s.etablissement_id = $1 AND s.destinataire_personne_id = $2 AND s.retire_le IS NULL AND s.vu_le IS NULL
                        AND s.debut <= $3 AND (s.fin IS NULL OR s.fin >= $3))",
        )
        .bind(etab)
        .bind(moi.personne)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(InvitationsNonVues { nombre: n })
    }

    /// Lève le badge d'un espace ou d'un partage à son ouverture (commande explicite : une lecture n'écrit rien,
    /// ND32-11). Sans argument, lève tout.
    pub async fn espaces_marquer_vu(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Option<Uuid>,
        partage_id: Option<Uuid>,
        maintenant: DateTime<Utc>,
    ) -> Result<i64, EntDbError> {
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let a = sqlx::query(
            "UPDATE ent_espace_membre_ajoute SET vu_le = $3
              WHERE etablissement_id = $1 AND personne_id = $2 AND vu_le IS NULL AND retire_le IS NULL
                AND ($4::uuid IS NULL OR espace_id = $4)",
        )
        .bind(etab)
        .bind(moi.personne)
        .bind(maintenant)
        .bind(espace_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        let b = sqlx::query(
            "UPDATE ent_espace_partage SET vu_le = $3
              WHERE etablissement_id = $1 AND destinataire_personne_id = $2 AND vu_le IS NULL AND retire_le IS NULL
                AND ($4::uuid IS NULL OR id = $4)",
        )
        .bind(etab)
        .bind(moi.personne)
        .bind(maintenant)
        .bind(partage_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        tx.commit().await.map_err(db)?;
        Ok((a + b) as i64)
    }

    /// Mon quota et ce qui l'occupe (ETR-260, NX32-18) : par espace, par famille de format, versions et corbeille comprises.
    pub async fn espaces_quota(&self, etab: Uuid, user: Uuid, role: &str, maintenant: DateTime<Utc>) -> Result<Vec<QuotaEspace>, EntDbError> {
        let mes = self.espaces_lister(etab, user, role, maintenant).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut out = Vec::new();
        for e in mes {
            let l = sqlx::query(
                "SELECT v.type_reel, COALESCE(SUM(v.taille), 0)::bigint
                   FROM ent_espace_version v
                   JOIN ent_espace_element el ON el.etablissement_id = v.etablissement_id AND el.id = v.element_id
                   JOIN ent_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
                  WHERE v.etablissement_id = $1 AND el.espace_id = $2 AND o.supprime_le IS NULL AND v.elaguee_le IS NULL
                  GROUP BY v.type_reel",
            )
            .bind(etab)
            .bind(e.id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let mut familles: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
            for r in &l {
                *familles.entry(famille_de_type(&r.get::<String, _>(0)).to_string()).or_default() += r.get::<i64, _>(1);
            }
            out.push(QuotaEspace {
                espace_id: e.id,
                libelle: e.libelle,
                type_espace: e.type_espace,
                quota_octets: e.quota_octets,
                octets_utilises: e.octets_utilises,
                octets_reserves: e.octets_reserves,
                par_type: familles.into_iter().collect(),
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(out)
    }

    /// Journal d'un espace : l'animateur (son espace) ; `espaces.journal.lire` (direction) pour les espaces collectifs,
    /// jamais pour un espace personnel ni pour le contenu (§20.2).
    pub async fn espaces_journal(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        espace_id: Uuid,
        limite: i64,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<LigneJournal>, EntDbError> {
        // Droit de direction lu avant d'ouvrir la transaction (le moteur de droits passe par le pool).
        let droit_journal = self.a_action_etablissement(etab, user, role, Action::EspacesJournalLire).await?;
        let (mut tx, moi) = self.espaces_debut(etab, user, role).await?;
        let jour = jour_a(&mut tx, etab, maintenant).await?;
        let e = ligne_espace(&mut tx, etab, espace_id, false).await?.ok_or_else(introuvable)?;
        let mon_role = role_dans_espace(&mut tx, etab, &e, moi.personne, jour).await?;
        let autorise = mon_role == Some(RoleEspace::Animateur)
            || (e.type_espace != "personnel" && droit_journal);
        if !autorise {
            return Err(introuvable());
        }
        let l = sqlx::query(
            "SELECT j.le, CASE WHEN j.acteur_personne_id IS NULL THEN NULL ELSE btrim(COALESCE(p.prenom || ' ', '') || p.nom) END,
                    j.action_code, j.detail_code, j.element_id
               FROM ent_espace_journal j
               LEFT JOIN ent_personne p ON p.etablissement_id = j.etablissement_id AND p.id = j.acteur_personne_id
              WHERE j.etablissement_id = $1 AND j.espace_id = $2
              ORDER BY j.le DESC, j.id
              LIMIT $3",
        )
        .bind(etab)
        .bind(espace_id)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(l.iter()
            .map(|r| LigneJournal { le: r.get(0), acteur_nom: r.get(1), action_code: r.get(2), detail_code: r.get(3), element_id: r.get(4) })
            .collect())
    }
}

// ───────────────────────── Membres (interne) ─────────────────────────

async fn ajouter_membre_interne(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Uuid,
    moi: Moi,
    m: &NouveauMembre,
    jour: NaiveDate,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let actifs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_espace_membre_ajoute
          WHERE etablissement_id = $1 AND espace_id = $2 AND retire_le IS NULL AND debut <= $3 AND (fin IS NULL OR fin >= $3)",
    )
    .bind(etab)
    .bind(espace)
    .bind(jour)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if actifs >= MEMBRES_AJOUTES_MAX {
        return Err(conflit("Cet espace a atteint le nombre maximal de membres."));
    }
    let id = Uuid::new_v4();
    let r = sqlx::query(
        "INSERT INTO ent_espace_membre_ajoute
            (id, etablissement_id, espace_id, personne_id, role, debut, motif_code, ajoute_par_personne_id, cree_le)
         VALUES ($1, $2, $3, $4, $5, $6, 'invitation', $7, $8)",
    )
    .bind(id)
    .bind(etab)
    .bind(espace)
    .bind(m.personne_id)
    .bind(m.role.code())
    .bind(jour)
    .bind(moi.personne)
    .bind(maintenant)
    .execute(&mut *conn)
    .await;
    match r {
        Ok(_) => {}
        // Exclusion de chevauchement (23P01) ou unicité : déjà membre.
        Err(sqlx::Error::Database(d)) if matches!(d.code().as_deref(), Some("23P01") | Some("23505")) => {
            return Err(conflit("Cette personne est déjà membre de l'espace."));
        }
        Err(e) => return Err(db(e)),
    }
    toucher_activite(conn, etab, espace, maintenant).await?;
    journaliser(conn, etab, Some(espace), None, Some(moi.personne), "membre.ajoute", Some(m.role.code()), maintenant).await?;
    let evt = publier_evenement(
        conn,
        etab,
        "espaces.membre_ajoute",
        id,
        Some(moi.user),
        json!({ "espace_id": espace, "personne_id": m.personne_id, "role": m.role.code() }),
    )
    .await?;
    inviter(conn, etab, m.personne_id, id, evt, maintenant).await?;
    Ok(())
}

/// Change le rôle d'un membre ajouté (retire la ligne active, en pose une nouvelle) ; ne touche pas aux avis.
async fn changer_role_interne(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: Uuid,
    personne: Uuid,
    role: RoleEspace,
    par: Uuid,
    jour: NaiveDate,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let courant: Option<(Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, vu_le FROM ent_espace_membre_ajoute
          WHERE etablissement_id = $1 AND espace_id = $2 AND personne_id = $3 AND retire_le IS NULL
            AND debut <= $4 AND (fin IS NULL OR fin >= $4)",
    )
    .bind(etab)
    .bind(espace)
    .bind(personne)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((id, vu)) = courant else { return Err(introuvable()) };
    sqlx::query(
        "UPDATE ent_espace_membre_ajoute SET retire_le = $3, retire_par_personne_id = $4, retire_motif_code = 'role_change'
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .bind(maintenant)
    .bind(par)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_espace_membre_ajoute
            (id, etablissement_id, espace_id, personne_id, role, debut, motif_code, ajoute_par_personne_id, vu_le, cree_le)
         VALUES ($1, $2, $3, $4, $5, $6, 'role_change', $7, $8, $9)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(espace)
    .bind(personne)
    .bind(role.code())
    .bind(jour)
    .bind(par)
    .bind(vu.or(Some(maintenant)))
    .bind(maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn retirer_membre_interne(
    conn: &mut PgConnection,
    etab: Uuid,
    espace: &EspaceLigne,
    personne: Uuid,
    par: Uuid,
    motif: &str,
    jour: NaiveDate,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    if espace.type_espace == "personnel" {
        return Err(conflit("On ne quitte pas son espace personnel."));
    }
    let cible: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, role FROM ent_espace_membre_ajoute
          WHERE etablissement_id = $1 AND espace_id = $2 AND personne_id = $3 AND retire_le IS NULL
            AND debut <= $4 AND (fin IS NULL OR fin >= $4)",
    )
    .bind(etab)
    .bind(espace.id)
    .bind(personne)
    .bind(jour)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((id, role)) = cible else { return Err(introuvable()) };
    if role == "animateur" {
        let autres: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_espace_membre_ajoute
              WHERE etablissement_id = $1 AND espace_id = $2 AND role = 'animateur' AND retire_le IS NULL
                AND personne_id <> $3 AND debut <= $4 AND (fin IS NULL OR fin >= $4)",
        )
        .bind(etab)
        .bind(espace.id)
        .bind(personne)
        .bind(jour)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if autres == 0 {
            return Err(conflit("Le dernier animateur ne part pas sans transfert : désignez d'abord un autre animateur."));
        }
    }
    sqlx::query(
        "UPDATE ent_espace_membre_ajoute SET retire_le = $3, retire_par_personne_id = $4, retire_motif_code = $5
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .bind(maintenant)
    .bind(par)
    .bind(motif)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    // Les partages que cette personne avait reçus *à titre personnel* ne sont pas touchés : ils sont indépendants de l'espace.
    toucher_activite(conn, etab, espace.id, maintenant).await?;
    journaliser(conn, etab, Some(espace.id), None, Some(par), "membre.retire", Some(motif), maintenant).await?;
    publier_evenement(conn, etab, "espaces.membre_retire", id, None, json!({ "espace_id": espace.id, "personne_id": personne })).await?;
    Ok(())
}
