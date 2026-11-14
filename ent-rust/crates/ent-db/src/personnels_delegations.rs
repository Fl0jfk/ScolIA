//! Personnels, lot P6 — délégations par objet, intérim de direction, pouvoirs de signature
//! (`06_PERSONNELS.md` §8.9, §10.7, PRS-80 à 84 ; migration 359).
//!
//! La délégation de **droits** (S2-5, `delegation.rs`) EXISTE : datée, fin obligatoire, anti-escalade, expiration au jour
//! de l'établissement. P6 ne la duplique pas : il l'enrichit (objet, agents, motif codé, avertissement J-3) et y ajoute
//! trois objets qui s'appuient sur la même table `ent_delegation` :
//!
//! | Objet                | Effet                                                                                       |
//! |----------------------|---------------------------------------------------------------------------------------------|
//! | `role`               | droits du catalogue (S2-5, inchangé)                                                        |
//! | `mission` (PRS-80)   | une mission du délégant copiée au délégataire (origine `delegation`), mêmes dates et périmètre |
//! | `signature` (PRS-80) | habilitation à signer certaines natures de documents, avec la formule « par délégation »    |
//! | `interim_direction` (PRS-81) | droits du chef d'établissement (hors `roles.gerer`, `securite.politique`, `delegations.gerer`, `signature.habiliter`) CALCULÉS à chaque requête + signature de toutes les natures |
//!
//! | Commande / port                                      | Droit                                                   |
//! |------------------------------------------------------|---------------------------------------------------------|
//! | `deleguer_mission_for_actor` (PRS-80)                | `delegations.gerer` ; la mission est celle du délégant  |
//! | `deleguer_signature_for_actor` (PRS-80)              | `signature.habiliter` ; le délégant est l'acteur        |
//! | `organiser_interim_for_actor` (PRS-81)               | `signature.habiliter` ; le délégant est le chef d'établissement |
//! | `habiliter_signature_for_actor`, `retirer_habilitation_for_actor` (PRS-82) | `signature.habiliter` et fonction de chef d'établissement |
//! | `delegation_revoquer` (PRS-83, `delegation.rs`)      | `delegations.gerer`, ou le délégant lui-même             |
//! | `delegations_balayer` (PRS-84, `delegation.rs`)      | tâche système : expiration et avertissement à J-3       |
//! | **`peut_signer(agent, nature, date)`** (socle 7)     | port : aucun droit, aucun contrôle d'acteur (le socle 7 contrôle le sien) |
//!
//! **Anti-escalade** : un délégant ne délègue jamais plus que ce qu'il a. Mission : la sienne, durée au plus la sienne,
//! même périmètre ; signature : natures qu'il signe LUI-MÊME par fonction ou habilitation (pas de sous-délégation) ;
//! intérim : borné au modèle de direction du chef, exclusions fixes ; habilitation : seul le chef d'établissement.
//! **Aucune délégation ne dépasse sa date de fin** : le jour est celui de l'établissement (`ent_aujourdhui_etab`),
//! donc le droit s'éteint le soir de la fin, à l'heure locale, sans tâche de fond.
//!
//! **Cohérence avec le socle 15** : la suppléance d'un enseignant absent (A5) passe par une INTERVENTION `suppleant` du
//! socle 6 (P3), pas par une délégation ; l'absence du chef d'établissement n'ouvre aucune délégation toute seule :
//! l'intérim se décide (PRS-81). Les délégations `source = 'remplacement'` du remplacement de courte durée (A6) restent
//! des délégations de droits ; ce module ne les touche pas.

use chrono::NaiveDate;
use ent_auth::{modele_par_defaut, Action, Portee, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use ent_types::personnels_delegations::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::dossier_personnes::aujourdhui_tx;
use crate::outbox::Publier;
use crate::personnels::{
    agent_du_user_tx, agent_tx, conflit, db, introuvable, invalide, journal_tx,
};
use crate::personnels_missions::{annuler_tx, mission_de_l_agent_tx, nommer_par_delegation_tx};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Durée maximale d'une délégation (jours), comme S2-5.
pub const DUREE_MAX_JOURS: i64 = crate::delegation::DELEGATION_DUREE_MAX_JOURS;
const MOTIFS: &[&str] = &["absence", "surcharge", "organisation", "autre"];
const MAX_NATURES: usize = 40;

/// Droits que l'intérim ne donne JAMAIS (aucune chaîne de délégations, aucune gouvernance des rôles ni des
/// pouvoirs de signature). Paramétrable au lot P13 ; valeur par défaut recommandée (D-031).
pub const EXCLUS_INTERIM: [Action; 4] = [
    Action::RolesGerer,
    Action::SecuritePolitique,
    Action::DelegationsGerer,
    Action::SignatureHabiliter,
];

/// Droits (établissement entier) d'un intérim de direction : le modèle du chef d'établissement hors exclusions.
pub fn droits_interim() -> Vec<(Action, Portee)> {
    modele_par_defaut(Role::Direction)
        .into_iter()
        .filter(|(a, p)| *p == Portee::Etablissement && !EXCLUS_INTERIM.contains(a))
        .collect()
}

fn motif(b: Option<&str>) -> Result<&'static str, EntDbError> {
    match b {
        None => Ok("autre"),
        Some(m) => MOTIFS
            .iter()
            .copied()
            .find(|x| *x == m)
            .ok_or_else(|| invalide("Motif de délégation inconnu.")),
    }
}

/// Natures de documents : minuscules, chiffres, `_` ; `*` réservé à l'intérim. Dédoublonnées, ordre conservé.
fn natures(d: &[String], etoile: bool) -> Result<Vec<String>, EntDbError> {
    if d.is_empty() || d.len() > MAX_NATURES {
        return Err(invalide("Une à quarante natures de documents."));
    }
    let mut out: Vec<String> = Vec::new();
    for n in d {
        let n = n.trim();
        let ok = (etoile && n == "*")
            || (n.len() >= 2
                && n.len() <= 60
                && n.starts_with(|c: char| c.is_ascii_lowercase())
                && n.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
        if !ok {
            return Err(invalide("Nature de document invalide."));
        }
        if !out.iter().any(|x| x == n) {
            out.push(n.to_string());
        }
    }
    Ok(out)
}

fn formule(f: Option<&str>) -> Result<Option<String>, EntDbError> {
    match f.map(str::trim).filter(|x| !x.is_empty()) {
        None => Ok(None),
        Some(x) if x.chars().count() <= 160 => Ok(Some(x.to_string())),
        Some(_) => Err(invalide(
            "Formule de signature trop longue (160 caractères).",
        )),
    }
}

async fn publier_personne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    id: Uuid,
    payload: serde_json::Value,
    actor: Uuid,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Personne,
        aggregate_id: id,
        eleve_id: None,
        payload,
        actor_user_id: Some(actor.to_string()),
    })
    .await?;
    Ok(())
}

/// Compte de l'agent (les délégations sont entre comptes : `ent_delegation.delegue_user_id`).
async fn compte_de_l_agent_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    let l: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT p.user_id FROM ent_agent a
           JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id
          WHERE a.etablissement_id = $1 AND a.id = $2",
    )
    .bind(etab)
    .bind(agent)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(l.and_then(|(u,)| u))
}

async fn etat_agent_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
    jour: NaiveDate,
) -> Result<String, EntDbError> {
    sqlx::query_scalar("SELECT ent_agent_etat($1, $2, $3)")
        .bind(etab)
        .bind(agent)
        .bind(jour)
        .fetch_one(conn)
        .await
        .map_err(db)
}

async fn est_chef_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
    jour: NaiveDate,
) -> Result<bool, EntDbError> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_agent_fonction af JOIN ent_fonction f ON f.id = af.fonction_id
                         WHERE af.etablissement_id = $1 AND af.agent_id = $2 AND f.code = 'chef_etablissement'
                           AND af.annule_le IS NULL AND af.du <= $3 AND (af.au IS NULL OR af.au >= $3))",
    )
    .bind(etab)
    .bind(agent)
    .bind(jour)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(ok)
}

// ---------------------------------------------------------------------------------------------
// Port du socle 7 : peut_signer
// ---------------------------------------------------------------------------------------------

/// Cœur de `peut_signer` : l'agent est actif à `jour` ; le chef d'établissement signe tout par sa fonction ; les
/// autres par habilitation, délégation de signature ou intérim, valides ce jour-là (révocation immédiate).
pub(crate) async fn pouvoir_signature_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent: Uuid,
    nature: &str,
    jour: NaiveDate,
) -> Result<PouvoirSignatureDto, EntDbError> {
    let refus = PouvoirSignatureDto {
        autorise: false,
        origine: None,
        habilitation_id: None,
        formule: None,
        jusqu_au: None,
    };
    if etat_agent_tx(conn, etab, agent, jour).await? != "actif" {
        return Ok(refus);
    }
    if est_chef_tx(conn, etab, agent, jour).await? {
        return Ok(PouvoirSignatureDto {
            autorise: true,
            origine: Some("fonction".into()),
            habilitation_id: None,
            formule: None,
            jusqu_au: None,
        });
    }
    // Préférence : habilitation permanente, puis délégation, puis intérim.
    let l: Option<(Uuid, String, Option<String>, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT h.id, h.origine, h.formule, h.au
           FROM ent_habilitation_signature h
           LEFT JOIN ent_delegation d ON d.etablissement_id = h.etablissement_id AND d.id = h.origine_ref
          WHERE h.etablissement_id = $1 AND h.agent_id = $2 AND h.revoquee_le IS NULL
            AND h.du <= $3 AND (h.au IS NULL OR h.au >= $3)
            AND ($4 = ANY (h.documents) OR '*' = ANY (h.documents))
            AND (h.origine = 'habilitation' OR (d.statut = 'accordee' AND d.debut <= $3 AND d.fin >= $3))
          ORDER BY CASE h.origine WHEN 'habilitation' THEN 0 WHEN 'delegation' THEN 1 ELSE 2 END, h.au NULLS FIRST, h.id
          LIMIT 1",
    )
    .bind(etab)
    .bind(agent)
    .bind(jour)
    .bind(nature)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(match l {
        None => refus,
        Some((id, origine, f, au)) => {
            let formule = match origine.as_str() {
                "habilitation" => f,
                _ => Some(f.unwrap_or_else(|| FORMULE_PAR_DELEGATION.to_string())),
            };
            PouvoirSignatureDto {
                autorise: true,
                origine: Some(origine),
                habilitation_id: Some(id),
                formule,
                jusqu_au: au,
            }
        }
    })
}

type LigneHabilitation = (
    Uuid,
    Uuid,
    Option<String>,
    Vec<String>,
    NaiveDate,
    Option<NaiveDate>,
    String,
    Option<Uuid>,
    Option<String>,
    bool,
);

const SELECT_HABILITATION: &str =
    "SELECT h.id, h.agent_id, nullif(btrim(coalesce(p.prenom, '') || ' ' || p.nom), ''),
        h.documents, h.du, h.au, h.origine, h.origine_ref, h.formule, (h.revoquee_le IS NOT NULL)
   FROM ent_habilitation_signature h
   JOIN ent_agent a ON a.etablissement_id = h.etablissement_id AND a.id = h.agent_id
   JOIN ent_personne p ON p.etablissement_id = a.etablissement_id AND p.id = a.personne_id";

fn dto_habilitation(l: LigneHabilitation, jour: NaiveDate) -> HabilitationDto {
    HabilitationDto {
        id: l.0,
        agent_id: l.1,
        agent_nom: l.2,
        documents: l.3,
        du: l.4,
        au: l.5,
        origine: l.6,
        origine_ref: l.7,
        formule: l.8,
        revoquee: l.9,
        active: !l.9 && l.4 <= jour && l.5.is_none_or(|a| a >= jour),
    }
}

async fn habilitation_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    jour: NaiveDate,
) -> Result<HabilitationDto, EntDbError> {
    let sql = format!("{SELECT_HABILITATION} WHERE h.etablissement_id = $1 AND h.id = $2");
    let l: LigneHabilitation = sqlx::query_as(&sql)
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
    Ok(dto_habilitation(l, jour))
}

#[allow(clippy::too_many_arguments)]
async fn inserer_habilitation_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    agent: Uuid,
    documents: &[String],
    du: NaiveDate,
    au: Option<NaiveDate>,
    origine: &str,
    origine_ref: Option<Uuid>,
    formule: Option<&str>,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_habilitation_signature (id, etablissement_id, agent_id, documents, du, au, origine,
                                                 origine_ref, formule, cree_par_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(id)
    .bind(etab)
    .bind(agent)
    .bind(documents)
    .bind(du)
    .bind(au)
    .bind(origine)
    .bind(origine_ref)
    .bind(formule)
    .bind(actor)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
async fn inserer_delegation_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    objet: &str,
    objet_ref: Option<Uuid>,
    delegant: (Uuid, Uuid),
    delegataire: (Uuid, Uuid),
    du: NaiveDate,
    au: NaiveDate,
    motif_code: &str,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_delegation (id, etablissement_id, delegue_user_id, delegant_user_id, debut, fin, motif,
                                     objet, objet_ref, motif_code, delegant_agent_id, delegataire_agent_id)
         VALUES ($1, $2, $3, $4, $5, $6, '', $7, $8, $9, $10, $11)",
    )
    .bind(id)
    .bind(etab)
    .bind(delegataire.0)
    .bind(delegant.0)
    .bind(du)
    .bind(au)
    .bind(objet)
    .bind(objet_ref)
    .bind(motif_code)
    .bind(delegant.1)
    .bind(delegataire.1)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(id)
}

/// Contrôles communs : dates (début non passé, fin obligatoire, durée plafonnée) et délégataire (agent en poste de
/// CET établissement, avec compte, distinct du délégant). Renvoie le compte du délégataire.
async fn controles_delegataire_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    delegant: Uuid,
    delegataire: Uuid,
    du: NaiveDate,
    au: NaiveDate,
    jour: NaiveDate,
) -> Result<Uuid, EntDbError> {
    if du < jour {
        return Err(invalide("Une délégation ne commence pas dans le passé."));
    }
    if au < du || (au - du).num_days() > DUREE_MAX_JOURS {
        return Err(invalide(
            "Fin obligatoire, après le début et d'un an au plus.",
        ));
    }
    if delegataire == delegant {
        return Err(invalide("On ne se délègue rien à soi-même."));
    }
    agent_tx(conn, etab, delegataire, true).await?;
    let (en_poste,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_agent_engagement e
                         WHERE e.etablissement_id = $1 AND e.agent_id = $2 AND e.annule_le IS NULL
                           AND e.du <= $3 AND (e.au IS NULL OR e.au >= $3))",
    )
    .bind(etab)
    .bind(delegataire)
    .bind(du)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !en_poste {
        return Err(invalide(
            "Le délégataire n'a aucun engagement à cette date.",
        ));
    }
    compte_de_l_agent_tx(conn, etab, delegataire)
        .await?
        .ok_or_else(|| {
            invalide("Le délégataire n'a pas de compte : une délégation passe par un compte.")
        })
}

impl PgEnt {
    // -----------------------------------------------------------------------------------------
    // Port du socle 7
    // -----------------------------------------------------------------------------------------

    /// **Port `peut_signer(agent, nature_document, date)`** (socle 7, documents) : l'agent peut-il apposer une signature
    /// au nom de l'établissement, pour cette nature de document, à cette date ? Renvoie l'origine du pouvoir et la
    /// FORMULE à imprimer (« Pour le chef d'établissement et par délégation » pour un délégataire ; `None` pour le chef
    /// d'établissement, qui signe en son nom). Aucun contrôle d'acteur : le socle 7 contrôle le sien ; un agent d'un
    /// autre établissement n'a jamais de pouvoir ici (cloisonnement par `etab`).
    pub async fn peut_signer(
        &self,
        etab: Uuid,
        agent_id: Uuid,
        nature_document: &str,
        date: NaiveDate,
    ) -> Result<PouvoirSignatureDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        pouvoir_signature_tx(&mut tx, etab, agent_id, nature_document.trim(), date).await
    }

    /// Variante par compte et au jour de l'établissement (le socle 7 connaît l'utilisateur connecté).
    pub async fn peut_signer_compte(
        &self,
        etab: Uuid,
        user_id: Uuid,
        nature_document: &str,
    ) -> Result<PouvoirSignatureDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let Some(agent) = agent_du_user_tx(&mut tx, etab, user_id).await? else {
            return Ok(PouvoirSignatureDto {
                autorise: false,
                origine: None,
                habilitation_id: None,
                formule: None,
                jusqu_au: None,
            });
        };
        pouvoir_signature_tx(&mut tx, etab, agent, nature_document.trim(), jour).await
    }

    // -----------------------------------------------------------------------------------------
    // PRS-82 : habilitations
    // -----------------------------------------------------------------------------------------

    /// Habilitations (toutes, historique compris) : `signature.habiliter` ou `delegations.gerer`.
    pub async fn habilitations_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<HabilitationDto>, EntDbError> {
        if !self
            .a_action_etablissement(etab, actor, role, Action::SignatureHabiliter)
            .await?
        {
            self.exiger_personnels(etab, actor, role, Action::DelegationsGerer)
                .await?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let sql = format!("{SELECT_HABILITATION} WHERE h.etablissement_id = $1 ORDER BY h.du DESC, h.id LIMIT 500");
        let l: Vec<LigneHabilitation> = sqlx::query_as(&sql)
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        Ok(l.into_iter().map(|x| dto_habilitation(x, jour)).collect())
    }

    /// Ses propres pouvoirs de signature (un agent lit les siens sans droit).
    pub async fn mes_pouvoirs_signature(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<HabilitationDto>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let Some(agent) = agent_du_user_tx(&mut tx, etab, user_id).await? else {
            return Ok(Vec::new());
        };
        let sql = format!(
            "{SELECT_HABILITATION} WHERE h.etablissement_id = $1 AND h.agent_id = $2 ORDER BY h.du DESC, h.id");
        let l: Vec<LigneHabilitation> = sqlx::query_as(&sql)
            .bind(etab)
            .bind(agent)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        Ok(l.into_iter().map(|x| dto_habilitation(x, jour)).collect())
    }

    /// PRS-82 — habilite un agent à signer certaines natures de documents. Seul le chef d'établissement (fonction)
    /// habilite, et seulement ce qu'il signe lui-même (anti-escalade). Début futur permis, jamais passé.
    pub async fn habiliter_signature_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &HabiliterBody,
    ) -> Result<HabilitationDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::SignatureHabiliter)
            .await?;
        let documents = natures(&b.documents, false)?;
        let formule = formule(b.formule.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let du = b.du.unwrap_or(jour);
        if du < jour {
            return Err(invalide("Une habilitation ne commence pas dans le passé."));
        }
        if b.au.is_some_and(|a| a < du) {
            return Err(invalide("La fin précède le début."));
        }
        let Some(moi) = agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Err(EntDbError::Forbidden);
        };
        if !est_chef_tx(&mut tx, etab, moi, jour).await? {
            return Err(EntDbError::Forbidden);
        }
        if b.agent_id == moi {
            return Err(invalide(
                "Le chef d'établissement signe déjà par sa fonction.",
            ));
        }
        agent_tx(&mut tx, etab, b.agent_id, true).await?;
        if etat_agent_tx(&mut tx, etab, b.agent_id, du).await? != "actif" {
            return Err(invalide("L'agent n'est pas en poste à cette date."));
        }
        for n in &documents {
            if !pouvoir_signature_tx(&mut tx, etab, moi, n, du)
                .await?
                .autorise
            {
                return Err(EntDbError::Forbidden);
            }
        }
        let id = inserer_habilitation_tx(
            &mut tx,
            etab,
            actor,
            b.agent_id,
            &documents,
            du,
            b.au,
            "habilitation",
            None,
            formule.as_deref(),
        )
        .await?;
        let payload = json!({ "habilitation_id": id, "agent_id": b.agent_id, "documents": documents, "du": du,
                              "au": b.au, "operation": "accordee" });
        journal_tx(
            &mut tx,
            etab,
            "signature_habilitation_modifiee",
            "Pouvoir de signature accordé",
            payload.clone(),
            actor,
        )
        .await?;
        publier_personne_tx(
            &mut tx,
            etab,
            types::SIGNATURE_HABILITATION_MODIFIEE,
            id,
            payload,
            actor,
        )
        .await?;
        let dto = habilitation_tx(&mut tx, etab, id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    /// PRS-82 — retire une habilitation (effet immédiat). Les pouvoirs issus d'une délégation se retirent en
    /// révoquant la délégation (PRS-83).
    pub async fn retirer_habilitation_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<HabilitationDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::SignatureHabiliter)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let (origine, revoquee): (String, bool) = sqlx::query_as(
            "SELECT origine, (revoquee_le IS NOT NULL) FROM ent_habilitation_signature
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        if origine != "habilitation" {
            return Err(conflit(
                "Ce pouvoir vient d'une délégation : révoquer la délégation.",
            ));
        }
        if revoquee {
            return Err(conflit("Cette habilitation est déjà retirée."));
        }
        sqlx::query(
            "UPDATE ent_habilitation_signature SET revoquee_le = now(), revoquee_par_user_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let payload = json!({ "habilitation_id": id, "operation": "retiree" });
        journal_tx(
            &mut tx,
            etab,
            "signature_habilitation_modifiee",
            "Pouvoir de signature retiré",
            payload.clone(),
            actor,
        )
        .await?;
        publier_personne_tx(
            &mut tx,
            etab,
            types::SIGNATURE_HABILITATION_MODIFIEE,
            id,
            payload,
            actor,
        )
        .await?;
        let dto = habilitation_tx(&mut tx, etab, id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }

    // -----------------------------------------------------------------------------------------
    // PRS-80 : délégation de signature et de mission
    // -----------------------------------------------------------------------------------------

    /// PRS-80 — délègue SA signature pour certaines natures, jusqu'à une date. Le délégant est l'acteur ; il signe
    /// lui-même ces natures par fonction ou habilitation (pas de sous-délégation) ; la délégation et son pouvoir
    /// s'éteignent à la fin, ensemble.
    pub async fn deleguer_signature_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &DeleguerSignatureBody,
    ) -> Result<DelegationObjetDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::SignatureHabiliter)
            .await?;
        let documents = natures(&b.documents, false)?;
        let motif_code = motif(b.motif_code.as_deref())?;
        let formule =
            formule(b.formule.as_deref())?.unwrap_or_else(|| FORMULE_PAR_DELEGATION.to_string());
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let du = b.du.unwrap_or(jour);
        let Some(moi) = agent_du_user_tx(&mut tx, etab, actor).await? else {
            return Err(EntDbError::Forbidden);
        };
        let compte =
            controles_delegataire_tx(&mut tx, etab, moi, b.delegataire_agent_id, du, b.au, jour)
                .await?;
        for n in &documents {
            let p = pouvoir_signature_tx(&mut tx, etab, moi, n, du).await?;
            if !p.autorise || !matches!(p.origine.as_deref(), Some("fonction" | "habilitation")) {
                return Err(EntDbError::Forbidden);
            }
        }
        let id = inserer_delegation_tx(
            &mut tx,
            etab,
            "signature",
            None,
            (actor, moi),
            (compte, b.delegataire_agent_id),
            du,
            b.au,
            motif_code,
        )
        .await?;
        let h = inserer_habilitation_tx(
            &mut tx,
            etab,
            actor,
            b.delegataire_agent_id,
            &documents,
            du,
            Some(b.au),
            "delegation",
            Some(id),
            Some(&formule),
        )
        .await?;
        let payload = json!({ "delegation_id": id, "objet": "signature", "delegant_agent_id": moi,
                              "delegataire_agent_id": b.delegataire_agent_id, "documents": documents,
                              "debut": du, "fin": b.au, "motif": motif_code });
        journal_tx(
            &mut tx,
            etab,
            "delegation_accordee",
            "Délégation de signature",
            payload.clone(),
            actor,
        )
        .await?;
        publier_personne_tx(&mut tx, etab, "delegation.accordee", id, payload, actor).await?;
        let dto = delegation_objet_tx(&mut tx, etab, id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(DelegationObjetDto {
            habilitation_id: Some(h),
            ..dto
        })
    }

    /// PRS-80 — délègue une MISSION du délégant (le titulaire de la mission) : le délégataire reçoit une copie
    /// (origine `delegation`), mêmes type et périmètre, dates incluses dans celles de la mission. La cardinalité du
    /// type s'applique (un PP en plus d'un PP déjà nommé : 409 si la classe est au maximum).
    pub async fn deleguer_mission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &DeleguerMissionBody,
    ) -> Result<DelegationObjetDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::DelegationsGerer)
            .await?;
        let motif_code = motif(b.motif_code.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let du = b.du.unwrap_or(jour);
        let (delegant,): (Uuid,) = sqlx::query_as(
            "SELECT agent_id FROM ent_mission WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NULL",
        )
        .bind(etab)
        .bind(b.mission_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let (type_id, annee_id, m_du, m_au, perimetres) =
            mission_de_l_agent_tx(&mut tx, etab, delegant, b.mission_id).await?;
        // Anti-escalade : la copie tient dans la mission (début et fin), jamais au-delà.
        if du < m_du || m_au.is_some_and(|a| b.au > a) {
            return Err(invalide("La délégation déborde la mission du délégant."));
        }
        let compte_delegant = compte_de_l_agent_tx(&mut tx, etab, delegant)
            .await?
            .ok_or_else(|| invalide("Le délégant n'a pas de compte."))?;
        let compte = controles_delegataire_tx(
            &mut tx,
            etab,
            delegant,
            b.delegataire_agent_id,
            du,
            b.au,
            jour,
        )
        .await?;
        let id = inserer_delegation_tx(
            &mut tx,
            etab,
            "mission",
            Some(b.mission_id),
            (compte_delegant, delegant),
            (compte, b.delegataire_agent_id),
            du,
            b.au,
            motif_code,
        )
        .await?;
        let derivee = nommer_par_delegation_tx(
            &mut tx,
            etab,
            actor,
            type_id,
            b.delegataire_agent_id,
            &perimetres,
            du,
            b.au,
            annee_id,
            id,
            jour,
        )
        .await?;
        let payload = json!({ "delegation_id": id, "objet": "mission", "mission_id": b.mission_id,
                              "mission_derivee_id": derivee, "delegant_agent_id": delegant,
                              "delegataire_agent_id": b.delegataire_agent_id, "debut": du, "fin": b.au,
                              "motif": motif_code });
        journal_tx(
            &mut tx,
            etab,
            "delegation_accordee",
            "Délégation de mission",
            payload.clone(),
            actor,
        )
        .await?;
        publier_personne_tx(&mut tx, etab, "delegation.accordee", id, payload, actor).await?;
        let dto = delegation_objet_tx(&mut tx, etab, id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(DelegationObjetDto {
            mission_derivee_id: Some(derivee),
            ..dto
        })
    }

    // -----------------------------------------------------------------------------------------
    // PRS-81 : intérim de direction
    // -----------------------------------------------------------------------------------------

    /// PRS-81 — organise l'intérim de direction : le délégataire reçoit, pour la période, les droits du chef
    /// d'établissement (modèle de direction, hors `roles.gerer`, `securite.politique`, `delegations.gerer` et
    /// `signature.habiliter`) et la signature de toutes les natures, avec la formule « par délégation ». L'acteur
    /// (chef d'établissement ou administrateur) ne donne jamais plus que le modèle du chef ; un seul intérim à la fois
    /// pour un même chef.
    pub async fn organiser_interim_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &OrganiserInterimBody,
    ) -> Result<DelegationObjetDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::SignatureHabiliter)
            .await?;
        let motif_code = motif(b.motif_code.as_deref())?;
        let formule =
            formule(b.formule.as_deref())?.unwrap_or_else(|| FORMULE_PAR_DELEGATION.to_string());
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let du = b.du.unwrap_or(jour);
        // Le délégant est le chef d'établissement : l'acteur s'il l'est, sinon le chef unique en fonction.
        let moi = agent_du_user_tx(&mut tx, etab, actor).await?;
        let chef = match moi {
            Some(m) if est_chef_tx(&mut tx, etab, m, jour).await? => m,
            _ => {
                let chefs: Vec<(Uuid,)> = sqlx::query_as(
                    "SELECT DISTINCT af.agent_id FROM ent_agent_fonction af JOIN ent_fonction f ON f.id = af.fonction_id
                      WHERE af.etablissement_id = $1 AND f.code = 'chef_etablissement' AND af.annule_le IS NULL
                        AND af.du <= $2 AND (af.au IS NULL OR af.au >= $2)",
                )
                .bind(etab)
                .bind(jour)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                match chefs.as_slice() {
                    [(c,)] => *c,
                    [] => return Err(invalide("Aucun chef d'établissement en fonction.")),
                    _ => {
                        return Err(invalide(
                            "Plusieurs chefs d'établissement : l'intérim se décide par l'un d'eux.",
                        ))
                    }
                }
            }
        };
        let compte_chef = compte_de_l_agent_tx(&mut tx, etab, chef)
            .await?
            .ok_or_else(|| invalide("Le chef d'établissement n'a pas de compte."))?;
        // Anti-escalade : l'intérim est borné au modèle de DIRECTION ; le chef doit le détenir.
        let base = self.auth_repo().role_in(etab, compte_chef).await?;
        if base.as_deref().and_then(Role::parse) != Some(Role::Direction) {
            return Err(invalide(
                "Le chef d'établissement n'a pas le profil de direction.",
            ));
        }
        let compte =
            controles_delegataire_tx(&mut tx, etab, chef, b.delegataire_agent_id, du, b.au, jour)
                .await?;
        let (deja,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_delegation
                             WHERE etablissement_id = $1 AND objet = 'interim_direction' AND statut = 'accordee'
                               AND delegant_agent_id = $2 AND debut <= $4 AND fin >= $3)",
        )
        .bind(etab)
        .bind(chef)
        .bind(du)
        .bind(b.au)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit("Un intérim est déjà organisé sur cette période."));
        }
        let id = inserer_delegation_tx(
            &mut tx,
            etab,
            "interim_direction",
            None,
            (compte_chef, chef),
            (compte, b.delegataire_agent_id),
            du,
            b.au,
            motif_code,
        )
        .await?;
        let h = inserer_habilitation_tx(
            &mut tx,
            etab,
            actor,
            b.delegataire_agent_id,
            &["*".to_string()],
            du,
            Some(b.au),
            "interim",
            Some(id),
            Some(&formule),
        )
        .await?;
        let payload = json!({ "delegation_id": id, "objet": "interim_direction", "delegant_agent_id": chef,
                              "delegataire_agent_id": b.delegataire_agent_id, "debut": du, "fin": b.au,
                              "motif": motif_code,
                              "exclusions": EXCLUS_INTERIM.iter().map(|a| a.code()).collect::<Vec<_>>() });
        journal_tx(
            &mut tx,
            etab,
            "delegation_accordee",
            "Intérim de direction",
            payload.clone(),
            actor,
        )
        .await?;
        publier_personne_tx(&mut tx, etab, "delegation.accordee", id, payload, actor).await?;
        let dto = delegation_objet_tx(&mut tx, etab, id, jour).await?;
        tx.commit().await.map_err(db)?;
        Ok(DelegationObjetDto {
            habilitation_id: Some(h),
            ..dto
        })
    }

    /// Délégations par objet (mission, signature, intérim) de l'établissement : `delegations.gerer` ou
    /// `signature.habiliter`. Les délégations de droits restent sur `/delegations`.
    pub async fn delegations_objets_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<DelegationObjetDto>, EntDbError> {
        if !self
            .a_action_etablissement(etab, actor, role, Action::SignatureHabiliter)
            .await?
        {
            self.exiger_personnels(etab, actor, role, Action::DelegationsGerer)
                .await?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_delegation WHERE etablissement_id = $1 AND objet <> 'role'
              ORDER BY created_at DESC, id LIMIT 500",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for (id,) in ids {
            out.push(delegation_objet_tx(&mut tx, etab, id, jour).await?);
        }
        Ok(out)
    }
}

type LigneObjet = (
    Uuid,
    String,
    Option<Uuid>,
    Option<Uuid>,
    NaiveDate,
    NaiveDate,
    String,
    String,
    Option<Uuid>,
);

async fn delegation_objet_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    jour: NaiveDate,
) -> Result<DelegationObjetDto, EntDbError> {
    let l: LigneObjet = sqlx::query_as(
        "SELECT d.id, d.objet, d.delegant_agent_id, d.delegataire_agent_id, d.debut, d.fin, d.motif_code, d.statut, d.objet_ref
           FROM ent_delegation d WHERE d.etablissement_id = $1 AND d.id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let etat = match l.7.as_str() {
        "revoquee" => "revoquee",
        _ if l.5 < jour => "expiree",
        _ if l.4 > jour => "a_venir",
        _ => "active",
    };
    let derivee: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_mission WHERE etablissement_id = $1 AND origine = 'delegation' AND origine_ref = $2
           AND annule_le IS NULL LIMIT 1",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let hab: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_habilitation_signature WHERE etablissement_id = $1 AND origine_ref = $2 LIMIT 1",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(DelegationObjetDto {
        id: l.0,
        objet: l.1,
        delegant_agent_id: l.2,
        delegataire_agent_id: l.3,
        du: l.4,
        au: l.5,
        motif_code: l.6,
        etat: etat.to_string(),
        mission_derivee_id: derivee.map(|(m,)| m),
        habilitation_id: hab.map(|(h,)| h),
    })
}

/// PRS-83 — à la révocation d'une délégation, ferme ce qu'elle a produit : habilitations de signature (révoquées
/// tout de suite), missions copiées (annulées : aucun droit). Les droits de rôle et d'intérim cessent déjà
/// (`statut <> 'accordee'`).
pub(crate) async fn cloturer_derives_tx(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    id: Uuid,
    actor: Uuid,
    jour: NaiveDate,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_habilitation_signature SET revoquee_le = now(), revoquee_par_user_id = $3
          WHERE etablissement_id = $1 AND origine_ref = $2 AND revoquee_le IS NULL",
    )
    .bind(etab)
    .bind(id)
    .bind(actor)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    let missions: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_mission WHERE etablissement_id = $1 AND origine = 'delegation' AND origine_ref = $2
            AND annule_le IS NULL",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    for (m,) in missions {
        annuler_tx(tx, etab, actor, m, jour).await?;
    }
    Ok(())
}
