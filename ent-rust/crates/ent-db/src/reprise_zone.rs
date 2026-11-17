//! Zone de reprise `reprise.*` (socle 28, lot R2) : chargement des lignes **comprises** du décodage strict,
//! contrôle de relecture, purge.
//!
//! La zone est une zone de **transit** : tables séparées des tables métier (schéma `reprise`), sans RLS
//! d'établissement ni clé étrangère vers le métier, invisible de tout utilisateur de l'ENT (aucun droit
//! pour `PUBLIC` ni pour le rôle de l'application, migration 2566). Chaque ligne porte l'exécution et
//! l'établissement cible ; **toute requête filtre sur `_execution_id` ET `_etablissement_cible`**.
//! Les noms de tables et de colonnes viennent d'un catalogue statique (`ent_reprise::zone`), jamais d'une
//! donnée. Les colonnes secrètes ne sont jamais chargées (absentes du catalogue).

use crate::reprise::ErreurReprise;
use chrono::{DateTime, Utc};
use ent_reprise::decodage::s3json::NatureS3;
use ent_reprise::decodage::{Decodage, DocumentS3};
use ent_reprise::simulation::reference_document_s3;
use ent_reprise::zone::{colonnes_transit, noms_tables_zone, tables_zone};
use serde_json::Value;
use sqlx::{PgPool, QueryBuilder};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Lignes par table de zone.
pub type Comptes = BTreeMap<String, u64>;

const MAX_BINDS: usize = 60_000;

fn ident(c: &str) -> String {
    format!("\"{}\"", c.replace('"', "\"\""))
}

fn texte(v: &Value, cle: &str) -> Result<String, ErreurReprise> {
    v.get(cle)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ErreurReprise::Refus(format!("document S3 : champ {cle} absent")))
}

fn instant(v: &Value, cle: &str) -> Result<DateTime<Utc>, ErreurReprise> {
    DateTime::parse_from_rfc3339(&texte(v, cle)?)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| ErreurReprise::Refus(format!("document S3 : {cle} n'est pas une date")))
}

fn entier(v: &Value, cle: &str) -> Result<i64, ErreurReprise> {
    v.get(cle)
        .and_then(Value::as_i64)
        .filter(|n| *n >= 0)
        .ok_or_else(|| {
            ErreurReprise::Refus(format!("document S3 : {cle} n'est pas un entier positif"))
        })
}

/// Charge en zone toutes les lignes comprises. Retourne le nombre de lignes par table de zone.
pub async fn charger_zone(
    pool: &PgPool,
    etab: Uuid,
    execution: Uuid,
    d: &Decodage,
) -> Result<Comptes, ErreurReprise> {
    let mut comptes = Comptes::new();
    let mut tx = pool.begin().await?;
    for t in tables_zone() {
        let Some(charge) = d.tables.get(&t.source) else {
            continue;
        };
        if charge.lignes.is_empty() {
            continue;
        }
        if charge.colonnes != t.colonnes {
            return Err(ErreurReprise::Refus(format!(
                "colonnes décodées différentes du catalogue de zone : {}",
                t.source
            )));
        }
        let transit = colonnes_transit(t.attr);
        let par_ligne = transit.len() + t.colonnes.len();
        let lot = (MAX_BINDS / par_ligne).max(1);
        let mut cols: Vec<String> = transit.iter().map(|c| (*c).to_string()).collect();
        cols.extend(t.colonnes.iter().map(|c| ident(c)));
        for paquet in charge.lignes.chunks(lot) {
            let mut qb = QueryBuilder::new(format!("INSERT INTO {} ({}) ", t.nom, cols.join(", ")));
            qb.push_values(paquet, |mut b, l| {
                b.push_bind(execution)
                    .push_bind(etab)
                    .push_bind(l.source_ligne as i64)
                    .push_bind(l.cle.clone())
                    .push_bind(l.empreinte.clone())
                    .push_bind(l.objet.clone())
                    .push_bind(match l.disposition {
                        ent_reprise::perimetre::Disposition::Reprise => "reprise",
                        ent_reprise::perimetre::Disposition::Rapprochement => "rapprochement",
                        _ => "archive",
                    });
                if t.attr {
                    b.push_bind(l.type_feuille.map_or("s", |f| f.code()));
                }
                for c in &l.cellules {
                    b.push_bind(c.clone());
                }
            });
            qb.build().execute(&mut *tx).await?;
        }
        comptes.insert(t.nom.clone(), charge.lignes.len() as u64);
    }
    charger_s3(&mut tx, etab, execution, d, &mut comptes).await?;
    tx.commit().await?;
    Ok(comptes)
}

async fn charger_s3(
    tx: &mut sqlx::PgConnection,
    etab: Uuid,
    execution: Uuid,
    d: &Decodage,
    comptes: &mut Comptes,
) -> Result<(), ErreurReprise> {
    for o in &d.objets_s3 {
        sqlx::query(
            "INSERT INTO reprise.next_s3_objet (_execution_id, _etablissement_cible, cle, taille, sha256, nature)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(execution)
        .bind(etab)
        .bind(&o.objet.cle)
        .bind(o.objet.taille as i64)
        .bind(&o.objet.sha256)
        .bind(format!("{:?}", o.nature))
        .execute(&mut *tx)
        .await?;
        *comptes.entry("reprise.next_s3_objet".into()).or_default() += 1;
    }
    for doc in d.documents_s3.iter().filter(|x| !x.expire) {
        match doc.nature {
            NatureS3::RentreeEnAttente => {
                charger_rentree(tx, etab, execution, doc, comptes).await?
            }
            NatureS3::DemandeEnAttente => {
                charger_demande(tx, etab, execution, doc, comptes).await?
            }
            _ => {}
        }
    }
    Ok(())
}

async fn charger_rentree(
    tx: &mut sqlx::PgConnection,
    etab: Uuid,
    execution: Uuid,
    doc: &DocumentS3,
    comptes: &mut Comptes,
) -> Result<(), ErreurReprise> {
    let (v, reference) = (&doc.valeur, reference_document_s3(&doc.cle));
    sqlx::query(
        "INSERT INTO reprise.next_s3_rentree_depot
             (_execution_id, _etablissement_cible, reference, sender_email, student_name, file_name, content_type, taille,
              file_key, item_id, item_title, establishment_id, establishment_label, created_at, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
    )
    .bind(execution)
    .bind(etab)
    .bind(&reference)
    .bind(texte(v, "senderEmail")?)
    .bind(v.get("studentName").and_then(Value::as_str))
    .bind(texte(v, "fileName")?)
    .bind(texte(v, "contentType")?)
    .bind(entier(v, "size")?)
    .bind(texte(v, "fileKey")?)
    .bind(texte(v, "itemId")?)
    .bind(texte(v, "itemTitle")?)
    .bind(texte(v, "establishmentId")?)
    .bind(texte(v, "establishmentLabel")?)
    .bind(instant(v, "createdAt")?)
    .bind(instant(v, "expiresAt")?)
    .execute(&mut *tx)
    .await?;
    *comptes
        .entry("reprise.next_s3_rentree_depot".into())
        .or_default() += 1;
    for (i, e) in v["recipientEmails"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        sqlx::query(
            "INSERT INTO reprise.next_s3_rentree_destinataire (_execution_id, _etablissement_cible, reference, rang, email)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(execution)
        .bind(etab)
        .bind(&reference)
        .bind(i as i32)
        .bind(e.as_str().unwrap_or_default())
        .execute(&mut *tx)
        .await?;
        *comptes
            .entry("reprise.next_s3_rentree_destinataire".into())
            .or_default() += 1;
    }
    Ok(())
}

async fn charger_demande(
    tx: &mut sqlx::PgConnection,
    etab: Uuid,
    execution: Uuid,
    doc: &DocumentS3,
    comptes: &mut Comptes,
) -> Result<(), ErreurReprise> {
    let (v, reference) = (&doc.valeur, reference_document_s3(&doc.cle));
    let contexte = v.get("parentContext");
    sqlx::query(
        "INSERT INTO reprise.next_s3_demande_attente
             (_execution_id, _etablissement_cible, reference, first_name, last_name, email, phone, subject, description,
              created_at, expires_at, parent_source, parent_matched)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
    )
    .bind(execution)
    .bind(etab)
    .bind(&reference)
    .bind(texte(v, "firstName")?)
    .bind(texte(v, "lastName")?)
    .bind(texte(v, "email")?)
    .bind(texte(v, "phone")?)
    .bind(texte(v, "subject")?)
    .bind(texte(v, "description")?)
    .bind(instant(v, "createdAt")?)
    .bind(instant(v, "expiresAt")?)
    .bind(contexte.and_then(|c| c.get("source")).and_then(Value::as_str))
    .bind(contexte.and_then(|c| c.get("matched")).and_then(Value::as_bool))
    .execute(&mut *tx)
    .await?;
    *comptes
        .entry("reprise.next_s3_demande_attente".into())
        .or_default() += 1;
    let cles = v["attachmentKeys"].as_array().cloned().unwrap_or_default();
    for (i, m) in v["attachmentMeta"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        sqlx::query(
            "INSERT INTO reprise.next_s3_demande_piece
                 (_execution_id, _etablissement_cible, reference, rang, piece_id, file_name, content_type, taille, uploaded_at, cle_objet)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(execution)
        .bind(etab)
        .bind(&reference)
        .bind(i as i32)
        .bind(texte(m, "id")?)
        .bind(texte(m, "fileName")?)
        .bind(texte(m, "contentType")?)
        .bind(entier(m, "size")?)
        .bind(instant(m, "uploadedAt")?)
        .bind(cles.get(i).and_then(Value::as_str).unwrap_or_default())
        .execute(&mut *tx)
        .await?;
        *comptes
            .entry("reprise.next_s3_demande_piece".into())
            .or_default() += 1;
    }
    for (i, e) in contexte
        .and_then(|c| c.get("children"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        sqlx::query(
            "INSERT INTO reprise.next_s3_demande_enfant (_execution_id, _etablissement_cible, reference, rang, ine, nom, prenom, classe)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(execution)
        .bind(etab)
        .bind(&reference)
        .bind(i as i32)
        .bind(texte(e, "ine")?)
        .bind(texte(e, "nom")?)
        .bind(texte(e, "prenom")?)
        .bind(e.get("classe").and_then(Value::as_str))
        .execute(&mut *tx)
        .await?;
        *comptes
            .entry("reprise.next_s3_demande_enfant".into())
            .or_default() += 1;
    }
    Ok(())
}

/// Lignes présentes en zone pour cette exécution, par table (tables vides omises).
pub async fn compter_zone(
    pool: &PgPool,
    etab: Uuid,
    execution: Uuid,
) -> Result<Comptes, ErreurReprise> {
    let mut m = Comptes::new();
    for nom in noms_tables_zone() {
        let n: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {nom} WHERE _execution_id = $1 AND _etablissement_cible = $2"
        ))
        .bind(execution)
        .bind(etab)
        .fetch_one(pool)
        .await?;
        if n > 0 {
            m.insert(nom, n as u64);
        }
    }
    Ok(m)
}

/// Relit la zone et vérifie qu'elle contient exactement ce que le décodage a compris.
pub async fn verifier_zone(
    pool: &PgPool,
    etab: Uuid,
    execution: Uuid,
    attendu: &Comptes,
) -> Result<(), ErreurReprise> {
    let relu = compter_zone(pool, etab, execution).await?;
    if &relu != attendu {
        return Err(ErreurReprise::Refus(
            "la zone relue ne correspond pas aux lignes comprises du décodage".into(),
        ));
    }
    Ok(())
}

/// Vide la zone de cette exécution (MSE-179). Retourne le nombre de lignes supprimées.
pub async fn vider_zone(pool: &PgPool, etab: Uuid, execution: Uuid) -> Result<u64, ErreurReprise> {
    let mut tx = pool.begin().await?;
    let mut total = 0;
    for nom in noms_tables_zone() {
        let r = sqlx::query(&format!(
            "DELETE FROM {nom} WHERE _execution_id = $1 AND _etablissement_cible = $2"
        ))
        .bind(execution)
        .bind(etab)
        .execute(&mut *tx)
        .await?;
        total += r.rows_affected();
    }
    tx.commit().await?;
    Ok(total)
}

/// Résultat d'une simulation exécutée contre la base.
#[derive(Debug)]
pub struct ResultatSimulation {
    pub execution: crate::reprise::Execution,
    pub decodage: ent_reprise::decodage::RapportDecodage,
    pub simulation: ent_reprise::simulation::RapportSimulation,
    pub zone: Comptes,
    pub rapport_empreinte: String,
}

/// **Simulation complète** contre la base : décodage strict de l'export, chargement en zone de transit,
/// relecture de contrôle, simulation avec les correspondances déjà enregistrées, trace de l'exécution
/// (comptes, refus, lignes à décider). **Aucune écriture dans les tables métier** ni dans la table de
/// correspondance. En cas d'erreur après l'ouverture, l'exécution est marquée `echec`.
///
/// Les accès aux tables `rep_*` passent par `crate::reprise` (`TenantTx`) ; ce fichier ne touche que la zone.
pub async fn executer_simulation(
    pool: &PgPool,
    etab: Uuid,
    options: &ent_reprise::decodage::OptionsDecodage,
    cible: crate::reprise::CibleExecution,
    operateur: Option<Uuid>,
) -> Result<ResultatSimulation, ErreurReprise> {
    use crate::reprise::{self, ModeExecution};
    use sha2::{Digest, Sha256};
    let decodage = ent_reprise::decodage::decoder_export(options)
        .map_err(|e| ErreurReprise::Refus(e.to_string()))?;
    let execution = reprise::ouvrir_execution(
        pool,
        etab,
        "next-schema-v1",
        ModeExecution::Simulation,
        cible,
        operateur,
    )
    .await?;
    let suite = async {
        let zone = charger_zone(pool, etab, execution.id, &decodage).await?;
        verifier_zone(pool, etab, execution.id, &zone).await?;
        let existantes = reprise::correspondances(pool, etab).await?;
        // Rapprochement (R3) : instantané de ScolIA (lecture seule, filtre d'établissement) ; le jour de référence est
        // celui de l'instant **fourni** aux options, jamais l'horloge du serveur.
        let cible = crate::reprise_rapprochement::charger_cible(pool, etab).await?;
        let simulation = ent_reprise::simulation::simuler_avec_rapprochement(
            &etab.to_string(),
            &decodage,
            &existantes,
            &cible,
            options.maintenant.date_naive(),
        );
        let json = serde_json::to_string(&(&decodage.rapport, &simulation))
            .map_err(|e| ErreurReprise::Refus(e.to_string()))?;
        let rapport_empreinte = format!("{:x}", Sha256::digest(json.as_bytes()));
        reprise::enregistrer_trace(
            pool,
            etab,
            execution,
            &decodage.rapport,
            &simulation,
            &rapport_empreinte,
        )
        .await?;
        Ok::<_, ErreurReprise>((zone, simulation, rapport_empreinte))
    }
    .await;
    match suite {
        Ok((zone, simulation, rapport_empreinte)) => Ok(ResultatSimulation {
            execution,
            decodage: decodage.rapport,
            simulation,
            zone,
            rapport_empreinte,
        }),
        Err(e) => {
            reprise::marquer_echec(pool, etab, execution).await?;
            Err(e)
        }
    }
}
