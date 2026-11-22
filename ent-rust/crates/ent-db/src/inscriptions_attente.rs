//! Socle 3, lot I5 — **liste d'attente**, **propositions de place** et **conditions de confirmation**
//! (`docs/socles/03_INSCRIPTIONS.md` §7.1, §8.7 ; INS-34, INS-35, §11.5).
//!
//! Principes (aucun n'est négociable) :
//! * le système **désigne** le suivant de la liste quand une place se libère ; il ne décide ni n'envoie jamais :
//!   une personne confirme l'envoi (qui est la décision « admis », prise, tracée et publiée), peut passer son tour
//!   (motif) ou constater qu'un délai est dépassé (jamais automatique) ;
//! * une proposition **désignée** n'est vue ni de la famille ni des autres candidats ; la famille ne lit que sa
//!   propre position ;
//! * la capacité de l'offre est tenue sous verrou ; l'ordre de la liste est celui du rang, sans trou (positions
//!   recalculées) ;
//! * les **conditions** de confirmation (réponse de la famille, contrat, frais, acompte, situation financière,
//!   passage, réserve) sont paramétrées par campagne (aucune par défaut : D-069) ; l'accord des responsables (I4) et
//!   les pièces bloquantes (I3) sont **calculés** et jamais saisis. Quand tout est rempli, le dossier admis devient
//!   `confirme` : c'est une conséquence mécanique de décisions humaines, pas une décision.

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{types, AggregateKind};
use ent_types::inscriptions_decisions::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::inscriptions::{
    commit, conflit, db, dossier_tx, introuvable, invalide, opt, publier, suivi_tx,
    transitionner_tx, DossierRow,
};
use crate::inscriptions_decisions::{
    decider_tx, dossier_visible_tx, dossiers_deportes, ContexteDecision,
};
use crate::inscriptions_etats::{etat_effectif, Cause};
use crate::inscriptions_modele;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Conditions que l'établissement peut exiger d'une campagne (en plus de la réserve, propre à une décision).
pub const CODES_CONDITIONS: [(&str, &str); 6] = [
    (
        "reponse_famille",
        "Réponse de la famille à la place proposée",
    ),
    ("contrat_signe", "Contrat de scolarisation signé"),
    ("frais_payes", "Frais d'inscription payés"),
    ("acompte_paye", "Acompte payé"),
    (
        "situation_financiere_examinee",
        "Situation financière examinée",
    ),
    (
        "passage",
        "Décision de passage de l'établissement d'origine",
    ),
];
/// Délai de réponse de la famille quand la campagne n'en fixe pas (valeur prudente du logiciel).
const DELAI_REPONSE_DEFAUT_JOURS: i64 = 8;

/// Un dossier dans cet état occupe une place de l'offre.
fn occupe_une_place(etat: &str) -> bool {
    matches!(etat, "admis" | "confirme" | "validee")
}

fn libelle_condition(code: &str) -> &'static str {
    CODES_CONDITIONS
        .iter()
        .find(|(c, _)| *c == code)
        .map_or("Condition", |(_, l)| *l)
}

// ───────────────────────────── Aides de transaction ─────────────────────────────

/// Offre du vœu retenu d'un dossier.
pub(crate) async fn offre_du_dossier_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
) -> Result<Option<Uuid>, EntDbError> {
    sqlx::query_scalar(
        "SELECT v.offre_id FROM ent_inscription_dossier d
           JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
          WHERE d.etablissement_id = $1 AND d.id = $2",
    )
    .bind(etab)
    .bind(dossier)
    .fetch_optional(conn)
    .await
    .map_err(db)
    .map(Option::flatten)
}

/// Remet les rangs d'attente d'une offre en suite 1, 2, 3… (un départ ne laisse pas de trou).
async fn compacter_rangs_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    offre: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_inscription_dossier d SET rang_attente = r.rn
           FROM (SELECT x.id, row_number() OVER (ORDER BY x.rang_attente NULLS LAST, x.id)::int AS rn
                   FROM ent_inscription_dossier x
                   JOIN ent_inscription_voeu v ON v.etablissement_id = x.etablissement_id AND v.id = x.voeu_retenu_id
                  WHERE x.etablissement_id = $1 AND v.offre_id = $2 AND x.etat = 'liste_attente') r
          WHERE d.etablissement_id = $1 AND d.id = r.id AND d.rang_attente IS DISTINCT FROM r.rn",
    )
    .bind(etab)
    .bind(offre)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Ferme les propositions ouvertes (désignée ou envoyée) d'un dossier : `annulee`, avec le motif.
pub(crate) async fn annuler_propositions_ouvertes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
    motif: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_inscription_proposition_place SET statut = 'annulee', motif = left($3, 500)
          WHERE etablissement_id = $1 AND dossier_id = $2 AND statut IN ('designee', 'envoyee')",
    )
    .bind(etab)
    .bind(dossier)
    .bind(motif)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Position (1, 2, 3…) du dossier sur la liste de son offre.
pub(crate) async fn position_attente_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
) -> Result<Option<i64>, EntDbError> {
    sqlx::query_scalar(
        "SELECT r.pos FROM (
            SELECT x.id, row_number() OVER (ORDER BY x.rang_attente NULLS LAST, x.id) AS pos
              FROM ent_inscription_dossier x
              JOIN ent_inscription_voeu v ON v.etablissement_id = x.etablissement_id AND v.id = x.voeu_retenu_id
             WHERE x.etablissement_id = $1 AND x.etat = 'liste_attente'
               AND v.offre_id = (SELECT v2.offre_id FROM ent_inscription_dossier d2
                                   JOIN ent_inscription_voeu v2
                                     ON v2.etablissement_id = d2.etablissement_id AND v2.id = d2.voeu_retenu_id
                                  WHERE d2.etablissement_id = $1 AND d2.id = $2)) r
          WHERE r.id = $2",
    )
    .bind(etab)
    .bind(dossier)
    .fetch_optional(conn)
    .await
    .map_err(db)
}

/// **Désigne** les suivants de la liste tant qu'il y a des places libres non déjà promises à une désignation.
/// Ne décide rien, n'envoie rien. Un dossier dont la décision date de cette même transaction (il vient de quitter
/// l'offre ou d'y être placé en attente) n'est jamais désigné pour la place qu'il vient de libérer.
pub(crate) async fn designer_suivants_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    offre: Uuid,
) -> Result<i64, EntDbError> {
    sqlx::query(
        "SELECT id FROM ent_inscription_offre WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(etab)
    .bind(offre)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let mut designes = 0;
    for _ in 0..100 {
        let Some(dispo) = inscriptions_modele::disponibles_tx(conn, etab, offre).await? else {
            break;
        };
        let promises: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_inscription_proposition_place
              WHERE etablissement_id = $1 AND offre_id = $2 AND origine = 'liste_attente' AND statut = 'designee'",
        )
        .bind(etab)
        .bind(offre)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if dispo - promises < 1 {
            break;
        }
        let suivant: Option<Uuid> = sqlx::query_scalar(
            "SELECT d.id FROM ent_inscription_dossier d
               JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
              WHERE d.etablissement_id = $1 AND v.offre_id = $2 AND d.etat = 'liste_attente'
                AND NOT EXISTS (SELECT 1 FROM ent_inscription_proposition_place p
                                 WHERE p.etablissement_id = d.etablissement_id AND p.dossier_id = d.id
                                   AND (p.statut IN ('designee', 'envoyee')
                                        OR (p.statut = 'ecartee' AND p.offre_id = $2)))
                AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision x
                                 WHERE x.etablissement_id = d.etablissement_id AND x.dossier_id = d.id
                                   AND x.decide_le = now())
              ORDER BY d.rang_attente NULLS LAST, d.id LIMIT 1
              FOR UPDATE OF d SKIP LOCKED",
        )
        .bind(etab)
        .bind(offre)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some(suivant) = suivant else {
            break;
        };
        let pos = position_attente_tx(conn, etab, suivant).await?;
        sqlx::query(
            "INSERT INTO ent_inscription_proposition_place
                (id, etablissement_id, dossier_id, offre_id, origine, statut, rang_au_moment)
             VALUES ($1,$2,$3,$4,'liste_attente','designee',$5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(suivant)
        .bind(offre)
        .bind(pos.and_then(|p| i32::try_from(p).ok()))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        suivi_tx(
            conn,
            etab,
            suivant,
            "liste_attente",
            None,
            Some("Une place est libre : proposition désignée, en attente de la confirmation de la direction"),
            false,
            None,
        )
        .await?;
        designes += 1;
    }
    Ok(designes)
}

/// Appelé par [`transitionner_tx`] après chaque changement d'état : ferme les propositions devenues sans objet,
/// remet les rangs en suite, **désigne** le suivant quand une place se libère.
pub(crate) async fn apres_transition_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    vers: &str,
) -> Result<(), EntDbError> {
    if d.nature != "admission" || vers == "bloque_desaccord" {
        return Ok(());
    }
    let avant = etat_effectif(&d.etat, d.etat_avant_blocage.as_deref());
    if avant == vers {
        return Ok(());
    }
    let quitte_la_liste = avant == "liste_attente";
    let libere_une_place = occupe_une_place(avant) && !occupe_une_place(vers);
    if !quitte_la_liste && !libere_une_place {
        return Ok(());
    }
    let offre = offre_du_dossier_tx(conn, etab, d.id).await?;
    if quitte_la_liste {
        if vers != "admis" {
            annuler_propositions_ouvertes_tx(
                conn,
                etab,
                d.id,
                "dossier sorti de la liste d'attente",
            )
            .await?;
        }
        if let Some(o) = offre {
            compacter_rangs_tx(conn, etab, o).await?;
            if vers != "admis" {
                // Une désignation qui tombe rend sa place : le suivant est désigné.
                designer_suivants_tx(conn, etab, o).await?;
            }
        }
    }
    if libere_une_place {
        annuler_propositions_ouvertes_tx(conn, etab, d.id, "place libérée par le dossier").await?;
        if let Some(o) = offre {
            designer_suivants_tx(conn, etab, o).await?;
        }
    }
    Ok(())
}

/// Prépare la proposition d'une admission : celle qui est ouverte (désignée depuis la liste) devient celle de la
/// décision ; sinon une nouvelle est créée, `designee` (elle ne part qu'à la publication).
pub(crate) async fn preparer_proposition_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    offre: Option<Uuid>,
    origine: &str,
    decision_id: Uuid,
) -> Result<(), EntDbError> {
    let ouverte: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_inscription_proposition_place
          WHERE etablissement_id = $1 AND dossier_id = $2 AND statut IN ('designee', 'envoyee') FOR UPDATE",
    )
    .bind(etab)
    .bind(d.id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some(p) = ouverte {
        sqlx::query(
            "UPDATE ent_inscription_proposition_place SET decision_id = $3, offre_id = COALESCE($4, offre_id)
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(p)
        .bind(decision_id)
        .bind(offre)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO ent_inscription_proposition_place
            (id, etablissement_id, dossier_id, offre_id, origine, statut, decision_id)
         VALUES ($1,$2,$3,$4,$5,'designee',$6)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(d.id)
    .bind(offre)
    .bind(if origine == "liste_attente" {
        "liste_attente"
    } else {
        "decision"
    })
    .bind(decision_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    // Une nouvelle proposition (autre offre, nouvelle décision) demande une nouvelle réponse de la famille.
    sqlx::query(
        "UPDATE ent_inscription_condition SET remplie = false, remplie_le = NULL, remplie_par = NULL,
                source = NULL, source_ref = NULL, motif = NULL
          WHERE etablissement_id = $1 AND dossier_id = $2 AND code = 'reponse_famille'",
    )
    .bind(etab)
    .bind(d.id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// La décision est publiée : la proposition **part** (délai de réponse décompté à partir de maintenant, décalé par
/// les fermetures du secrétariat).
pub(crate) async fn envoyer_proposition_publiee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
    par: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<(), EntDbError> {
    let delai: Option<i16> = sqlx::query_scalar(
        "SELECT c.delai_reponse_jours FROM ent_inscription_dossier d
           JOIN ent_inscription_campagne c ON c.etablissement_id = d.etablissement_id AND c.id = d.campagne_id
          WHERE d.etablissement_id = $1 AND d.id = $2",
    )
    .bind(etab)
    .bind(dossier)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let n = delai.map_or(DELAI_REPONSE_DEFAUT_JOURS, i64::from);
    let jour: NaiveDate = sqlx::query_scalar("SELECT (ent_aujourdhui_etab($2, $3) + $1::int)")
        .bind(i32::try_from(n).unwrap_or(8))
        .bind(etab)
        .bind(maintenant)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let avant = inscriptions_modele::echeance_effective_tx(conn, etab, jour).await?;
    let envoyee: Option<Uuid> = sqlx::query_scalar(
        "UPDATE ent_inscription_proposition_place
            SET statut = 'envoyee', envoyee_le = $3, envoyee_par = $4, repondre_avant = $5
          WHERE etablissement_id = $1 AND dossier_id = $2 AND statut = 'designee'
        RETURNING id",
    )
    .bind(etab)
    .bind(dossier)
    .bind(maintenant)
    .bind(par)
    .bind(avant)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some(p) = envoyee {
        sqlx::query(
            "UPDATE ent_inscription_dossier SET echeance_famille = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(dossier)
        .bind(avant)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        publier(
            conn,
            etab,
            types::INSCRIPTION_PLACE_PROPOSEE,
            AggregateKind::Admission,
            dossier,
            None,
            json!({ "dossier_id": dossier, "proposition_id": p }),
            par,
        )
        .await?;
    }
    Ok(())
}

// ───────────────────────────── Conditions ─────────────────────────────

/// Crée les conditions d'un dossier admis : celles de la campagne, et la réserve de la décision (qui se **refait**
/// si la décision est revue : une réserve nouvelle n'est pas levée d'avance).
pub(crate) async fn instancier_conditions_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    reserve: Option<&str>,
) -> Result<(), EntDbError> {
    let codes: Vec<String> = sqlx::query_scalar(
        "SELECT code FROM ent_inscription_condition_param WHERE etablissement_id = $1 AND campagne_id = $2",
    )
    .bind(etab)
    .bind(d.campagne_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for code in codes {
        sqlx::query(
            "INSERT INTO ent_inscription_condition (id, etablissement_id, dossier_id, code, libelle)
             VALUES ($1,$2,$3,$4,$5) ON CONFLICT (dossier_id, code) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(d.id)
        .bind(&code)
        .bind(libelle_condition(&code))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    match reserve {
        Some(r) => {
            sqlx::query(
                "INSERT INTO ent_inscription_condition (id, etablissement_id, dossier_id, code, libelle)
                 VALUES ($1,$2,$3,'reserve',$4)
                 ON CONFLICT (dossier_id, code) DO UPDATE
                    SET libelle = EXCLUDED.libelle, remplie = false, remplie_le = NULL, remplie_par = NULL,
                        source = NULL, source_ref = NULL, motif = NULL
                  WHERE ent_inscription_condition.libelle IS DISTINCT FROM EXCLUDED.libelle",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(d.id)
            .bind(r)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
        None => {
            sqlx::query(
                "DELETE FROM ent_inscription_condition
                  WHERE etablissement_id = $1 AND dossier_id = $2 AND code = 'reserve'",
            )
            .bind(etab)
            .bind(d.id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
    }
    Ok(())
}

type LigneCondition = (
    String,
    String,
    bool,
    Option<String>,
    Option<DateTime<Utc>>,
    Option<String>,
);

/// Conditions d'un dossier : celles qui sont saisies, plus les deux qui sont **calculées** (accord des
/// responsables, pièces bloquantes). `famille` : le motif d'une levée et tout détail propre à une personne sont
/// retirés.
pub(crate) async fn evaluer_conditions_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    famille: bool,
) -> Result<(Vec<ConditionDto>, bool), EntDbError> {
    let rows: Vec<LigneCondition> = sqlx::query_as(
        "SELECT code, libelle, remplie, source, remplie_le, motif FROM ent_inscription_condition
          WHERE etablissement_id = $1 AND dossier_id = $2 ORDER BY cree_le, code",
    )
    .bind(etab)
    .bind(d.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let a_des_saisies = !rows.is_empty();
    let mut out: Vec<ConditionDto> = rows
        .into_iter()
        .map(|r| ConditionDto {
            code: r.0,
            libelle: r.1,
            remplie: r.2,
            source: r.3,
            remplie_le: r.4,
            motif: if famille { None } else { r.5 },
            detail: None,
        })
        .collect();
    // Calculées : lues à chaque fois, jamais stockées.
    let accords = crate::inscriptions_accords::exiger_accords_tx(&mut *conn, etab, d).await;
    let (accords_ok, accords_detail) = match accords {
        Ok(()) => (true, None),
        Err(EntDbError::ConflitDossier(m)) => (false, Some(m.to_string())),
        Err(e) => return Err(e),
    };
    out.push(ConditionDto {
        code: "accord_responsables".into(),
        libelle: "Accord des responsables".into(),
        remplie: accords_ok,
        source: Some("calculee".into()),
        remplie_le: None,
        motif: None,
        // Message générique : il ne dit jamais quel responsable a répondu quoi.
        detail: accords_detail
            .map(|_| "accord des titulaires de l'autorité parentale attendu".to_string()),
    });
    let manquantes =
        crate::inscriptions_pieces::bloquantes_manquantes_tx(&mut *conn, etab, d.id).await?;
    out.push(ConditionDto {
        code: "pieces_bloquantes".into(),
        libelle: "Pièces indispensables".into(),
        remplie: manquantes.is_empty(),
        source: Some("calculee".into()),
        remplie_le: None,
        motif: None,
        detail: (!manquantes.is_empty()).then(|| {
            format!(
                "{} pièce(s) à fournir ou à faire contrôler",
                manquantes.len()
            )
        }),
    });
    Ok((out, a_des_saisies))
}

/// Conditions **saisies** non remplies d'un dossier (libellés) : la validation les exige.
pub(crate) async fn conditions_saisies_manquantes_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
) -> Result<Vec<String>, EntDbError> {
    sqlx::query_scalar(
        "SELECT libelle FROM ent_inscription_condition
          WHERE etablissement_id = $1 AND dossier_id = $2 AND NOT remplie ORDER BY cree_le, code",
    )
    .bind(etab)
    .bind(dossier)
    .fetch_all(conn)
    .await
    .map_err(db)
}

/// Réévalue les conditions : si le dossier est `admis`, qu'il a des conditions saisies et qu'elles sont toutes
/// remplies (avec les deux calculées), il devient `confirme`. Conséquence mécanique de décisions humaines.
pub(crate) async fn reevaluer_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    auteur: Option<Uuid>,
    famille: bool,
) -> Result<ConditionsDossierDto, EntDbError> {
    let d = dossier_tx(conn, etab, id).await?;
    let (mut conditions, a_des_saisies) = evaluer_conditions_tx(conn, etab, &d, famille).await?;
    let toutes = conditions.iter().all(|c| c.remplie);
    let mut confirme = d.etat == "confirme";
    if d.etat == "admis" && a_des_saisies && toutes {
        transitionner_tx(
            conn,
            etab,
            &d,
            "confirme",
            Cause::Etape,
            Some("Conditions de confirmation remplies"),
            auteur,
        )
        .await?;
        confirme = true;
    }
    conditions.sort_by_key(|c| c.remplie);
    Ok(ConditionsDossierDto {
        conditions,
        toutes_remplies: toutes,
        confirme,
    })
}

/// Conditions lues par la famille : sans le motif d'une levée, sans détail sur un responsable.
pub(crate) async fn conditions_famille_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
) -> Result<Vec<ConditionDto>, EntDbError> {
    if !matches!(d.etat.as_str(), "admis" | "confirme") {
        return Ok(Vec::new());
    }
    let (c, _) = evaluer_conditions_tx(conn, etab, d, true).await?;
    Ok(c)
}

pub(crate) async fn proposition_famille_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
) -> Result<Option<PropositionFamilleDto>, EntDbError> {
    // `designee` ne se lit jamais : rien n'est parti.
    let r: Option<(Uuid, String, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT id, statut, repondre_avant FROM ent_inscription_proposition_place
          WHERE etablissement_id = $1 AND dossier_id = $2
            AND statut IN ('envoyee', 'acceptee', 'declinee', 'expiree')
          ORDER BY designee_le DESC, id DESC LIMIT 1",
    )
    .bind(etab)
    .bind(dossier)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(r.map(|r| PropositionFamilleDto {
        id: r.0,
        statut: r.1,
        repondre_avant: r.2,
    }))
}

type LignePropositionBrute = (
    Uuid,
    Uuid,
    String,
    String,
    String,
    Uuid,
    String,
    String,
    Option<i32>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<NaiveDate>,
    Option<DateTime<Utc>>,
    Option<String>,
);

fn proposition_dto(r: LignePropositionBrute, jour: NaiveDate) -> PropositionDto {
    let a_constater = r.7 == "envoyee" && r.11.is_some_and(|j| j < jour);
    PropositionDto {
        id: r.0,
        dossier_id: r.1,
        reference: r.2,
        eleve_nom: r.3,
        eleve_prenom: r.4,
        offre_id: r.5,
        origine: r.6,
        statut: r.7,
        rang_au_moment: r.8,
        designee_le: r.9,
        envoyee_le: r.10,
        repondre_avant: r.11,
        repondu_le: r.12,
        expiree_a_constater: a_constater,
        motif: r.13,
    }
}

const SELECT_PROPOSITION: &str = "SELECT p.id, p.dossier_id, d.reference, d.eleve_nom, d.eleve_prenom,
        COALESCE(p.offre_id, '00000000-0000-0000-0000-000000000000'::uuid), p.origine, p.statut, p.rang_au_moment,
        p.designee_le, p.envoyee_le, p.repondre_avant, p.repondu_le, p.motif
   FROM ent_inscription_proposition_place p
   JOIN ent_inscription_dossier d ON d.etablissement_id = p.etablissement_id AND d.id = p.dossier_id";

async fn proposition_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    jour: NaiveDate,
) -> Result<PropositionDto, EntDbError> {
    sqlx::query_as::<_, LignePropositionBrute>(&format!(
        "{SELECT_PROPOSITION} WHERE p.etablissement_id = $1 AND p.id = $2"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .map(|r| proposition_dto(r, jour))
    .ok_or_else(introuvable)
}

async fn jour_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    instant: DateTime<Utc>,
) -> Result<NaiveDate, EntDbError> {
    sqlx::query_scalar("SELECT ent_aujourdhui_etab($1, $2)")
        .bind(etab)
        .bind(instant)
        .fetch_one(conn)
        .await
        .map_err(db)
}

// ───────────────────────────── Service ─────────────────────────────

impl PgEnt {
    // ───────────────────────── Conditions ─────────────────────────

    pub async fn conditions_campagne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne: Uuid,
    ) -> Result<Vec<String>, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        campagne_admission_tx(&mut tx, etab, campagne).await?;
        sqlx::query_scalar(
            "SELECT code FROM ent_inscription_condition_param
              WHERE etablissement_id = $1 AND campagne_id = $2 ORDER BY code",
        )
        .bind(etab)
        .bind(campagne)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    /// Fixe les conditions de confirmation d'une campagne. Elles ne s'appliquent qu'aux décisions prises ensuite :
    /// un dossier déjà admis garde ses conditions (jamais de condition nouvelle imposée en silence).
    pub async fn definir_conditions_campagne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne: Uuid,
        b: &ConditionsCampagneBody,
    ) -> Result<Vec<String>, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let mut codes: Vec<String> = b.codes.iter().map(|c| c.trim().to_string()).collect();
        codes.sort();
        codes.dedup();
        if codes
            .iter()
            .any(|c| !CODES_CONDITIONS.iter().any(|(k, _)| k == c))
        {
            return Err(invalide(
                "condition inconnue (reponse_famille, contrat_signe, frais_payes, acompte_paye, \
                 situation_financiere_examinee, passage)",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        campagne_admission_tx(&mut tx, etab, campagne).await?;
        sqlx::query(
            "DELETE FROM ent_inscription_condition_param WHERE etablissement_id = $1 AND campagne_id = $2",
        )
        .bind(etab)
        .bind(campagne)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for c in &codes {
            sqlx::query(
                "INSERT INTO ent_inscription_condition_param (etablissement_id, campagne_id, code)
                 VALUES ($1,$2,$3)",
            )
            .bind(etab)
            .bind(campagne)
            .bind(c)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        commit(tx).await?;
        Ok(codes)
    }

    pub async fn conditions_dossier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ConditionsDossierDto, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_visible_tx(&mut tx, etab, actor, id).await?;
        let (mut conditions, _) = evaluer_conditions_tx(&mut tx, etab, &d, false).await?;
        let toutes = conditions.iter().all(|c| c.remplie);
        conditions.sort_by_key(|c| c.remplie);
        Ok(ConditionsDossierDto {
            conditions,
            toutes_remplies: toutes,
            confirme: d.etat == "confirme",
        })
    }

    /// Réévalue les conditions et confirme le dossier si tout est rempli.
    pub async fn reevaluer_conditions_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ConditionsDossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        dossier_visible_tx(&mut tx, etab, actor, id).await?;
        let r = reevaluer_tx(&mut tx, etab, id, Some(actor), false).await?;
        commit(tx).await?;
        Ok(r)
    }

    /// Constate qu'une condition est remplie (contrat reçu, paiement constaté, réserve levée…). Le constat est
    /// tracé (auteur, motif, objet source). La réponse de la famille ne se constate pas à sa place : elle se
    /// donne par son espace, ou se lève par la direction avec un motif.
    pub async fn remplir_condition_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        code: &str,
        b: &RemplirConditionBody,
    ) -> Result<ConditionsDossierDto, EntDbError> {
        // Une réserve posée par la direction ne se lève pas au secrétariat (levée avec motif : `lever`).
        let action = if code == "reserve" {
            Action::InscriptionsDecider
        } else {
            Action::InscriptionsGerer
        };
        self.ins_exiger(etab, actor, role, action).await?;
        if code == "reponse_famille" {
            return Err(conflit(
                "la réponse de la famille se donne dans son espace (ou se lève, avec motif, par la direction)",
            ));
        }
        let motif = opt(&b.motif);
        if motif.as_deref().is_some_and(|m| m.chars().count() > 500) {
            return Err(invalide("motif : 500 caractères au plus"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        dossier_visible_tx(&mut tx, etab, actor, id).await?;
        let n = sqlx::query(
            "UPDATE ent_inscription_condition
                SET remplie = true, remplie_le = now(), remplie_par = $4,
                    source = CASE WHEN $5::uuid IS NULL THEN 'personnel' ELSE 'evenement' END,
                    source_ref = $5, motif = $6
              WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3 AND NOT remplie",
        )
        .bind(etab)
        .bind(id)
        .bind(code)
        .bind(actor)
        .bind(b.source_ref)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("condition absente de ce dossier ou déjà remplie"));
        }
        suivi_tx(
            &mut tx,
            etab,
            id,
            "condition",
            None,
            Some("Condition de confirmation remplie"),
            false,
            Some(actor),
        )
        .await?;
        let r = reevaluer_tx(&mut tx, etab, id, Some(actor), false).await?;
        commit(tx).await?;
        Ok(r)
    }

    /// Lève une condition avec motif (direction, `inscriptions.decider`) : tracée, jamais silencieuse.
    pub async fn lever_condition_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        code: &str,
        b: &MotifBody,
    ) -> Result<ConditionsDossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let motif = b.motif.trim();
        if motif.chars().count() < 3 || motif.chars().count() > 500 {
            return Err(invalide("motif de la levée : de 3 à 500 caractères"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        dossier_visible_tx(&mut tx, etab, actor, id).await?;
        let n = sqlx::query(
            "UPDATE ent_inscription_condition
                SET remplie = true, remplie_le = now(), remplie_par = $4, source = 'levee', source_ref = NULL,
                    motif = $5
              WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3 AND NOT remplie",
        )
        .bind(etab)
        .bind(id)
        .bind(code)
        .bind(actor)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("condition absente de ce dossier ou déjà remplie"));
        }
        suivi_tx(
            &mut tx,
            etab,
            id,
            "condition",
            None,
            Some("Condition de confirmation levée par la direction"),
            false,
            Some(actor),
        )
        .await?;
        let r = reevaluer_tx(&mut tx, etab, id, Some(actor), false).await?;
        commit(tx).await?;
        Ok(r)
    }

    // ───────────────────────── Liste d'attente ─────────────────────────

    pub async fn liste_attente_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        offre: Uuid,
    ) -> Result<ListeAttenteDto, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        liste_attente_tx(&mut tx, etab, actor, offre).await
    }

    /// Désigne explicitement les suivants (capacité augmentée, place rendue). Ne décide ni n'envoie rien.
    pub async fn designer_suivants_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        offre: Uuid,
    ) -> Result<ListeAttenteDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        offre_existe_tx(&mut tx, etab, offre).await?;
        designer_suivants_tx(&mut tx, etab, offre).await?;
        let r = liste_attente_tx(&mut tx, etab, actor, offre).await?;
        commit(tx).await?;
        Ok(r)
    }

    pub async fn propositions_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne: Option<Uuid>,
        statut: Option<String>,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<PropositionDto>, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        if let Some(s) = &statut {
            if ![
                "designee", "envoyee", "acceptee", "declinee", "expiree", "ecartee", "annulee",
            ]
            .contains(&s.as_str())
            {
                return Err(invalide("statut de proposition inconnu"));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_tx(&mut tx, etab, maintenant).await?;
        let deportes = dossiers_deportes(&mut tx, etab, actor).await?;
        let rows = sqlx::query_as::<_, LignePropositionBrute>(&format!(
            "{SELECT_PROPOSITION}
              WHERE p.etablissement_id = $1 AND ($2::uuid IS NULL OR d.campagne_id = $2)
                AND ($3::text IS NULL OR p.statut = $3)
              ORDER BY p.designee_le DESC, p.id LIMIT 500"
        ))
        .bind(etab)
        .bind(campagne)
        .bind(statut)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .filter(|r| !deportes.contains(&r.1))
            .map(|r| proposition_dto(r, jour))
            .collect())
    }

    /// INS-34 : **une personne confirme l'envoi** de la place désignée. L'envoi est la décision « admis » : prise,
    /// motivée, tracée (auteur, date) et publiée dans la foulée ; la place est comptée au vœu retenu, sous verrou ;
    /// si l'offre est pleine, l'envoi n'a lieu qu'avec une confirmation de dépassement.
    pub async fn envoyer_proposition_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &EnvoyerPropositionBody,
        maintenant: DateTime<Utc>,
    ) -> Result<PropositionDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        if !b.confirmer {
            return Err(invalide(
                "l'envoi d'une place se confirme explicitement (confirmer = true)",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p: Option<(Uuid, Option<Uuid>, String, String, Option<i32>)> = sqlx::query_as(
            "SELECT dossier_id, offre_id, origine, statut, rang_au_moment
               FROM ent_inscription_proposition_place
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (dossier, offre, origine, statut, rang) = p.ok_or_else(introuvable)?;
        if statut != "designee" || origine != "liste_attente" {
            return Err(conflit(
                "seule une place désignée depuis la liste d'attente s'envoie",
            ));
        }
        dossier_visible_tx(&mut tx, etab, actor, dossier).await?;
        let motif_interne = opt(&b.motif_interne).unwrap_or_else(|| {
            format!(
                "Place libérée : proposition depuis la liste d'attente (rang {})",
                rang.map_or_else(|| "?".to_string(), |r| r.to_string())
            )
        });
        let corps = DecisionCompleteBody {
            sens: "admis".into(),
            offre_id: offre,
            motif_interne,
            motif_communique: b.motif_communique.clone(),
            confirmer_depassement: b.confirmer_depassement,
            ..Default::default()
        };
        decider_tx(
            &mut tx,
            etab,
            dossier,
            &ContexteDecision {
                actor,
                body: &corps,
                revoir: false,
                origine: "liste_attente",
                commission_id: None,
                maintenant,
                motif_obligatoire: true,
                publier_tout_de_suite: true,
                publier_par_defaut: true,
                communiquer_par_defaut: false,
            },
        )
        .await?;
        let jour = jour_tx(&mut tx, etab, maintenant).await?;
        let dto = proposition_tx(&mut tx, etab, id, jour).await?;
        commit(tx).await?;
        Ok(dto)
    }

    /// Passer son tour : la place désignée n'est pas proposée à ce dossier (motif obligatoire, tracé) ; le suivant
    /// est désigné. Le dossier garde son rang et peut être admis par une décision ordinaire.
    pub async fn ecarter_proposition_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &MotifBody,
        maintenant: DateTime<Utc>,
    ) -> Result<PropositionDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let motif = b.motif.trim();
        if motif.chars().count() < 3 || motif.chars().count() > 500 {
            return Err(invalide("motif : de 3 à 500 caractères"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p: Option<(Uuid, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT dossier_id, offre_id, statut FROM ent_inscription_proposition_place
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (dossier, offre, statut) = p.ok_or_else(introuvable)?;
        if statut != "designee" {
            return Err(conflit(
                "seule une place désignée (pas encore envoyée) peut être écartée",
            ));
        }
        dossier_visible_tx(&mut tx, etab, actor, dossier).await?;
        sqlx::query(
            "UPDATE ent_inscription_proposition_place SET statut = 'ecartee', motif = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        suivi_tx(
            &mut tx,
            etab,
            dossier,
            "liste_attente",
            None,
            Some("Tour passé (motif interne)"),
            false,
            Some(actor),
        )
        .await?;
        if let Some(o) = offre {
            designer_suivants_tx(&mut tx, etab, o).await?;
        }
        let jour = jour_tx(&mut tx, etab, maintenant).await?;
        let dto = proposition_tx(&mut tx, etab, id, jour).await?;
        commit(tx).await?;
        Ok(dto)
    }

    /// Constate qu'un délai de réponse est dépassé (jamais automatique) : la proposition est `expiree`, le dossier
    /// `sans_suite`, la place se libère et le suivant est désigné. Refusé tant que le délai n'est pas dépassé.
    pub async fn constater_expiration_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<PropositionDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_tx(&mut tx, etab, maintenant).await?;
        let p: Option<(Uuid, String, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT dossier_id, statut, repondre_avant FROM ent_inscription_proposition_place
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (dossier, statut, avant) = p.ok_or_else(introuvable)?;
        if statut != "envoyee" {
            return Err(conflit(
                "seule une proposition envoyée et sans réponse peut expirer",
            ));
        }
        if !avant.is_some_and(|j| j < jour) {
            return Err(conflit("le délai de réponse n'est pas dépassé"));
        }
        let d = dossier_visible_tx(&mut tx, etab, actor, dossier).await?;
        if d.etat != "admis" {
            return Err(conflit(
                "le dossier n'est plus à l'état admis : l'expiration ne s'applique pas",
            ));
        }
        sqlx::query(
            "UPDATE ent_inscription_proposition_place SET statut = 'expiree', motif = 'Délai de réponse dépassé'
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let d = dossier_tx(&mut tx, etab, d.id).await?;
        if d.etat == "admis" {
            transitionner_tx(
                &mut tx,
                etab,
                &d,
                "sans_suite",
                Cause::Etape,
                Some("Délai de réponse dépassé"),
                Some(actor),
            )
            .await?;
        }
        let dto = proposition_tx(&mut tx, etab, id, jour).await?;
        commit(tx).await?;
        Ok(dto)
    }

    /// INS-35 : la famille répond à la place proposée (déposant vérifié seulement). Accepter remplit la
    /// condition « réponse de la famille » ; décliner clôt le dossier (désistement) et libère la place.
    pub async fn repondre_proposition_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
        b: &ReponsePropositionBody,
        maintenant: DateTime<Utc>,
    ) -> Result<DecisionFamilleDto, EntDbError> {
        let (_, d0) = self.dossier_du_deposant(etab, jeton, id).await?;
        if !["acceptee", "declinee"].contains(&b.reponse.as_str()) {
            return Err(invalide("réponse : acceptee ou declinee"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, d0.id).await?;
        let p: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_inscription_proposition_place
              WHERE etablissement_id = $1 AND dossier_id = $2 AND statut = 'envoyee' FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let p = p.ok_or_else(|| conflit("aucune place proposée à laquelle répondre"))?;
        if d.etat != "admis" {
            return Err(conflit("aucune place proposée à laquelle répondre"));
        }
        sqlx::query(
            "UPDATE ent_inscription_proposition_place SET statut = $3, repondu_le = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(p)
        .bind(&b.reponse)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if b.reponse == "acceptee" {
            sqlx::query(
                "UPDATE ent_inscription_condition
                    SET remplie = true, remplie_le = $3, source = 'famille', motif = NULL
                  WHERE etablissement_id = $1 AND dossier_id = $2 AND code = 'reponse_famille' AND NOT remplie",
            )
            .bind(etab)
            .bind(id)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            suivi_tx(
                &mut tx,
                etab,
                id,
                "reponse",
                None,
                Some("La famille a accepté la place proposée"),
                true,
                None,
            )
            .await?;
        } else {
            transitionner_tx(
                &mut tx,
                etab,
                &d,
                "desiste",
                Cause::Etape,
                Some("Place déclinée par la famille"),
                None,
            )
            .await?;
        }
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_REPONSE_FAMILLE,
            AggregateKind::Admission,
            id,
            None,
            json!({ "dossier_id": id, "proposition_id": p, "reponse": b.reponse }),
            None,
        )
        .await?;
        if b.reponse == "acceptee" {
            reevaluer_tx(&mut tx, etab, id, None, true).await?;
        }
        commit(tx).await?;
        self.decision_famille(etab, jeton, id).await
    }
}

async fn campagne_admission_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne: Uuid,
) -> Result<(), EntDbError> {
    let nature: Option<String> = sqlx::query_scalar(
        "SELECT nature FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(campagne)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    if nature.ok_or_else(introuvable)? != "admission" {
        return Err(conflit(
            "les conditions de confirmation concernent les campagnes d'admission",
        ));
    }
    Ok(())
}

async fn offre_existe_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    offre: Uuid,
) -> Result<(), EntDbError> {
    let ok: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ent_inscription_offre WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(offre)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    ok.map(|_| ()).ok_or_else(introuvable)
}

async fn liste_attente_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    offre: Uuid,
) -> Result<ListeAttenteDto, EntDbError> {
    let o: Option<(String, Option<i32>)> = sqlx::query_as(
        "SELECT libelle_public, capacite FROM ent_inscription_offre WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(offre)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let (libelle, capacite) = o.ok_or_else(introuvable)?;
    let disponibles = inscriptions_modele::disponibles_tx(conn, etab, offre).await?;
    let deportes = dossiers_deportes(conn, etab, actor).await?;
    let rows: Vec<(Uuid, String, String, String, i64, Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT d.id, d.reference, d.eleve_nom, d.eleve_prenom,
                row_number() OVER (ORDER BY d.rang_attente NULLS LAST, d.id),
                p.id, p.statut
           FROM ent_inscription_dossier d
           JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
      LEFT JOIN ent_inscription_proposition_place p
             ON p.etablissement_id = d.etablissement_id AND p.dossier_id = d.id AND p.statut IN ('designee', 'envoyee')
          WHERE d.etablissement_id = $1 AND v.offre_id = $2 AND d.etat = 'liste_attente'
          ORDER BY d.rang_attente NULLS LAST, d.id",
    )
    .bind(etab)
    .bind(offre)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(ListeAttenteDto {
        offre_id: offre,
        libelle,
        capacite: capacite.map(i64::from),
        disponibles,
        lignes: rows
            .into_iter()
            .filter(|r| !deportes.contains(&r.0))
            .map(|r| AttenteLigneDto {
                dossier_id: r.0,
                reference: r.1,
                eleve_nom: r.2,
                eleve_prenom: r.3,
                position: r.4,
                proposition_id: r.5,
                proposition_statut: r.6,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_occupees_par_les_seuls_etats_qui_comptent() {
        for e in ["admis", "confirme", "validee"] {
            assert!(occupe_une_place(e));
        }
        for e in [
            "liste_attente",
            "en_instruction",
            "refuse",
            "desiste",
            "sans_suite",
            "annule",
        ] {
            assert!(!occupe_une_place(e));
        }
    }

    #[test]
    fn chaque_condition_a_un_libelle() {
        for (code, libelle) in CODES_CONDITIONS {
            assert_eq!(libelle_condition(code), libelle);
        }
    }
}
