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
    decider_tx, dossier_visible_tx, ecartes_parmi, ContexteDecision,
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

/// Confirmation explicite d'une admission directe qui reprend une place désignée à la liste d'attente.
#[derive(Clone, Copy)]
pub(crate) struct PasserDevant<'a> {
    pub confirmer: bool,
    pub motif: Option<&'a str>,
}

/// D-069 M5. Une admission **directe** (hors liste d'attente) sur une offre dont les places libres sont déjà
/// désignées à des dossiers de la liste ne les consomme pas en silence : il faut une confirmation explicite avec
/// un motif ; la désignation reprise est alors annulée (tracée, motif), le dossier garde son rang et son tour
/// reviendra à la prochaine place libre.
pub(crate) async fn reprendre_place_designee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    offre: Uuid,
    dossier: Uuid,
    disponibles: i64,
    passer_devant: PasserDevant<'_>,
) -> Result<(), EntDbError> {
    let designees: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, dossier_id FROM ent_inscription_proposition_place
          WHERE etablissement_id = $1 AND offre_id = $2 AND origine = 'liste_attente' AND statut = 'designee'
            AND dossier_id <> $3
          ORDER BY rang_au_moment DESC NULLS LAST, designee_le DESC, id
          FOR UPDATE",
    )
    .bind(etab)
    .bind(offre)
    .bind(dossier)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let a_reprendre = i64::try_from(designees.len()).unwrap_or(0) - (disponibles - 1).max(0);
    if a_reprendre <= 0 {
        return Ok(());
    }
    let motif = passer_devant.motif.map(str::trim).filter(|m| !m.is_empty());
    let Some(motif) = motif.filter(|m| passer_devant.confirmer && m.chars().count() >= 3) else {
        return Err(conflit(
            "la dernière place est déjà désignée à un dossier de la liste d'attente : \
             confirmer explicitement (confirmer_passer_devant) avec un motif (motif_passer_devant)",
        ));
    };
    if motif.chars().count() > 400 {
        return Err(invalide("motif : 400 caractères au plus"));
    }
    let texte = format!("Place reprise par une admission directe : {motif}");
    for (proposition, autre) in designees
        .into_iter()
        .take(usize::try_from(a_reprendre).unwrap_or(0))
    {
        sqlx::query(
            "UPDATE ent_inscription_proposition_place SET statut = 'annulee', motif = left($3, 500)
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(proposition)
        .bind(&texte)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        suivi_tx(
            conn,
            etab,
            autre,
            "liste_attente",
            None,
            Some("Place désignée reprise par une admission directe (motif interne) : le dossier garde son rang"),
            false,
            None,
        )
        .await?;
    }
    suivi_tx(
        conn,
        etab,
        dossier,
        "liste_attente",
        None,
        Some(&format!("Admission directe sur une place désignée à la liste d'attente : {motif}")),
        false,
        None,
    )
    .await?;
    Ok(())
}

/// **Le** calcul du rang (passe 4) : le rang n'est plus stocké, il se **calcule en lecture** par la fonction SQL
/// `ent_inscription_rang_attente` (`row_number()` sur `ordre_attente`, jamais renuméroté). Tous les appelants passent par ici.
/// `famille = false` : le rang réel vu par le personnel, parmi les dossiers en liste. `famille = true` : le rang vu par la
/// famille, parmi les seuls dossiers dont l'entrée ou la sortie est **publiée** (`ent_inscription_en_liste_visible`).
async fn rang_calcule_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
    famille: bool,
) -> Result<Option<i64>, EntDbError> {
    sqlx::query_scalar("SELECT ent_inscription_rang_attente($1, $2, $3)")
        .bind(etab)
        .bind(dossier)
        .bind(famille)
        .fetch_one(conn)
        .await
        .map_err(db)
}

/// Position (1, 2, 3…) du dossier sur la liste **réelle** de son offre (lue par le personnel, jamais par la famille :
/// voir [`rang_famille_tx`]).
pub(crate) async fn position_attente_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
) -> Result<Option<i64>, EntDbError> {
    rang_calcule_tx(conn, etab, dossier, false).await
}

/// **Rang lu par la famille** (D-069 règle 1, M-A) : l'**unique** calcul, utilisé partout où la famille lit son rang
/// (`dossier_famille`, `decision_famille`). Un dossier compte dans la file selon sa dernière décision **publiée**
/// (fonction SQL `ent_inscription_en_liste_visible`) : il reste compté tant que sa sortie de liste n'est pas
/// publiée, il n'y entre qu'une fois son entrée publiée ; l'ordre est celui de l'entrée sur la liste, que les
/// départs non publiés ne décalent donc jamais. `None` : le dossier lui-même n'est pas visiblement en liste, ou une
/// décision plus récente que la dernière publiée existe (rien ne se lit alors).
pub(crate) async fn rang_famille_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
) -> Result<Option<i64>, EntDbError> {
    if crate::inscriptions_decisions::visibilite_famille_tx(conn, etab, dossier)
        .await?
        .is_some()
    {
        return Ok(None);
    }
    rang_calcule_tx(conn, etab, dossier, true).await
}

/// **La** règle de lecture du rang par la famille (passe 3) : seul un dossier réellement en liste d'attente a un rang, lu
/// par [`rang_famille_tx`] (qui masque tout ce que la famille n'a pas lu). Utilisée par `dossier_famille` **et** par
/// `decision_famille` : un dossier en liste sans aucune décision n'a pas un rang d'un côté et pas de l'autre.
pub(crate) async fn rang_lu_par_la_famille_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
    etat_reel: &str,
) -> Result<Option<i64>, EntDbError> {
    if etat_reel != "liste_attente" {
        return Ok(None);
    }
    rang_famille_tx(conn, etab, dossier).await
}

/// Premier dossier de la liste à désigner (`$1` établissement, `$2` offre, `$3` exclus, `$4` numéro de la place
/// courante), dans l'ordre d'entrée sur la liste (`ordre_attente`). Aucun verrou de dossier : l'appelant tient le verrou
/// **de l'offre**, que toute écriture de la liste prend d'abord (passe 4).
const SUIVANT_A_DESIGNER: &str = "SELECT d.id FROM ent_inscription_dossier d
       JOIN ent_inscription_liste_attente($1, $2, false) l ON l.dossier_id = d.id
      WHERE d.etablissement_id = $1
        AND NOT EXISTS (SELECT 1 FROM ent_inscription_proposition_place p
                         WHERE p.etablissement_id = d.etablissement_id AND p.dossier_id = d.id
                           AND p.statut IN ('designee', 'envoyee'))
        AND NOT EXISTS (SELECT 1 FROM ent_inscription_proposition_place e
                         WHERE e.etablissement_id = d.etablissement_id AND e.dossier_id = d.id
                           AND e.offre_id = $2 AND e.statut = 'ecartee' AND e.place_no = $4)
        AND NOT (d.id = ANY($3))
      ORDER BY l.pos LIMIT 1";

/// Journal **en ajout seul** des levées, constats et acceptations au guichet (`ent_inscription_condition_levee`).
#[allow(clippy::too_many_arguments)] // une ligne de journal : chaque champ est une colonne
async fn journaliser_condition_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
    code: &str,
    genre: &str,
    motif: &str,
    auteur: Uuid,
    source_ref: Option<Uuid>,
    quand: DateTime<Utc>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_inscription_condition_levee
            (id, etablissement_id, dossier_id, code, motif, auteur_id, cree_le, genre, source_ref)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(dossier)
    .bind(code)
    .bind(motif)
    .bind(auteur)
    .bind(quand)
    .bind(genre)
    .bind(source_ref)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Offres que touche un dossier : celle de son vœu retenu et celle de la liste où il est (ou a été) placé. Lues **sans verrou**.
async fn offres_des_dossiers_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossiers: &[Uuid],
) -> Result<Vec<Uuid>, EntDbError> {
    let mut offres: Vec<Uuid> = sqlx::query_scalar(
        "SELECT o FROM (
            SELECT v.offre_id AS o FROM ent_inscription_dossier d
              JOIN ent_inscription_voeu v ON v.etablissement_id = d.etablissement_id AND v.id = d.voeu_retenu_id
             WHERE d.etablissement_id = $1 AND d.id = ANY($2)
            UNION
            SELECT d.offre_attente_id FROM ent_inscription_dossier d
             WHERE d.etablissement_id = $1 AND d.id = ANY($2)) t
          WHERE o IS NOT NULL",
    )
    .bind(etab)
    .bind(dossiers)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    offres.sort_unstable();
    offres.dedup();
    Ok(offres)
}

/// **L'unique fonction qui pose l'ordre des verrous** (passe 4) : **campagne → offres → dossiers**.
/// * la campagne (si l'appelant en a besoin) en `FOR NO KEY UPDATE`, **avant** les offres ;
/// * **un verrou unique et grossier par offre** : `pg_advisory_xact_lock` sur la clé dérivée de l'identifiant de l'offre
///   (`ent_inscription_offre_cle`), offres triées par UUID. Toute écriture qui touche la liste d'attente, les places, les
///   propositions ou la désignation d'une offre commence par là ; le rang ne se stocke plus, aucune écriture n'a donc à
///   toucher les autres dossiers de la liste ;
/// * les dossiers ensuite, triés par UUID, en `FOR NO KEY UPDATE` (comme un `UPDATE` ordinaire) ;
/// * les propositions en dernier (par `verrouiller_proposition_tx`).
///
/// On **attend** : l'ordre étant unique, attendre ne produit jamais d'interblocage. Les verrous déjà tenus ne coûtent rien
/// (réentrants). Aucun autre `FOR UPDATE` sur une offre ou un dossier n'existe dans les fichiers d'inscription (garde statique).
pub(crate) async fn verrouiller_dans_l_ordre_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne: Option<Uuid>,
    offres: &[Uuid],
    dossiers: &[Uuid],
) -> Result<(), EntDbError> {
    if let Some(c) = campagne {
        sqlx::query(
            "SELECT id FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2 FOR NO KEY UPDATE",
        )
        .bind(etab)
        .bind(c)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    let mut offres = offres.to_vec();
    offres.sort_unstable();
    offres.dedup();
    for o in offres {
        sqlx::query("SELECT pg_advisory_xact_lock(ent_inscription_offre_cle($1))")
            .bind(o)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
    }
    let mut dossiers = dossiers.to_vec();
    dossiers.sort_unstable();
    dossiers.dedup();
    if !dossiers.is_empty() {
        sqlx::query(
            "SELECT id FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = ANY($2)
              ORDER BY id FOR NO KEY UPDATE",
        )
        .bind(etab)
        .bind(&dossiers)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

/// Verrouille des `dossiers` **avec** les offres qu'ils touchent (lues d'abord sans verrou, plus `offres_en_plus` : l'offre
/// que l'appelant va retenir) : offres, puis dossiers, par [`verrouiller_dans_l_ordre_tx`]. Les offres sont relues sous
/// verrou : un dossier déplacé entre-temps vers une offre non verrouillée donne un **conflit réessayable** (la transaction
/// s'arrête, elle n'attend jamais hors ordre). `dossier_tx` passe par ici : tout verrou de dossier est précédé du verrou de
/// ses offres.
pub(crate) async fn verrouiller_dossiers_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne: Option<Uuid>,
    dossiers: &[Uuid],
    offres_en_plus: &[Uuid],
) -> Result<(), EntDbError> {
    let mut offres = offres_des_dossiers_tx(conn, etab, dossiers).await?;
    offres.extend_from_slice(offres_en_plus);
    verrouiller_dans_l_ordre_tx(conn, etab, campagne, &offres, dossiers).await?;
    let relues = offres_des_dossiers_tx(conn, etab, dossiers).await?;
    if relues.iter().any(|o| !offres.contains(o)) {
        return Err(conflit(crate::inscriptions::EN_COURS_DE_MODIFICATION));
    }
    Ok(())
}

/// Une **nouvelle** place est libre sur l'offre (place rendue, capacité en hausse) : nouveau numéro de place. Les
/// dossiers qui avaient passé leur tour pour une place précédente redeviennent désignables (D-069 M-C).
pub(crate) async fn place_liberee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    offre: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_inscription_offre SET liberations = liberations + 1 WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(offre)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// **Désigne** les suivants de la liste tant qu'il y a des places libres non déjà promises à une désignation.
/// Ne décide rien, n'envoie rien. `exclure` : les dossiers qui viennent de quitter l'offre ou d'y être placés en
/// attente ne sont pas désignés pour la place qui se libère à l'instant. **Passer son tour** ne vaut que pour la place
/// concernée, **durablement** (D-069 M-C) : un dossier dont une proposition `ecartee` porte le **numéro de la place
/// courante** (`ent_inscription_offre.liberations`) n'est plus désigné pour elle — ni par la désignation automatique,
/// ni par l'appel explicite sans exclusion — mais redevient désignable à son rang pour une place libérée ensuite.
pub(crate) async fn designer_suivants_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    offre: Uuid,
    exclure: &[Uuid],
) -> Result<i64, EntDbError> {
    // Passe 4 : le verrou de l'offre est pris **en premier** par tout appelant (`verrouiller_dossiers_tx`,
    // `verrouiller_dans_l_ordre_tx`) ; le reprendre ici ne coûte rien s'il est déjà tenu (réentrant) et garantit l'exclusion
    // sinon. Aucun dossier n'est verrouillé : toute écriture qui change la place d'un dossier dans la liste tient d'abord ce
    // verrou, la liste lue ici est donc stable ; le rang ne se stocke plus.
    verrouiller_dans_l_ordre_tx(conn, etab, None, &[offre], &[]).await?;
    let place_no: i64 = sqlx::query_scalar(
        "SELECT liberations FROM ent_inscription_offre WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(offre)
    .fetch_one(&mut *conn)
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
        let suivant: Option<Uuid> = sqlx::query_scalar(SUIVANT_A_DESIGNER)
            .bind(etab)
            .bind(offre)
            .bind(exclure)
            .bind(place_no)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
        let Some(suivant) = suivant else {
            break;
        };
        let pos = position_attente_tx(conn, etab, suivant).await?;
        sqlx::query(
            "INSERT INTO ent_inscription_proposition_place
                (id, etablissement_id, dossier_id, offre_id, origine, statut, rang_au_moment, place_no)
             VALUES ($1,$2,$3,$4,'liste_attente','designee',$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(suivant)
        .bind(offre)
        .bind(pos.and_then(|p| i32::try_from(p).ok()))
        .bind(place_no)
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
/// **désigne** le suivant quand une place se libère (le rang n'est plus stocké : rien à remettre en suite).
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
    // Passe 4 : le verrou de l'offre est déjà tenu (`dossier_tx` le prend avant le dossier) ; le reprendre ne coûte rien. Le
    // rang ne se stocke plus : un départ ne touche aucun autre dossier de la liste, il n'y a donc rien à compacter ni à
    // verrouiller en plus.
    if let Some(o) = offre {
        verrouiller_dans_l_ordre_tx(conn, etab, None, &[o], &[]).await?;
    }
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
            if vers != "admis" {
                // Une désignation qui tombe rend sa place : le suivant est désigné.
                designer_suivants_tx(conn, etab, o, &[d.id]).await?;
            }
        }
    }
    if libere_une_place {
        annuler_propositions_ouvertes_tx(conn, etab, d.id, "place libérée par le dossier").await?;
        if let Some(o) = offre {
            place_liberee_tx(conn, etab, o).await?;
            designer_suivants_tx(conn, etab, o, &[d.id]).await?;
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
    let ouverte: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, statut FROM ent_inscription_proposition_place
          WHERE etablissement_id = $1 AND dossier_id = $2 AND statut IN ('designee', 'envoyee') FOR UPDATE",
    )
    .bind(etab)
    .bind(d.id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some((p, statut)) = ouverte {
        if statut == "envoyee" {
            // D-069 M6 : une proposition **envoyée** n'est jamais réécrite (la famille l'a lue, elle a un délai).
            // Elle est annulée, avec sa trace, et une nouvelle proposition prend sa place.
            sqlx::query(
                "UPDATE ent_inscription_proposition_place
                    SET statut = 'annulee', motif = 'Proposition envoyée remplacée par une nouvelle décision'
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(p)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            suivi_tx(
                conn,
                etab,
                d.id,
                "liste_attente",
                None,
                Some("Proposition de place envoyée annulée : remplacée par une nouvelle décision"),
                false,
                None,
            )
            .await?;
        } else {
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
    // Une nouvelle proposition (autre offre, nouvelle décision) demande une nouvelle réponse de la famille : la
    // condition est réinitialisée, explicitement tracée (la levée éventuelle reste au journal, jamais effacée).
    let reinit = sqlx::query(
        "UPDATE ent_inscription_condition SET remplie = false, remplie_le = NULL, remplie_par = NULL,
                source = NULL, source_ref = NULL, motif = NULL
          WHERE etablissement_id = $1 AND dossier_id = $2 AND code = 'reponse_famille' AND remplie",
    )
    .bind(etab)
    .bind(d.id)
    .execute(&mut *conn)
    .await
    .map_err(db)?
    .rows_affected();
    if reinit > 0 {
        suivi_tx(
            conn,
            etab,
            d.id,
            "condition",
            None,
            Some("Réponse de la famille réinitialisée : une nouvelle proposition demande une nouvelle réponse"),
            false,
            None,
        )
        .await?;
    }
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
            let retiree = sqlx::query(
                "DELETE FROM ent_inscription_condition
                  WHERE etablissement_id = $1 AND dossier_id = $2 AND code = 'reserve'",
            )
            .bind(etab)
            .bind(d.id)
            .execute(&mut *conn)
            .await
            .map_err(db)?
            .rows_affected();
            if retiree > 0 {
                suivi_tx(
                    conn,
                    etab,
                    d.id,
                    "condition",
                    None,
                    Some("Réserve retirée : la nouvelle décision n'en porte plus"),
                    false,
                    None,
                )
                .await?;
            }
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

/// `voit_motif` : le motif d'une proposition (« Tour passé… », « Place reprise par une admission directe : … ») peut
/// parler d'un autre candidat : il ne se lit qu'avec `inscriptions.decider`, comme le motif interne des décisions.
fn proposition_dto(r: LignePropositionBrute, jour: NaiveDate, voit_motif: bool) -> PropositionDto {
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
        motif: if voit_motif { r.13 } else { None },
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
    voit_motif: bool,
) -> Result<PropositionDto, EntDbError> {
    sqlx::query_as::<_, LignePropositionBrute>(&format!(
        "{SELECT_PROPOSITION} WHERE p.etablissement_id = $1 AND p.id = $2"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .map(|r| proposition_dto(r, jour, voit_motif))
    .ok_or_else(introuvable)
}

/// Une proposition lue sous verrou, **après** le verrou du dossier.
struct PropositionVerrouillee {
    dossier_id: Uuid,
    offre_id: Option<Uuid>,
    origine: String,
    statut: String,
    rang_au_moment: Option<i32>,
    repondre_avant: Option<NaiveDate>,
    decision_id: Option<Uuid>,
}

/// Ordre des verrous **unique** (passe 4) : le contrôle d'écart d'abord (**avant** tout verrou de lot : un acteur écarté
/// reçoit « introuvable » sans avoir rien tenu), puis les offres et le dossier (`verrouiller_dossiers_tx`), la proposition en
/// dernier. Garde de déport/lien appliquée au passage (une proposition d'un dossier dont on est écarté n'existe pas).
async fn verrouiller_proposition_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    id: Uuid,
) -> Result<PropositionVerrouillee, EntDbError> {
    let dossier: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT dossier_id, offre_id FROM ent_inscription_proposition_place WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let (dossier, offre) = dossier.ok_or_else(introuvable)?;
    dossier_visible_tx(conn, etab, actor, dossier).await?;
    let offres: Vec<Uuid> = offre.into_iter().collect();
    verrouiller_dossiers_tx(conn, etab, None, &[dossier], &offres).await?;
    dossier_tx(conn, etab, dossier).await?;
    let p: Option<(Uuid, Option<Uuid>, String, String, Option<i32>, Option<NaiveDate>, Option<Uuid>)> = sqlx::query_as(
        "SELECT dossier_id, offre_id, origine, statut, rang_au_moment, repondre_avant, decision_id
           FROM ent_inscription_proposition_place
          WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    let p = p.ok_or_else(introuvable)?;
    Ok(PropositionVerrouillee {
        dossier_id: p.0,
        offre_id: p.1,
        origine: p.2,
        statut: p.3,
        rang_au_moment: p.4,
        repondre_avant: p.5,
        decision_id: p.6,
    })
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
        maintenant: DateTime<Utc>,
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
                SET remplie = true, remplie_le = $7, remplie_par = $4,
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
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("condition absente de ce dossier ou déjà remplie"));
        }
        // D-069 F14 : le constat est journalisé **en ajout seul**, comme la levée (code, motif, auteur, objet source).
        journaliser_condition_tx(
            &mut tx,
            etab,
            id,
            code,
            "remplie",
            motif.as_deref().filter(|m| m.chars().count() >= 3).unwrap_or("Constat sans motif"),
            actor,
            b.source_ref,
            maintenant,
        )
        .await?;
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
        maintenant: DateTime<Utc>,
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
                SET remplie = true, remplie_le = $6, remplie_par = $4, source = 'levee', source_ref = NULL,
                    motif = $5
              WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3 AND NOT remplie",
        )
        .bind(etab)
        .bind(id)
        .bind(code)
        .bind(actor)
        .bind(motif)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(conflit("condition absente de ce dossier ou déjà remplie"));
        }
        // D-069 M7 : la levée est **journalisée en ajout seul** (code, motif, auteur). La condition peut être
        // réinitialisée ensuite (nouvelle proposition, nouvelle réserve) : cette trace, elle, reste.
        journaliser_condition_tx(&mut tx, etab, id, code, "levee", motif, actor, None, maintenant).await?;
        suivi_tx(
            &mut tx,
            etab,
            id,
            "condition",
            None,
            Some(&format!("Condition « {code} » levée par la direction (motif au journal des levées)")),
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
        verrouiller_dans_l_ordre_tx(&mut tx, etab, None, &[offre], &[]).await?;
        designer_suivants_tx(&mut tx, etab, offre, &[]).await?;
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
        let (_, voit_motif, _) = self.ins_personnel(etab, actor, role).await?;
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
        // Les dossiers dont l'acteur est écarté sont retirés **avant** la limite : une liste tronquée ne cache jamais une
        // proposition qui lui est due derrière des lignes qu'il n'a pas le droit de voir.
        let candidats: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT p.dossier_id FROM ent_inscription_proposition_place p
               JOIN ent_inscription_dossier d ON d.etablissement_id = p.etablissement_id AND d.id = p.dossier_id
              WHERE p.etablissement_id = $1 AND ($2::uuid IS NULL OR d.campagne_id = $2)
                AND ($3::text IS NULL OR p.statut = $3)",
        )
        .bind(etab)
        .bind(campagne)
        .bind(&statut)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let deportes: Vec<Uuid> = ecartes_parmi(&mut tx, etab, actor, &candidats).await?.into_iter().collect();
        let rows = sqlx::query_as::<_, LignePropositionBrute>(&format!(
            "{SELECT_PROPOSITION}
              WHERE p.etablissement_id = $1 AND ($2::uuid IS NULL OR d.campagne_id = $2)
                AND ($3::text IS NULL OR p.statut = $3) AND NOT (p.dossier_id = ANY($4))
              ORDER BY p.designee_le DESC, p.id LIMIT 500"
        ))
        .bind(etab)
        .bind(campagne)
        .bind(statut)
        .bind(&deportes)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| proposition_dto(r, jour, voit_motif))
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
        let p = verrouiller_proposition_tx(&mut tx, etab, actor, id).await?;
        let (dossier, offre, origine, statut, rang) =
            (p.dossier_id, p.offre_id, p.origine, p.statut, p.rang_au_moment);
        if statut != "designee" || origine != "liste_attente" {
            return Err(conflit(
                "seule une place désignée depuis la liste d'attente s'envoie",
            ));
        }
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
        let dto = proposition_tx(&mut tx, etab, id, jour, true).await?;
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
        let p = verrouiller_proposition_tx(&mut tx, etab, actor, id).await?;
        let (dossier, offre, statut) = (p.dossier_id, p.offre_id, p.statut);
        if statut != "designee" {
            return Err(conflit(
                "seule une place désignée (pas encore envoyée) peut être écartée",
            ));
        }
        // Passe 3 (6.5) : seule une désignation de la liste d'attente, sans décision attachée, se passe. Une place
        // rattachée à une décision (admission directe, programmée ou non publiée) se défait par une nouvelle décision.
        if p.origine != "liste_attente" || p.decision_id.is_some() {
            return Err(conflit(
                "cette place est rattachée à une décision : passer par une nouvelle décision pour la modifier",
            ));
        }
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
            designer_suivants_tx(&mut tx, etab, o, &[dossier]).await?;
        }
        let jour = jour_tx(&mut tx, etab, maintenant).await?;
        let dto = proposition_tx(&mut tx, etab, id, jour, true).await?;
        commit(tx).await?;
        Ok(dto)
    }

    /// **Acceptation au guichet** (D-069 F15) : le secrétariat enregistre l'acceptation de la place par la famille
    /// (venue au guichet, par téléphone). Mêmes effets que la réponse de la famille — proposition `acceptee`,
    /// condition « réponse de la famille » remplie —, mais **tracée comme telle** : motif obligatoire, auteur, journal
    /// en ajout seul (genre `guichet`), suivi interne, événement ; jamais confondue avec la réponse donnée par la
    /// famille elle-même.
    pub async fn accepter_proposition_au_guichet_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &MotifBody,
        maintenant: DateTime<Utc>,
    ) -> Result<PropositionDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let voit_motif = self.ins_peut(etab, actor, role, Action::InscriptionsDecider).await?;
        let motif = b.motif.trim();
        if motif.chars().count() < 3 || motif.chars().count() > 500 {
            return Err(invalide("motif : de 3 à 500 caractères (comment la famille a répondu)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = jour_tx(&mut tx, etab, maintenant).await?;
        let p = verrouiller_proposition_tx(&mut tx, etab, actor, id).await?;
        // Refus **distincts** (D-069 F15, passe 3) : chaque cas a son message exact.
        match p.statut.as_str() {
            "envoyee" => {}
            "acceptee" | "declinee" => return Err(conflit("proposition déjà répondue")),
            "designee" => return Err(conflit("place désignée, pas encore envoyée à la famille : rien à accepter")),
            "expiree" => return Err(conflit("proposition expirée : le délai de réponse a été constaté dépassé")),
            _ => return Err(conflit("proposition annulée ou écartée : rien à accepter")),
        }
        let d = dossier_visible_tx(&mut tx, etab, actor, p.dossier_id).await?;
        if d.etat != "admis" {
            return Err(conflit("le dossier n'est plus à l'état admis : aucune place à accepter"));
        }
        // Passé `repondre_avant`, le guichet n'accepte plus tant que l'expiration n'est pas constatée : la direction la
        // constate (puis décide), le secrétariat n'efface pas un délai par un motif.
        if p.repondre_avant.is_some_and(|j| j < jour) {
            return Err(conflit(
                "le délai de réponse est dépassé : l'expiration est à constater avant toute autre suite",
            ));
        }
        sqlx::query(
            "UPDATE ent_inscription_proposition_place SET statut = 'acceptee', repondu_le = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_inscription_condition
                SET remplie = true, remplie_le = $3, remplie_par = $4, source = 'personnel', motif = $5
              WHERE etablissement_id = $1 AND dossier_id = $2 AND code = 'reponse_famille' AND NOT remplie",
        )
        .bind(etab)
        .bind(p.dossier_id)
        .bind(maintenant)
        .bind(actor)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser_condition_tx(
            &mut tx,
            etab,
            p.dossier_id,
            "reponse_famille",
            "guichet",
            motif,
            actor,
            Some(id),
            maintenant,
        )
        .await?;
        suivi_tx(
            &mut tx,
            etab,
            p.dossier_id,
            "reponse",
            None,
            Some("Acceptation de la place enregistrée au guichet par le secrétariat (motif au journal)"),
            true,
            Some(actor),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_REPONSE_FAMILLE,
            AggregateKind::Admission,
            p.dossier_id,
            None,
            json!({ "dossier_id": p.dossier_id, "proposition_id": id, "reponse": "acceptee", "guichet": true }),
            Some(actor),
        )
        .await?;
        reevaluer_tx(&mut tx, etab, p.dossier_id, Some(actor), false).await?;
        // La famille est prévenue : sa proposition n'est plus « à répondre » (statut `acceptee`) et elle lit la suite.
        PgEnt::prevenir_familles_tx(
            &mut tx,
            etab,
            &[p.dossier_id],
            "Votre réponse à la place proposée a été enregistrée",
            "Le secrétariat a enregistré votre acceptation de la place proposée à votre enfant. Vous pouvez suivre la suite dans votre espace.",
            &format!("guichet-{id}"),
        )
        .await?;
        let dto = proposition_tx(&mut tx, etab, id, jour, voit_motif).await?;
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
        let p = verrouiller_proposition_tx(&mut tx, etab, actor, id).await?;
        let (dossier, statut, avant) = (p.dossier_id, p.statut, p.repondre_avant);
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
        let dto = proposition_tx(&mut tx, etab, id, jour, true).await?;
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
        // Ordre des verrous (passe 4) : l'offre de la proposition envoyée et celles du dossier, puis le dossier, puis la
        // proposition — avant toute autre lecture sous verrou.
        let offre_p: Option<Option<Uuid>> = sqlx::query_scalar(
            "SELECT offre_id FROM ent_inscription_proposition_place
              WHERE etablissement_id = $1 AND dossier_id = $2 AND statut = 'envoyee'",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let offres: Vec<Uuid> = offre_p.flatten().into_iter().collect();
        verrouiller_dossiers_tx(&mut tx, etab, None, &[d0.id], &offres).await?;
        let d = dossier_tx(&mut tx, etab, d0.id).await?;
        // Règle 1 : la famille ne répond que sur ce qui est publié ; une décision non publiée en cours ne se révèle pas.
        crate::inscriptions_decisions::exiger_dossier_stable_famille_tx(&mut tx, etab, id).await?;
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
    let rows: Vec<(Uuid, String, String, String, i64, Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT d.id, d.reference, d.eleve_nom, d.eleve_prenom,
                l.pos, p.id, p.statut
           FROM ent_inscription_liste_attente($1, $2, false) l
           JOIN ent_inscription_dossier d ON d.etablissement_id = $1 AND d.id = l.dossier_id
      LEFT JOIN ent_inscription_proposition_place p
             ON p.etablissement_id = d.etablissement_id AND p.dossier_id = d.id AND p.statut IN ('designee', 'envoyee')
          ORDER BY l.pos",
    )
    .bind(etab)
    .bind(offre)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let ids: Vec<Uuid> = rows.iter().map(|r| r.0).collect();
    let deportes = ecartes_parmi(conn, etab, actor, &ids).await?;
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
