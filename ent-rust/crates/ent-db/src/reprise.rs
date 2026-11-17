//! Reprise Next.js → ScolIA (socle 28, lot R2) : exécutions, comptes, refus, lignes « à décider » et table
//! de correspondance, côté base. Toutes les requêtes passent par `TenantTx` (RLS d'établissement).
//!
//! **La simulation n'écrit rien dans les tables métier** : elle écrit seulement la trace de l'exécution
//! (`rep_execution*`, `rep_refus`) ; la zone de transit est dans [`crate::reprise_zone`]. L'enregistrement
//! des correspondances n'est permis que pour une exécution qui n'est pas une simulation, et ne crée aucun
//! objet métier (les objets sont créés par les lots R3 et suivants, par les commandes des socles
//! propriétaires).

use crate::tenant_tx::TenantTx;
use ent_reprise::correspondance::{id_derive, Existante, Existantes, Statut, SOURCE_NEXT};
use ent_reprise::decodage::RapportDecodage;
use ent_reprise::simulation::{Action, RapportSimulation};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ErreurReprise {
    #[error("erreur base de données : {0}")]
    Base(#[from] sqlx::Error),
    #[error("{0}")]
    Refus(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeExecution {
    Simulation,
    Repetition,
    Application,
}

impl ModeExecution {
    fn code(self) -> &'static str {
        match self {
            ModeExecution::Simulation => "simulation",
            ModeExecution::Repetition => "repetition",
            ModeExecution::Application => "application",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CibleExecution {
    Jetable,
    Preprod,
    Production,
}

impl CibleExecution {
    fn code(self) -> &'static str {
        match self {
            CibleExecution::Jetable => "jetable",
            CibleExecution::Preprod => "preprod",
            CibleExecution::Production => "production",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Execution {
    pub id: Uuid,
    pub numero: i32,
}

/// Ouvre une exécution numérotée (par établissement) : état `en_cours`.
pub async fn ouvrir_execution(
    pool: &PgPool,
    etab: Uuid,
    format_version: &str,
    mode: ModeExecution,
    cible: CibleExecution,
    operateur: Option<Uuid>,
) -> Result<Execution, ErreurReprise> {
    let mut tx = TenantTx::begin(pool, etab).await?;
    // Une exécution à la fois par établissement pour la numérotation.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 2564))")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await?;
    let numero: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(numero), 0)::int + 1 FROM rep_execution WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_one(&mut *tx)
    .await?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO rep_execution (id, etablissement_id, numero, source, format_version, mode, cible, operateur_user_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(etab)
    .bind(numero)
    .bind(SOURCE_NEXT)
    .bind(format_version)
    .bind(mode.code())
    .bind(cible.code())
    .bind(operateur)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Execution { id, numero })
}

/// Correspondances déjà enregistrées (source Next.js) de l'établissement.
pub async fn correspondances(pool: &PgPool, etab: Uuid) -> Result<Existantes, ErreurReprise> {
    let mut tx = TenantTx::begin(pool, etab).await?;
    let lignes = sqlx::query(
        "SELECT source_objet, source_id, cible_id, empreinte_source, modifie_depuis_reprise, statut
           FROM rep_correspondance WHERE etablissement_id = $1 AND source = $2",
    )
    .bind(etab)
    .bind(SOURCE_NEXT)
    .fetch_all(&mut *tx)
    .await?;
    tx.rollback().await?;
    let mut m = Existantes::new();
    for l in lignes {
        let statut: String = l.get("statut");
        let cible: Uuid = l.get("cible_id");
        m.insert(
            (l.get("source_objet"), l.get("source_id")),
            Existante {
                cible_id: cible.to_string(),
                empreinte_source: l.get("empreinte_source"),
                modifie_depuis_reprise: l.get("modifie_depuis_reprise"),
                statut: Statut::depuis(&statut)
                    .ok_or_else(|| ErreurReprise::Refus(format!("statut inconnu : {statut}")))?,
            },
        );
    }
    Ok(m)
}

fn message_court(m: &str) -> String {
    m.chars().take(500).collect()
}

/// Écrit la trace d'une exécution **sans toucher au métier** : comptes par objet, refus, lignes à décider,
/// puis clôture (`terminee`). Une seule transaction.
pub async fn enregistrer_trace(
    pool: &PgPool,
    etab: Uuid,
    exec: Execution,
    decodage: &RapportDecodage,
    simulation: &RapportSimulation,
    rapport_empreinte: &str,
) -> Result<(), ErreurReprise> {
    if !simulation.invariants_ok() {
        return Err(ErreurReprise::Refus(
            "invariant de la simulation violé : trace refusée".into(),
        ));
    }
    let mut tx = TenantTx::begin(pool, etab).await?;
    for (objet, c) in &simulation.par_objet {
        sqlx::query(
            "INSERT INTO rep_execution_compte
                 (execution_id, etablissement_id, objet, lues, creees, mises_a_jour, inchangees, archivees, ignorees, refusees, a_decider, rapprochees)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(exec.id)
        .bind(etab)
        .bind(objet)
        .bind(c.lues as i64)
        .bind(c.creees as i64)
        .bind(c.mises_a_jour as i64)
        .bind(c.inchangees as i64)
        .bind(c.archivees as i64)
        .bind(c.ignorees as i64)
        .bind(c.refusees as i64)
        .bind(c.a_decider as i64)
        .bind(c.rapprochees as i64)
        .execute(&mut *tx)
        .await?;
    }
    for r in &decodage.refus {
        sqlx::query(
            "INSERT INTO rep_refus (id, execution_id, etablissement_id, source_objet, source_id, source_ligne, regle_code, colonne, message)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(Uuid::new_v4())
        .bind(exec.id)
        .bind(etab)
        .bind(&r.objet)
        .bind(&r.source_id)
        .bind(i64::try_from(r.source_ligne).ok())
        .bind(&r.code)
        .bind(&r.colonne)
        .bind(message_court(&r.message))
        .execute(&mut *tx)
        .await?;
    }
    for a in &simulation.a_decider {
        sqlx::query(
            "INSERT INTO rep_execution_a_decider (execution_id, etablissement_id, objet, source_id, cible_objet, cible_id, motif)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(exec.id)
        .bind(etab)
        .bind(&a.objet)
        .bind(&a.source_id)
        .bind(&a.cible_objet)
        .bind(Uuid::parse_str(&a.cible_id).map_err(|_| ErreurReprise::Refus("identifiant cible invalide".into()))?)
        .bind(&a.motif)
        .execute(&mut *tx)
        .await?;
    }
    crate::reprise_rapprochement::ecrire_file(&mut *tx, etab, exec, &simulation.file).await?;
    sqlx::query(
        "UPDATE rep_execution SET etat = 'terminee', fin_le = now(), rapport_empreinte = $3,
                duree_s = GREATEST(0, EXTRACT(EPOCH FROM (now() - debut_le))::int)
          WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_cours'",
    )
    .bind(etab)
    .bind(exec.id)
    .bind(rapport_empreinte)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Marque une exécution en échec (jamais en silence).
pub async fn marquer_echec(
    pool: &PgPool,
    etab: Uuid,
    exec: Execution,
) -> Result<(), ErreurReprise> {
    let mut tx = TenantTx::begin(pool, etab).await?;
    sqlx::query(
        "UPDATE rep_execution SET etat = 'echec', fin_le = now() WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_cours'",
    )
    .bind(etab)
    .bind(exec.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Enregistre les correspondances que l'application poserait : une par création ou mise à jour proposée,
/// une `a_decider` par ligne laissée à la décision. **Interdit pour une simulation.** Ne crée aucun objet
/// métier, n'écrase jamais une correspondance « modifiée depuis la reprise », ne change jamais un `cible_id`
/// déjà posé (stabilité). Idempotent : rejouer ne change rien.
pub async fn enregistrer_correspondances(
    pool: &PgPool,
    etab: Uuid,
    exec: Execution,
    simulation: &RapportSimulation,
) -> Result<u64, ErreurReprise> {
    let mut tx = TenantTx::begin(pool, etab).await?;
    let mode: Option<String> = sqlx::query_scalar(
        "SELECT mode FROM rep_execution WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(exec.id)
    .fetch_optional(&mut *tx)
    .await?;
    match mode.as_deref() {
        Some("simulation") => {
            return Err(ErreurReprise::Refus(
                "une simulation n'enregistre aucune correspondance".into(),
            ))
        }
        None => return Err(ErreurReprise::Refus("exécution inconnue".into())),
        _ => {}
    }
    let mut ecrites = 0u64;
    let etab_txt = etab.to_string();
    for p in &simulation.propositions {
        let cible = Uuid::parse_str(&p.cible_id)
            .map_err(|_| ErreurReprise::Refus("identifiant cible invalide".into()))?;
        let id = Uuid::parse_str(&id_derive(
            &etab_txt,
            "rep_correspondance",
            &p.objet,
            &p.source_id,
        ))
        .expect("uuid dérivé");
        let r = sqlx::query(
            "INSERT INTO rep_correspondance
                 (id, etablissement_id, source, source_objet, source_id, cible_objet, cible_id, regle_code, statut, execution_id, empreinte_source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'derive_sha256', 'creee', $8, $9)
             ON CONFLICT (etablissement_id, source, source_objet, source_id) DO UPDATE
                SET empreinte_source = EXCLUDED.empreinte_source, execution_id = EXCLUDED.execution_id, updated_at = now()
              WHERE rep_correspondance.modifie_depuis_reprise = false
                AND rep_correspondance.statut IN ('creee', 'rapprochee')
                AND rep_correspondance.empreinte_source IS DISTINCT FROM EXCLUDED.empreinte_source",
        )
        .bind(id)
        .bind(etab)
        .bind(SOURCE_NEXT)
        .bind(&p.objet)
        .bind(&p.source_id)
        .bind(&p.cible_objet)
        .bind(cible)
        .bind(exec.id)
        .bind(&p.empreinte)
        .execute(&mut *tx)
        .await?;
        ecrites += r.rows_affected();
        debug_assert!(matches!(p.action, Action::Creee | Action::MiseAJour));
    }
    for x in &simulation.rapprochees {
        let cible = Uuid::parse_str(&x.cible_id)
            .map_err(|_| ErreurReprise::Refus("identifiant cible invalide".into()))?;
        let id = Uuid::parse_str(&id_derive(
            &etab_txt,
            "rep_correspondance",
            &x.objet,
            &x.source_id,
        ))
        .expect("uuid dérivé");
        // Le lien remplace l'éventuelle ligne « à décider » (cible provisoire dérivée) ; un lien déjà posé garde sa
        // cible : seule l'empreinte de la source est tenue à jour.
        let r = sqlx::query(
            "INSERT INTO rep_correspondance
                 (id, etablissement_id, source, source_objet, source_id, cible_objet, cible_id, regle_code, statut, execution_id, empreinte_source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'rapprochee', $9, $10)
             ON CONFLICT (etablissement_id, source, source_objet, source_id) DO UPDATE
                SET cible_id = CASE WHEN rep_correspondance.statut = 'a_decider' THEN EXCLUDED.cible_id ELSE rep_correspondance.cible_id END,
                    regle_code = CASE WHEN rep_correspondance.statut = 'a_decider' THEN EXCLUDED.regle_code ELSE rep_correspondance.regle_code END,
                    statut = 'rapprochee',
                    empreinte_source = EXCLUDED.empreinte_source, execution_id = EXCLUDED.execution_id, updated_at = now()
              WHERE rep_correspondance.modifie_depuis_reprise = false
                AND rep_correspondance.statut IN ('a_decider', 'rapprochee')
                AND (rep_correspondance.statut = 'a_decider'
                     OR rep_correspondance.empreinte_source IS DISTINCT FROM EXCLUDED.empreinte_source)",
        )
        .bind(id)
        .bind(etab)
        .bind(SOURCE_NEXT)
        .bind(&x.objet)
        .bind(&x.source_id)
        .bind(&x.cible_objet)
        .bind(cible)
        .bind(&x.regle)
        .bind(exec.id)
        .bind(&x.empreinte)
        .execute(&mut *tx)
        .await?;
        ecrites += r.rows_affected();
    }
    for a in &simulation.a_decider {
        let cible = Uuid::parse_str(&a.cible_id)
            .map_err(|_| ErreurReprise::Refus("identifiant cible invalide".into()))?;
        let id = Uuid::parse_str(&id_derive(
            &etab_txt,
            "rep_correspondance",
            &a.objet,
            &a.source_id,
        ))
        .expect("uuid dérivé");
        let r = sqlx::query(
            "INSERT INTO rep_correspondance
                 (id, etablissement_id, source, source_objet, source_id, cible_objet, cible_id, regle_code, statut, execution_id, empreinte_source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'a_decider', 'a_decider', $8, $9)
             ON CONFLICT (etablissement_id, source, source_objet, source_id) DO NOTHING",
        )
        .bind(id)
        .bind(etab)
        .bind(SOURCE_NEXT)
        .bind(&a.objet)
        .bind(&a.source_id)
        .bind(&a.cible_objet)
        .bind(cible)
        .bind(exec.id)
        .bind(&a.empreinte)
        .execute(&mut *tx)
        .await?;
        ecrites += r.rows_affected();
    }
    tx.commit().await?;
    Ok(ecrites)
}

/// Date de purge de la zone de cette exécution (MSE-179).
pub async fn noter_zone_videe(
    pool: &PgPool,
    etab: Uuid,
    exec: Execution,
) -> Result<(), ErreurReprise> {
    let mut tx = TenantTx::begin(pool, etab).await?;
    sqlx::query(
        "UPDATE rep_execution SET zone_videe_le = now() WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(exec.id)
    .execute(&mut *tx)
    .await?;
    // Les résumés minimisés de la file « à décider » partent avec la zone (MSE-179).
    crate::reprise_rapprochement::purger_resumes(&mut *tx, etab, exec).await?;
    tx.commit().await?;
    Ok(())
}
