//! Moteur de droits paramétrables (D-017, `docs/ROLES_ET_DROITS.md`).
//!
//! Droits effectifs d'une personne dans un établissement = modèle par défaut de
//! son profil de base (`ent_membership.role`) ∪ droits des rôles qui lui sont
//! attribués (actifs à la date). Familles et élèves : jamais de rôle libre.
//! Lus à chaque requête : une modification s'applique immédiatement.

use std::collections::HashSet;

use chrono::NaiveDate;
use domain_eleve::{today_utc, DomainError};
use ent_auth::{modele_par_defaut, Action, Portee, Role};
use serde_json::json;
use uuid::Uuid;

use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("droits_repo", err))
}

/// Nombre maximal de droits par rôle et de cibles par droit (bornes).
const MAX_DROITS: usize = 64;
const MAX_CIBLES: usize = 200;

/// Un droit : action + portée (+ cibles pour la portée « désignés »).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Droit {
    pub action: Action,
    pub portee: Portee,
    pub cibles: Vec<Uuid>,
}

/// Rôle paramétré par l'établissement.
#[derive(Debug, Clone)]
pub struct RoleEtab {
    pub id: Uuid,
    pub nom: String,
    pub description: Option<String>,
    pub modele: Option<String>,
    pub droits: Vec<Droit>,
    pub membres: Vec<Uuid>,
}

/// Données d'un rôle à créer ou modifier.
#[derive(Debug, Clone)]
pub struct RoleSaisie {
    pub nom: String,
    pub description: Option<String>,
    pub modele: Option<String>,
    pub droits: Vec<Droit>,
}

impl PgEnt {
    /// Droits effectifs (modèle du profil de base + rôles attribués actifs).
    pub async fn droits_effectifs(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role_base: &str,
    ) -> Result<Vec<Droit>, EntDbError> {
        let Some(base) = Role::parse(role_base) else {
            return Ok(Vec::new());
        };
        let mut droits: Vec<Droit> = modele_par_defaut(base)
            .into_iter()
            .map(|(action, portee)| Droit {
                action,
                portee,
                cibles: Vec::new(),
            })
            .collect();
        if !base.is_staff() {
            return Ok(droits); // garde-fou 1 : familles et élèves hors rôles libres
        }
        let rows: Vec<(String, String, Vec<Uuid>)> = sqlx::query_as(
            "SELECT d.action, d.portee, d.cibles
               FROM ent_role_attribution a
               JOIN ent_role_droit d
                 ON d.role_id = a.role_id AND d.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.user_id = $2
                AND a.debut <= $3 AND (a.fin IS NULL OR a.fin >= $3)",
        )
        .bind(etablissement_id)
        .bind(user_id)
        .bind(today_utc())
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        for (a, p, cibles) in rows {
            // Codes inconnus (catalogue réduit depuis) : ignorés, jamais élargis.
            if let (Some(action), Some(portee)) = (Action::parse(&a), Portee::parse(&p)) {
                droits.push(Droit {
                    action,
                    portee,
                    cibles,
                });
            }
        }
        // S2-5 (ADM-04) : délégations datées en cours. Lues à chaque requête : la date de
        // fin suffit à les éteindre, sans tâche de fond.
        droits.extend(
            self.droits_delegues(etablissement_id, user_id, today_utc())
                .await?,
        );
        Ok(droits)
    }

    /// L'acteur a-t-il l'action sur l'établissement entier ?
    pub async fn a_action_etablissement(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role_base: &str,
        action: Action,
    ) -> Result<bool, EntDbError> {
        Ok(self
            .droits_effectifs(etablissement_id, user_id, role_base)
            .await?
            .iter()
            .any(|d| d.action == action && d.portee == Portee::Etablissement))
    }

    /// L'acteur peut-il exercer `action` sur cet élève ? (union des portées)
    pub async fn autoriser_eleve(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role_base: &str,
        action: Action,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let droits = self
            .droits_effectifs(etablissement_id, user_id, role_base)
            .await?;
        let jour = today_utc();
        let mut designes: Vec<Uuid> = Vec::new();
        let (mut classes, mut groupes) = (false, false);
        for d in droits.iter().filter(|d| d.action == action) {
            match d.portee {
                Portee::Etablissement => return Ok(true),
                Portee::MesClasses => classes = true,
                Portee::MesGroupes => groupes = true,
                Portee::Designes => designes.extend(d.cibles.iter().copied()),
            }
        }
        if classes
            && self
                .prof_suit_eleve_le(etablissement_id, user_id, eleve_id, jour)
                .await?
        {
            return Ok(true);
        }
        if groupes
            && self
                .eleve_dans_groupes_de(etablissement_id, user_id, eleve_id, jour)
                .await?
        {
            return Ok(true);
        }
        if !designes.is_empty()
            && self
                .eleve_dans_cibles(etablissement_id, &designes, eleve_id, jour)
                .await?
        {
            return Ok(true);
        }
        Ok(false)
    }

    /// Élèves sur lesquels l'acteur peut exercer `action` : `None` = tout
    /// l'établissement ; sinon l'ensemble (filtre des listes).
    pub async fn eleves_autorises(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role_base: &str,
        action: Action,
    ) -> Result<Option<HashSet<Uuid>>, EntDbError> {
        let droits = self
            .droits_effectifs(etablissement_id, user_id, role_base)
            .await?;
        let jour = today_utc();
        let mut ens: HashSet<Uuid> = HashSet::new();
        let mut designes: Vec<Uuid> = Vec::new();
        let (mut classes, mut groupes) = (false, false);
        for d in droits.iter().filter(|d| d.action == action) {
            match d.portee {
                Portee::Etablissement => return Ok(None),
                Portee::MesClasses => classes = true,
                Portee::MesGroupes => groupes = true,
                Portee::Designes => designes.extend(d.cibles.iter().copied()),
            }
        }
        if classes {
            ens.extend(self.eleves_suivis_par_prof(etablissement_id, user_id, jour).await?);
        }
        if groupes || !designes.is_empty() {
            let rows: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT a.eleve_id
                   FROM ent_affectation a
                  WHERE a.etablissement_id = $1
                    AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $4)
                    AND (($2 AND a.groupe_id IN (SELECT r.groupe_id FROM ent_groupe_responsable r
                                                  WHERE r.etablissement_id = $1 AND r.user_id = $3))
                         OR a.classe_id = ANY($5) OR a.groupe_id = ANY($5))",
            )
            .bind(etablissement_id)
            .bind(groupes)
            .bind(user_id)
            .bind(jour)
            .bind(&designes)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
            ens.extend(rows.into_iter().map(|(id,)| id));
        }
        Ok(Some(ens))
    }

    async fn eleve_dans_groupes_de(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
               SELECT 1 FROM ent_affectation a
                 JOIN ent_groupe_responsable r
                   ON r.groupe_id = a.groupe_id AND r.etablissement_id = a.etablissement_id
                WHERE a.etablissement_id = $1 AND r.user_id = $2 AND a.eleve_id = $3
                  AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $4))",
        )
        .bind(etablissement_id)
        .bind(user_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(ok)
    }

    async fn eleve_dans_cibles(
        &self,
        etablissement_id: Uuid,
        cibles: &[Uuid],
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
               SELECT 1 FROM ent_affectation a
                WHERE a.etablissement_id = $1 AND a.eleve_id = $3
                  AND (a.classe_id = ANY($2) OR a.groupe_id = ANY($2))
                  AND a.date_debut <= $4 AND (a.date_fin IS NULL OR a.date_fin >= $4))",
        )
        .bind(etablissement_id)
        .bind(cibles)
        .bind(eleve_id)
        .bind(jour)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(ok)
    }

    // ───────────────────────────── Administration ─────────────────────────────

    /// Rôles de l'établissement (droits et membres). Lecture : `roles.gerer`.
    pub async fn list_roles_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
    ) -> Result<Vec<RoleEtab>, EntDbError> {
        self.exiger_roles_gerer(etablissement_id, actor_user_id, actor_role)
            .await?;
        self.lire_roles(etablissement_id, None).await
    }

    async fn lire_roles(
        &self,
        etablissement_id: Uuid,
        un: Option<Uuid>,
    ) -> Result<Vec<RoleEtab>, EntDbError> {
        let roles: Vec<(Uuid, String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT id, nom, description, modele FROM ent_role
              WHERE etablissement_id = $1 AND ($2::uuid IS NULL OR id = $2)
              ORDER BY lower(nom)",
        )
        .bind(etablissement_id)
        .bind(un)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(roles.len());
        for (id, nom, description, modele) in roles {
            let droits: Vec<(String, String, Vec<Uuid>)> = sqlx::query_as(
                "SELECT action, portee, cibles FROM ent_role_droit
                  WHERE etablissement_id = $1 AND role_id = $2 ORDER BY action, portee",
            )
            .bind(etablissement_id)
            .bind(id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
            let membres: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT user_id FROM ent_role_attribution
                  WHERE etablissement_id = $1 AND role_id = $2 ORDER BY created_at",
            )
            .bind(etablissement_id)
            .bind(id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
            out.push(RoleEtab {
                id,
                nom,
                description,
                modele,
                droits: droits
                    .into_iter()
                    .filter_map(|(a, p, cibles)| {
                        Some(Droit {
                            action: Action::parse(&a)?,
                            portee: Portee::parse(&p)?,
                            cibles,
                        })
                    })
                    .collect(),
                membres: membres.into_iter().map(|(u,)| u).collect(),
            });
        }
        Ok(out)
    }

    /// Crée un rôle (garde-fous : `roles.gerer`, droits valides, pas d'escalade).
    pub async fn creer_role_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        saisie: RoleSaisie,
    ) -> Result<RoleEtab, EntDbError> {
        let saisie = self
            .valider_saisie(etablissement_id, actor_user_id, actor_role, saisie)
            .await?;
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_role (id, etablissement_id, nom, description, modele, created_by_user_id)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(&saisie.nom)
        .bind(&saisie.description)
        .bind(&saisie.modele)
        .bind(actor_user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        ecrire_droits(&mut tx, etablissement_id, id, &saisie.droits).await?;
        tx.commit().await.map_err(db)?;
        self.journal_role(etablissement_id, actor_user_id, actor_role, "role_cree", id, &saisie)
            .await;
        self.role_par_id(etablissement_id, id).await
    }

    /// Remplace nom, description et droits d'un rôle (mêmes garde-fous).
    pub async fn modifier_role_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        role_id: Uuid,
        saisie: RoleSaisie,
    ) -> Result<RoleEtab, EntDbError> {
        let saisie = self
            .valider_saisie(etablissement_id, actor_user_id, actor_role, saisie)
            .await?;
        // L'acteur doit aussi couvrir les droits actuels (on ne retire pas ce
        // qu'on ne pourrait pas donner : évite de neutraliser un rôle supérieur).
        let actuel = self.role_par_id(etablissement_id, role_id).await?;
        self.exiger_couverture(etablissement_id, actor_user_id, actor_role, &actuel.droits)
            .await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let res = sqlx::query(
            "UPDATE ent_role SET nom = $3, description = $4, updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(role_id)
        .bind(&saisie.nom)
        .bind(&saisie.description)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound.into());
        }
        sqlx::query("DELETE FROM ent_role_droit WHERE etablissement_id = $1 AND role_id = $2")
            .bind(etablissement_id)
            .bind(role_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        ecrire_droits(&mut tx, etablissement_id, role_id, &saisie.droits).await?;
        tx.commit().await.map_err(db)?;
        self.journal_role(etablissement_id, actor_user_id, actor_role, "role_modifie", role_id, &saisie)
            .await;
        self.role_par_id(etablissement_id, role_id).await
    }

    /// Supprime un rôle (et ses attributions). L'acteur doit couvrir ses droits.
    pub async fn supprimer_role_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        role_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_roles_gerer(etablissement_id, actor_user_id, actor_role)
            .await?;
        let actuel = self.role_par_id(etablissement_id, role_id).await?;
        self.exiger_couverture(etablissement_id, actor_user_id, actor_role, &actuel.droits)
            .await?;
        sqlx::query("DELETE FROM ent_role WHERE etablissement_id = $1 AND id = $2")
            .bind(etablissement_id)
            .bind(role_id)
            .execute(self.pool())
            .await
            .map_err(db)?;
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "role_supprime",
            "Suppression d'un rôle",
            json!({ "role_id": role_id.to_string(), "nom": actuel.nom }),
        )
        .await;
        Ok(())
    }

    /// Attribue un rôle à un **personnel** de l'établissement (jamais à une
    /// famille ou un élève), avec date de fin facultative.
    pub async fn attribuer_role_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        role_id: Uuid,
        user_id: Uuid,
        fin: Option<NaiveDate>,
    ) -> Result<(), EntDbError> {
        self.exiger_roles_gerer(etablissement_id, actor_user_id, actor_role)
            .await?;
        let role = self.role_par_id(etablissement_id, role_id).await?;
        self.exiger_couverture(etablissement_id, actor_user_id, actor_role, &role.droits)
            .await?;
        let base = self
            .auth_repo()
            .role_in(etablissement_id, user_id)
            .await?
            .and_then(|r| Role::parse(&r));
        match base {
            Some(r) if r.is_staff() => {}
            Some(_) => return Err(DomainError::ValeurInvalide.into()), // garde-fou 1
            None => return Err(DomainError::NotFound.into()),
        }
        if fin.is_some_and(|f| f < today_utc()) {
            return Err(DomainError::ValeurInvalide.into());
        }
        sqlx::query(
            "INSERT INTO ent_role_attribution
                 (role_id, etablissement_id, user_id, debut, fin, attribue_par_user_id)
             VALUES ($1, $2, $3, CURRENT_DATE, $4, $5)
             ON CONFLICT (role_id, user_id) DO UPDATE
                SET fin = EXCLUDED.fin, attribue_par_user_id = EXCLUDED.attribue_par_user_id",
        )
        .bind(role_id)
        .bind(etablissement_id)
        .bind(user_id)
        .bind(fin)
        .bind(actor_user_id)
        .execute(self.pool())
        .await
        .map_err(db)?;
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "role_attribue",
            "Attribution d'un rôle",
            json!({ "role_id": role_id.to_string(), "user_id": user_id.to_string(),
                    "fin": fin.map(|d| d.to_string()) }),
        )
        .await;
        Ok(())
    }

    /// Retire un rôle à une personne.
    pub async fn retirer_role_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        role_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, EntDbError> {
        self.exiger_roles_gerer(etablissement_id, actor_user_id, actor_role)
            .await?;
        let role = self.role_par_id(etablissement_id, role_id).await?;
        self.exiger_couverture(etablissement_id, actor_user_id, actor_role, &role.droits)
            .await?;
        let res = sqlx::query(
            "DELETE FROM ent_role_attribution
              WHERE etablissement_id = $1 AND role_id = $2 AND user_id = $3",
        )
        .bind(etablissement_id)
        .bind(role_id)
        .bind(user_id)
        .execute(self.pool())
        .await
        .map_err(db)?;
        if res.rows_affected() > 0 {
            self.audit_compte(
                etablissement_id,
                actor_user_id,
                actor_role,
                "role_retire",
                "Retrait d'un rôle",
                json!({ "role_id": role_id.to_string(), "user_id": user_id.to_string() }),
            )
            .await;
        }
        Ok(res.rows_affected() > 0)
    }

    /// Fixe les responsables d'un groupe ou atelier (personnels de l'établissement).
    pub async fn definir_responsables_groupe_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        groupe_id: Uuid,
        responsables: &[Uuid],
    ) -> Result<Vec<Uuid>, EntDbError> {
        self.exiger_roles_gerer(etablissement_id, actor_user_id, actor_role)
            .await?;
        if responsables.len() > MAX_CIBLES {
            return Err(DomainError::ValeurInvalide.into());
        }
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_groupe WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etablissement_id)
        .bind(groupe_id)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        if !existe {
            return Err(DomainError::NotFound.into());
        }
        let mut uniques: Vec<Uuid> = Vec::new();
        for u in responsables {
            let base = self
                .auth_repo()
                .role_in(etablissement_id, *u)
                .await?
                .and_then(|r| Role::parse(&r));
            if !base.is_some_and(|r| r.is_staff()) {
                return Err(DomainError::ValeurInvalide.into());
            }
            if !uniques.contains(u) {
                uniques.push(*u);
            }
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "DELETE FROM ent_groupe_responsable WHERE etablissement_id = $1 AND groupe_id = $2",
        )
        .bind(etablissement_id)
        .bind(groupe_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for u in &uniques {
            sqlx::query(
                "INSERT INTO ent_groupe_responsable (groupe_id, etablissement_id, user_id)
                 VALUES ($1, $2, $3)",
            )
            .bind(groupe_id)
            .bind(etablissement_id)
            .bind(u)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "groupe_responsables",
            "Responsables d'un groupe ou atelier",
            json!({ "groupe_id": groupe_id.to_string(),
                    "responsables": uniques.iter().map(|u| u.to_string()).collect::<Vec<_>>() }),
        )
        .await;
        Ok(uniques)
    }

    // ───────────────────────────── Garde-fous ─────────────────────────────

    async fn exiger_roles_gerer(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
    ) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etablissement_id, actor_user_id, actor_role, Action::RolesGerer)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Anti-escalade (garde-fou 2) : chaque droit doit être couvert par ceux de
    /// l'acteur. Le profil Admin est exempté.
    pub(crate) async fn exiger_couverture(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        droits: &[Droit],
    ) -> Result<(), EntDbError> {
        if Role::parse(actor_role) == Some(Role::Admin) {
            return Ok(());
        }
        let miens = self
            .droits_effectifs(etablissement_id, actor_user_id, actor_role)
            .await?;
        for d in droits {
            let couvert = miens.iter().any(|m| {
                m.action == d.action
                    && m.portee.couvre(d.portee)
                    && (m.portee == Portee::Etablissement
                        || d.portee != Portee::Designes
                        || d.cibles.iter().all(|c| m.cibles.contains(c)))
            });
            if !couvert {
                return Err(EntDbError::Forbidden);
            }
        }
        Ok(())
    }

    async fn valider_saisie(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        saisie: RoleSaisie,
    ) -> Result<RoleSaisie, EntDbError> {
        self.exiger_roles_gerer(etablissement_id, actor_user_id, actor_role)
            .await?;
        let nom: String = saisie.nom.trim().chars().filter(|c| !c.is_control()).collect();
        if nom.is_empty() || nom.chars().count() > 80 {
            return Err(DomainError::ValeurInvalide.into());
        }
        let description = saisie
            .description
            .map(|d| d.trim().chars().filter(|c| !c.is_control() || *c == '\n').take(500).collect::<String>())
            .filter(|d| !d.is_empty());
        if saisie.droits.len() > MAX_DROITS {
            return Err(DomainError::ValeurInvalide.into());
        }
        let mut droits: Vec<Droit> = Vec::new();
        for d in saisie.droits {
            if !d.action.portees_admises().contains(&d.portee) {
                return Err(DomainError::ValeurInvalide.into());
            }
            let mut cibles = d.cibles.clone();
            cibles.sort();
            cibles.dedup();
            if d.portee == Portee::Designes {
                if cibles.is_empty() || cibles.len() > MAX_CIBLES {
                    return Err(DomainError::ValeurInvalide.into());
                }
                // Les cibles doivent être des classes ou groupes de CET établissement.
                let (n,): (i64,) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM ent_classe WHERE etablissement_id = $1 AND id = ANY($2))
                          + (SELECT count(*) FROM ent_groupe WHERE etablissement_id = $1 AND id = ANY($2))",
                )
                .bind(etablissement_id)
                .bind(&cibles)
                .fetch_one(self.pool())
                .await
                .map_err(db)?;
                if n as usize != cibles.len() {
                    return Err(DomainError::ValeurInvalide.into());
                }
            } else {
                cibles.clear();
            }
            if !droits
                .iter()
                .any(|x| x.action == d.action && x.portee == d.portee)
            {
                droits.push(Droit {
                    action: d.action,
                    portee: d.portee,
                    cibles,
                });
            }
        }
        self.exiger_couverture(etablissement_id, actor_user_id, actor_role, &droits)
            .await?;
        Ok(RoleSaisie {
            nom,
            description,
            modele: saisie.modele.filter(|m| Role::parse(m).is_some()),
            droits,
        })
    }

    async fn role_par_id(&self, etablissement_id: Uuid, id: Uuid) -> Result<RoleEtab, EntDbError> {
        self.lire_roles(etablissement_id, Some(id))
            .await?
            .into_iter()
            .next()
            .ok_or(DomainError::NotFound.into())
    }

    async fn journal_role(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        action: &str,
        role_id: Uuid,
        saisie: &RoleSaisie,
    ) {
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            action,
            "Rôle paramétré",
            json!({
                "role_id": role_id.to_string(),
                "nom": saisie.nom,
                "droits": saisie.droits.iter().map(|d| json!({
                    "action": d.action.code(),
                    "portee": d.portee.code(),
                    "cibles": d.cibles.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }),
        )
        .await;
    }
}

async fn ecrire_droits(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    etablissement_id: Uuid,
    role_id: Uuid,
    droits: &[Droit],
) -> Result<(), EntDbError> {
    for d in droits {
        sqlx::query(
            "INSERT INTO ent_role_droit (role_id, etablissement_id, action, portee, cibles)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(role_id)
        .bind(etablissement_id)
        .bind(d.action.code())
        .bind(d.portee.code())
        .bind(&d.cibles)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    }
    Ok(())
}
