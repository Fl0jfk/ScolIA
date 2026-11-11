//! S2-5 — délégations de droits datées (ADM-04).
//!
//! Un directeur (action `delegations.gerer`) donne à un personnel, pour une durée dont la fin
//! est **obligatoire**, un sous-ensemble de droits du catalogue (D-017) : remplaçant,
//! organisateur d'une sortie, professeur principal par intérim… Le moteur de droits lit les
//! délégations en cours à chaque requête (`droits_effectifs`) : la date de fin éteint le droit
//! le lendemain sans tâche de fond ; le balayage ne fait qu'enregistrer l'expiration et publier
//! `delegation.expiree` une seule fois.
//!
//! Garde-fous : personnel de l'établissement uniquement (jamais famille ni élève) ; pas de
//! délégation à soi-même ; anti-escalade (on ne délègue que ce que l'on détient, avec une
//! portée au plus égale) ; `roles.gerer`, `securite.politique` et `delegations.gerer` ne sont
//! jamais délégables (pas de chaîne de délégations) ; durée plafonnée (366 jours [H]).
//! Remplacement : la délégation de remplacement est émise par l'organisation du remplacement
//! (droits attachés à la fonction, limités à la classe et au jour) et suit son cycle.

use chrono::NaiveDate;
use domain_eleve::today_utc;
use ent_auth::{Action, Portee, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::droits_repo::Droit;
use crate::outbox::Publier;
use crate::ref_commun::{Acteur, db, introuvable, invalide, publier_agg};
use crate::service::{EntDbError, PgEnt};

/// Durée maximale d'une délégation (jours) [H : à confirmer par l'établissement].
pub const DELEGATION_DUREE_MAX_JOURS: i64 = 366;

/// Droits que l'on ne délègue jamais.
const NON_DELEGABLES: [Action; 3] = [
    Action::RolesGerer,
    Action::SecuritePolitique,
    Action::DelegationsGerer,
];

/// Droit délégué tel qu'échangé avec l'API (JSON d'échange) ; stocké dans `ent_delegation_droit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroitDelegue {
    pub action: String,
    pub portee: String,
    #[serde(default)]
    pub cibles: Vec<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DelegationSaisie {
    pub delegue_user_id: Uuid,
    pub droits: Vec<DroitDelegue>,
    /// Début (défaut : aujourd'hui) ; jamais dans le passé.
    #[serde(default)]
    pub debut: Option<NaiveDate>,
    pub fin: NaiveDate,
    #[serde(default)]
    pub motif: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    pub id: Uuid,
    pub delegue_user_id: Uuid,
    pub delegue_nom: Option<String>,
    pub delegant_user_id: Uuid,
    pub delegant_nom: Option<String>,
    pub droits: Vec<DroitDelegue>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub motif: String,
    pub source: String,
    /// `a_venir`, `active`, `expiree`, `revoquee` (calculé à la lecture : la date prime).
    pub etat: String,
}

#[derive(Debug, FromRow)]
struct LigneDelegation {
    id: Uuid,
    delegue_user_id: Uuid,
    delegue_nom: Option<String>,
    delegant_user_id: Uuid,
    delegant_nom: Option<String>,
    debut: NaiveDate,
    fin: NaiveDate,
    motif: String,
    source: String,
    statut: String,
}

impl LigneDelegation {
    fn vers(self, jour: NaiveDate, droits: Vec<DroitDelegue>) -> Delegation {
        let etat = match self.statut.as_str() {
            "revoquee" => "revoquee",
            _ if self.fin < jour => "expiree",
            _ if self.debut > jour => "a_venir",
            _ => "active",
        };
        Delegation {
            id: self.id,
            delegue_user_id: self.delegue_user_id,
            delegue_nom: self.delegue_nom,
            delegant_user_id: self.delegant_user_id,
            delegant_nom: self.delegant_nom,
            droits,
            debut: self.debut,
            fin: self.fin,
            motif: self.motif,
            source: self.source,
            etat: etat.to_string(),
        }
    }
}

const SELECT_DELEGATION: &str = "SELECT d.id, d.delegue_user_id,
        coalesce(nullif(btrim(u1.display_name), ''), u1.email) AS delegue_nom,
        d.delegant_user_id,
        coalesce(nullif(btrim(u2.display_name), ''), u2.email) AS delegant_nom,
        d.debut, d.fin, d.motif, d.source, d.statut
   FROM ent_delegation d
   JOIN ent_user u1 ON u1.id = d.delegue_user_id
   JOIN ent_user u2 ON u2.id = d.delegant_user_id
  WHERE d.etablissement_id = $1";

/// Convertit les droits saisis en droits du moteur (validation du catalogue et des portées).
pub fn droits_du_catalogue(saisis: &[DroitDelegue]) -> Result<Vec<Droit>, EntDbError> {
    if saisis.is_empty() || saisis.len() > 20 {
        return Err(invalide());
    }
    let mut out: Vec<Droit> = Vec::new();
    for s in saisis {
        let action = Action::parse(&s.action).ok_or_else(invalide)?;
        let portee = Portee::parse(&s.portee).ok_or_else(invalide)?;
        // Lot P4 : `mon_perimetre`, `moi` et `sites_designes` ne se délèguent pas (leurs cibles viennent
        // de l'attribution ou du rôle, pas de la délégation).
        if NON_DELEGABLES.contains(&action)
            || !action.portees_admises().contains(&portee)
            || matches!(portee, Portee::MonPerimetre | Portee::Moi | Portee::SitesDesignes)
        {
            return Err(invalide());
        }
        let cibles = if portee == Portee::Designes {
            s.cibles.clone()
        } else {
            Vec::new()
        };
        if (portee == Portee::Designes && (cibles.is_empty() || cibles.len() > 50))
            || (portee != Portee::Designes && !s.cibles.is_empty())
        {
            return Err(invalide());
        }
        if out.iter().any(|d| d.action == action && d.portee == portee) {
            return Err(invalide());
        }
        out.push(Droit {
            action,
            portee,
            cibles,
        });
    }
    Ok(out)
}

/// Droits (et cibles désignées) des délégations `ids`, lus dans les tables typées (D-036).
pub(crate) async fn charger_droits(
    pool: &sqlx::PgPool,
    etab: Uuid,
    ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, Vec<DroitDelegue>>, EntDbError> {
    let mut out: std::collections::HashMap<Uuid, Vec<DroitDelegue>> =
        std::collections::HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let droits: Vec<(Uuid, i16, String, String)> = sqlx::query_as(
        "SELECT delegation_id, ordre, action, portee FROM ent_delegation_droit
          WHERE etablissement_id = $1 AND delegation_id = ANY($2)
          ORDER BY delegation_id, ordre",
    )
    .bind(etab)
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(db)?;
    let cibles: Vec<(Uuid, i16, Uuid)> = sqlx::query_as(
        "SELECT delegation_id, ordre, cible_id FROM ent_delegation_droit_cible
          WHERE etablissement_id = $1 AND delegation_id = ANY($2)
          ORDER BY delegation_id, ordre, cible_id",
    )
    .bind(etab)
    .bind(ids)
    .fetch_all(pool)
    .await
    .map_err(db)?;
    for (deleg, ordre, action, portee) in droits {
        let cibles: Vec<Uuid> = cibles
            .iter()
            .filter(|(d, o, _)| *d == deleg && *o == ordre)
            .map(|(_, _, c)| *c)
            .collect();
        out.entry(deleg).or_default().push(DroitDelegue {
            action,
            portee,
            cibles,
        });
    }
    Ok(out)
}

/// Remplace les droits d'une délégation par `droits` (dans la transaction de l'appelant).
pub(crate) async fn ecrire_droits_tx(
    tx: &mut sqlx::PgConnection,
    etab: Uuid,
    delegation_id: Uuid,
    droits: &[DroitDelegue],
) -> Result<(), EntDbError> {
    sqlx::query("DELETE FROM ent_delegation_droit WHERE etablissement_id = $1 AND delegation_id = $2")
        .bind(etab)
        .bind(delegation_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    for (ordre, d) in droits.iter().enumerate() {
        sqlx::query(
            "INSERT INTO ent_delegation_droit (etablissement_id, delegation_id, ordre, action, portee)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(etab)
        .bind(delegation_id)
        .bind(ordre as i16)
        .bind(&d.action)
        .bind(&d.portee)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for cible in &d.cibles {
            sqlx::query(
                "INSERT INTO ent_delegation_droit_cible (etablissement_id, delegation_id, ordre, cible_id)
                 VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
            )
            .bind(etab)
            .bind(delegation_id)
            .bind(ordre as i16)
            .bind(cible)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
    }
    Ok(())
}

impl PgEnt {
    /// Droits des délégations en cours à `jour` (appelé par le moteur de droits).
    pub(crate) async fn droits_delegues(
        &self,
        etab: Uuid,
        user_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Droit>, EntDbError> {
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_delegation
              WHERE etablissement_id = $1 AND delegue_user_id = $2 AND statut = 'accordee'
                AND debut <= $3 AND fin >= $3",
        )
        .bind(etab)
        .bind(user_id)
        .bind(jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = ids.into_iter().map(|(id,)| id).collect();
        let mut par_delegation = charger_droits(self.pool(), etab, &ids).await?;
        let mut out = Vec::new();
        for id in &ids {
            for s in par_delegation.remove(id).unwrap_or_default() {
                // Codes inconnus ou interdits (catalogue réduit depuis) : ignorés, jamais élargis.
                if let (Some(action), Some(portee)) =
                    (Action::parse(&s.action), Portee::parse(&s.portee))
                {
                    if !NON_DELEGABLES.contains(&action) {
                        out.push(Droit {
                            action,
                            portee,
                            cibles: s.cibles,
                        });
                    }
                }
            }
        }
        Ok(out)
    }

    /// ADM-04 : accorde une délégation datée.
    pub async fn delegation_accorder(
        &self,
        a: &Acteur<'_>,
        s: &DelegationSaisie,
    ) -> Result<Delegation, EntDbError> {
        self.exiger_referentiel(a, Action::DelegationsGerer).await?;
        let etab = a.etablissement_id;
        let aujourdhui = today_utc();
        let debut = s.debut.unwrap_or(aujourdhui);
        if debut < aujourdhui
            || s.fin < debut
            || (s.fin - debut).num_days() > DELEGATION_DUREE_MAX_JOURS
            || s.motif.len() > 300
            || s.delegue_user_id == a.user_id
        {
            return Err(invalide());
        }
        // Le délégué est un personnel de CET établissement.
        let role = self.auth_repo().role_in(etab, s.delegue_user_id).await?;
        match role.as_deref().and_then(Role::parse) {
            Some(r) if r.is_staff() => {}
            Some(_) => return Err(invalide()),
            None => return Err(introuvable()),
        }
        let droits = droits_du_catalogue(&s.droits)?;
        // Cibles « désignées » : classes ou groupes de l'établissement.
        for d in droits.iter().filter(|d| d.portee == Portee::Designes) {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM ent_classe WHERE etablissement_id = $1 AND id = ANY($2))
                      + (SELECT count(*) FROM ent_groupe WHERE etablissement_id = $1 AND id = ANY($2))",
            )
            .bind(etab)
            .bind(&d.cibles)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if n as usize != d.cibles.len() {
                return Err(introuvable());
            }
        }
        // Anti-escalade : on ne délègue que ce que l'on détient (délégations reçues comprises).
        self.exiger_couverture(etab, a.user_id, a.role, &droits)
            .await?;
        let id = Uuid::new_v4();
        let stock: Vec<DroitDelegue> = s
            .droits
            .iter()
            .map(|d| DroitDelegue {
                action: d.action.trim().to_string(),
                portee: d.portee.trim().to_string(),
                cibles: if d.portee.trim() == "designes" {
                    d.cibles.clone()
                } else {
                    Vec::new()
                },
            })
            .collect();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_delegation
               (id, etablissement_id, delegue_user_id, delegant_user_id, debut, fin, motif)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(s.delegue_user_id)
        .bind(a.user_id)
        .bind(debut)
        .bind(s.fin)
        .bind(s.motif.trim())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        ecrire_droits_tx(&mut tx, etab, id, &stock).await?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Personne,
            "delegation.accordee",
            id,
            json!({ "delegation_id": id, "delegue_user_id": s.delegue_user_id,
                    "actions": stock.iter().map(|d| d.action.clone()).collect::<Vec<_>>(),
                    "debut": debut, "fin": s.fin }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        self.audit_compte(
            etab,
            a.user_id,
            a.role,
            "delegation_accordee",
            "Délégation de droits datée",
            json!({ "delegation_id": id.to_string(), "delegue_user_id": s.delegue_user_id.to_string(),
                    "fin": s.fin.to_string() }),
        )
        .await;
        self.delegation_lire(etab, id)
            .await?
            .ok_or_else(introuvable)
    }

    async fn delegation_lire(
        &self,
        etab: Uuid,
        id: Uuid,
    ) -> Result<Option<Delegation>, EntDbError> {
        let sql = format!("{SELECT_DELEGATION} AND d.id = $2");
        let Some(l) = sqlx::query_as::<_, LigneDelegation>(&sql)
            .bind(etab)
            .bind(id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?
        else {
            return Ok(None);
        };
        let mut droits = charger_droits(self.pool(), etab, &[l.id]).await?;
        let d = droits.remove(&l.id).unwrap_or_default();
        Ok(Some(l.vers(today_utc(), d)))
    }

    /// Révoque une délégation (effet immédiat) ; sans effet si déjà révoquée ou expirée.
    pub async fn delegation_revoquer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Delegation, EntDbError> {
        self.exiger_referentiel(a, Action::DelegationsGerer).await?;
        let etab = a.etablissement_id;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let touche: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE ent_delegation
                SET statut = 'revoquee', revoquee_le = now(), revoquee_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'accordee' AND fin >= $4
              RETURNING delegue_user_id",
        )
        .bind(etab)
        .bind(id)
        .bind(a.user_id)
        .bind(today_utc())
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((delegue,)) = touche {
            let ev = publier_agg(
                &mut tx,
                a,
                AggregateKind::Personne,
                "delegation.revoquee",
                id,
                json!({ "delegation_id": id, "delegue_user_id": delegue }),
            )
            .await?;
            tx.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&[ev]);
        }
        self.delegation_lire(etab, id)
            .await?
            .ok_or_else(introuvable)
    }

    /// Délégations de l'établissement (`delegations.gerer`), récentes d'abord.
    pub async fn delegations_lister(
        &self,
        a: &Acteur<'_>,
        en_cours_seulement: bool,
    ) -> Result<Vec<Delegation>, EntDbError> {
        self.exiger_referentiel(a, Action::DelegationsGerer).await?;
        let jour = today_utc();
        let sql = format!(
            "{SELECT_DELEGATION}
                AND (NOT $2 OR (d.statut = 'accordee' AND d.fin >= $3))
              ORDER BY d.created_at DESC LIMIT 500"
        );
        let lignes = sqlx::query_as::<_, LigneDelegation>(&sql)
            .bind(a.etablissement_id)
            .bind(en_cours_seulement)
            .bind(jour)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        self.avec_droits(a.etablissement_id, lignes, jour).await
    }

    /// Délégations reçues par l'acteur (lecture de SES droits temporaires).
    pub async fn delegations_mes(&self, a: &Acteur<'_>) -> Result<Vec<Delegation>, EntDbError> {
        let jour = today_utc();
        let sql = format!(
            "{SELECT_DELEGATION}
                AND d.delegue_user_id = $2
              ORDER BY d.created_at DESC LIMIT 100"
        );
        let lignes = sqlx::query_as::<_, LigneDelegation>(&sql)
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        self.avec_droits(a.etablissement_id, lignes, jour).await
    }

    async fn avec_droits(
        &self,
        etab: Uuid,
        lignes: Vec<LigneDelegation>,
        jour: NaiveDate,
    ) -> Result<Vec<Delegation>, EntDbError> {
        let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
        let mut droits = charger_droits(self.pool(), etab, &ids).await?;
        Ok(lignes
            .into_iter()
            .map(|l| {
                let d = droits.remove(&l.id).unwrap_or_default();
                l.vers(jour, d)
            })
            .collect())
    }

    /// Balayage d'expiration (tâche périodique, tous établissements) : marque `expiree` les
    /// délégations dont la fin est passée et publie `delegation.expiree` **une seule fois**
    /// (la mise à jour conditionnelle rend le balayage idempotent). Renvoie le nombre expiré.
    pub async fn delegations_balayer(&self, jour: NaiveDate) -> Result<usize, EntDbError> {
        let mut tx = self.pool().begin().await.map_err(db)?;
        let lignes: Vec<(Uuid, Uuid, Uuid, NaiveDate)> = sqlx::query_as(
            "UPDATE ent_delegation SET statut = 'expiree', expiree_le = now()
              WHERE statut = 'accordee' AND fin < $1
              RETURNING id, etablissement_id, delegue_user_id, fin",
        )
        .bind(jour)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut evenements = Vec::new();
        for (id, etab, delegue, fin) in &lignes {
            let conn: &mut sqlx::PgConnection = &mut tx;
            evenements.push(
                conn.publier(MetierEventDraft {
                    etablissement_id: *etab,
                    type_name: "delegation.expiree".to_string(),
                    aggregate: AggregateKind::Personne,
                    aggregate_id: *id,
                    eleve_id: None,
                    payload: json!({ "delegation_id": id, "delegue_user_id": delegue, "fin": fin }),
                    actor_user_id: None,
                })
                .await?,
            );
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evenements);
        Ok(lignes.len())
    }
}
