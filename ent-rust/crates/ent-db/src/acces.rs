//! Cycle de vie d'un accès à un établissement (socle 6, lot P0 ; `06_PERSONNELS.md` §2.2 n° 1-2,
//! §8.4, §10.3).
//!
//! Avant ce lot, un accès (`ent_membership`) n'avait ni état ni fin : un contractuel parti gardait
//! son accès indéfiniment et un établissement pouvait perdre son dernier administrateur. Ce module
//! porte les commandes :
//!
//! | Commande                      | Effet                                                              |
//! |-------------------------------|--------------------------------------------------------------------|
//! | [`PgEnt::acces_fermer_for_actor`]   | état `ferme` : fin définitive (départ, fin de contrat, mutation) |
//! | [`PgEnt::acces_suspendre_for_actor`] | état `suspendu` : mesure conservatoire ou congé, réversible      |
//! | [`PgEnt::acces_rouvrir_for_actor`]  | retour à `ouvert` (reprise, erreur de saisie)                    |
//! | [`PgEnt::acces_programmer_fin_for_actor`] | date de fin : l'accès expire le soir de cette date (jour de l'établissement) |
//!
//! Garanties (toutes testées sur Postgres) :
//! * un accès fermé, suspendu ou échu **ne permet plus de se connecter** (mot de passe, passkey,
//!   SSO) et **invalide les sessions et jetons déjà ouverts** : la fermeture révoque les sessions
//!   *de cet établissement* (pas celles des autres établissements de la personne) et le contrôle de
//!   session relit l'état à chaque requête ;
//! * un établissement **ne perd jamais son dernier administrateur** (fermeture, suspension, date de
//!   fin, changement de profil, suppression) : contrôle ici pour un message clair, et déclencheur
//!   SQL (migration 350) pour tout autre chemin, y compris deux retraits simultanés ;
//! * cloisonnement : la cible est cherchée par `(établissement, personne)` ; un compte hors de
//!   l'établissement de l'acteur est refusé de façon uniforme (403, pas de 404) ;
//! * chaque commande est journalisée (acteur réel, cible, motif, jamais d'adresse en clair).
//!
//! Droit d'agir : Admin ou Direction, jamais sur soi-même ; la Direction ne touche ni un Admin ni
//! une autre Direction (même hiérarchie que la réinitialisation des identifiants). Seuls les
//! personnels sont concernés ici : l'accès des familles et des élèves relève des socles 3 et 4.

use chrono::NaiveDate;
use serde_json::json;
use uuid::Uuid;

use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, OperationCompte, PgEnt, RefusMetier};

const MSG_DERNIER_ADMIN: &str = "Un établissement ne peut pas perdre son dernier administrateur.";

/// Motifs de fermeture (codes stables, vérifiés par la base).
pub const MOTIFS_FERMETURE: &[&str] = &["depart", "fin_contrat", "mutation", "erreur", "autre"];
/// Motifs de suspension (codes stables, vérifiés par la base).
pub const MOTIFS_SUSPENSION: &[&str] = &["conservatoire", "conge", "autre"];

/// Horizon maximal d'une date de fin (garde contre une saisie aberrante).
const HORIZON_FIN_ANNEES: i64 = 10;

/// État d'un accès.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatAcces {
    Ouvert,
    Suspendu,
    Ferme,
}

impl EtatAcces {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ouvert => "ouvert",
            Self::Suspendu => "suspendu",
            Self::Ferme => "ferme",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "ouvert" => Some(Self::Ouvert),
            "suspendu" => Some(Self::Suspendu),
            "ferme" => Some(Self::Ferme),
            _ => None,
        }
    }
}

/// Lecture de l'état d'un accès (sans donnée personnelle).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccesEtat {
    pub user_id: Uuid,
    pub etat: EtatAcces,
    pub motif: Option<String>,
    pub date_fin: Option<NaiveDate>,
}

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("acces", err))
}

/// Le déclencheur de la migration 350 lève `dernier_administrateur` (SQLSTATE 23514) : message
/// clair et refus de conflit plutôt qu'une erreur de valeur générique.
fn mappe_ecriture(err: sqlx::Error) -> EntDbError {
    if let sqlx::Error::Database(d) = &err {
        if d.message().contains("dernier_administrateur") {
            return EntDbError::Refus(RefusMetier::Conflit, MSG_DERNIER_ADMIN);
        }
    }
    db(err)
}

impl PgEnt {
    /// Jour courant de l'établissement (fuseau du paramètre d'établissement, Europe/Paris par
    /// défaut) : la définition unique est la fonction SQL `ent_aujourdhui_etab` (migration 265).
    pub async fn aujourdhui_etablissement(
        &self,
        etablissement_id: Uuid,
    ) -> Result<NaiveDate, EntDbError> {
        let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1)")
            .bind(etablissement_id)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
        Ok(jour)
    }

    /// État de l'accès d'une personne à l'établissement (`None` si elle n'en est pas membre).
    pub async fn acces_etat(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<AccesEtat>, EntDbError> {
        let row: Option<(String, Option<String>, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT etat, etat_motif, date_fin FROM ent_membership
              WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etablissement_id)
        .bind(user_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        Ok(row.and_then(|(etat, motif, date_fin)| {
            Some(AccesEtat {
                user_id,
                etat: EtatAcces::parse(&etat)?,
                motif,
                date_fin,
            })
        }))
    }

    /// Ferme l'accès d'un personnel (départ définitif). Idempotent. Révoque les sessions de la
    /// personne **dans cet établissement** et supprime ses appareils de notification ici.
    pub async fn acces_fermer_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        cible_user_id: Uuid,
        motif: &str,
    ) -> Result<EtatAcces, EntDbError> {
        if !MOTIFS_FERMETURE.contains(&motif) {
            return Err(EntDbError::Refus(
                RefusMetier::Invalide,
                "Motif de fermeture inconnu.",
            ));
        }
        self.changer_etat_acces(
            etablissement_id,
            actor_user_id,
            actor_role,
            cible_user_id,
            EtatAcces::Ferme,
            Some(motif),
        )
        .await
    }

    /// Suspend l'accès d'un personnel (mesure conservatoire, congé long). Réversible, même effet
    /// immédiat sur les connexions et les sessions que la fermeture. Refusé sur un accès fermé.
    pub async fn acces_suspendre_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        cible_user_id: Uuid,
        motif: &str,
    ) -> Result<EtatAcces, EntDbError> {
        if !MOTIFS_SUSPENSION.contains(&motif) {
            return Err(EntDbError::Refus(
                RefusMetier::Invalide,
                "Motif de suspension inconnu.",
            ));
        }
        self.changer_etat_acces(
            etablissement_id,
            actor_user_id,
            actor_role,
            cible_user_id,
            EtatAcces::Suspendu,
            Some(motif),
        )
        .await
    }

    /// Rouvre un accès suspendu ou fermé (reprise de poste, fermeture faite par erreur).
    pub async fn acces_rouvrir_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        cible_user_id: Uuid,
    ) -> Result<EtatAcces, EntDbError> {
        self.changer_etat_acces(
            etablissement_id,
            actor_user_id,
            actor_role,
            cible_user_id,
            EtatAcces::Ouvert,
            None,
        )
        .await
    }

    async fn changer_etat_acces(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        cible_user_id: Uuid,
        nouvel_etat: EtatAcces,
        motif: Option<&str>,
    ) -> Result<EtatAcces, EntDbError> {
        // Droit et hiérarchie : 403 uniforme (jamais de 404 qui révélerait l'appartenance).
        self.cible_compte_autorisee(
            etablissement_id,
            actor_user_id,
            actor_role,
            cible_user_id,
            OperationCompte::Acces,
        )
        .await?;

        let mut tx = self.pool().begin().await.map_err(db)?;
        let row: Option<(String, String, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT role, etat, date_fin FROM ent_membership
              WHERE etablissement_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(cible_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((role, etat_actuel, date_fin)) = row else {
            return Err(EntDbError::Forbidden);
        };
        let etat_actuel = EtatAcces::parse(&etat_actuel).ok_or(EntDbError::Forbidden)?;
        if etat_actuel == nouvel_etat {
            return Ok(nouvel_etat); // idempotent : rien à écrire, rien à journaliser deux fois
        }
        if nouvel_etat == EtatAcces::Suspendu && etat_actuel == EtatAcces::Ferme {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "L'accès est fermé : le rouvrir avant de le suspendre.",
            ));
        }
        // Message clair avant le déclencheur (qui reste le filet pour tout autre chemin).
        if nouvel_etat != EtatAcces::Ouvert
            && role == "admin"
            && etat_actuel == EtatAcces::Ouvert
            && date_fin.is_none()
            && !Self::autre_administrateur_durable(&mut tx, etablissement_id, cible_user_id).await?
        {
            return Err(EntDbError::Refus(RefusMetier::Conflit, MSG_DERNIER_ADMIN));
        }

        sqlx::query(
            "UPDATE ent_membership
                SET etat = $3, etat_depuis = now(), etat_motif = $4, etat_par_user_id = $5
              WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etablissement_id)
        .bind(cible_user_id)
        .bind(nouvel_etat.as_str())
        .bind(motif)
        .bind(actor_user_id)
        .execute(&mut *tx)
        .await
        .map_err(mappe_ecriture)?;

        let mut sessions_revoquees = 0u64;
        if nouvel_etat != EtatAcces::Ouvert {
            // Sessions de CET établissement seulement : les autres accès de la personne restent.
            sessions_revoquees = sqlx::query(
                "UPDATE ent_session SET revoked_at = now()
                  WHERE etablissement_id = $1 AND user_id = $2 AND revoked_at IS NULL",
            )
            .bind(etablissement_id)
            .bind(cible_user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
            // Plus de notification poussée vers les appareils de la personne pour cet établissement.
            sqlx::query("DELETE FROM ent_push_device WHERE etablissement_id = $1 AND user_id = $2")
                .bind(etablissement_id)
                .bind(cible_user_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;

        let action = match nouvel_etat {
            EtatAcces::Ferme => "acces_ferme",
            EtatAcces::Suspendu => "acces_suspendu",
            EtatAcces::Ouvert => "acces_rouvert",
        };
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            action,
            match nouvel_etat {
                EtatAcces::Ferme => "Fermeture de l'accès d'un membre du personnel",
                EtatAcces::Suspendu => "Suspension de l'accès d'un membre du personnel",
                EtatAcces::Ouvert => "Réouverture de l'accès d'un membre du personnel",
            },
            json!({
                "cible_user_id": cible_user_id.to_string(),
                "cible_role": role,
                "etat_precedent": etat_actuel.as_str(),
                "motif": motif,
                "sessions_revoquees": sessions_revoquees,
            }),
        )
        .await;
        Ok(nouvel_etat)
    }

    /// Programme (ou retire, avec `None`) la date de fin d'un accès : il expire le soir de cette
    /// date, dans le fuseau de l'établissement. Une date de fin retire à un administrateur sa
    /// qualité de dernier recours : refusée s'il est le dernier administrateur durable.
    pub async fn acces_programmer_fin_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        actor_role: &str,
        cible_user_id: Uuid,
        fin: Option<NaiveDate>,
    ) -> Result<Option<NaiveDate>, EntDbError> {
        self.cible_compte_autorisee(
            etablissement_id,
            actor_user_id,
            actor_role,
            cible_user_id,
            OperationCompte::Acces,
        )
        .await?;
        if let Some(f) = fin {
            let aujourdhui = self.aujourdhui_etablissement(etablissement_id).await?;
            if f < aujourdhui || f > aujourdhui + chrono::Duration::days(365 * HORIZON_FIN_ANNEES) {
                return Err(EntDbError::Refus(
                    RefusMetier::Invalide,
                    "La date de fin doit être comprise entre aujourd'hui et dix ans.",
                ));
            }
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let row: Option<(String, String, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT role, etat, date_fin FROM ent_membership
              WHERE etablissement_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(cible_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((role, etat, ancienne)) = row else {
            return Err(EntDbError::Forbidden);
        };
        if ancienne == fin {
            return Ok(fin);
        }
        if fin.is_some()
            && role == "admin"
            && etat == "ouvert"
            && ancienne.is_none()
            && !Self::autre_administrateur_durable(&mut tx, etablissement_id, cible_user_id).await?
        {
            return Err(EntDbError::Refus(RefusMetier::Conflit, MSG_DERNIER_ADMIN));
        }
        sqlx::query(
            "UPDATE ent_membership SET date_fin = $3
              WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etablissement_id)
        .bind(cible_user_id)
        .bind(fin)
        .execute(&mut *tx)
        .await
        .map_err(mappe_ecriture)?;
        tx.commit().await.map_err(db)?;
        self.audit_compte(
            etablissement_id,
            actor_user_id,
            actor_role,
            "acces_fin_programmee",
            "Date de fin d'accès modifiée",
            json!({
                "cible_user_id": cible_user_id.to_string(),
                "cible_role": role,
                "ancienne": ancienne.map(|d| d.to_string()),
                "nouvelle": fin.map(|d| d.to_string()),
            }),
        )
        .await;
        Ok(fin)
    }

    /// Existe-t-il, dans l'établissement, un autre administrateur « durable » (profil `admin`,
    /// accès ouvert, sans date de fin) que `sauf_user_id` ?
    async fn autre_administrateur_durable(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        etablissement_id: Uuid,
        sauf_user_id: Uuid,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (
               SELECT 1 FROM ent_membership
                WHERE etablissement_id = $1 AND user_id <> $2
                  AND role = 'admin' AND etat = 'ouvert' AND date_fin IS NULL
                  AND NOT ent_agent_acces_suspendu(user_id, etablissement_id))",
        )
        .bind(etablissement_id)
        .bind(sauf_user_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db)?;
        Ok(ok)
    }
}
