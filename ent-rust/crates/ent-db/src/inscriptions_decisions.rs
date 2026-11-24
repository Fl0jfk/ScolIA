//! Socle 3, lot I5 — **décisions**, avis internes, déport, commissions et publication
//! (`docs/socles/03_INSCRIPTIONS.md` §7.1, §8.7, §11.4 : INS-04, INS-31 à INS-33, INS-62 à INS-65).
//!
//! Règles tenues ici :
//! * une décision est **une ligne en ajout seul** (`ent_inscription_decision`, déclencheur de la migration 313) ;
//!   la revoir crée une ligne qui remplace la précédente, avec un motif interne, jamais une écriture ;
//! * le **motif interne** est obligatoire et ne quitte jamais le personnel ; la famille lit le motif communiqué,
//!   selon la règle de la campagne, **une fois la décision publiée** ;
//! * **aucune décision n'est prise par une règle** : le système désigne (liste d'attente) ou calcule (conditions),
//!   une personne décide, envoie, publie ;
//! * la **capacité** d'une offre est tenue sous verrou (un dépassement se confirme et se trace) ;
//! * un membre du personnel qui s'est **déporté** d'un dossier n'y décide plus, n'y lit plus d'avis et ne le voit
//!   plus (INS-63) ;
//! * rien de ce qui concerne un autre responsable ou un autre candidat n'atteint la famille.
//!
//! Le temps est toujours **passé en paramètre** (`maintenant`) : aucun test ne dépend de l'heure réelle.

use chrono::{DateTime, Utc};
use domain_eleve::DomainError;
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind};
use ent_types::inscriptions_decisions::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::inscriptions::{
    commit, conflit, db, dossier_tx, introuvable, invalide, opt, publier, suivi_tx,
    transitionner_tx, CampagneRow, DossierRow, COLS_CAMPAGNE,
};
use crate::inscriptions_etats::Cause;
use crate::inscriptions_modele;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Sens de décision connus.
pub const SENS_DECISION: [&str; 5] = [
    "admis",
    "admis_sous_reserve",
    "liste_attente",
    "refuse",
    "ajourne",
];
const CONTEXTES_AVIS: [&str; 3] = ["entretien", "commission", "pedagogique"];
const SENS_AVIS: [&str; 4] = ["favorable", "reserve", "defavorable", "sans_avis"];
/// Plage calme des publications (heure de l'établissement) : de 20 h à 8 h et le dimanche. Valeur du logiciel
/// (R-I7) ; le calendrier des jours fériés relève du socle 5.
const PLAGE_CALME_DEBUT: i32 = 20;
const PLAGE_CALME_FIN: i32 = 8;

/// État du dossier que produit un sens de décision.
pub fn etat_cible(sens: &str) -> &'static str {
    match sens {
        "admis" | "admis_sous_reserve" => "admis",
        "liste_attente" => "liste_attente",
        "refuse" => "refuse",
        _ => "en_instruction",
    }
}

/// **Famille d'états compatibles** avec un sens de décision (D-069, M6) : la décision programmée reste publiable tant que
/// l'état **effectif** du dossier (blocage pris en compte) est dans cette famille ; sinon elle est caduque. Une égalité
/// stricte avec `etat_cible` déclarait caduque une décision que rien n'avait contredite.
/// * `admis`, `admis_sous_reserve` → `admis` ou `confirme` : la famille qui a répondu à la place, ou dont les conditions sont
///   remplies, n'invalide pas l'admission qu'on s'apprête à lui annoncer ;
/// * `liste_attente` → `liste_attente` ; `refuse` → `refuse` : aucun autre état ne les porte ;
/// * `ajourne` → `en_instruction` ou `attente_famille` : l'ajournement reporte l'examen ; que le dossier attende une pièce de
///   la famille ou reste en instruction ne le contredit pas. Tout autre état (admis, refusé, clos, annulé…) le rend caduc.
pub(crate) fn etats_compatibles(sens: &str) -> &'static [&'static str] {
    match sens {
        "admis" | "admis_sous_reserve" => &["admis", "confirme"],
        "liste_attente" => &["liste_attente"],
        "refuse" => &["refuse"],
        _ => &["en_instruction", "attente_famille"],
    }
}

fn prend_une_place(sens: &str) -> bool {
    matches!(sens, "admis" | "admis_sous_reserve")
}

fn longueur(s: &str) -> usize {
    s.chars().count()
}

/// Une publication programmée ne va pas au-delà de ce délai (borne du logiciel, F3).
const PUBLICATION_MAX_JOURS: i64 = 90;
/// Plafonds d'une séance de commission (F3).
const DOSSIERS_PAR_SEANCE_MAX: usize = 500;

/// Les membres d'une commission sont des **membres du personnel de l'établissement**, en poste (D-069 E2/F3).
async fn valider_membres_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    membres: &[Uuid],
) -> Result<(), EntDbError> {
    let distincts: std::collections::HashSet<&Uuid> = membres.iter().collect();
    if distincts.is_empty() {
        return Ok(());
    }
    let lignes: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT user_id, role FROM ent_membership
          WHERE etablissement_id = $1 AND etat = 'ouvert' AND user_id = ANY($2)",
    )
    .bind(etab)
    .bind(membres)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    let ok = lignes
        .iter()
        .filter(|(_, r)| Role::parse(r).is_some_and(|r| r.is_staff()))
        .count();
    if ok != distincts.len() {
        return Err(invalide(
            "membres : des membres du personnel de l'établissement uniquement (en poste)",
        ));
    }
    Ok(())
}

fn borner_publication(quand: DateTime<Utc>, maintenant: DateTime<Utc>) -> Result<(), EntDbError> {
    if quand > maintenant + chrono::Duration::days(PUBLICATION_MAX_JOURS) {
        return Err(invalide(
            "publication programmée : 90 jours au plus à l'avance",
        ));
    }
    Ok(())
}

/// Rattache (ou détache, `None`) les lignes de suivi écrites ensuite dans la transaction à une décision : voir
/// `suivi_tx`. Réglage local à la transaction ; jamais posé hors de `decider_tx`.
async fn marquer_suivi_decision_tx(
    conn: &mut PgConnection,
    dossier: Uuid,
    decision: Option<Uuid>,
) -> Result<(), EntDbError> {
    let valeur = decision.map_or_else(String::new, |d| format!("{dossier}|{d}"));
    sqlx::query("SELECT set_config('ent.suivi_decision', $1, true)")
        .bind(valeur)
        .execute(conn)
        .await
        .map_err(db)?;
    Ok(())
}

/// Paramètres d'une décision prise dans une transaction (appelant : direction, commission, liste d'attente).
pub(crate) struct ContexteDecision<'a> {
    pub actor: Uuid,
    pub body: &'a DecisionCompleteBody,
    pub revoir: bool,
    pub origine: &'static str,
    pub commission_id: Option<Uuid>,
    pub maintenant: DateTime<Utc>,
    /// Faux seulement pour l'ancienne route (admission sans motif : « Admission prononcée »).
    pub motif_obligatoire: bool,
    /// Publication immédiate, quelle que soit `publier_le` (envoi d'une place de la liste d'attente).
    pub publier_tout_de_suite: bool,
    /// Vrai : sans heure donnée, la décision est publiée aussitôt. Faux (séance de commission) : elle reste prise
    /// mais non publiée, pour une publication groupée décidée par une personne.
    pub publier_par_defaut: bool,
    /// Ancienne route : sans motif communiqué explicite, le motif est communiqué (si la campagne le permet).
    pub communiquer_par_defaut: bool,
}

/// Le membre a déclaré son **déport** de ce dossier (INS-63).
pub(crate) async fn deporte_declare_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
    membre: Uuid,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM ent_inscription_deport
                         WHERE etablissement_id = $1 AND dossier_id = $2 AND membre_id = $3)",
    )
    .bind(etab)
    .bind(dossier)
    .bind(membre)
    .fetch_one(conn)
    .await
    .map_err(db)
}

/// Le membre est **lié** au candidat (compte ou adresse d'un responsable, du déposant ou d'un contact, deux comptes
/// d'une même personne : fonction SQL `ent_inscription_membre_lie`, migration 313).
pub(crate) async fn lie_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
    membre: Uuid,
) -> Result<bool, EntDbError> {
    sqlx::query_scalar("SELECT ent_inscription_membre_lie($1, $2, $3)")
        .bind(etab)
        .bind(dossier)
        .bind(membre)
        .fetch_one(conn)
        .await
        .map_err(db)
}

/// **Garde unique** (D-069 règle 2) : le membre est écarté d'office d'un dossier s'il s'en est déporté **ou** s'il
/// est lié au candidat. Toute lecture ou action du personnel sur un dossier passe par ici (code), et la base
/// refuse en second verrou l'avis et la décision d'un membre écarté.
pub(crate) async fn ecarte_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
    membre: Uuid,
) -> Result<bool, EntDbError> {
    if deporte_declare_tx(conn, etab, dossier, membre).await? {
        return Ok(true);
    }
    lie_tx(conn, etab, dossier, membre).await
}

/// Parmi `dossiers`, ceux dont `membre` est écarté (déport déclaré ou lien) : **une seule requête** pour toute une
/// liste (D-069 F11), jamais une garde par ligne et jamais un balayage de l'établissement.
pub(crate) async fn ecartes_parmi(
    conn: &mut PgConnection,
    etab: Uuid,
    membre: Uuid,
    dossiers: &[Uuid],
) -> Result<std::collections::HashSet<Uuid>, EntDbError> {
    if dossiers.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let v: Vec<Uuid> = sqlx::query_scalar(
        "SELECT x.id FROM unnest($3::uuid[]) AS x(id)
          WHERE EXISTS (SELECT 1 FROM ent_inscription_deport g
                         WHERE g.etablissement_id = $1 AND g.dossier_id = x.id AND g.membre_id = $2)
             OR ent_inscription_membre_lie($1, x.id, $2)",
    )
    .bind(etab)
    .bind(membre)
    .bind(dossiers)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(v.into_iter().collect())
}

/// Instant dans une **plage calme** (20 h – 8 h ou dimanche) à l'heure de l'établissement.
async fn en_plage_calme(
    conn: &mut PgConnection,
    etab: Uuid,
    instant: DateTime<Utc>,
) -> Result<bool, EntDbError> {
    let (heure, jour): (i32, i32) = sqlx::query_as(
        "WITH f AS (
             SELECT (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $1) AS z
         )
         SELECT extract(hour FROM ($2 AT TIME ZONE COALESCE(f.z, 'Europe/Paris')))::int,
                extract(isodow FROM ($2 AT TIME ZONE COALESCE(f.z, 'Europe/Paris')))::int
           FROM f",
    )
    .bind(etab)
    .bind(instant)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(!(PLAGE_CALME_FIN..PLAGE_CALME_DEBUT).contains(&heure) || jour == 7)
}

// ───────────────────────────── Décider ─────────────────────────────

/// Prend une décision sur un dossier (INS-04 ★, INS-33 ★, INS-65). Toutes les règles sont ici ; l'appelant
/// ouvre la transaction, contrôle le droit, valide la transaction. Renvoie l'identifiant de la décision.
pub(crate) async fn decider_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    c: &ContexteDecision<'_>,
) -> Result<Uuid, EntDbError> {
    let b = c.body;
    if !SENS_DECISION.contains(&b.sens.as_str()) {
        return Err(invalide(
            "décision : admis, admis_sous_reserve, liste_attente, refuse ou ajourne",
        ));
    }
    let motif = match opt(&Some(b.motif_interne.clone())) {
        Some(m) => m,
        None if !c.motif_obligatoire && b.sens == "admis" => "Admission prononcée".to_string(),
        None => {
            return Err(invalide(
                "motif interne obligatoire (3 caractères au moins)",
            ))
        }
    };
    if longueur(&motif) < 3 {
        return Err(invalide("motif interne trop court (3 caractères au moins)"));
    }
    if longueur(&motif) > 1000 {
        return Err(invalide("motif interne trop long"));
    }
    let reserve = opt(&b.reserve);
    match (b.sens.as_str(), &reserve) {
        ("admis_sous_reserve", None) => {
            return Err(invalide("la condition de la réserve est obligatoire"))
        }
        ("admis_sous_reserve", Some(r)) if longueur(r) < 3 || longueur(r) > 500 => {
            return Err(invalide("réserve : de 3 à 500 caractères"));
        }
        ("admis_sous_reserve", _) => {}
        (_, Some(_)) => {
            return Err(invalide(
                "une réserve ne s'attache qu'à une admission sous réserve",
            ))
        }
        _ => {}
    }
    let explicite = opt(&b.motif_communique);
    let mut motif_communique = explicite
        .clone()
        .or_else(|| c.communiquer_par_defaut.then(|| motif.clone()));
    if motif_communique
        .as_deref()
        .is_some_and(|m| longueur(m) > 1000)
    {
        return Err(invalide("motif communiqué trop long"));
    }

    // Ordre des verrous unique (passe 4) : **offres → dossier**, posé par `verrouiller_dossiers_tx` (un seul point). Les
    // offres touchées sont celles du dossier (qu'il quitte) et celle qu'il retient (deux déplacements croisés A→B et B→A,
    // même entre deux campagnes, se verrouillent dans le même ordre). Le contrôle d'écart passe **avant** tout verrou : un
    // acteur écarté reçoit « introuvable » sans avoir tenu aucun verrou de lot.
    if let Some((campagne_id, niveau)) = sqlx::query_as::<_, (Uuid, Option<String>)>(
        "SELECT campagne_id, niveau_demande FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    {
        if ecarte_tx(conn, etab, id, c.actor).await? {
            return Err(introuvable());
        }
        let cible_voulue = !matches!(b.sens.as_str(), "refuse" | "ajourne");
        // Une offre voulue invalide se refuse plus loin, à sa place dans l'ordre des refus : ici on ne fait que calculer ce
        // qu'il faut verrouiller.
        let cible: Option<Uuid> = if cible_voulue {
            inscriptions_modele::offre_cible_tx(conn, etab, id, campagne_id, niveau.as_deref(), b.offre_id)
                .await
                .ok()
                .flatten()
        } else {
            None
        };
        let en_plus: Vec<Uuid> = cible.into_iter().collect();
        crate::inscriptions_attente::verrouiller_dossiers_tx(conn, etab, None, &[id], &en_plus).await?;
        // Relue sous verrou : une offre cible qui a changé entre-temps (vœux modifiés) se rejoue, jamais d'attente hors ordre.
        if cible_voulue {
            let relue = inscriptions_modele::offre_cible_tx(conn, etab, id, campagne_id, niveau.as_deref(), b.offre_id)
                .await
                .ok()
                .flatten();
            if relue != cible {
                return Err(conflit(crate::inscriptions::EN_COURS_DE_MODIFICATION));
            }
        }
    }
    let d = dossier_tx(conn, etab, id).await?;
    // Garde unique (règle 2) : déporté ou lié au candidat, on ne décide pas — et le dossier n'existe pas pour soi.
    // Elle précède tout constat sur le dossier (nature, blocage) : un membre écarté n'en apprend rien.
    if ecarte_tx(conn, etab, id, c.actor).await? {
        return Err(introuvable());
    }
    if d.nature != "admission" {
        return Err(conflit(
            "une réinscription se valide sans décision d'admission",
        ));
    }
    if d.etat == "bloque_desaccord" {
        return Err(conflit(
            "dossier bloqué : désaccord des responsables (levée avec pièce par la direction)",
        ));
    }
    // Dernière décision = bout de la chaîne `remplace_decision_id` (règle 4), jamais un tri sur `now()`.
    let precedente: Option<(Uuid, String, Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT d.id, d.sens, d.offre_id, d.reserve FROM ent_inscription_decision d
          WHERE d.etablissement_id = $1 AND d.dossier_id = $2
            AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision y WHERE y.remplace_decision_id = d.id)",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;

    let e = d.etat.as_str();
    let initial = matches!(
        e,
        "deposee" | "en_instruction" | "attente_famille" | "liste_attente"
    );
    let revisable = matches!(e, "admis" | "confirme" | "refuse");
    if !initial && !revisable {
        return Err(conflit("décision impossible depuis l'état actuel"));
    }
    if revisable && !c.revoir {
        return Err(conflit(
            "dossier déjà décidé : la décision se revoit (motif obligatoire)",
        ));
    }
    if c.revoir && precedente.is_none() {
        return Err(conflit("aucune décision à revoir"));
    }
    let cible = etat_cible(&b.sens);
    if e == "confirme" && !matches!(cible, "liste_attente" | "refuse") {
        return Err(conflit(
            "un dossier confirmé ne se revoit que vers la liste d'attente ou le refus",
        ));
    }
    if e == "refuse" && cible == "refuse" {
        return Err(conflit("dossier déjà refusé"));
    }
    if e == cible && matches!(cible, "admis" | "liste_attente") {
        let identique = precedente.as_ref().is_some_and(|p| {
            p.1 == b.sens && (b.offre_id.is_none() || b.offre_id == p.2) && p.3 == reserve
        });
        if !c.revoir || identique {
            return Err(conflit("la décision ne change rien"));
        }
    }

    // Offre (vœu) actuellement retenue : sert à libérer la place si la décision la déplace.
    let offre_avant: Option<Uuid> =
        crate::inscriptions_attente::offre_du_dossier_tx(conn, etab, id).await?;

    let campagne = sqlx::query_as::<_, CampagneRow>(&format!(
        "SELECT {COLS_CAMPAGNE} FROM ent_inscription_campagne
          WHERE etablissement_id = $1 AND id = $2"
    ))
    .bind(etab)
    .bind(d.campagne_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if campagne.motif_communique == "jamais" {
        if explicite.is_some() {
            return Err(invalide(
                "cette campagne ne communique pas de motif aux familles",
            ));
        }
    } else if campagne.motif_communique == "toujours"
        && b.sens == "refuse"
        && motif_communique.is_none()
    {
        return Err(invalide(
            "cette campagne exige un motif communiqué pour un refus",
        ));
    }
    if campagne.motif_communique == "jamais" {
        motif_communique = None;
    }

    if prend_une_place(&b.sens) {
        crate::inscriptions_accords::exiger_accords_tx(&mut *conn, etab, &d).await?;
    }

    // Prise en charge implicite : on décide depuis l'instruction (§7.1).
    let mut cur = d.clone();
    if matches!(cur.etat.as_str(), "deposee" | "attente_famille") {
        cur = transitionner_tx(
            conn,
            etab,
            &cur,
            "en_instruction",
            Cause::Etape,
            Some("Prise en charge avec la décision"),
            Some(c.actor),
        )
        .await?;
    }

    // Capacité : sous verrou, un dépassement se confirme et se trace.
    let meme_place = e == "admis"
        && prend_une_place(&b.sens)
        && (b.offre_id.is_none() || b.offre_id == offre_avant);
    let place = if matches!(b.sens.as_str(), "refuse" | "ajourne") || meme_place {
        None
    } else {
        inscriptions_modele::retenir_voeu_tx(
            conn,
            etab,
            &cur,
            b.offre_id,
            prend_une_place(&b.sens),
            b.confirmer_depassement,
            crate::inscriptions_attente::PasserDevant {
                confirmer: b.confirmer_passer_devant,
                motif: b.motif_passer_devant.as_deref(),
            },
        )
        .await?
    };
    let offre_decision =
        place
            .as_ref()
            .map(|p| p.offre_id)
            .or(if meme_place { offre_avant } else { None });

    let decision_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_inscription_decision
            (id, etablissement_id, dossier_id, sens, offre_id, reserve, motif_interne, motif_communique,
             depassement_capacite, alternative, remplace_decision_id, decide_par, commission_id, origine)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
    )
    .bind(decision_id)
    .bind(etab)
    .bind(id)
    .bind(&b.sens)
    .bind(offre_decision)
    .bind(&reserve)
    .bind(&motif)
    .bind(&motif_communique)
    .bind(place.as_ref().is_some_and(|p| p.depassement))
    .bind(b.alternative)
    .bind(precedente.as_ref().map(|p| p.0))
    .bind(c.actor)
    .bind(c.commission_id)
    .bind(c.origine)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    // Tout ce que cette décision écrit au suivi (changement d'état, motif communiqué) lui est **rattaché** : la
    // famille ne le lit qu'une fois la décision publiée, ni avant ni après une décision ultérieure (règle 1).
    marquer_suivi_decision_tx(conn, id, Some(decision_id)).await?;
    sqlx::query(
        "UPDATE ent_inscription_dossier SET decision_motif = $3, decide_par = $4, decide_le = $5
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .bind(&motif)
    .bind(c.actor)
    .bind(c.maintenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;

    if cur.etat != cible {
        // Le suivi est lu par la famille : jamais le motif interne, seulement ce que la campagne communique.
        cur = transitionner_tx(
            conn,
            etab,
            &cur,
            cible,
            Cause::Decision,
            motif_communique.as_deref(),
            Some(c.actor),
        )
        .await?;
    }

    // Conditions et proposition : seulement pour une admission ; sinon une proposition ouverte tombe.
    if prend_une_place(&b.sens) {
        crate::inscriptions_attente::instancier_conditions_tx(conn, etab, &cur, reserve.as_deref())
            .await?;
        crate::inscriptions_attente::preparer_proposition_tx(
            conn,
            etab,
            &cur,
            offre_decision,
            c.origine,
            decision_id,
        )
        .await?;
    } else {
        crate::inscriptions_attente::annuler_propositions_ouvertes_tx(
            conn,
            etab,
            id,
            "décision revue",
        )
        .await?;
    }
    // Place déplacée par la décision (autre offre) : la liste d'attente de l'ancienne offre avance.
    if let (Some(avant), Some(apres)) = (offre_avant, offre_decision) {
        if avant != apres && matches!(e, "admis" | "confirme") {
            crate::inscriptions_attente::place_liberee_tx(conn, etab, avant).await?;
            crate::inscriptions_attente::designer_suivants_tx(conn, etab, avant, &[id]).await?;
        }
    }

    marquer_suivi_decision_tx(conn, id, None).await?;

    // Publication : tout de suite (défaut), ou à l'instant demandé (INS-62).
    match b.publier_le {
        Some(quand) if quand > c.maintenant && !c.publier_tout_de_suite => {
            borner_publication(quand, c.maintenant)?;
            if en_plage_calme(conn, etab, quand).await? && !b.confirmer_plage_calme {
                return Err(conflit(
                    "publication entre 20 h et 8 h ou un dimanche : à confirmer (heure de l'établissement)",
                ));
            }
            sqlx::query(
                "UPDATE ent_inscription_decision SET publication_prevue_le = $3
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(decision_id)
            .bind(quand)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
        None if !c.publier_par_defaut && !c.publier_tout_de_suite => {}
        _ => {
            publier_decision_tx(conn, etab, decision_id, Some(c.actor), c.maintenant).await?;
        }
    }
    Ok(decision_id)
}

/// Issue d'une tentative de publication (passe 3, correctif 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Publication {
    /// La décision est publiée (famille prévenue, événement émis).
    Publiee,
    /// L'état du dossier ne correspond plus à la décision : elle est **caduque**, tracée, jamais publiée ni notifiée.
    Caduque,
    /// Rien à publier (déjà publiée, remplacée ou déjà caduque).
    Aucune,
}

/// **Alerte la direction** (D-069, M6) : une décision programmée n'a pas pu être publiée, la famille n'en sait rien et ne
/// la saura jamais : une personne doit reprendre le dossier. Avis **interne** dans l'ENT (aucun envoi externe), un par
/// décision et par destinataire (idempotent) ; le titre ne porte **aucun nom d'élève**, le corps seulement la référence du
/// dossier. Destinataires : la direction de l'établissement et l'auteur de la décision, **tant que leur accès est ouvert** (F-e) ;
/// à défaut, repli sur les personnes qui détiennent le droit `InscriptionsGerer` (passe 7, F-2), puis trace interne du dossier (passe 6, F6).
async fn alerter_direction_caduque_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    decision_id: Uuid,
    dossier: Uuid,
) -> Result<(), EntDbError> {
    let reference: String = sqlx::query_scalar(
        "SELECT reference FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(dossier)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    // Destinataires : la direction et l'auteur de la décision, **tant que leur accès est ouvert** (F-e : définition unique
    // `ent_acces_ouvert`). Passe 6 (F6) : si aucun n'a un accès ouvert (direction partie, auteur parti), **repli prudent** sur
    // les accès ouverts de l'administrateur et du secrétariat (qui instruisent les dossiers) ; s'il n'y en a pas non plus,
    // l'alerte est **tracée** dans le suivi interne du dossier (jamais lue de la famille) pour que la reprise la retrouve.
    let mut destinataires: Vec<Uuid> = sqlx::query_scalar(
        "SELECT u.user_id
           FROM (SELECT m.user_id FROM ent_membership m
                  WHERE m.etablissement_id = $1 AND m.role = 'direction'
                 UNION
                 SELECT d.decide_par FROM ent_inscription_decision d
                  WHERE d.etablissement_id = $1 AND d.id = $2 AND d.decide_par IS NOT NULL) u
          WHERE ent_acces_ouvert(u.user_id, $1)",
    )
    .bind(etab)
    .bind(decision_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    if destinataires.is_empty() {
        destinataires = destinataires_de_repli_tx(conn, etab).await?;
    }
    if destinataires.is_empty() {
        suivi_tx(
            conn,
            etab,
            dossier,
            "decision",
            None,
            Some("Alerte de décision caduque sans destinataire : aucun accès ouvert (direction, auteur, administration)"),
            false,
            None,
        )
        .await?;
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO ent_notification
            (id, etablissement_id, destinataire_user_id, eleve_id, kind, titre, corps, source_event_id)
         SELECT gen_random_uuid(), $1, u, NULL, 'inscription_decision_caduque',
                'Décision d''inscription programmée non publiée',
                'Une décision programmée n''a pas été publiée : l''état du dossier ' || $3 || ' a changé depuis. \
                 La famille n''en a rien su ; le dossier est à reprendre (décisions caduques de la campagne).',
                $2
           FROM unnest($4::uuid[]) AS u
         ON CONFLICT (source_event_id, destinataire_user_id, kind) DO NOTHING",
    )
    .bind(etab)
    .bind(decision_id)
    .bind(&reference)
    .bind(&destinataires)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Repli de l'alerte de décision caduque (passe 7, F-2) : les personnes à accès ouvert qui **détiennent le droit**
/// `InscriptionsGerer` sur l'établissement entier (celui qui instruit les dossiers), et non un rôle de base nommé. Deux
/// sources, comme `droits_effectifs` : le **modèle par défaut du rôle** et les **attributions de rôle** en cours (jour de
/// l'établissement). Les délégations datées ne sont pas consultées (pas de destinataire désigné par délégation) : limite
/// documentée. Une porte `accorde` : une action qui en implique une autre (`accordee_par`) compte.
async fn destinataires_de_repli_tx(conn: &mut PgConnection, etab: Uuid) -> Result<Vec<Uuid>, EntDbError> {
    use crate::droits_ctx::accorde;
    use ent_auth::{modele_par_defaut, Portee};
    let suffit = |a: Action, p: Portee| p == Portee::Etablissement && accorde(a, Action::InscriptionsGerer);
    let membres: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT m.user_id, m.role FROM ent_membership m
          WHERE m.etablissement_id = $1 AND ent_acces_ouvert(m.user_id, $1)",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let attributions: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT a.user_id, d.action, d.portee
           FROM ent_role_attribution a
           JOIN ent_role_droit d ON d.role_id = a.role_id AND d.etablissement_id = a.etablissement_id
          WHERE a.etablissement_id = $1 AND a.clos_le IS NULL
            AND a.debut <= ent_aujourdhui_etab($1) AND (a.fin IS NULL OR a.fin >= ent_aujourdhui_etab($1))
            AND ent_acces_ouvert(a.user_id, $1)",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    // Contre-revue du pilote (passe 7) : garde-fou 1 de `droits_effectifs` : un compte dont le rôle de base n'est pas du
    // personnel (famille, élève) ne reçoit aucun droit d'attribution.
    let personnels: std::collections::HashSet<Uuid> = membres
        .iter()
        .filter(|(_, role)| Role::parse(role).is_some_and(|r| r.is_staff()))
        .map(|(u, _)| *u)
        .collect();
    let mut ids: Vec<Uuid> = membres
        .into_iter()
        .filter(|(_, role)| {
            Role::parse(role).is_some_and(|r| r.is_staff() && modele_par_defaut(r).into_iter().any(|(a, p)| suffit(a, p)))
        })
        .map(|(u, _)| u)
        .collect();
    ids.extend(attributions.into_iter().filter_map(|(u, a, p)| {
        let (a, p) = (Action::parse(&a)?, Portee::parse(&p)?);
        (personnels.contains(&u) && suffit(a, p)).then_some(u)
    }));
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Rend une décision visible de la famille : proposition envoyée, suivi visible, message **sans le contenu** de
/// la décision (la famille la lit dans son espace). Seule la décision la plus récente d'un dossier se publie ;
/// une décision remplacée avant publication ne sera jamais lue. Renvoie vrai si une publication a eu lieu.
pub(crate) async fn publier_decision_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    decision_id: Uuid,
    par: Option<Uuid>,
    maintenant: DateTime<Utc>,
) -> Result<Publication, EntDbError> {
    // Ordre des verrous unique (passe 4) : les offres du dossier, puis le dossier, avant la décision et la proposition.
    // Contre-revue des suites (M3) : l'offre portée par la décision elle-même est verrouillée aussi (la sentinelle de 316
    // la contrôle à la publication), sans supposer qu'elle coïncide avec le vœu retenu du dossier.
    let dossier_lu: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT dossier_id, offre_id FROM ent_inscription_decision WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(decision_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if let Some((dossier, offre)) = dossier_lu {
        let offres_en_plus: Vec<Uuid> = offre.into_iter().collect();
        crate::inscriptions_attente::verrouiller_dossiers_tx(conn, etab, None, &[dossier], &offres_en_plus).await?;
    }
    // Garde de l'état courant (passe 3, correctif 3) : une décision prise puis programmée correspondait à l'état du
    // dossier **à ce moment** ; si le dossier a bougé depuis (annulé, retiré, repris), elle est caduque. Le dossier
    // est verrouillé ci-dessus : l'état lu ne change plus jusqu'à la fin de la transaction.
    let a_publier: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT d.sens, dos.etat, dos.etat_avant_blocage
           FROM ent_inscription_decision d
           JOIN ent_inscription_dossier dos ON dos.etablissement_id = d.etablissement_id AND dos.id = d.dossier_id
          WHERE d.etablissement_id = $1 AND d.id = $2 AND d.publiee_le IS NULL AND d.caduque_le IS NULL
            AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision y
                             WHERE y.etablissement_id = d.etablissement_id AND y.remplace_decision_id = d.id)",
    )
    .bind(etab)
    .bind(decision_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((sens_lu, etat_lu, avant_blocage)) = a_publier else {
        return Ok(Publication::Aucune);
    };
    let etat_actuel = crate::inscriptions_etats::etat_effectif(&etat_lu, avant_blocage.as_deref());
    if !etats_compatibles(&sens_lu).contains(&etat_actuel) {
        let motif = format!(
            "Décision caduque : le dossier est à l'état « {etat_actuel} », la décision « {sens_lu} » supposait « {} »",
            etats_compatibles(&sens_lu).join(" ou ")
        );
        let dossier: Uuid = sqlx::query_scalar(
            "UPDATE ent_inscription_decision SET caduque_le = $3, caduque_motif = $4
              WHERE etablissement_id = $1 AND id = $2 RETURNING dossier_id",
        )
        .bind(etab)
        .bind(decision_id)
        .bind(maintenant)
        .bind(&motif)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        // Une place désignée pour cette décision ne part jamais : la proposition ouverte tombe, avec sa trace.
        crate::inscriptions_attente::annuler_propositions_ouvertes_tx(conn, etab, dossier, "décision caduque")
            .await?;
        // Interne seulement : la famille ne lit rien d'une décision qu'elle n'a jamais vue.
        suivi_tx(conn, etab, dossier, "decision", None, Some(&motif), false, par).await?;
        alerter_direction_caduque_tx(conn, etab, decision_id, dossier).await?;
        return Ok(Publication::Caduque);
    }
    let publiee: Option<(Uuid, String, Option<Uuid>, bool, bool, Option<Uuid>)> = sqlx::query_as(
        "UPDATE ent_inscription_decision d SET publiee_le = $3, publiee_par = $4
          WHERE d.etablissement_id = $1 AND d.id = $2 AND d.publiee_le IS NULL AND d.caduque_le IS NULL
            AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision y
                             WHERE y.etablissement_id = d.etablissement_id AND y.remplace_decision_id = d.id)
        RETURNING d.dossier_id, d.sens, d.offre_id, d.depassement_capacite, d.remplace_decision_id IS NOT NULL,
                  d.decide_par",
    )
    .bind(etab)
    .bind(decision_id)
    .bind(maintenant)
    .bind(par)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((dossier, sens, offre, depassement, revue, decide_par)) = publiee else {
        return Ok(Publication::Aucune);
    };
    if prend_une_place(&sens) {
        crate::inscriptions_attente::envoyer_proposition_publiee_tx(
            conn, etab, dossier, par, maintenant,
        )
        .await?;
    }
    suivi_tx(
        conn,
        etab,
        dossier,
        "decision",
        None,
        Some("Une décision a été prise sur votre dossier : elle est disponible dans votre espace."),
        true,
        par,
    )
    .await?;
    PgEnt::prevenir_familles_tx(
        conn,
        etab,
        &[dossier],
        "Décision concernant votre dossier d'inscription",
        "Une décision a été prise sur le dossier d'inscription de votre enfant. Vous pouvez la consulter dans votre espace.",
        &format!("decision-{decision_id}"),
    )
    .await?;
    // Le fait « décision prise » n'est émis qu'à la **publication** (F7) : un abonné ne le voit jamais avant la
    // famille. Le motif (texte libre) n'entre pas dans l'événement (§4.1-3).
    publier(
        conn,
        etab,
        types::ADMISSION_DECIDEE,
        AggregateKind::Admission,
        dossier,
        None,
        json!({
            "dossier_id": dossier,
            "decision_id": decision_id,
            "decision": sens,
            "offre_id": offre,
            "depassement_capacite": depassement,
            "revue": revue,
        }),
        decide_par.or(par),
    )
    .await?;
    publier(
        conn,
        etab,
        types::INSCRIPTION_DECISIONS_PUBLIEES,
        AggregateKind::Admission,
        dossier,
        None,
        json!({ "dossier_id": dossier, "decision_id": decision_id }),
        par,
    )
    .await?;
    Ok(Publication::Publiee)
}

// ───────────────────────────── Lecture ─────────────────────────────

type LigneDecision = (
    Uuid,
    Uuid,
    String,
    Option<Uuid>,
    Option<String>,
    String,
    Option<String>,
    bool,
    bool,
    Option<Uuid>,
    Option<Uuid>,
    String,
    Option<Uuid>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

/// Chaîne des décisions d'un dossier (`$1` établissement, `$2` dossier), de la première à la plus récente :
/// l'ordre est celui des **remplacements** (`remplace_decision_id`), jamais celui de l'heure (D-069 règle 4).
/// `rang_chaine` donne la place dans la chaîne.
pub(crate) const CHAINE: &str = "WITH RECURSIVE chaine AS (
    SELECT d.*, 1 AS rang_chaine FROM ent_inscription_decision d
     WHERE d.etablissement_id = $1 AND d.dossier_id = $2 AND d.remplace_decision_id IS NULL
    UNION ALL
    SELECT x.*, c.rang_chaine + 1 FROM ent_inscription_decision x
      JOIN chaine c ON x.remplace_decision_id = c.id
     WHERE x.etablissement_id = $1 AND x.dossier_id = $2)";

const COLS_DECISION: &str = "id, dossier_id, sens, offre_id, reserve, motif_interne, motif_communique,
    depassement_capacite, alternative, remplace_decision_id, commission_id, origine, decide_par, decide_le,
    publication_prevue_le, publiee_le";

fn decision_dto(r: LigneDecision) -> DecisionDto {
    DecisionDto {
        id: r.0,
        dossier_id: r.1,
        sens: r.2,
        offre_id: r.3,
        reserve: r.4,
        motif_interne: Some(r.5),
        motif_communique: r.6,
        depassement_capacite: r.7,
        alternative: r.8,
        remplace_decision_id: r.9,
        commission_id: r.10,
        origine: r.11,
        decide_par: r.12,
        decide_le: r.13,
        publication_prevue_le: r.14,
        publiee_le: r.15,
    }
}

async fn decision_par_id_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<DecisionDto, EntDbError> {
    sqlx::query_as::<_, LigneDecision>(&format!(
        "SELECT {COLS_DECISION} FROM ent_inscription_decision WHERE etablissement_id = $1 AND id = $2"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .map(decision_dto)
    .ok_or_else(introuvable)
}

/// Projet de courrier d'une décision. Texte pur : aucune donnée d'un autre responsable ni d'un autre candidat,
/// jamais le motif interne.
pub fn rediger_courrier(
    sens: &str,
    reference: &str,
    prenom: &str,
    nom: &str,
    campagne: &str,
    offre: Option<&str>,
    motif_communique: Option<&str>,
    reserve: Option<&str>,
    repondre_avant: Option<chrono::NaiveDate>,
) -> (String, String) {
    let objet = format!("Dossier d'inscription {reference} : décision");
    let eleve = format!("{prenom} {nom}");
    let offre = offre.map(|o| format!(" ({o})")).unwrap_or_default();
    let mut corps = format!("Madame, Monsieur,\n\nNous avons examiné le dossier d'inscription de {eleve} ({campagne}).\n\n");
    corps.push_str(&match sens {
        "admis" => format!("Nous avons le plaisir de vous informer que {eleve} est admis(e){offre}."),
        "admis_sous_reserve" => format!(
            "Nous avons le plaisir de vous informer que {eleve} est admis(e){offre}, sous la réserve suivante : {}.",
            reserve.unwrap_or("à préciser")
        ),
        "liste_attente" => format!(
            "À ce jour, nous ne pouvons pas confirmer l'inscription de {eleve}{offre} : le dossier est placé sur liste \
             d'attente. Vous serez prévenu(e) si une place se libère."
        ),
        "refuse" => format!("Nous avons le regret de vous informer que nous ne pouvons pas donner suite à la demande pour {eleve}."),
        _ => format!("La décision concernant {eleve} est ajournée : nous reviendrons vers vous."),
    });
    if let Some(m) = motif_communique {
        corps.push_str(&format!("\n\nMotif : {m}"));
    }
    if let (true, Some(j)) = (
        matches!(sens, "admis" | "admis_sous_reserve"),
        repondre_avant,
    ) {
        corps.push_str(&format!(
            "\n\nMerci de nous répondre, dans votre espace, avant le {}.",
            j.format("%d/%m/%Y")
        ));
    }
    corps.push_str("\n\nCordialement,\nLa direction");
    (objet, corps)
}

impl PgEnt {
    /// INS-04 ★ / INS-33 ★ : décider ou revoir. Droit `inscriptions.decider` ; contrôles dans [`decider_tx`].
    pub async fn decider_complete_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        body: &DecisionCompleteBody,
        revoir: bool,
        maintenant: DateTime<Utc>,
    ) -> Result<DecisionDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let decision = decider_tx(
            &mut tx,
            etab,
            id,
            &ContexteDecision {
                actor,
                body,
                revoir,
                origine: "direction",
                commission_id: None,
                maintenant,
                motif_obligatoire: true,
                publier_tout_de_suite: false,
                publier_par_defaut: true,
                communiquer_par_defaut: false,
            },
        )
        .await?;
        let dto = decision_par_id_tx(&mut tx, etab, decision).await?;
        commit(tx).await?;
        Ok(dto)
    }

    /// Historique des décisions d'un dossier (personnel : le motif interne y figure).
    pub async fn decisions_dossier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Vec<DecisionDto>, EntDbError> {
        let (_, decider, _) = self.ins_personnel(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        dossier_visible_tx(&mut tx, etab, actor, id).await?;
        // Historique dans l'ordre de la **chaîne** des remplacements (règle 4), jamais d'un tri sur l'heure.
        let rows = sqlx::query_as::<_, LigneDecision>(&format!(
            "{CHAINE} SELECT {COLS_DECISION} FROM chaine ORDER BY rang_chaine"
        ))
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        // Le motif interne est réservé à `inscriptions.decider` (M8).
        Ok(rows
            .into_iter()
            .map(decision_dto)
            .map(|mut d| {
                if !decider {
                    d.motif_interne = None;
                }
                d
            })
            .collect())
    }

    /// Publie (ou programme) une décision (INS-62). Seule la plus récente d'un dossier se publie.
    pub async fn publier_decision_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        decision_id: Uuid,
        b: &PublierBody,
        maintenant: DateTime<Utc>,
    ) -> Result<PublicationDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r =
            publier_ou_programmer_tx(&mut tx, etab, &[decision_id], actor, b, maintenant, false)
                .await?;
        commit(tx).await?;
        Ok(r)
    }

    /// Tâche de fond : publie les décisions dont l'heure programmée est venue. Aucune décision n'est prise ici :
    /// une personne l'a prise et a choisi l'heure. Renvoie le nombre de décisions publiées.
    pub async fn publier_decisions_echues_tache(
        &self,
        maintenant: DateTime<Utc>,
    ) -> Result<i64, EntDbError> {
        let etabs: Vec<(Uuid,)> = crate::tenant_tx::lire_par_etablissement(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("decisions_inscription_publication"),
            "SELECT DISTINCT etablissement_id FROM ent_inscription_decision
              WHERE publiee_le IS NULL AND caduque_le IS NULL AND publication_prevue_le IS NOT NULL",
        )
        .await
        .map_err(db)?;
        let mut total = 0;
        for (etab,) in etabs {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let ids: Vec<Uuid> = sqlx::query_scalar(
                "SELECT d.id FROM ent_inscription_decision d
                  WHERE d.etablissement_id = $1 AND d.publiee_le IS NULL AND d.caduque_le IS NULL
                    AND d.publication_prevue_le <= $2
                    AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision y
                                     WHERE y.etablissement_id = d.etablissement_id AND y.remplace_decision_id = d.id)
                  ORDER BY d.publication_prevue_le, d.id",
            )
            .bind(etab)
            .bind(maintenant)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            drop(tx);
            // Une transaction par décision : une décision en échec n'en retient aucune autre.
            for id in ids {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                match publier_decision_tx(&mut tx, etab, id, None, maintenant).await {
                    Ok(Publication::Publiee) => {
                        commit(tx).await?;
                        total += 1;
                    }
                    // Caduque : la trace s'écrit (et la décision ne reviendra plus dans la tâche).
                    Ok(Publication::Caduque) => commit(tx).await?,
                    Ok(Publication::Aucune) => {}
                    Err(e) => tracing::warn!("publication d'une décision d'inscription : {e}"),
                }
            }
        }
        Ok(total)
    }

    /// Projet de courrier d'une décision (jamais le motif interne).
    pub async fn courrier_decision_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        decision_id: Uuid,
    ) -> Result<CourrierDto, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: (Uuid, String, String, String, String, String, Option<String>, Option<String>, Option<String>, Option<chrono::NaiveDate>) =
            sqlx::query_as(
                "SELECT d.dossier_id, d.sens, dos.reference, dos.eleve_prenom, dos.eleve_nom, c.libelle,
                        o.libelle_public, d.motif_communique, d.reserve,
                        (SELECT p.repondre_avant FROM ent_inscription_proposition_place p
                          WHERE p.etablissement_id = d.etablissement_id AND p.decision_id = d.id)
                   FROM ent_inscription_decision d
                   JOIN ent_inscription_dossier dos ON dos.etablissement_id = d.etablissement_id AND dos.id = d.dossier_id
                   JOIN ent_inscription_campagne c ON c.etablissement_id = dos.etablissement_id AND c.id = dos.campagne_id
              LEFT JOIN ent_inscription_offre o ON o.etablissement_id = d.etablissement_id AND o.id = d.offre_id
                  WHERE d.etablissement_id = $1 AND d.id = $2",
            )
            .bind(etab)
            .bind(decision_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        dossier_visible_tx(&mut tx, etab, actor, r.0).await?;
        let (objet, corps) = rediger_courrier(
            &r.1,
            &r.2,
            &r.3,
            &r.4,
            &r.5,
            r.6.as_deref(),
            r.7.as_deref(),
            r.8.as_deref(),
            r.9,
        );
        Ok(CourrierDto {
            decision_id,
            objet,
            corps,
        })
    }

    // ───────────────────────── Avis et déport ─────────────────────────

    /// INS-31 : donner un avis interne (ajout seul). Droit d'instruire ; jamais pour un membre déporté.
    pub async fn donner_avis_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &AvisBody,
    ) -> Result<AvisDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer)
            .await?;
        if !CONTEXTES_AVIS.contains(&b.contexte.as_str()) {
            return Err(invalide("contexte : entretien, commission ou pedagogique"));
        }
        if !SENS_AVIS.contains(&b.sens.as_str()) {
            return Err(invalide(
                "sens de l'avis : favorable, reserve, defavorable ou sans_avis",
            ));
        }
        let texte = b.texte.trim();
        if longueur(texte) < 3 || longueur(texte) > 2000 {
            return Err(invalide("avis : de 3 à 2000 caractères"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, id).await?;
        if ecarte_tx(&mut tx, etab, id, actor).await? {
            return Err(introuvable());
        }
        if d.nature != "admission" {
            return Err(conflit("les avis concernent les admissions"));
        }
        let aid = Uuid::new_v4();
        let cree_le: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO ent_inscription_avis (id, etablissement_id, dossier_id, auteur_id, contexte, sens, texte)
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING cree_le",
        )
        .bind(aid)
        .bind(etab)
        .bind(id)
        .bind(actor)
        .bind(&b.contexte)
        .bind(&b.sens)
        .bind(texte)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        suivi_tx(
            &mut tx,
            etab,
            id,
            "avis",
            None,
            Some("Avis interne ajouté"),
            false,
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        Ok(AvisDto {
            id: aid,
            dossier_id: id,
            auteur_id: actor,
            contexte: b.contexte.clone(),
            sens: b.sens.clone(),
            texte: texte.to_string(),
            cree_le,
            auteur_deporte: false,
        })
    }

    /// Lecture des avis : la direction (`inscriptions.decider`) les lit tous ; un autre membre du personnel ne lit
    /// que les siens. Un membre déporté ne lit rien.
    pub async fn avis_dossier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Vec<AvisDto>, EntDbError> {
        let (_, decider, _) = self.ins_personnel(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        dossier_visible_tx(&mut tx, etab, actor, id).await?;
        let rows: Vec<(Uuid, Uuid, Uuid, String, String, String, DateTime<Utc>, bool)> = sqlx::query_as(
            "SELECT a.id, a.dossier_id, a.auteur_id, a.contexte, a.sens, a.texte, a.cree_le,
                    EXISTS (SELECT 1 FROM ent_inscription_deport x
                             WHERE x.etablissement_id = a.etablissement_id AND x.dossier_id = a.dossier_id
                               AND x.membre_id = a.auteur_id)
               FROM ent_inscription_avis a
              WHERE a.etablissement_id = $1 AND a.dossier_id = $2 AND ($3 OR a.auteur_id = $4)
              ORDER BY a.cree_le, a.id",
        )
        .bind(etab)
        .bind(id)
        .bind(decider)
        .bind(actor)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| AvisDto {
                id: r.0,
                dossier_id: r.1,
                auteur_id: r.2,
                contexte: r.3,
                sens: r.4,
                texte: r.5,
                cree_le: r.6,
                // Un déport déclaré depuis marque les avis déjà donnés (règle 2).
                auteur_deporte: r.7,
            })
            .collect())
    }

    /// INS-63 : état du déport du lecteur et conflit suggéré (même adresse qu'un responsable ou que le déposant).
    pub async fn deport_etat_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<DeportEtatDto, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let deporte = deporte_declare_tx(&mut tx, etab, id, actor).await?;
        if !deporte {
            // Introuvable si le dossier n'est pas de l'établissement, ou si le lecteur en est écarté (lien).
            dossier_visible_tx(&mut tx, etab, actor, id).await?;
        }
        // Un lien avec le candidat est un déport **d'office** : la garde ci-dessus répond « introuvable » ; il n'y a
        // donc pas de « conflit possible » à suggérer (D-069 F8 : le champ est supprimé).
        Ok(DeportEtatDto { deporte })
    }

    /// INS-63 : se déporter d'un dossier. Irréversible par soi-même (la direction pourrait rétablir par une
    /// intervention sur la base, tracée) : on ne voit plus le dossier ni ses avis.
    pub async fn se_deporter_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &DeportBody,
    ) -> Result<DeportEtatDto, EntDbError> {
        self.ins_personnel(etab, actor, role).await?;
        let motif = b.motif.trim();
        if longueur(motif) < 3 || longueur(motif) > 500 {
            return Err(invalide("motif du déport : de 3 à 500 caractères"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Idempotent (D-069 F7) : déjà déporté, la réponse est la même, sans nouvelle ligne, trace ni événement.
        if deporte_declare_tx(&mut tx, etab, id, actor).await? {
            return Ok(DeportEtatDto { deporte: true });
        }
        dossier_visible_tx(&mut tx, etab, actor, id).await?;
        sqlx::query(
            "INSERT INTO ent_inscription_deport (id, etablissement_id, dossier_id, membre_id, motif)
             VALUES ($1,$2,$3,$4,$5) ON CONFLICT (dossier_id, membre_id) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(actor)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        suivi_tx(
            &mut tx,
            etab,
            id,
            "deport",
            None,
            Some("Un membre du personnel s'est déporté"),
            false,
            Some(actor),
        )
        .await?;
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_DEPORT,
            AggregateKind::Admission,
            id,
            None,
            json!({ "dossier_id": id }),
            Some(actor),
        )
        .await?;
        commit(tx).await?;
        Ok(DeportEtatDto { deporte: true })
    }

    // ───────────────────────── Commissions ─────────────────────────

    /// INS-32 : créer une séance de commission (droit de décider).
    pub async fn creer_commission_admission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        b: &CommissionBody,
    ) -> Result<CommissionDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let intitule = b.intitule.trim();
        if longueur(intitule) < 3 || longueur(intitule) > 120 {
            return Err(invalide("intitulé : de 3 à 120 caractères"));
        }
        if b.membres.len() > 30 {
            return Err(invalide("trente membres au plus"));
        }
        if b.dossier_ids.len() > DOSSIERS_PAR_SEANCE_MAX {
            return Err(invalide("cinq cents dossiers au plus par séance"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        valider_membres_tx(&mut tx, etab, &b.membres).await?;
        let nature: Option<String> = sqlx::query_scalar(
            "SELECT nature FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(b.campagne_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if nature.ok_or_else(introuvable)? != "admission" {
            return Err(conflit("une commission examine des admissions"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_inscription_commission
                (id, etablissement_id, campagne_id, intitule, seance_le, membres, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(b.campagne_id)
        .bind(intitule)
        .bind(b.seance_le)
        .bind(&b.membres)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        ajouter_dossiers_tx(&mut tx, etab, actor, id, b.campagne_id, &b.dossier_ids).await?;
        let dto = commission_dto_tx(&mut tx, etab, actor, id).await?;
        commit(tx).await?;
        Ok(dto)
    }

    pub async fn commission_admission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<CommissionDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        commission_dto_tx(&mut tx, etab, actor, id).await
    }

    pub async fn list_commissions_admission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        campagne_id: Option<Uuid>,
    ) -> Result<Vec<CommissionDto>, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_inscription_commission
              WHERE etablissement_id = $1 AND ($2::uuid IS NULL OR campagne_id = $2)
              ORDER BY seance_le DESC, id LIMIT 200",
        )
        .bind(etab)
        .bind(campagne_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            out.push(commission_dto_tx(&mut tx, etab, actor, id).await?);
        }
        Ok(out)
    }

    pub async fn modifier_dossiers_commission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &CommissionDossiersBody,
    ) -> Result<CommissionDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if b.ajouter.len() > DOSSIERS_PAR_SEANCE_MAX || b.retirer.len() > DOSSIERS_PAR_SEANCE_MAX {
            return Err(invalide("cinq cents dossiers au plus par modification"));
        }
        let (campagne, statut) = commission_ouverte_tx(&mut tx, etab, id).await?;
        if statut == "cloturee" {
            return Err(conflit(
                "séance clôturée : la liste des dossiers ne change plus",
            ));
        }
        ajouter_dossiers_tx(&mut tx, etab, actor, id, campagne, &b.ajouter).await?;
        let total: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_inscription_commission_dossier
              WHERE etablissement_id = $1 AND commission_id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if total > i64::try_from(DOSSIERS_PAR_SEANCE_MAX).unwrap_or(500) {
            return Err(invalide("cinq cents dossiers au plus par séance"));
        }
        if !b.retirer.is_empty() {
            if !ecartes_parmi(&mut tx, etab, actor, &b.retirer)
                .await?
                .is_empty()
            {
                return Err(introuvable());
            }
            let deja: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_inscription_decision
                  WHERE etablissement_id = $1 AND commission_id = $2 AND dossier_id = ANY($3)",
            )
            .bind(etab)
            .bind(id)
            .bind(&b.retirer)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if deja > 0 {
                return Err(conflit(
                    "un dossier déjà décidé en séance ne se retire pas de la liste",
                ));
            }
            sqlx::query(
                "DELETE FROM ent_inscription_commission_dossier
                  WHERE etablissement_id = $1 AND commission_id = $2 AND dossier_id = ANY($3)",
            )
            .bind(etab)
            .bind(id)
            .bind(&b.retirer)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let dto = commission_dto_tx(&mut tx, etab, actor, id).await?;
        commit(tx).await?;
        Ok(dto)
    }

    /// Décisions en lot (INS-32) : chaque dossier est traité dans **sa propre transaction** ; un échec (capacité,
    /// désaccord, motif manquant…) n'annule pas les autres et figure au récapitulatif. Un dossier absent de la
    /// liste de la séance est refusé. Les décisions publiées programmées partagent l'heure choisie.
    pub async fn decider_en_lot_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        commission_id: Uuid,
        b: &DecisionsEnLotBody,
        maintenant: DateTime<Utc>,
    ) -> Result<RecapitulatifLotDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        if b.decisions.is_empty() || b.decisions.len() > 200 {
            return Err(invalide("de 1 à 200 décisions par lot"));
        }
        {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let (_, statut) = commission_ouverte_tx(&mut tx, etab, commission_id).await?;
            if statut == "cloturee" {
                return Err(conflit("séance clôturée"));
            }
            // D-069 E2 : en séance, seul un membre de la commission décide.
            let membres: Vec<Uuid> = sqlx::query_scalar(
                "SELECT membres FROM ent_inscription_commission WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(commission_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !membres.contains(&actor) {
                return Err(EntDbError::Forbidden);
            }
        }
        let mut recap = RecapitulatifLotDto::default();
        for item in &b.decisions {
            let r = self
                .une_decision_de_commission(etab, actor, commission_id, item, maintenant)
                .await;
            match r {
                Ok(decision_id) => {
                    recap.reussies += 1;
                    recap.resultats.push(DecisionEnLotResultat {
                        dossier_id: item.dossier_id,
                        ok: true,
                        decision_id: Some(decision_id),
                        erreur: None,
                    });
                }
                Err(e) => {
                    recap.refusees += 1;
                    recap.resultats.push(DecisionEnLotResultat {
                        dossier_id: item.dossier_id,
                        ok: false,
                        decision_id: None,
                        erreur: Some(message_lot(&e)),
                    });
                }
            }
        }
        Ok(recap)
    }

    async fn une_decision_de_commission(
        &self,
        etab: Uuid,
        actor: Uuid,
        commission_id: Uuid,
        item: &DecisionEnLotItem,
        maintenant: DateTime<Utc>,
    ) -> Result<Uuid, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let dans_la_liste: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_inscription_commission_dossier
                             WHERE etablissement_id = $1 AND commission_id = $2 AND dossier_id = $3)",
        )
        .bind(etab)
        .bind(commission_id)
        .bind(item.dossier_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !dans_la_liste {
            return Err(conflit("dossier absent de la liste de la séance"));
        }
        // Jamais de revue silencieuse dans un lot : un dossier déjà décidé se revoit par la route de revue.
        let revoir = false;
        let decision = decider_tx(
            &mut tx,
            etab,
            item.dossier_id,
            &ContexteDecision {
                actor,
                body: &item.decision,
                revoir,
                origine: "commission",
                commission_id: Some(commission_id),
                maintenant,
                motif_obligatoire: true,
                publier_tout_de_suite: false,
                publier_par_defaut: false,
                communiquer_par_defaut: false,
            },
        )
        .await?;
        sqlx::query(
            "UPDATE ent_inscription_commission_dossier SET examine_le = $4
              WHERE etablissement_id = $1 AND commission_id = $2 AND dossier_id = $3",
        )
        .bind(etab)
        .bind(commission_id)
        .bind(item.dossier_id)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        commit(tx).await?;
        Ok(decision)
    }

    /// Publie ensemble les décisions d'une séance (INS-62) : envois groupés, à l'instant choisi ou tout de suite.
    pub async fn publier_commission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &PublierBody,
        maintenant: DateTime<Utc>,
    ) -> Result<PublicationDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        commission_ouverte_tx(&mut tx, etab, id).await?;
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT d.id FROM ent_inscription_decision d
              WHERE d.etablissement_id = $1 AND d.commission_id = $2 AND d.publiee_le IS NULL
                AND d.caduque_le IS NULL
                AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision y
                                 WHERE y.etablissement_id = d.etablissement_id AND y.remplace_decision_id = d.id)
              ORDER BY d.id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let r = publier_ou_programmer_tx(&mut tx, etab, &ids, actor, b, maintenant, true).await?;
        commit(tx).await?;
        Ok(r)
    }

    /// Clôture la séance : procès-verbal en texte (le document signé relève du socle 7). Les dossiers de la liste
    /// non décidés restent à instruire ; ils ne sont **jamais** décidés par la clôture.
    pub async fn cloturer_commission_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        id: Uuid,
        b: &CommissionClotureBody,
        maintenant: DateTime<Utc>,
    ) -> Result<CommissionDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider)
            .await?;
        let pv = opt(&b.pv_texte);
        if pv.as_deref().is_some_and(|p| longueur(p) > 20000) {
            return Err(invalide("procès-verbal : 20 000 caractères au plus"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (_, statut) = commission_ouverte_tx(&mut tx, etab, id).await?;
        if statut == "cloturee" {
            return Err(conflit("séance déjà clôturée"));
        }
        sqlx::query(
            "UPDATE ent_inscription_commission SET statut = 'cloturee', pv_texte = $3, cloturee_le = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&pv)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_COMMISSION_TENUE,
            AggregateKind::Admission,
            id,
            None,
            json!({ "commission_id": id }),
            Some(actor),
        )
        .await?;
        let dto = commission_dto_tx(&mut tx, etab, actor, id).await?;
        commit(tx).await?;
        Ok(dto)
    }

    // ───────────────────────── Famille ─────────────────────────

    /// Ce que la famille lit de la décision **publiée** (déposant vérifié seulement) : jamais le motif interne,
    /// jamais un avis, jamais un autre candidat, jamais un autre responsable.
    pub async fn decision_famille(
        &self,
        etab: Uuid,
        jeton: &str,
        id: Uuid,
    ) -> Result<DecisionFamilleDto, EntDbError> {
        let (_, d) = self.dossier_du_deposant(etab, jeton, id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Option<(String, Option<String>, Option<String>, Option<String>, DateTime<Utc>)> = sqlx::query_as(
            &format!(
            "{CHAINE}
             SELECT d.sens, o.libelle_public, d.motif_communique, d.reserve, d.publiee_le
               FROM chaine d
          LEFT JOIN ent_inscription_offre o ON o.etablissement_id = d.etablissement_id AND o.id = d.offre_id
              WHERE d.publiee_le IS NOT NULL
              ORDER BY d.rang_chaine DESC LIMIT 1"),
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((sens, offre, motif_communique, reserve, publiee_le)) = r else {
            return Ok(DecisionFamilleDto::default());
        };
        // Une décision plus récente mais non publiée ne se lit pas : ni position, ni conditions, ni proposition.
        let masque = visibilite_famille_tx(&mut tx, etab, id).await?.is_some();
        let (position, proposition, conditions, actions) = if masque {
            (None, None, Vec::new(), Vec::new())
        } else {
            let position =
                crate::inscriptions_attente::rang_lu_par_la_famille_tx(&mut tx, etab, id, &d.etat).await?;
            let proposition =
                crate::inscriptions_attente::proposition_famille_tx(&mut tx, etab, id).await?;
            let conditions =
                crate::inscriptions_attente::conditions_famille_tx(&mut tx, etab, &d).await?;
            let mut actions = Vec::new();
            if proposition.as_ref().is_some_and(|p| p.statut == "envoyee") {
                actions.push("repondre".to_string());
            }
            (position, proposition, conditions, actions)
        };
        Ok(DecisionFamilleDto {
            publiee: true,
            sens: Some(sens),
            offre,
            motif_communique,
            reserve,
            publiee_le: Some(publiee_le),
            position_attente: position,
            proposition,
            conditions,
            actions,
        })
    }
}

/// **État visible** (D-069 règle 1) : la **seule** fonction qui dit à la famille où en est son dossier, pour
/// l'affichage **et** pour autoriser ses actions (`exiger_dossier_stable_famille_tx`). Décision **prise mais non
/// publiée** : si la dernière décision de la chaîne n'est pas publiée, renvoie l'état que la famille peut encore
/// lire (celui de la dernière décision **publiée** de la chaîne, sinon l'instruction). Tout se lit dans la chaîne
/// `remplace_decision_id` (D-069 F4) : jamais un tri ni un minimum sur des dates. Les lignes du suivi, elles, sont
/// rattachées à la décision en cours à leur écriture (`suivi_tx`) et ne se lisent qu'avec elle.
pub(crate) async fn visibilite_famille_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
) -> Result<Option<String>, EntDbError> {
    let derniere_publiee: Option<bool> = sqlx::query_scalar(&format!(
        "{CHAINE} SELECT (publiee_le IS NOT NULL OR caduque_le IS NOT NULL) FROM chaine
          ORDER BY rang_chaine DESC LIMIT 1"
    ))
    .bind(etab)
    .bind(dossier)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if derniere_publiee != Some(false) {
        return Ok(None);
    }
    let publie: Option<String> = sqlx::query_scalar(&format!(
        "{CHAINE} SELECT sens FROM chaine WHERE publiee_le IS NOT NULL ORDER BY rang_chaine DESC LIMIT 1"
    ))
    .bind(etab)
    .bind(dossier)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let visible = publie.as_deref().map_or("en_instruction", etat_cible);
    Ok(Some(visible.to_string()))
}

/// Garde des actions de la famille : tant qu'une décision plus récente que la dernière publiée existe, **aucune**
/// action n'est possible et le refus est celui d'une modification concurrente (message neutre).
pub(crate) async fn exiger_dossier_stable_famille_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier: Uuid,
) -> Result<(), EntDbError> {
    if visibilite_famille_tx(conn, etab, dossier).await?.is_some() {
        return Err(EntDbError::ConflitDossier(
            crate::inscriptions::EN_COURS_DE_MODIFICATION,
        ));
    }
    Ok(())
}

impl PgEnt {
    /// Un membre du personnel déporté d'un dossier n'y fait plus rien : « introuvable » (INS-63).
    pub(crate) async fn refuser_si_ecarte(
        &self,
        etab: Uuid,
        actor: Uuid,
        dossier: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if ecarte_tx(&mut tx, etab, dossier, actor).await? {
            return Err(introuvable());
        }
        Ok(())
    }

    /// Applique la visibilité famille à des résumés de dossiers (espace et démarche de la famille).
    pub(crate) async fn masquer_resumes_famille(
        &self,
        etab: Uuid,
        liste: &mut [ent_types::inscriptions::DossierResumeDto],
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        for r in liste.iter_mut() {
            if let Some(visible) = visibilite_famille_tx(&mut tx, etab, r.id).await? {
                r.statut = visible.clone();
                r.etat = visible;
            }
        }
        Ok(())
    }
}

/// Dossier lu par le personnel : introuvable s'il est d'un autre établissement ou si le lecteur s'en est déporté.
pub(crate) async fn dossier_visible_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    id: Uuid,
) -> Result<DossierRow, EntDbError> {
    let d = sqlx::query_as::<_, DossierRow>(&format!(
        "SELECT {} FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = $2",
        crate::inscriptions::COLS_DOSSIER
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    if ecarte_tx(conn, etab, id, actor).await? {
        return Err(introuvable());
    }
    Ok(d)
}

async fn publier_ou_programmer_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    ids: &[Uuid],
    actor: Uuid,
    b: &PublierBody,
    maintenant: DateTime<Utc>,
    ignorer_ecartes: bool,
) -> Result<PublicationDto, EntDbError> {
    let mut r = PublicationDto::default();
    let programme = b.publier_le.filter(|q| *q > maintenant);
    if let Some(quand) = programme {
        borner_publication(quand, maintenant)?;
        if en_plage_calme(conn, etab, quand).await? && !b.confirmer_plage_calme {
            return Err(conflit(
                "publication entre 20 h et 8 h ou un dimanche : à confirmer (heure de l'établissement)",
            ));
        }
    }
    // Passe 4 (M1b) : on **résout d'abord** les dossiers de toutes les décisions, on écarte ceux dont l'acteur est écarté
    // (contrôle d'écart **avant** tout verrou de lot), puis on verrouille **une seule fois**, offres puis dossiers triés par
    // UUID. Ensuite seulement on publie : aucune publication ne prend un verrou de plus.
    let mut dossier_de_decision: Vec<(Uuid, Uuid)> = Vec::with_capacity(ids.len());
    // Contre-revue des suites (M3) : l'offre propre de chaque décision publiée est verrouillée avec celles des dossiers.
    let mut offre_de_decision: Vec<(Uuid, Uuid)> = Vec::new();
    for id in ids {
        let (dossier, offre): (Uuid, Option<Uuid>) = sqlx::query_as(
            "SELECT dossier_id, offre_id FROM ent_inscription_decision WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        dossier_de_decision.push((*id, dossier));
        if let Some(o) = offre {
            offre_de_decision.push((dossier, o));
        }
    }
    let tous: Vec<Uuid> = dossier_de_decision.iter().map(|(_, d)| *d).collect();
    let ecartes = ecartes_parmi(conn, etab, actor, &tous).await?;
    if !ignorer_ecartes && !ecartes.is_empty() {
        return Err(introuvable());
    }
    let a_verrouiller: Vec<Uuid> = tous.iter().copied().filter(|d| !ecartes.contains(d)).collect();
    let offres_en_plus: Vec<Uuid> = offre_de_decision
        .iter()
        .filter(|(d, _)| !ecartes.contains(d))
        .map(|(_, o)| *o)
        .collect();
    crate::inscriptions_attente::verrouiller_dossiers_tx(conn, etab, None, &a_verrouiller, &offres_en_plus).await?;
    for (id, dossier_de) in &dossier_de_decision {
        let etat: Option<(Option<DateTime<Utc>>, bool, bool)> = sqlx::query_as(
            "SELECT d.publiee_le, d.caduque_le IS NOT NULL, EXISTS (SELECT 1 FROM ent_inscription_decision y
                                           WHERE y.etablissement_id = d.etablissement_id AND y.remplace_decision_id = d.id)
               FROM ent_inscription_decision d WHERE d.etablissement_id = $1 AND d.id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let (deja, caduque, remplacee) = etat.ok_or_else(introuvable)?;
        if ecartes.contains(dossier_de) {
            // Publication d'une séance (D-069 F10) : une décision dont l'acteur est écarté ne se publie pas à sa place, mais
            // n'empêche pas les autres ; le récapitulatif la signale par un nombre.
            r.ignorees += 1;
            continue;
        }
        if deja.is_some() {
            return Err(conflit("décision déjà publiée"));
        }
        if caduque {
            return Err(conflit("décision caduque : l'état du dossier a changé, elle ne se publie plus"));
        }
        if remplacee {
            return Err(conflit("décision remplacée : elle ne se publie plus"));
        }
        if let Some(quand) = programme {
            sqlx::query(
                "UPDATE ent_inscription_decision SET publication_prevue_le = $3
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .bind(quand)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            r.programmees += 1;
        } else {
            match publier_decision_tx(conn, etab, *id, Some(actor), maintenant).await? {
                Publication::Publiee => {
                    r.publiees += 1;
                    r.familles_prevenues += 1;
                }
                Publication::Caduque => r.caduques += 1,
                Publication::Aucune => {}
            }
        }
    }
    Ok(r)
}

async fn commission_ouverte_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<(Uuid, String), EntDbError> {
    sqlx::query_as(
        // FOR NO KEY UPDATE (contre-revue de la passe 4, M-1) : compatible avec le KEY SHARE de la clé étrangère
        // `commission_id` d'une décision de la même séance ; FOR UPDATE formait un cycle avec une décision qui tient
        // l'offre et le dossier (publication de séance ↔ décision de séance).
        "SELECT campagne_id, statut FROM ent_inscription_commission
          WHERE etablissement_id = $1 AND id = $2 FOR NO KEY UPDATE",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

async fn ajouter_dossiers_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    commission: Uuid,
    campagne: Uuid,
    dossiers: &[Uuid],
) -> Result<(), EntDbError> {
    if dossiers.is_empty() {
        return Ok(());
    }
    let ok: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ent_inscription_dossier
          WHERE etablissement_id = $1 AND campagne_id = $2 AND id = ANY($3) AND nature = 'admission'",
    )
    .bind(etab)
    .bind(campagne)
    .bind(dossiers)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let distincts: std::collections::HashSet<&Uuid> = dossiers.iter().collect();
    if ok != i64::try_from(distincts.len()).unwrap_or(-1) {
        // Dossier inconnu, d'une autre campagne ou d'un autre établissement : même réponse.
        return Err(introuvable());
    }
    // D-069 F9 : un dossier dont l'acteur est écarté (déport ou lien) n'existe pas pour lui : même réponse.
    if !ecartes_parmi(conn, etab, actor, dossiers).await?.is_empty() {
        return Err(introuvable());
    }
    sqlx::query(
        "INSERT INTO ent_inscription_commission_dossier (etablissement_id, commission_id, dossier_id, ordre)
         SELECT $1, $2, x.id, x.ord::int FROM unnest($3::uuid[]) WITH ORDINALITY AS x(id, ord)
         ON CONFLICT DO NOTHING",
    )
    .bind(etab)
    .bind(commission)
    .bind(dossiers)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn commission_dto_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    id: Uuid,
) -> Result<CommissionDto, EntDbError> {
    let c: (Uuid, String, DateTime<Utc>, String, Vec<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT campagne_id, intitule, seance_le, statut, membres, pv_texte FROM ent_inscription_commission
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let lignes: Vec<(Uuid, String, String, String, Option<String>, String, Option<String>, Option<DateTime<Utc>>, i64)> =
        sqlx::query_as(
            "SELECT d.id, d.reference, d.eleve_nom, d.eleve_prenom, d.niveau_demande, d.etat,
                    (SELECT x.sens FROM ent_inscription_decision x
                      WHERE x.etablissement_id = d.etablissement_id AND x.dossier_id = d.id
                      AND NOT EXISTS (SELECT 1 FROM ent_inscription_decision y WHERE y.remplace_decision_id = x.id)
                      LIMIT 1),
                    cd.examine_le,
                    (SELECT count(*) FROM ent_inscription_avis a
                      WHERE a.etablissement_id = d.etablissement_id AND a.dossier_id = d.id)
               FROM ent_inscription_commission_dossier cd
               JOIN ent_inscription_dossier d ON d.etablissement_id = cd.etablissement_id AND d.id = cd.dossier_id
              WHERE cd.etablissement_id = $1 AND cd.commission_id = $2
              ORDER BY cd.ordre, d.eleve_nom, d.eleve_prenom, d.id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    let ids: Vec<Uuid> = lignes.iter().map(|l| l.0).collect();
    let deportes = ecartes_parmi(conn, etab, actor, &ids).await?;
    let places = places_campagne_tx(conn, etab, c.0).await?;
    Ok(CommissionDto {
        id,
        campagne_id: c.0,
        intitule: c.1,
        seance_le: c.2,
        statut: c.3,
        membres: c.4,
        dossiers: lignes
            .into_iter()
            // Un membre déporté ne voit plus le dossier, même dans la liste de la séance.
            .filter(|l| !deportes.contains(&l.0))
            .map(|l| CommissionDossierDto {
                dossier_id: l.0,
                reference: l.1,
                eleve_nom: l.2,
                eleve_prenom: l.3,
                niveau_demande: l.4,
                etat: l.5,
                derniere_decision: l.6,
                examine_le: l.7,
                nb_avis: l.8,
            })
            .collect(),
        places,
        pv_texte: c.5,
    })
}

/// Places restantes **en direct** par offre de la campagne (jamais figées à la création de la séance).
pub(crate) async fn places_campagne_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    campagne: Uuid,
) -> Result<Vec<PlacesOffreDto>, EntDbError> {
    let rows: Vec<(Uuid, String, Option<i32>, Option<i64>, i64)> = sqlx::query_as(
        "SELECT o.id, o.libelle_public, v.capacite,
                (v.capacite - v.reserve_reinscription - v.proposees - v.confirmees - v.validees)::bigint,
                v.liste_attente
           FROM ent_inscription_offre o
           JOIN ent_inscription_offre_occupation v ON v.etablissement_id = o.etablissement_id AND v.offre_id = o.id
          WHERE o.etablissement_id = $1 AND o.campagne_id = $2 AND o.archivee_le IS NULL
          ORDER BY o.niveau_code, o.parcours_code, o.id",
    )
    .bind(etab)
    .bind(campagne)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| PlacesOffreDto {
            offre_id: r.0,
            libelle: r.1,
            capacite: r.2.map(i64::from),
            disponibles: r.3,
            liste_attente: r.4,
        })
        .collect())
}

/// Message du récapitulatif de lot (F6) : ce que la personne peut corriger, jamais une erreur technique (SQL,
/// base, sérialisation) ni un détail interne.
fn message_lot(e: &EntDbError) -> String {
    match e {
        EntDbError::Domain(DomainError::NotFound) => "dossier introuvable".to_string(),
        EntDbError::SaisieInvalide(m) => (*m).to_string(),
        EntDbError::ConflitDossier(m) => (*m).to_string(),
        EntDbError::ConflitDossierDetail(m) => m.clone(),
        EntDbError::Forbidden => "action non autorisée".to_string(),
        _ => "décision non enregistrée : réessayez ou contactez l'administrateur".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etat_cible_de_chaque_sens() {
        assert_eq!(etat_cible("admis"), "admis");
        assert_eq!(etat_cible("admis_sous_reserve"), "admis");
        assert_eq!(etat_cible("liste_attente"), "liste_attente");
        assert_eq!(etat_cible("refuse"), "refuse");
        assert_eq!(etat_cible("ajourne"), "en_instruction");
    }

    #[test]
    fn courrier_sans_motif_interne_ni_autre_personne() {
        let j = chrono::NaiveDate::from_ymd_opt(2027, 6, 30);
        for sens in SENS_DECISION {
            let (objet, corps) = rediger_courrier(
                sens,
                "ADM-2027-0001",
                "Luna",
                "Lovegood",
                "Rentrée 2027",
                Some("6e"),
                None,
                Some("bulletin"),
                j,
            );
            assert!(objet.contains("ADM-2027-0001"));
            assert!(corps.contains("Luna Lovegood"));
            assert!(!corps.to_lowercase().contains("interne"));
        }
        let (_, admis) = rediger_courrier("admis", "R", "A", "B", "C", None, None, None, j);
        assert!(admis.contains("30/06/2027"));
        let (_, refus) = rediger_courrier(
            "refuse",
            "R",
            "A",
            "B",
            "C",
            None,
            Some("capacité"),
            None,
            j,
        );
        assert!(refus.contains("Motif : capacité") && !refus.contains("30/06/2027"));
    }
}
