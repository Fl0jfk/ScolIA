//! Socle 8, lot N1 — service de notification : créer, annuler et lire un avis (`08_NOTIFICATIONS.md` §6.3).
//!
//! Le service travaille **dans la transaction de l'appelant** : le fait et l'avis sont écrits ensemble ou pas du
//! tout, et aucune remise ne part avant le commit (c'est l'expédition qui remet, plus tard). Il est
//! idempotent (clé d'idempotence par nature) et rejouable. L'établissement est toujours celui de l'appelant ;
//! une nature d'une autre brique est refusée à l'émission explicite (`aviser` : `source_brique` = brique de la
//! nature).

use super::catalogue::NATURES;
use super::contrat::*;
use super::destinataires::{personnes_de_l_eleve, planifier_canaux, sujet_gele, Destinations};
use super::reglages::{local_de, lire_parametre, lire_pref_categorie, lire_reglage, vers_utc};
use super::temps::{decider, ContexteTemps, Moment};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RefusAvis {
    #[error("nature inconnue : {0}")]
    NatureInconnue(String),
    #[error("nature désactivée pour l'établissement")]
    NatureDesactivee,
    #[error("sujet élève manquant")]
    SujetManquant,
    #[error("sujet gelé (décès de l'élève) : aucun avis automatique")]
    SujetGele,
    #[error("une brique n'émet que ses natures")]
    BriqueEtrangere,
    #[error("paramètre « {0} » refusé (valeur trop longue)")]
    ParametreTropLong(&'static str),
    #[error("texte libre refusé pour cette nature")]
    TexteLibreRefuse,
    #[error("base de données : {0}")]
    Base(#[from] sqlx::Error),
}

/// Demande de création d'un avis.
pub struct DemandeAvis {
    pub etablissement_id: Uuid,
    pub nature: &'static NatureAvisDecl,
    /// Élève sujet (natures `TypeSujet::Eleve`).
    pub eleve_id: Option<Uuid>,
    pub cle: String,
    pub parametres: Vec<Param>,
    /// Brique qui émet (un émetteur explicite ne peut utiliser que ses natures) ; `None` = événement.
    pub emetteur_brique: Option<&'static str>,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub cause_event_id: Option<Uuid>,
    /// Instant du fait ; rappel programmé = instant futur.
    pub effet_le: DateTime<Utc>,
    /// Monter d'un cran dans les bornes de la nature, jamais descendre.
    pub urgence_plus: Option<Urgence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvisCree {
    pub avis_id: Uuid,
    /// Faux si la clé d'idempotence avait déjà produit cet avis (rejeu).
    pub cree: bool,
    pub retenus: Vec<Uuid>,
    pub exclus: Vec<(Option<Uuid>, String)>,
}

fn param_colonnes(v: &ValeurParam) -> (Option<&str>, Option<Uuid>, Option<chrono::NaiveDate>) {
    match v {
        ValeurParam::Texte(t) => (Some(t.as_str()), None, None),
        ValeurParam::Uuid(u) => (None, Some(*u), None),
        ValeurParam::Jour(j) => (None, None, Some(*j)),
    }
}

/// Réglage de l'établissement pour la nature (`None` = jamais réglée : défauts).
#[derive(Debug, Clone, Copy, Default)]
pub struct ReglageNature {
    pub desactivee: bool,
    pub via_brique: bool,
}

pub async fn reglage_nature(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    nature: &str,
) -> Result<ReglageNature, sqlx::Error> {
    let r: Option<(bool, bool)> = sqlx::query_as(
        "SELECT desactivee, via_brique FROM ent_ntf_nature_etab WHERE etablissement_id = $1 AND nature = $2",
    )
    .bind(etablissement_id)
    .bind(nature)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(r.map(|(desactivee, via_brique)| ReglageNature { desactivee, via_brique }).unwrap_or_default())
}

/// Délai de confirmation effectif : réglage de l'établissement pour l'absence, sinon celui de la nature.
async fn delai_confirmation(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    nature: &NatureAvisDecl,
) -> Result<Duration, sqlx::Error> {
    if nature.code == super::catalogue::ABSENCE_CONSTATEE.code {
        let r: Option<(i16,)> = sqlx::query_as(
            "SELECT delai_confirmation_absence_min FROM ent_ntf_parametre WHERE etablissement_id = $1",
        )
        .bind(etablissement_id)
        .fetch_optional(&mut *conn)
        .await?;
        if let Some((m,)) = r {
            return Ok(Duration::minutes(i64::from(m)));
        }
    }
    Ok(Duration::minutes(nature.delai_defaut_min()))
}

/// Crée l'avis, ses paramètres, ses destinataires (retenus ou exclus) et ses remises planifiées.
/// Idempotent : la même clé pour la même nature ne crée rien de plus.
pub async fn aviser(conn: &mut PgConnection, d: DemandeAvis) -> Result<AvisCree, RefusAvis> {
    let n = d.nature;
    if !NATURES.iter().any(|x| x.code == n.code) {
        return Err(RefusAvis::NatureInconnue(n.code.to_string()));
    }
    if let Some(b) = d.emetteur_brique {
        if b != n.brique {
            return Err(RefusAvis::BriqueEtrangere);
        }
    }
    for (cle, v) in &d.parametres {
        if let ValeurParam::Texte(t) = v {
            if t.chars().count() > PARAM_TEXTE_MAX {
                return Err(RefusAvis::ParametreTropLong(cle));
            }
        }
    }
    let reglage = reglage_nature(conn, d.etablissement_id, n.code).await?;
    if reglage.desactivee {
        return Err(RefusAvis::NatureDesactivee);
    }
    if n.sujet == TypeSujet::Eleve && d.eleve_id.is_none() {
        return Err(RefusAvis::SujetManquant);
    }
    if let (TypeSujet::Eleve, Some(eleve)) = (n.sujet, d.eleve_id) {
        if sujet_gele(conn, d.etablissement_id, "eleve", eleve).await? {
            return Err(RefusAvis::SujetGele);
        }
    }
    let urgence = match d.urgence_plus {
        Some(plus) => n.urgence.plus(plus, if n.brique_u3_autorisee { Urgence::U3 } else { Urgence::U2 }),
        None => n.urgence,
    };
    let delai = delai_confirmation(conn, d.etablissement_id, n).await?;
    let confirme_apres = d.effet_le + delai;
    let avis_id = Uuid::new_v4();
    let insere: Option<(Uuid,)> = sqlx::query_as(
        "INSERT INTO ent_ntf_avis
            (id, etablissement_id, nature, nature_version, categorie, urgence, sensibilite, sujet_type,
             sujet_id, eleve_id, cle_idempotence, source_brique, source_type, source_id, cause_event_id,
             statut, effet_le, confirme_apres, conservation_preuve)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,'planifie',$16,$17,$18)
         ON CONFLICT (etablissement_id, cle_idempotence, nature) DO NOTHING
         RETURNING id",
    )
    .bind(avis_id)
    .bind(d.etablissement_id)
    .bind(n.code)
    .bind(n.version as i16)
    .bind(n.categorie.code())
    .bind(urgence.code())
    .bind(n.sensibilite.code())
    .bind(n.sujet.code())
    .bind(d.eleve_id)
    .bind(d.eleve_id)
    .bind(&d.cle)
    .bind(n.brique)
    .bind(&d.source_type)
    .bind(d.source_id)
    .bind(d.cause_event_id)
    .bind(d.effet_le)
    .bind(confirme_apres)
    .bind(n.conservation_preuve)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((avis_id,)) = insere else {
        let (existant,): (Uuid,) = sqlx::query_as(
            "SELECT id FROM ent_ntf_avis WHERE etablissement_id = $1 AND cle_idempotence = $2 AND nature = $3",
        )
        .bind(d.etablissement_id)
        .bind(&d.cle)
        .bind(n.code)
        .fetch_one(&mut *conn)
        .await?;
        return Ok(AvisCree { avis_id: existant, cree: false, retenus: Vec::new(), exclus: Vec::new() });
    };
    for (cle, v) in &d.parametres {
        let (t, u, j) = param_colonnes(v);
        sqlx::query(
            "INSERT INTO ent_ntf_avis_param (etablissement_id, avis_id, cle, valeur_texte, valeur_uuid, valeur_jour)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (avis_id, cle) DO NOTHING",
        )
        .bind(d.etablissement_id)
        .bind(avis_id)
        .bind(*cle)
        .bind(t)
        .bind(u)
        .bind(j)
        .execute(&mut *conn)
        .await?;
    }
    let personnes = match (n.sujet, d.eleve_id) {
        (TypeSujet::Eleve, Some(eleve)) => {
            personnes_de_l_eleve(conn, d.etablissement_id, eleve, n.droit_requis, d.effet_le).await?
        }
        _ => Vec::new(),
    };
    let sms_u2_actif: bool = sqlx::query_scalar("SELECT sms_u2_actif FROM ent_ntf_parametre WHERE etablissement_id = $1")
        .bind(d.etablissement_id)
        .fetch_optional(&mut *conn)
        .await?
        .unwrap_or(false);
    let parametre = lire_parametre(conn, d.etablissement_id).await?;
    let local_confirme = local_de(conn, &parametre.fuseau, confirme_apres).await?;
    let mut retenus = Vec::new();
    let mut exclus = Vec::new();
    let mut destinations = Destinations::default();
    let raison = |compte: bool| if compte { "responsable_lien_actif" } else { "responsable_sans_compte" };
    for p in personnes {
        let dest_id = Uuid::new_v4();
        let l = &p.ligne;
        // Un destinataire par personne, retenu ou exclu avec sa raison (relue à chaque remise, R-N1).
        let inseree = sqlx::query(
            "INSERT INTO ent_ntf_destinataire
                (id, etablissement_id, avis_id, compte_id, personne_id, foyer_id, raison_code, retenu, exclusion_code)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT DO NOTHING",
        )
        .bind(dest_id)
        .bind(d.etablissement_id)
        .bind(avis_id)
        .bind(l.compte_id)
        .bind(l.personne_id)
        .bind(l.foyer_id)
        .bind(raison(l.compte_id.is_some()))
        .bind(p.exclusion.is_none())
        .bind(p.exclusion)
        .execute(&mut *conn)
        .await?;
        if inseree.rows_affected() == 0 {
            continue;
        }
        if let Some(code) = p.exclusion {
            exclus.push((Some(l.personne_id), code.to_string()));
            continue;
        }
        // N3 : préférences de la personne pour la catégorie, puis le moment de chaque canal (heures calmes à
        // trois étages, urgence, résumé, budget de bruit, pause), calculé à l'heure de l'établissement.
        let reglage = lire_reglage(conn, d.etablissement_id, l.personne_id).await?;
        let pref = lire_pref_categorie(conn, d.etablissement_id, l.personne_id, n.categorie).await?;
        let plan = planifier_canaux(n, l, sms_u2_actif, Some(pref));
        let u0_du_jour: i64 = sqlx::query_scalar(
            "SELECT count(DISTINCT a.id)
               FROM ent_ntf_remise r
               JOIN ent_ntf_destinataire x ON x.etablissement_id = r.etablissement_id AND x.id = r.destinataire_id
               JOIN ent_ntf_avis a ON a.etablissement_id = x.etablissement_id AND a.id = x.avis_id
              WHERE r.etablissement_id = $1 AND x.personne_id = $2 AND a.urgence = 'u0' AND r.canal <> 'inapp'
                AND r.statut <> 'annulee' AND a.id <> $3
                AND ent_aujourdhui_etab(r.etablissement_id, r.planifiee_pour) = $4",
        )
        .bind(d.etablissement_id)
        .bind(l.personne_id)
        .bind(avis_id)
        .bind(local_confirme.date())
        .fetch_one(&mut *conn)
        .await?;
        let ctx = ContexteTemps {
            local: local_confirme,
            etab: parametre.calme,
            personne: reglage.calme,
            indispo_jusqua: None,
            pause_jusqua: match reglage.pause_jusqua {
                Some(p) => Some(local_de(conn, &parametre.fuseau, p).await?),
                None => None,
            },
            mode_resume: if pref.resume && reglage.mode_resume == super::temps::ModeResume::Immediat {
                super::temps::ModeResume::Quotidien
            } else {
                reglage.mode_resume
            },
            heure_resume: reglage.heure_resume,
            vacances_hebdo: parametre.vacances_hebdo,
            u0_du_jour: u0_du_jour.max(0) as u32,
            budget_u0: parametre.budget_u0,
        };
        let regroupable = matches!(n.regroupement, Regroupement::Resume);
        // Fratrie (R-N20) : un seul avis externe par foyer et demi-journée quand la personne le demande.
        let fratrie_deja_servie: bool = if reglage.regrouper_fratrie && n.regroupement == Regroupement::FoyerDemiJournee {
            sqlx::query_scalar(
                "SELECT EXISTS (
                    SELECT 1 FROM ent_ntf_remise r
                      JOIN ent_ntf_destinataire x ON x.etablissement_id = r.etablissement_id AND x.id = r.destinataire_id
                      JOIN ent_ntf_avis a ON a.etablissement_id = x.etablissement_id AND a.id = x.avis_id
                     WHERE r.etablissement_id = $1 AND x.personne_id = $2 AND a.nature = $3 AND a.id <> $4
                       AND a.eleve_id IS DISTINCT FROM $5 AND r.canal <> 'inapp'
                       AND r.statut IN ('planifiee', 'transmise', 'remise')
                       AND ent_aujourdhui_etab(r.etablissement_id, r.planifiee_pour) = $7
                       AND ((r.planifiee_pour AT TIME ZONE $6)::time < TIME '12:00') = $8)",
            )
            .bind(d.etablissement_id)
            .bind(l.personne_id)
            .bind(n.code)
            .bind(avis_id)
            .bind(d.eleve_id)
            .bind(&parametre.fuseau)
            .bind(local_confirme.date())
            .bind(local_confirme.time() < chrono::NaiveTime::from_hms_opt(12, 0, 0).expect("midi"))
            .fetch_one(&mut *conn)
            .await?
        } else {
            false
        };
        for canal in &plan.canaux {
            // Une seule remise par destination et par avis : les suivantes sont tracées, jamais envoyées (CN-33).
            let nouveau = destinations.reserver(*canal, l);
            let (instant, motif_plan) = match decider(urgence, canal.externe(), regroupable, &ctx) {
                Moment::Immediat => (confirme_apres.max(d.effet_le), if delai > Duration::zero() { "delai_confirmation" } else { "immediat" }),
                Moment::Differe(t) => (vers_utc(conn, &parametre.fuseau, t).await?, "heures_calmes"),
                Moment::Resume(t, m) => (vers_utc(conn, &parametre.fuseau, t).await?, m.code()),
            };
            let ecartee = if !nouveau {
                Some("doublon_destination")
            } else if canal.externe() && fratrie_deja_servie {
                Some("regroupe_fratrie")
            } else {
                None
            };
            sqlx::query(
                "INSERT INTO ent_ntf_remise
                    (id, etablissement_id, destinataire_id, canal, planifiee_pour, motif_planification, statut, motif)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (destinataire_id, canal) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(d.etablissement_id)
            .bind(dest_id)
            .bind(canal.code())
            .bind(instant.max(confirme_apres))
            .bind(motif_plan)
            .bind(if ecartee.is_some() { "ignoree" } else { "planifiee" })
            .bind(ecartee)
            .execute(&mut *conn)
            .await?;
        }
        if plan.a_prevenir {
            // Aucun canal n'atteint cette personne alors que la nature l'exige : tâche manuelle (§9.2).
            sqlx::query(
                "INSERT INTO ent_ntf_a_prevenir (id, etablissement_id, destinataire_id, echeance)
                 VALUES ($1,$2,$3,$4) ON CONFLICT (destinataire_id) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(d.etablissement_id)
            .bind(dest_id)
            .bind(confirme_apres + Duration::hours(24))
            .execute(&mut *conn)
            .await?;
        }
        retenus.push(dest_id);
    }
    Ok(AvisCree { avis_id, cree: true, retenus, exclus })
}

/// Issue d'une annulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Annulation {
    /// Avis annulé avant toute remise : rien n'est parti.
    Annule,
    /// Déjà remis : la correction est due si la nature la prévoit (lot N8).
    DejaRemis { correction_due: bool },
    /// Aucun avis correspondant (ou déjà annulé).
    Introuvable,
}

/// Annule un avis non encore remis (ses remises et ses destinataires passent à `annule`).
pub async fn annuler_avis(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    avis_id: Uuid,
    motif: &str,
    maintenant: DateTime<Utc>,
) -> Result<Annulation, sqlx::Error> {
    let a: Option<(String, String)> = sqlx::query_as(
        "SELECT statut, nature FROM ent_ntf_avis WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(etablissement_id)
    .bind(avis_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((statut, nature)) = a else {
        return Ok(Annulation::Introuvable);
    };
    if statut == "annule" {
        return Ok(Annulation::Introuvable);
    }
    if statut != "planifie" {
        let due = super::catalogue::nature(&nature).is_some_and(|n| n.correction_si_remis);
        return Ok(Annulation::DejaRemis { correction_due: due });
    }
    sqlx::query(
        "UPDATE ent_ntf_remise SET statut = 'annulee', motif = $3
          WHERE etablissement_id = $1 AND statut IN ('planifiee', 'reservee')
            AND destinataire_id IN (SELECT id FROM ent_ntf_destinataire WHERE etablissement_id = $1 AND avis_id = $2)",
    )
    .bind(etablissement_id)
    .bind(avis_id)
    .bind(motif)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "UPDATE ent_ntf_destinataire SET statut = 'annule' WHERE etablissement_id = $1 AND avis_id = $2 AND statut = 'a_remettre'",
    )
    .bind(etablissement_id)
    .bind(avis_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "UPDATE ent_ntf_avis SET statut = 'annule', annule_le = $3, annule_motif = $4
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etablissement_id)
    .bind(avis_id)
    .bind(maintenant)
    .bind(motif)
    .execute(&mut *conn)
    .await?;
    Ok(Annulation::Annule)
}

/// État d'un avis, sans adresse ni numéro (lecture de pilotage).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct EtatDestinataire {
    pub destinataire_id: Uuid,
    pub compte_id: Option<Uuid>,
    pub statut: String,
    pub notifie_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EtatAvis {
    pub statut: String,
    pub nature: String,
    pub destinataires: Vec<EtatDestinataire>,
}

pub async fn etat(
    conn: &mut PgConnection,
    etablissement_id: Uuid,
    avis_id: Uuid,
) -> Result<Option<EtatAvis>, sqlx::Error> {
    let a: Option<(String, String)> =
        sqlx::query_as("SELECT statut, nature FROM ent_ntf_avis WHERE etablissement_id = $1 AND id = $2")
            .bind(etablissement_id)
            .bind(avis_id)
            .fetch_optional(&mut *conn)
            .await?;
    let Some((statut, nature)) = a else {
        return Ok(None);
    };
    let destinataires = sqlx::query_as::<_, EtatDestinataire>(
        "SELECT id AS destinataire_id, compte_id, statut, notifie_le FROM ent_ntf_destinataire
          WHERE etablissement_id = $1 AND avis_id = $2 AND retenu ORDER BY created_at, id",
    )
    .bind(etablissement_id)
    .bind(avis_id)
    .fetch_all(&mut *conn)
    .await?;
    Ok(Some(EtatAvis { statut, nature, destinataires }))
}
