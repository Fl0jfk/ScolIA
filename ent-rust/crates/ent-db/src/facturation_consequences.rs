//! Socle 24, lot F2 — **application des conséquences** d'un changement de payeur (§9.2, FAC-176).
//!
//! Les conséquences sont calculées et **proposées** par `facturation_payeurs` quand une répartition
//! devient opposable. Les accepter ne crée que des **brouillons** : un avoir au payeur sortant (par la
//! commande d'avoir du service) et une ligne au payeur entrant (par la commande de facturation du
//! service) ; l'émission de ces pièces reste un geste humain (`facturation.emettre`, N3). Aucune pièce
//! émise n'est jamais modifiée : le déclencheur d'immutabilité de la migration 520 le garantit en base.
//!
//! Limite assumée (F4 la lève) : la ligne du payeur entrant reprend le **tarif de la ligne d'origine**
//! (pas le quotient du payeur entrant) ; l'écart est visible à la comptabilité avant émission.

use chrono::{Datelike, Months, NaiveDate};
use ent_events::{types, AggregateKind};
use ent_types::finances_comptes::{DecisionConsequencesBody, ResultatConsequencesDto};
use serde_json::json;
use sqlx::PgConnection;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::facturation_service::{journaliser_tx, Acte, Chemin};
use crate::finances::{
    conflit, db, inserer_avoirs, inserer_lignes, introuvable, lignes_existantes, publier_finance, AvoirAFaire,
    LigneAFacturer,
};
use crate::outbox::Causalite;
use crate::service::EntDbError;

#[derive(sqlx::FromRow)]
struct ConsequenceRow {
    id: Uuid,
    eleve_id: Uuid,
    sens: String,
    compte_id: Uuid,
    ligne_id: Uuid,
    facture_id: Uuid,
    date_effet: NaiveDate,
    mois_total: i32,
    mois_apres: i32,
    montant_centimes: i64,
    statut: String,
}

#[derive(sqlx::FromRow)]
struct LigneSource {
    article_id: Option<Uuid>,
    categorie: Option<String>,
    libelle: String,
    periode_debut: Option<NaiveDate>,
    periode_fin: Option<NaiveDate>,
    total: i64,
    annee_scolaire: Option<String>,
}

pub(crate) async fn decider_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    role: &str,
    groupe: Uuid,
    b: &DecisionConsequencesBody,
) -> Result<ResultatConsequencesDto, EntDbError> {
    let rows: Vec<ConsequenceRow> = sqlx::query_as(
        "SELECT id, eleve_id, sens, compte_id, ligne_id, facture_id, date_effet, mois_total, mois_apres,
                montant_centimes, statut
           FROM ent_fin_consequence
          WHERE etablissement_id = $1 AND designation_groupe_id = $2
          ORDER BY sens, id FOR UPDATE",
    )
    .bind(etab)
    .bind(groupe)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    if rows.is_empty() {
        return Err(introuvable());
    }
    if rows.iter().any(|r| r.statut != "proposee") {
        return Err(conflit("ces conséquences ont déjà été décidées"));
    }
    let eleve = rows[0].eleve_id;
    let date_effet = rows[0].date_effet;
    if !b.appliquer {
        let motif = b.motif.as_deref().map(str::trim).filter(|m| !m.is_empty());
        let Some(motif) = motif else {
            return Err(crate::finances::invalide("un refus se motive (300 caractères)"));
        };
        sqlx::query(
            "UPDATE ent_fin_consequence SET statut = 'refusee', decide_par = $3, decide_le = now(), motif_refus = $4
              WHERE etablissement_id = $1 AND designation_groupe_id = $2 AND statut = 'proposee'",
        )
        .bind(etab)
        .bind(groupe)
        .bind(actor)
        .bind(motif.chars().take(300).collect::<String>())
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        journaliser_tx(
            conn,
            &Acte::new(etab, Some(actor), role, Chemin::Comptes, "consequences.refuser", "consequence")
                .objet(groupe)
                .eleve(eleve)
                .motif(motif),
        )
        .await?;
        return Ok(ResultatConsequencesDto {
            appliquees: 0,
            refusees: rows.len() as i32,
            avoirs_brouillons: Vec::new(),
            factures_brouillons: Vec::new(),
        });
    }

    // M3 : verrou des foyers concernés (ordre stable) **avant** de lire les lignes existantes, pour que deux
    // décisions concurrentes ne créditent pas deux fois la même ligne.
    let mut foyers: Vec<Uuid> = Vec::new();
    for r in &rows {
        let f: Option<(Uuid,)> = sqlx::query_as(
            "SELECT foyer_id FROM ent_fin_compte WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(r.compte_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if let Some((f,)) = f {
            if !foyers.contains(&f) {
                foyers.push(f);
            }
        }
    }
    foyers.sort();
    for f in &foyers {
        crate::finances::verrouiller_foyer(conn, etab, *f).await?;
    }

    // 1. Avoirs au payeur sortant (par pièce d'origine).
    let ids_lignes: Vec<Uuid> = rows.iter().filter(|r| r.sens == "credit").map(|r| r.ligne_id).collect();
    let existantes = lignes_existantes(conn, etab, &ids_lignes).await?;
    let mut avoirs: Vec<AvoirAFaire> = Vec::new();
    let mut annee_avoir: Option<String> = None;
    for r in rows.iter().filter(|r| r.sens == "credit") {
        let Some(l) = existantes.iter().find(|l| l.id == r.ligne_id) else {
            continue;
        };
        let signe = if l.total < 0 { -1 } else { 1 };
        // Jamais plus que ce qui reste à créditer sur la ligne : reste **signé** (M3), valable aussi pour une
        // ligne de réduction (total négatif) ; un reste de signe contraire ne laisse rien à créditer.
        let reste = l.total - l.deja_credite;
        let disponible = if reste.signum() == l.total.signum() { reste.abs() } else { 0 };
        let credit = signe * r.montant_centimes.min(disponible);
        avoirs.push(AvoirAFaire {
            origine: l.clone(),
            eleve_id: Some(r.eleve_id),
            credit,
            libelle: format!("Changement de payeur au {} — {}", r.date_effet.format("%d/%m/%Y"), l.libelle),
            source_cle: format!("chgpayeur:{}", r.id),
        });
        let annee: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT annee_scolaire FROM ent_fac_facture WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(r.facture_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        annee_avoir = annee_avoir.or(annee.and_then(|a| a.0));
    }
    let res_avoirs = inserer_avoirs(
        conn,
        etab,
        annee_avoir.as_deref(),
        "Changement de payeur",
        avoirs,
        Some(actor),
        Causalite::default(),
    )
    .await?;

    // 2. Lignes au payeur entrant (brouillons, une pièce par foyer entrant).
    let mut lignes: Vec<LigneAFacturer> = Vec::new();
    let mut annee_debit: Option<String> = None;
    let mut foyers: BTreeMap<Uuid, Uuid> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.sens == "debit") {
        let src: Option<LigneSource> = sqlx::query_as(
            "SELECT l.article_id, l.categorie, l.libelle, l.periode_debut, l.periode_fin,
                    (l.quantite::BIGINT * l.montant_unitaire_centimes)::BIGINT AS total, f.annee_scolaire
               FROM ent_fac_ligne l
               JOIN ent_fac_facture f ON f.etablissement_id = l.etablissement_id AND f.id = l.facture_id
              WHERE l.etablissement_id = $1 AND l.id = $2",
        )
        .bind(etab)
        .bind(r.ligne_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some(src) = src else { continue };
        let foyer = match foyers.get(&r.compte_id) {
            Some(f) => *f,
            None => {
                let (f,): (Uuid,) = sqlx::query_as(
                    "SELECT foyer_id FROM ent_fin_compte WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(r.compte_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                foyers.insert(r.compte_id, f);
                f
            }
        };
        let signe = if src.total < 0 { -1 } else { 1 };
        // Premier mois transféré : le mois qui suit ceux que garde l'ancien payeur.
        let periode = match (src.periode_debut, src.periode_fin) {
            (Some(deb), Some(fin)) => {
                let gardes = (r.mois_total - r.mois_apres).max(0) as u32;
                deb.with_day(1)
                    .and_then(|d| d.checked_add_months(Months::new(gardes)))
                    .map(|debut| (debut, fin))
            }
            _ => None,
        };
        annee_debit = annee_debit.or(src.annee_scolaire.clone());
        lignes.push(LigneAFacturer {
            foyer_id: foyer,
            eleve_id: Some(r.eleve_id),
            article_id: src.article_id,
            categorie: src.categorie.clone().unwrap_or_else(|| "frais".into()),
            libelle: format!("Changement de payeur au {} — {}", r.date_effet.format("%d/%m/%Y"), src.libelle),
            quantite: 1,
            unitaire: signe * r.montant_centimes,
            source_cle: format!("chgpayeur:{}", r.id),
            periode,
        });
    }
    let res_lignes = inserer_lignes(
        conn,
        etab,
        annee_debit.as_deref(),
        "changement_payeur",
        "Changement de payeur",
        lignes,
        Some(actor),
        Causalite::default(),
    )
    .await?;

    let n = rows.len() as i32;
    sqlx::query(
        "UPDATE ent_fin_consequence SET statut = 'appliquee', decide_par = $3, decide_le = now()
          WHERE etablissement_id = $1 AND designation_groupe_id = $2 AND statut = 'proposee'",
    )
    .bind(etab)
    .bind(groupe)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journaliser_tx(
        conn,
        &Acte::new(etab, Some(actor), role, Chemin::Comptes, "consequences.appliquer", "consequence")
            .objet(groupe)
            .eleve(eleve)
            .detail(&format!(
                "changement de payeur au {date_effet} : {} avoir(s) et {} pièce(s) en brouillon, à émettre",
                res_avoirs.factures.len(),
                res_lignes.factures.len()
            )),
    )
    .await?;
    publier_finance(
        conn,
        etab,
        types::FACTURATION_CHANGEMENT_PAYEUR_APPLIQUE,
        AggregateKind::Eleve,
        eleve,
        Some(eleve),
        json!({ "groupe_id": groupe, "date_effet": date_effet, "avoirs": res_avoirs.factures.len(),
                "pieces": res_lignes.factures.len() }),
        Some(actor),
        Causalite::default(),
    )
    .await?;
    Ok(ResultatConsequencesDto {
        appliquees: n,
        refusees: 0,
        avoirs_brouillons: res_avoirs.factures,
        factures_brouillons: res_lignes.factures,
    })
}
