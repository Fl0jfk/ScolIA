//! Finances reliées (S2-9), suite : prélèvements SEPA (FIN-11), pré-notification,
//! rejets (FIN-12), règlements manuels et lettrage (FIN-08), relances graduées
//! avec confirmation humaine (FIN-13), export comptable (FIN-16).
//!
//! Règles :
//! * un prélèvement par **échéance** non couverte arrivée à terme, pour les
//!   foyers en paiement par prélèvement avec un mandat actif ; **pré-notification**
//!   au moins `jours_prenotification` jours avant l'exécution (paramètre) ;
//! * fichier `pain.008.001.02` (CORE) ou CSV ; seuls les prélèvements
//!   pré-notifiés partent ; l'IBAN complet ne sort que dans ce fichier ;
//! * **rejet** : échéance rouverte (re-présentée au lot suivant), frais
//!   éventuels facturés, payeur notifié ; motif définitif ou trop de rejets →
//!   **retour au paiement manuel** ;
//! * **règlement manuel** imputé sur les factures les plus anciennes, trop-perçu
//!   gardé au crédit du foyer ; une facture soldée reçoit un **code de lettrage** ;
//! * **relances** : proposées par le système (jamais avant l'échéance), envoyées
//!   seulement après confirmation d'une personne habilitée ;
//! * **export comptable** `scolia_ecritures_v1` d'une période close ; chaque pièce
//!   n'est exportée qu'une fois ; empreinte SHA-256 conservée.

#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use domain_eleve::{
    masquer_iban, plan_sepa_crediteur, render_sepa_csv, render_sepa_pain008, PrelevementSepa,
    PrelevementStatut, SepaLigneExport,
};
use ent_auth::Action;
use ent_events::{types, AggregateKind};
use ent_types::finances::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgConnection;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

use crate::finances::{
    aujourd_hui, conflit, db, echeances_couvertes, emettre_piece_tx, euros, facture_row,
    inserer_lignes, introuvable, invalide, mode_paiement, notifier_payeurs, opt_texte, paliers_tx,
    params_tx, poser_mode, prochain_numero, publier, verrou_foyer, FactureRow, LigneAFacturer,
    SELECT_FACTURE,
};
use crate::finances_calcul::{self as calcul, Ecriture};
use crate::outbox::Causalite;
use crate::service::{EntDbError, PgEnt};

const T_MANDAT: &str = "ent_fac_sepa_mandat";

/// Solde la facture si plus rien n'est dû ; pose alors le code de lettrage sur
/// toutes ses imputations. Renvoie le code posé.
pub(crate) async fn solder_si_paye(
    conn: &mut PgConnection,
    etab: Uuid,
    facture: Uuid,
) -> Result<Option<String>, EntDbError> {
    let f = facture_row(conn, etab, facture).await?;
    if f.statut != "emise" || f.type_piece != "facture" || f.reste() > 0 {
        return Ok(None);
    }
    let n = prochain_numero(conn, etab, "lettrage").await?;
    let code = format!("L{n:06}");
    sqlx::query(
        "UPDATE ent_fac_facture SET statut = 'soldee', soldee_at = now() WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(facture)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_fin_lettrage SET code = $3 WHERE etablissement_id = $1 AND facture_id = $2 AND code IS NULL",
    )
    .bind(etab)
    .bind(facture)
    .bind(&code)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    publier(
        conn,
        etab,
        types::FAC_FACTURE_SOLDEE,
        AggregateKind::Facture,
        facture,
        None,
        json!({ "facture_id": facture, "foyer_id": f.foyer_id, "lettrage": code }),
        None,
        Causalite::default(),
    )
    .await?;
    Ok(Some(code))
}

/// Encaissement imputé sur une facture (+ trace d'imputation pour le lettrage).
async fn encaisser_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    facture: Uuid,
    montant: i64,
    moyen: &str,
    reference: Option<&str>,
    encaisse_le: NaiveDate,
    reglement: Option<Uuid>,
    actor: Option<Uuid>,
) -> Result<(Uuid, Option<String>), EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_fac_encaissement (id, etablissement_id, facture_id, montant_centimes, moyen, reference,
                encaisse_at, created_by_user_id, reglement_id)
         VALUES ($1, $2, $3, $4, $5, $6, ($7::DATE)::TIMESTAMPTZ + interval '12 hours', $8, $9)",
    )
    .bind(id)
    .bind(etab)
    .bind(facture)
    .bind(montant)
    .bind(moyen)
    .bind(reference)
    .bind(encaisse_le)
    .bind(actor)
    .bind(reglement)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_fin_lettrage (id, etablissement_id, facture_id, piece, piece_id, montant_centimes)
         VALUES ($1, $2, $3, 'encaissement', $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(facture)
    .bind(id)
    .bind(montant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let code = solder_si_paye(conn, etab, facture).await?;
    Ok((id, code))
}

/// Facture de frais (rejet, relance) : créée puis émise dans la même transaction
/// (le geste humain qui la déclenche vaut décision). Idempotente par `source`.
async fn facturer_frais(
    conn: &mut PgConnection,
    etab: Uuid,
    foyer: Uuid,
    montant: i64,
    libelle: &str,
    source: &str,
    actor: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    if montant <= 0 {
        return Ok(None);
    }
    let p = params_tx(conn, etab).await?;
    let annee = crate::finances::annee_de(&p, aujourd_hui());
    let r = inserer_lignes(
        conn,
        etab,
        Some(&annee),
        "frais",
        "Frais",
        vec![LigneAFacturer {
            foyer_id: foyer,
            eleve_id: None,
            article_id: None,
            categorie: "frais".into(),
            libelle: libelle.into(),
            quantite: 1,
            unitaire: montant,
            source_cle: source.into(),
            periode: None,
        }],
        Some(actor),
        Causalite::default(),
    )
    .await?;
    let Some(id) = r.factures.first().copied() else {
        return Ok(None);
    };
    let f = facture_row(conn, etab, id).await?;
    if f.statut == "brouillon" {
        emettre_piece_tx(
            conn,
            etab,
            &p,
            &f,
            aujourd_hui(),
            Some(actor),
            Causalite::default(),
        )
        .await?;
    }
    Ok(Some(id))
}

#[derive(sqlx::FromRow)]
struct PrelevementRow {
    id: Uuid,
    foyer_id: Uuid,
    foyer: Option<String>,
    facture_id: Uuid,
    facture_numero: String,
    echeance_id: Option<Uuid>,
    montant_centimes: i64,
    date_execution: NaiveDate,
    statut: String,
    presentation: i16,
    end_to_end_id: String,
    mandat_id: Uuid,
    rum: String,
    iban: String,
    bic: Option<String>,
    titulaire: String,
    signe_at: DateTime<Utc>,
    prenotification_id: Option<Uuid>,
    motif_rejet: Option<String>,
    created_at: DateTime<Utc>,
}

const SELECT_PRELEVEMENT: &str = "SELECT p.id, p.foyer_id, fo.libelle AS foyer, p.facture_id, f.numero AS facture_numero,
        p.echeance_id, p.montant_centimes, p.date_execution, p.statut, p.presentation, p.end_to_end_id,
        p.mandat_id, m.rum, m.iban, m.bic, m.titulaire, m.signe_at, p.prenotification_id, p.motif_rejet, p.created_at
   FROM ent_fac_sepa_prelevement p
   JOIN ent_fac_sepa_mandat m ON m.id = p.mandat_id AND m.etablissement_id = p.etablissement_id
   JOIN ent_fac_facture f ON f.id = p.facture_id AND f.etablissement_id = p.etablissement_id
   LEFT JOIN ent_foyer fo ON fo.id = p.foyer_id AND fo.etablissement_id = p.etablissement_id";

impl PrelevementRow {
    fn dto(
        &self,
        champs: &crate::champs_chiffres::ChampsSensibles,
    ) -> Result<PrelevementDto, EntDbError> {
        let iban = champs.ouvrir(T_MANDAT, "iban", self.mandat_id, &self.iban)?;
        Ok(PrelevementDto {
            id: self.id,
            foyer_id: self.foyer_id,
            foyer: self
                .foyer
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("Foyer {}", &self.foyer_id.simple().to_string()[..8])),
            facture_id: self.facture_id,
            facture_numero: self.facture_numero.clone(),
            echeance_id: self.echeance_id,
            montant_centimes: self.montant_centimes,
            date_execution: self.date_execution,
            statut: self.statut.clone(),
            presentation: self.presentation,
            end_to_end_id: self.end_to_end_id.clone(),
            rum: self.rum.clone(),
            iban_masque: masquer_iban(&iban),
            prenotifie: self.prenotification_id.is_some(),
            motif_rejet: self.motif_rejet.clone(),
        })
    }
}

impl PgEnt {
    // -- Prélèvements (FIN-11) ---------------------------------------------------

    /// Prépare les prélèvements des échéances arrivées à terme à `date_execution`
    /// et envoie les pré-notifications. Idempotent (une échéance n'a qu'un
    /// prélèvement en cours).
    pub async fn preparer_prelevements_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &PreparationBody,
    ) -> Result<RapportPreparationDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let p = params_tx(&mut tx, etab).await?;
        let today = aujourd_hui();
        if (b.date_execution - today).num_days() < p.jours_prenotification as i64 {
            return Err(invalide(
                "date d'exécution trop proche : délai de pré-notification non respecté",
            ));
        }
        let foyers: Vec<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT DISTINCT f.foyer_id, fo.libelle FROM ent_fac_facture f
               LEFT JOIN ent_foyer fo ON fo.id = f.foyer_id AND fo.etablissement_id = f.etablissement_id
              WHERE f.etablissement_id = $1 AND f.statut = 'emise' AND f.type_piece = 'facture'
              ORDER BY f.foyer_id",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut rapport = RapportPreparationDto {
            date_execution: Some(b.date_execution),
            ..Default::default()
        };
        for (foyer, libelle) in foyers {
            verrou_foyer(&mut tx, etab, foyer).await?;
            let factures: Vec<FactureRow> = sqlx::query_as(&format!(
                "{SELECT_FACTURE} WHERE f.etablissement_id = $1 AND f.foyer_id = $2 AND f.statut = 'emise'
                   AND f.type_piece = 'facture' ORDER BY f.emise_at, f.created_at"
            ))
            .bind(etab)
            .bind(foyer)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let mut dues = Vec::new();
            for f in &factures {
                for (e, couvert) in echeances_couvertes(&mut tx, etab, f).await? {
                    if e.date_echeance <= b.date_execution
                        && couvert < e.montant_centimes
                        && e.prelevement.is_none()
                    {
                        dues.push((f.id, f.numero.clone(), e.id, e.montant_centimes - couvert));
                    }
                }
            }
            if dues.is_empty() {
                continue;
            }
            let nom = libelle
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("Foyer {}", &foyer.simple().to_string()[..8]));
            if mode_paiement(&mut tx, etab, foyer).await? == "manuel" {
                rapport.ecartes.push(FoyerEcarteDto {
                    foyer_id: foyer,
                    foyer: nom,
                    motif: "paiement manuel".into(),
                });
                continue;
            }
            let mandat: Option<(Uuid, String)> = sqlx::query_as(
                "SELECT id, rum FROM ent_fac_sepa_mandat
                  WHERE etablissement_id = $1 AND foyer_id = $2 AND statut = 'actif'",
            )
            .bind(etab)
            .bind(foyer)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some((mandat_id, rum)) = mandat else {
                rapport.ecartes.push(FoyerEcarteDto {
                    foyer_id: foyer,
                    foyer: nom,
                    motif: "aucun mandat SEPA actif".into(),
                });
                continue;
            };
            let mut ids = Vec::new();
            let mut total = 0;
            for (facture_id, numero, echeance_id, montant) in dues {
                let (rejets,): (i64,) = sqlx::query_as(
                    "SELECT count(*) FROM ent_fac_sepa_prelevement
                      WHERE etablissement_id = $1 AND echeance_id = $2 AND statut = 'rejete'",
                )
                .bind(etab)
                .bind(echeance_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                let id = Uuid::new_v4();
                let e2e = format!(
                    "{}-{}",
                    numero.chars().take(20).collect::<String>(),
                    &id.simple().to_string()[..10]
                );
                sqlx::query(
                    "INSERT INTO ent_fac_sepa_prelevement (id, etablissement_id, mandat_id, facture_id, foyer_id,
                            montant_centimes, date_execution, end_to_end_id, statut, created_by_user_id,
                            echeance_id, presentation)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'prevu', $9, $10, $11)",
                )
                .bind(id)
                .bind(etab)
                .bind(mandat_id)
                .bind(facture_id)
                .bind(foyer)
                .bind(montant)
                .bind(b.date_execution)
                .bind(&e2e)
                .bind(actor)
                .bind(echeance_id)
                .bind((rejets + 1) as i16)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                ids.push(id);
                total += montant;
            }
            let (prenotif,): (Uuid,) = sqlx::query_as(
                "INSERT INTO ent_fin_prenotification (id, etablissement_id, foyer_id, mandat_id, date_execution,
                        montant_centimes, nb_prelevements, created_by)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                 ON CONFLICT (etablissement_id, foyer_id, date_execution) DO UPDATE
                    SET montant_centimes = ent_fin_prenotification.montant_centimes + EXCLUDED.montant_centimes,
                        nb_prelevements = ent_fin_prenotification.nb_prelevements + EXCLUDED.nb_prelevements,
                        envoyee_le = now()
                 RETURNING id",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(foyer)
            .bind(mandat_id)
            .bind(b.date_execution)
            .bind(total)
            .bind(ids.len() as i32)
            .bind(actor)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_fac_sepa_prelevement SET prenotification_id = $3
                  WHERE etablissement_id = $1 AND id = ANY($2)",
            )
            .bind(etab)
            .bind(&ids)
            .bind(prenotif)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let evt = publier(
                &mut tx,
                etab,
                types::FAC_SEPA_PRENOTIFICATION,
                AggregateKind::Facture,
                foyer,
                None,
                json!({ "foyer_id": foyer, "date_execution": b.date_execution, "montant_centimes": total,
                        "prelevements": ids.len(), "rum": rum }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
            notifier_payeurs(
                &mut tx,
                etab,
                foyer,
                evt.id,
                "fin.prenotification",
                "Prélèvement à venir",
                &format!(
                    "Un prélèvement de {} sera présenté le {} sur le mandat {rum}.",
                    euros(total),
                    b.date_execution.format("%d/%m/%Y")
                ),
            )
            .await?;
            rapport.prelevements_crees += ids.len() as i64;
            rapport.prenotifications += 1;
            rapport.montant_centimes += total;
        }
        if rapport.prelevements_crees > 0 {
            publier(
                &mut tx,
                etab,
                types::FAC_SEPA_PRELEVEMENT,
                AggregateKind::Facture,
                etab,
                None,
                json!({ "date_execution": b.date_execution, "prelevements": rapport.prelevements_crees,
                        "montant_centimes": rapport.montant_centimes }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(rapport)
    }

    pub async fn prelevements_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        statut: Option<String>,
        date: Option<NaiveDate>,
    ) -> Result<Vec<PrelevementDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<PrelevementRow> = sqlx::query_as(&format!(
            "{SELECT_PRELEVEMENT} WHERE p.etablissement_id = $1 AND ($2::TEXT IS NULL OR p.statut = $2)
               AND ($3::DATE IS NULL OR p.date_execution = $3)
             ORDER BY p.date_execution DESC, p.created_at DESC LIMIT 2000"
        ))
        .bind(etab)
        .bind(statut)
        .bind(date)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let champs = crate::champs_chiffres::ChampsSensibles::depuis_env();
        rows.iter().map(|r| r.dto(&champs)).collect()
    }

    /// Fichier de prélèvements d'une date d'exécution (pré-notifiés seulement).
    /// Renvoie (contenu, type `pain008` ou `csv`). Les lignes passent à `exporte`.
    pub async fn exporter_prelevements_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ExportPrelevementsBody,
    ) -> Result<(Vec<u8>, &'static str), EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let format = b
            .format
            .as_deref()
            .unwrap_or("pain008")
            .trim()
            .to_ascii_lowercase();
        let pain = match format.as_str() {
            "pain008" | "pain.008" | "xml" => true,
            "csv" => false,
            _ => return Err(invalide("format : pain008 ou csv")),
        };
        let mut tx = self.fin_tx(etab).await?;
        let p = params_tx(&mut tx, etab).await?;
        let crediteur = if pain {
            Some(
                plan_sepa_crediteur(
                    p.crediteur_nom.as_deref().unwrap_or(""),
                    p.crediteur_iban.as_deref().unwrap_or(""),
                    p.crediteur_bic.as_deref().unwrap_or(""),
                    p.crediteur_ics.as_deref().unwrap_or(""),
                )
                .map_err(|_| {
                    invalide("créancier SEPA incomplet : nom, IBAN, BIC et ICS dans les paramètres")
                })?,
            )
        } else {
            None
        };
        let rows: Vec<PrelevementRow> = sqlx::query_as(&format!(
            "{SELECT_PRELEVEMENT} WHERE p.etablissement_id = $1 AND p.statut = 'prevu' AND p.date_execution = $2
               AND p.prenotification_id IS NOT NULL ORDER BY p.created_at FOR UPDATE OF p"
        ))
        .bind(etab)
        .bind(b.date_execution)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        if rows.is_empty() {
            return Err(conflit(
                "aucun prélèvement pré-notifié à exporter pour cette date",
            ));
        }
        let champs = crate::champs_chiffres::ChampsSensibles::depuis_env();
        let mut lignes = Vec::new();
        let mut seq: HashMap<Uuid, &'static str> = HashMap::new();
        for r in &rows {
            if let std::collections::hash_map::Entry::Vacant(place) = seq.entry(r.mandat_id) {
                let (avant,): (i64,) = sqlx::query_as(
                    "SELECT count(*) FROM ent_fac_sepa_prelevement
                      WHERE etablissement_id = $1 AND mandat_id = $2 AND statut IN ('exporte', 'execute')",
                )
                .bind(etab)
                .bind(r.mandat_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                place.insert(if avant == 0 { "FRST" } else { "RCUR" });
            }
            lignes.push(SepaLigneExport {
                prelevement: PrelevementSepa {
                    id: r.id,
                    etablissement_id: etab,
                    mandat_id: r.mandat_id,
                    facture_id: r.facture_id,
                    foyer_id: r.foyer_id,
                    montant_centimes: r.montant_centimes,
                    date_execution: r.date_execution,
                    end_to_end_id: r.end_to_end_id.clone(),
                    statut: PrelevementStatut::Prevu,
                    encaissement_id: None,
                    motif_rejet: None,
                    rejete_at: None,
                    created_by_user_id: None,
                    created_at: r.created_at,
                },
                rum: r.rum.clone(),
                iban: champs.ouvrir(T_MANDAT, "iban", r.mandat_id, &r.iban)?,
                bic: champs.ouvrir_opt(T_MANDAT, "bic", r.mandat_id, r.bic.clone())?,
                titulaire: champs.ouvrir(T_MANDAT, "titulaire", r.mandat_id, &r.titulaire)?,
                mandat_signe_at: r.signe_at,
                facture_numero: r.facture_numero.clone(),
            });
        }
        let (contenu, nature) = match &crediteur {
            Some(c) => {
                let n = prochain_numero(&mut tx, etab, "sepa-lot").await?;
                let msg_id = format!("SDD-{}-{n:06}", aujourd_hui().format("%Y%m%d"));
                (
                    render_sepa_pain008(c, &lignes, &msg_id, Utc::now(), &seq)?,
                    "pain008",
                )
            }
            None => (render_sepa_csv(&lignes), "csv"),
        };
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        sqlx::query(
            "UPDATE ent_fac_sepa_prelevement SET statut = 'exporte'
              WHERE etablissement_id = $1 AND id = ANY($2) AND statut = 'prevu'",
        )
        .bind(etab)
        .bind(&ids)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::FAC_SEPA_EXPORT,
            AggregateKind::Facture,
            etab,
            None,
            json!({ "date_execution": b.date_execution, "nb": ids.len(), "format": nature,
                    "montant_centimes": rows.iter().map(|r| r.montant_centimes).sum::<i64>() }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok((contenu, nature))
    }

    /// Constate l'exécution des prélèvements exportés d'une date passée (aucun
    /// rejet reçu) : encaissements, lettrage.
    pub async fn constater_execution_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ExecutionBody,
    ) -> Result<RapportExecutionDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        if b.date_execution > aujourd_hui() {
            return Err(invalide(
                "exécution constatée seulement à partir de la date d'échéance",
            ));
        }
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, Uuid, Uuid, i64, String)> = sqlx::query_as(
            "SELECT id, foyer_id, facture_id, montant_centimes, end_to_end_id FROM ent_fac_sepa_prelevement
              WHERE etablissement_id = $1 AND statut = 'exporte' AND date_execution = $2
              ORDER BY foyer_id, created_at FOR UPDATE",
        )
        .bind(etab)
        .bind(b.date_execution)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut rapport = RapportExecutionDto::default();
        for (id, foyer, facture, montant, e2e) in rows {
            verrou_foyer(&mut tx, etab, foyer).await?;
            let (enc, _) = encaisser_tx(
                &mut tx,
                etab,
                facture,
                montant,
                "prelevement",
                Some(&e2e),
                b.date_execution,
                None,
                Some(actor),
            )
            .await?;
            sqlx::query(
                "UPDATE ent_fac_sepa_prelevement SET statut = 'execute', encaissement_id = $3
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(enc)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            rapport.executes += 1;
            rapport.montant_centimes += montant;
        }
        if rapport.executes > 0 {
            publier(
                &mut tx,
                etab,
                types::FAC_SEPA_EXECUTE,
                AggregateKind::Facture,
                etab,
                None,
                json!({ "date_execution": b.date_execution, "nb": rapport.executes,
                        "montant_centimes": rapport.montant_centimes }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(rapport)
    }

    // -- Rejets (FIN-12) ---------------------------------------------------------

    /// Traite le rejet d'un prélèvement exporté : échéance rouverte, frais,
    /// notification, retour au paiement manuel si motif définitif ou rejets répétés.
    pub async fn traiter_rejet_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        prelevement: Uuid,
        b: &RejetBody,
    ) -> Result<RejetDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let code = b.code.trim().to_ascii_uppercase();
        let (libelle, definitif) =
            calcul::motif_rejet(&code).ok_or(invalide("code de rejet SEPA inconnu"))?;
        let mut tx = self.fin_tx(etab).await?;
        let r: Option<(Uuid, Uuid, Option<Uuid>, i64, String)> = sqlx::query_as(
            "SELECT foyer_id, facture_id, echeance_id, montant_centimes, statut FROM ent_fac_sepa_prelevement
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(prelevement)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (foyer, facture, echeance, montant, statut) = r.ok_or_else(introuvable)?;
        if statut != "exporte" {
            return Err(conflit(
                "seul un prélèvement exporté (transmis à la banque) peut être rejeté",
            ));
        }
        verrou_foyer(&mut tx, etab, foyer).await?;
        let p = params_tx(&mut tx, etab).await?;
        sqlx::query(
            "UPDATE ent_fac_sepa_prelevement SET statut = 'rejete', motif_rejet = $3, rejete_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(prelevement)
        .bind(&code)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let (rejets,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_fin_rejet
              WHERE etablissement_id = $1 AND foyer_id = $2 AND traite_le > now() - interval '365 days'",
        )
        .bind(etab)
        .bind(foyer)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let manuel = definitif || rejets + 1 >= p.rejets_avant_manuel as i64;
        let suite = if manuel { "manuel" } else { "representation" };
        if manuel {
            poser_mode(
                &mut tx,
                etab,
                foyer,
                "manuel",
                Some(&format!("rejet de prélèvement ({code})")),
                Some(actor),
            )
            .await?;
        }
        let frais_facture = facturer_frais(
            &mut tx,
            etab,
            foyer,
            p.frais_rejet_centimes,
            &format!("Frais de rejet de prélèvement ({code})"),
            &format!("rejet:{prelevement}:frais"),
            actor,
        )
        .await?;
        let id = Uuid::new_v4();
        let (traite_le,): (DateTime<Utc>,) = sqlx::query_as(
            "INSERT INTO ent_fin_rejet (id, etablissement_id, prelevement_id, foyer_id, facture_id, echeance_id,
                    code_motif, libelle_motif, montant_centimes, frais_centimes, facture_frais_id, suite, traite_par)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING traite_le",
        )
        .bind(id)
        .bind(etab)
        .bind(prelevement)
        .bind(foyer)
        .bind(facture)
        .bind(echeance)
        .bind(&code)
        .bind(libelle)
        .bind(montant)
        .bind(if frais_facture.is_some() { p.frais_rejet_centimes } else { 0 })
        .bind(frais_facture)
        .bind(suite)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            if e.as_database_error().and_then(|d| d.code()).is_some_and(|c| c == "23505") {
                conflit("rejet déjà traité")
            } else {
                db(e)
            }
        })?;
        let evt = publier(
            &mut tx,
            etab,
            types::FAC_SEPA_REJET,
            AggregateKind::Facture,
            facture,
            None,
            json!({ "prelevement_id": prelevement, "facture_id": facture, "foyer_id": foyer, "code": code,
                    "suite": suite, "frais_facture_id": frais_facture }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        notifier_payeurs(
            &mut tx,
            etab,
            foyer,
            evt.id,
            "fin.rejet",
            "Prélèvement rejeté",
            &format!(
                "Le prélèvement de {} a été rejeté par votre banque ({libelle}). {}",
                euros(montant),
                if manuel {
                    "Merci de régler cette échéance par un autre moyen ; les prochaines échéances ne seront plus prélevées."
                } else {
                    "L'échéance sera présentée à nouveau au prochain prélèvement."
                }
            ),
        )
        .await?;
        let foyer_nom = facture_row(&mut tx, etab, facture).await?.libelle_foyer();
        tx.commit().await.map_err(db)?;
        Ok(RejetDto {
            id,
            prelevement_id: prelevement,
            foyer_id: foyer,
            foyer: foyer_nom,
            facture_id: facture,
            code_motif: code,
            libelle_motif: libelle.into(),
            montant_centimes: montant,
            frais_centimes: if frais_facture.is_some() {
                p.frais_rejet_centimes
            } else {
                0
            },
            facture_frais_id: frais_facture,
            suite: suite.into(),
            traite_le,
        })
    }

    pub async fn rejets_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<RejetDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, Uuid, Uuid, Option<String>, Uuid, String, String, i64, i64, Option<Uuid>, String, DateTime<Utc>)> =
            sqlx::query_as(
                "SELECT r.id, r.prelevement_id, r.foyer_id, fo.libelle, r.facture_id, r.code_motif, r.libelle_motif,
                        r.montant_centimes, r.frais_centimes, r.facture_frais_id, r.suite, r.traite_le
                   FROM ent_fin_rejet r
                   LEFT JOIN ent_foyer fo ON fo.id = r.foyer_id AND fo.etablissement_id = r.etablissement_id
                  WHERE r.etablissement_id = $1 ORDER BY r.traite_le DESC LIMIT 500",
            )
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| RejetDto {
                id: r.0,
                prelevement_id: r.1,
                foyer_id: r.2,
                foyer: r
                    .3
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| format!("Foyer {}", &r.2.simple().to_string()[..8])),
                facture_id: r.4,
                code_motif: r.5,
                libelle_motif: r.6,
                montant_centimes: r.7,
                frais_centimes: r.8,
                facture_frais_id: r.9,
                suite: r.10,
                traite_le: r.11,
            })
            .collect())
    }

    // -- Règlements manuels et lettrage (FIN-08) ---------------------------------

    pub async fn enregistrer_reglement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ReglementBody,
    ) -> Result<ReglementDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        if !["especes", "cheque", "virement", "cb", "autre"].contains(&b.moyen.as_str()) {
            return Err(invalide("moyen : especes, cheque, virement, cb, autre"));
        }
        if !(1..=100_000_000).contains(&b.montant_centimes) {
            return Err(invalide("montant entre 0,01 € et 1 000 000 €"));
        }
        let recu_le = b.recu_le.unwrap_or_else(aujourd_hui);
        if recu_le > aujourd_hui() {
            return Err(invalide("date de réception dans le futur"));
        }
        let reference = opt_texte(&b.reference, 80);
        let mut tx = self.fin_tx(etab).await?;
        crate::finances::foyer_existe(&mut tx, etab, b.foyer_id).await?;
        verrou_foyer(&mut tx, etab, b.foyer_id).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_fin_reglement (id, etablissement_id, foyer_id, montant_centimes, moyen, reference,
                    recu_le, non_impute_centimes, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.foyer_id)
        .bind(b.montant_centimes)
        .bind(&b.moyen)
        .bind(&reference)
        .bind(recu_le)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Imputation : factures émises les plus anciennes d'abord.
        let factures: Vec<FactureRow> = sqlx::query_as(&format!(
            "{SELECT_FACTURE} WHERE f.etablissement_id = $1 AND f.foyer_id = $2 AND f.statut = 'emise'
               AND f.type_piece = 'facture' ORDER BY f.emise_at, f.created_at"
        ))
        .bind(etab)
        .bind(b.foyer_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut restant = b.montant_centimes;
        let mut imputations = Vec::new();
        for f in factures {
            if restant == 0 {
                break;
            }
            let part = f.reste().min(restant);
            if part <= 0 {
                continue;
            }
            let (_, code) = encaisser_tx(
                &mut tx,
                etab,
                f.id,
                part,
                &b.moyen,
                reference.as_deref(),
                recu_le,
                Some(id),
                Some(actor),
            )
            .await?;
            restant -= part;
            imputations.push(ImputationDto {
                facture_id: f.id,
                facture_numero: f.numero.clone(),
                montant_centimes: part,
                lettrage: code,
            });
        }
        sqlx::query(
            "UPDATE ent_fin_reglement SET non_impute_centimes = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(restant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::FAC_REGLEMENT,
            AggregateKind::Facture,
            b.foyer_id,
            None,
            json!({ "reglement_id": id, "foyer_id": b.foyer_id, "montant_centimes": b.montant_centimes,
                    "moyen": b.moyen, "impute_centimes": b.montant_centimes - restant }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ReglementDto {
            id,
            foyer_id: b.foyer_id,
            montant_centimes: b.montant_centimes,
            moyen: b.moyen.clone(),
            reference,
            recu_le,
            non_impute_centimes: restant,
            imputations,
        })
    }

    // -- Relances graduées (FIN-13) -----------------------------------------------

    /// Propose les relances dues (aucun envoi) : un palier par facture en retard,
    /// le plus haut atteint et pas encore proposé ; jamais avant l'échéance ni
    /// pendant un prélèvement en cours.
    pub async fn proposer_relances_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<RapportRelancesDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let paliers = paliers_tx(&mut tx, etab).await?;
        let seuils: Vec<(i16, i16)> = paliers.iter().map(|p| (p.rang, p.jours_retard)).collect();
        let factures: Vec<FactureRow> = sqlx::query_as(&format!(
            "{SELECT_FACTURE} WHERE f.etablissement_id = $1 AND f.statut = 'emise' AND f.type_piece = 'facture'
             ORDER BY f.emise_at"
        ))
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let today = aujourd_hui();
        let mut nouvelles = Vec::new();
        for f in factures {
            let mut retard_max = 0_i64;
            let mut du = 0_i64;
            for (e, c) in echeances_couvertes(&mut tx, etab, &f).await? {
                if e.date_echeance < today
                    && c < e.montant_centimes
                    && !matches!(e.prelevement.as_deref(), Some("prevu") | Some("exporte"))
                {
                    retard_max = retard_max.max((today - e.date_echeance).num_days());
                    du += e.montant_centimes - c;
                }
            }
            if du <= 0 {
                continue;
            }
            let deja: Vec<i16> = sqlx::query_as::<_, (i16,)>(
                "SELECT palier FROM ent_fin_relance WHERE etablissement_id = $1 AND facture_id = $2",
            )
            .bind(etab)
            .bind(f.id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
            .into_iter()
            .map(|r| r.0)
            .collect();
            let Some(rang) = calcul::palier_a_proposer(retard_max, &seuils, &deja) else {
                continue;
            };
            let palier = paliers
                .iter()
                .find(|p| p.rang == rang)
                .expect("palier connu");
            let id = Uuid::new_v4();
            let r = sqlx::query(
                "INSERT INTO ent_fin_relance (id, etablissement_id, facture_id, foyer_id, palier, libelle,
                        montant_du_centimes, jours_retard, frais_centimes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (etablissement_id, facture_id, palier) DO NOTHING",
            )
            .bind(id)
            .bind(etab)
            .bind(f.id)
            .bind(f.foyer_id)
            .bind(rang)
            .bind(&palier.libelle)
            .bind(du)
            .bind(retard_max as i32)
            .bind(palier.frais_centimes)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if r.rows_affected() == 1 {
                publier(
                    &mut tx,
                    etab,
                    types::FAC_RELANCE_PROPOSEE,
                    AggregateKind::Facture,
                    f.id,
                    None,
                    json!({ "relance_id": id, "facture_id": f.id, "foyer_id": f.foyer_id, "palier": rang,
                            "montant_du_centimes": du }),
                    Some(actor),
                    Causalite::default(),
                )
                .await?;
                nouvelles.push(id);
            }
        }
        let proposees = relances_tx(&mut tx, etab, None)
            .await?
            .into_iter()
            .filter(|r| nouvelles.contains(&r.id))
            .collect();
        tx.commit().await.map_err(db)?;
        Ok(RapportRelancesDto { proposees })
    }

    pub async fn relances_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        statut: Option<String>,
    ) -> Result<Vec<RelanceDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let r = relances_tx(&mut tx, etab, statut.as_deref()).await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Confirmation humaine ★ : envoi de la relance (notification au payeur),
    /// frais éventuels du palier facturés. `confirmer = false` : annulation.
    pub async fn decider_relance_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        confirmer: bool,
    ) -> Result<RelanceDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let r: Option<(Uuid, Uuid, i16, String, i64, i64, String)> = sqlx::query_as(
            "SELECT facture_id, foyer_id, palier, libelle, montant_du_centimes, frais_centimes, statut
               FROM ent_fin_relance WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (facture, foyer, palier, libelle, du, frais, statut) = r.ok_or_else(introuvable)?;
        if statut != "proposee" {
            return Err(conflit("relance déjà traitée"));
        }
        if confirmer {
            let frais_facture = facturer_frais(
                &mut tx,
                etab,
                foyer,
                frais,
                &format!("Frais de relance ({libelle})"),
                &format!("relance:{id}:frais"),
                actor,
            )
            .await?;
            sqlx::query(
                "UPDATE ent_fin_relance SET statut = 'envoyee', decide_par = $3, decide_le = now(), facture_frais_id = $4
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(actor)
            .bind(frais_facture)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let numero = facture_row(&mut tx, etab, facture).await?.numero;
            let evt = publier(
                &mut tx,
                etab,
                types::FAC_RAPPEL_ENVOYE,
                AggregateKind::Facture,
                facture,
                None,
                json!({ "relance_id": id, "facture_id": facture, "foyer_id": foyer, "palier": palier,
                        "montant_du_centimes": du }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
            notifier_payeurs(
                &mut tx,
                etab,
                foyer,
                evt.id,
                "fin.relance",
                &libelle,
                &format!(
                    "Sauf erreur de notre part, {} restent dus sur la facture {numero}. Merci de régulariser ou de contacter le service comptable.",
                    euros(du)
                ),
            )
            .await?;
        } else {
            sqlx::query(
                "UPDATE ent_fin_relance SET statut = 'annulee', decide_par = $3, decide_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            publier(
                &mut tx,
                etab,
                types::FAC_RELANCE_ANNULEE,
                AggregateKind::Facture,
                facture,
                None,
                json!({ "relance_id": id, "facture_id": facture, "palier": palier }),
                Some(actor),
                Causalite::default(),
            )
            .await?;
        }
        let r = relances_tx(&mut tx, etab, None)
            .await?
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    // -- Export comptable (FIN-16 ★) ---------------------------------------------

    pub async fn exporter_comptabilite_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &ExportComptableBody,
    ) -> Result<ExportComptableDto, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationGerer)
            .await?;
        if b.fin < b.debut || (b.fin - b.debut).num_days() > 366 {
            return Err(invalide("période : un an au plus, fin après début"));
        }
        if b.fin >= aujourd_hui() {
            return Err(invalide("période close seulement (fin avant aujourd'hui)"));
        }
        let mut tx = self.fin_tx(etab).await?;
        let p = params_tx(&mut tx, etab).await?;
        let aux =
            |foyer: Uuid| format!("F{}", foyer.simple().to_string()[..8].to_ascii_uppercase());
        let pieces: Vec<(Uuid, String, String, Uuid, i64, DateTime<Utc>)> = sqlx::query_as(
            "SELECT f.id, f.type_piece, f.numero, f.foyer_id, f.total_centimes, f.emise_at
               FROM ent_fac_facture f
              WHERE f.etablissement_id = $1 AND f.statut <> 'brouillon' AND f.emise_at IS NOT NULL
                AND f.emise_at::DATE BETWEEN $2 AND $3
                AND NOT EXISTS (SELECT 1 FROM ent_fin_export_piece x WHERE x.etablissement_id = f.etablissement_id
                                AND x.piece_type = f.type_piece AND x.piece_id = f.id)
              ORDER BY f.emise_at, f.numero",
        )
        .bind(etab)
        .bind(b.debut)
        .bind(b.fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let encs: Vec<(Uuid, Uuid, String, Uuid, i64, DateTime<Utc>, String, Option<String>)> =
            sqlx::query_as(
                "SELECT e.id, e.facture_id, f.numero, f.foyer_id, e.montant_centimes, e.encaisse_at, e.moyen,
                        (SELECT x.code FROM ent_fin_lettrage x WHERE x.etablissement_id = e.etablissement_id
                          AND x.facture_id = e.facture_id AND x.code IS NOT NULL LIMIT 1)
                   FROM ent_fac_encaissement e
                   JOIN ent_fac_facture f ON f.id = e.facture_id AND f.etablissement_id = e.etablissement_id
                  WHERE e.etablissement_id = $1 AND e.encaisse_at::DATE BETWEEN $2 AND $3
                    AND NOT EXISTS (SELECT 1 FROM ent_fin_export_piece x WHERE x.etablissement_id = e.etablissement_id
                                    AND x.piece_type = 'encaissement' AND x.piece_id = e.id)
                  ORDER BY e.encaisse_at, e.id",
            )
            .bind(etab)
            .bind(b.debut)
            .bind(b.fin)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        if pieces.is_empty() && encs.is_empty() {
            return Err(conflit("aucune pièce à exporter sur cette période"));
        }
        let mut ecritures: Vec<Ecriture> = Vec::new();
        let mut num = 0_u32;
        for (id, type_piece, numero, foyer, total, emise) in &pieces {
            num += 1;
            let date = emise.date_naive();
            let comptes: Vec<(String, i64)> = sqlx::query_as(
                "SELECT CASE WHEN l.categorie = 'reduction' THEN '709000'
                             WHEN l.categorie = 'bourse' THEN '441000'
                             ELSE coalesce(a.compte_produit, '706000') END AS compte,
                        sum(l.quantite::BIGINT * l.montant_unitaire_centimes)::BIGINT
                   FROM ent_fac_ligne l
                   LEFT JOIN ent_fac_ligne o ON o.id = l.ligne_origine_id AND o.etablissement_id = l.etablissement_id
                   LEFT JOIN ent_fin_article a ON a.id = coalesce(l.article_id, o.article_id)
                        AND a.etablissement_id = l.etablissement_id
                  WHERE l.etablissement_id = $1 AND l.facture_id = $2
                  GROUP BY 1 ORDER BY 1",
            )
            .bind(etab)
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let lettrage: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT code FROM ent_fin_lettrage WHERE etablissement_id = $1 AND facture_id = $2
                   AND code IS NOT NULL LIMIT 1",
            )
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let libelle = if type_piece == "avoir" {
                format!("Avoir {numero}")
            } else {
                format!("Facture {numero}")
            };
            let ligne = |compte: &str,
                         aux: Option<String>,
                         debit: i64,
                         credit: i64,
                         lettrage: Option<String>| Ecriture {
                journal: p.journal_ventes.clone(),
                numero: num,
                date,
                compte: compte.into(),
                auxiliaire: aux,
                piece: numero.clone(),
                libelle: libelle.clone(),
                debit,
                credit,
                lettrage,
            };
            // Client : débit pour une facture, crédit pour un avoir.
            let code = lettrage.and_then(|l| l.0);
            if *total >= 0 {
                ecritures.push(ligne(&p.compte_clients, Some(aux(*foyer)), *total, 0, code));
            } else {
                ecritures.push(ligne(
                    &p.compte_clients,
                    Some(aux(*foyer)),
                    0,
                    -*total,
                    code,
                ));
            }
            for (compte, s) in comptes {
                if s > 0 {
                    ecritures.push(ligne(&compte, None, 0, s, None));
                } else if s < 0 {
                    ecritures.push(ligne(&compte, None, -s, 0, None));
                }
            }
        }
        for (_, _, numero, foyer, montant, le, moyen, code) in &encs {
            num += 1;
            let libelle = format!("Règlement {numero} ({moyen})");
            for (compte, aux_, debit, credit, lettrage) in [
                (p.compte_banque.clone(), None, *montant, 0, None),
                (
                    p.compte_clients.clone(),
                    Some(aux(*foyer)),
                    0,
                    *montant,
                    code.clone(),
                ),
            ] {
                ecritures.push(Ecriture {
                    journal: p.journal_banque.clone(),
                    numero: num,
                    date: le.date_naive(),
                    compte,
                    auxiliaire: aux_,
                    piece: numero.clone(),
                    libelle: libelle.clone(),
                    debit,
                    credit,
                    lettrage,
                });
            }
        }
        let (debit, credit) = calcul::equilibre(&ecritures)
            .map_err(|_| conflit("écritures déséquilibrées : export refusé"))?;
        let contenu = calcul::rendre_ecritures(&ecritures, aujourd_hui());
        let empreinte = format!("{:x}", Sha256::digest(contenu.as_bytes()));
        let n = prochain_numero(&mut tx, etab, &format!("export-{}", aujourd_hui().year())).await?;
        let numero = format!("EXP{}-{n:04}", aujourd_hui().year());
        let id = Uuid::new_v4();
        let (created_at,): (DateTime<Utc>,) = sqlx::query_as(
            "INSERT INTO ent_fin_export (id, etablissement_id, numero, debut, fin, format, nb_ecritures,
                    total_debit_centimes, total_credit_centimes, empreinte, created_by)
             VALUES ($1, $2, $3, $4, $5, 'scolia_ecritures_v1', $6, $7, $8, $9, $10) RETURNING created_at",
        )
        .bind(id)
        .bind(etab)
        .bind(&numero)
        .bind(b.debut)
        .bind(b.fin)
        .bind(ecritures.len() as i32)
        .bind(debit)
        .bind(credit)
        .bind(&empreinte)
        .bind(actor)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut marques: BTreeMap<(String, Uuid), ()> = BTreeMap::new();
        for (pid, type_piece, ..) in &pieces {
            marques.insert((type_piece.clone(), *pid), ());
        }
        for (eid, ..) in &encs {
            marques.insert(("encaissement".into(), *eid), ());
        }
        for (piece_type, piece_id) in marques.keys() {
            sqlx::query(
                "INSERT INTO ent_fin_export_piece (etablissement_id, export_id, piece_type, piece_id)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(etab)
            .bind(id)
            .bind(piece_type)
            .bind(piece_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        publier(
            &mut tx,
            etab,
            types::FAC_EXPORT_COMPTABLE,
            AggregateKind::Facture,
            etab,
            None,
            json!({ "export_id": id, "numero": numero, "debut": b.debut, "fin": b.fin,
                    "ecritures": ecritures.len(), "empreinte": empreinte }),
            Some(actor),
            Causalite::default(),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ExportComptableDto {
            id,
            numero,
            debut: b.debut,
            fin: b.fin,
            format: "scolia_ecritures_v1".into(),
            nb_ecritures: ecritures.len() as i32,
            total_debit_centimes: debit,
            total_credit_centimes: credit,
            empreinte,
            created_at,
            contenu: Some(contenu),
        })
    }

    pub async fn exports_comptables_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<ExportComptableDto>, EntDbError> {
        self.fin_exiger(etab, actor, role, Action::FacturationLire)
            .await?;
        let mut tx = self.fin_tx(etab).await?;
        let rows: Vec<(Uuid, String, NaiveDate, NaiveDate, String, i32, i64, i64, String, DateTime<Utc>)> =
            sqlx::query_as(
                "SELECT id, numero, debut, fin, format, nb_ecritures, total_debit_centimes, total_credit_centimes,
                        empreinte, created_at
                   FROM ent_fin_export WHERE etablissement_id = $1 ORDER BY created_at DESC LIMIT 200",
            )
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| ExportComptableDto {
                id: r.0,
                numero: r.1,
                debut: r.2,
                fin: r.3,
                format: r.4,
                nb_ecritures: r.5,
                total_debit_centimes: r.6,
                total_credit_centimes: r.7,
                empreinte: r.8,
                created_at: r.9,
                contenu: None,
            })
            .collect())
    }
}

#[derive(sqlx::FromRow)]
struct RelanceRow {
    id: Uuid,
    facture_id: Uuid,
    numero: String,
    foyer_id: Uuid,
    foyer: Option<String>,
    palier: i16,
    libelle: String,
    montant_du_centimes: i64,
    jours_retard: i32,
    frais_centimes: i64,
    statut: String,
    proposee_le: DateTime<Utc>,
    decide_le: Option<DateTime<Utc>>,
}

async fn relances_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    statut: Option<&str>,
) -> Result<Vec<RelanceDto>, EntDbError> {
    let rows: Vec<RelanceRow> = sqlx::query_as(
        "SELECT r.id, r.facture_id, f.numero, r.foyer_id, fo.libelle AS foyer, r.palier, r.libelle,
                r.montant_du_centimes, r.jours_retard, r.frais_centimes, r.statut, r.proposee_le, r.decide_le
           FROM ent_fin_relance r
           JOIN ent_fac_facture f ON f.id = r.facture_id AND f.etablissement_id = r.etablissement_id
           LEFT JOIN ent_foyer fo ON fo.id = r.foyer_id AND fo.etablissement_id = r.etablissement_id
          WHERE r.etablissement_id = $1 AND ($2::TEXT IS NULL OR r.statut = $2)
          ORDER BY r.proposee_le DESC LIMIT 1000",
    )
    .bind(etab)
    .bind(statut)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| RelanceDto {
            id: r.id,
            facture_id: r.facture_id,
            facture_numero: r.numero,
            foyer_id: r.foyer_id,
            foyer: r
                .foyer
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("Foyer {}", &r.foyer_id.simple().to_string()[..8])),
            palier: r.palier,
            libelle: r.libelle,
            montant_du_centimes: r.montant_du_centimes,
            jours_retard: r.jours_retard,
            frais_centimes: r.frais_centimes,
            statut: r.statut,
            proposee_le: r.proposee_le,
            decide_le: r.decide_le,
        })
        .collect())
}
