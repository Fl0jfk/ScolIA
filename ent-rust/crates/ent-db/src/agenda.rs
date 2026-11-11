//! Agenda (socle 19, lot AG1) : événements, séries matérialisées, audience, participants,
//! réponses, coupon, journal, **vue agenda calculée** par sources et iCal ponctuel minimisé.
//!
//! Principes (`19_AGENDA_RDV.md` §20) :
//! - **R-AG1** : l'agenda *lit* les sources (événements, rendez-vous, emploi du temps, calendrier) à la
//!   date ; une source absente ou en erreur est **dite** (`sources_indisponibles`), jamais tue ;
//! - **R-AG3** : chaque fait important est au journal en ajout seul ;
//! - **R-AG5** : un lecteur ne voit que ce que sa place autorise (invité, organisateur, partie
//!   publique) ; hors de là, l'événement est introuvable (404, indiscernable d'un inconnu) ;
//! - **R-AG7** : un événement `restreint` n'expose jamais son titre dans les avis ni dans l'iCal ;
//! - **R-AG8** : l'audience est résolue **à la publication, sous les droits de l'auteur**, avec un
//!   aperçu et une confirmation par empreinte (l'audience a pu changer entre-temps) ;
//! - heures murales converties avec le fuseau de l'**établissement** (jamais Europe/Paris en dur) ;
//! - plage de lecture bornée (42 jours) : aucune lecture « de tout ».

// Requêtes à colonnes nombreuses (tuples SQL) et services à paramètres explicites : voulu ici.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use crate::campagnes::{eleves_de_audience, responsables_de, sha256_hex, valider_audience};
use crate::edt_admin::{lundi_de, CibleEdt};
use crate::notifications::{db, notifier_personne, EnvoiAPlanifier};
use crate::outbox::Publier;
use crate::ref_commun::Acteur;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use ent_types::agenda::*;
use serde_json::json;
use std::collections::BTreeSet;
use uuid::Uuid;

pub mod types_agenda {
    pub const EVENEMENT_PUBLIE: &str = "agenda.evenement_publie";
    pub const EVENEMENT_MODIFIE: &str = "agenda.evenement_modifie";
    pub const EVENEMENT_ANNULE: &str = "agenda.evenement_annule";
    pub const SERIE_CREEE: &str = "agenda.serie_creee";
    pub const REPONSE: &str = "agenda.reponse";
}

/// Plage maximale d'une lecture d'agenda (R-AG1 / ND-07 : aucune lecture sans borne).
pub const PLAGE_MAX_JOURS: i64 = 42;
/// Occurrences maximales d'une série.
pub const OCCURRENCES_MAX: usize = 150;

fn introuvable() -> EntDbError {
    EntDbError::Domain(domain_eleve::DomainError::NotFound)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

fn tz_sql(param: &str) -> String {
    format!(
        "COALESCE((SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = {param}), 'Europe/Paris')"
    )
}

/// Heure murale de l'établissement → instants UTC. `jour_fin` inclus ; sans heures : journée entière
/// (de minuit local au minuit suivant la dernière journée).
async fn instants(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    jour: NaiveDate,
    h_debut: Option<NaiveTime>,
    jour_fin: NaiveDate,
    h_fin: Option<NaiveTime>,
) -> Result<(DateTime<Utc>, DateTime<Utc>, bool), EntDbError> {
    let minuit = NaiveTime::MIN;
    let (d, f, entiere) = match (h_debut, h_fin) {
        (Some(a), Some(b)) => ((jour, a), (jour_fin, b), false),
        (None, None) => ((jour, minuit), (jour_fin + Duration::days(1), minuit), true),
        _ => return Err(invalide("heure de début et heure de fin vont ensemble")),
    };
    let tz = tz_sql("$1");
    let (debut, fin): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(&format!(
        "SELECT ($2::date + $3::time) AT TIME ZONE {tz}, ($4::date + $5::time) AT TIME ZONE {tz}"
    ))
    .bind(etab)
    .bind(d.0)
    .bind(d.1)
    .bind(f.0)
    .bind(f.1)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if fin <= debut {
        return Err(invalide("la fin doit être après le début"));
    }
    Ok((debut, fin, entiere))
}

async fn journaliser(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    objet: &str,
    objet_id: Uuid,
    action: &str,
    par: Option<Uuid>,
    avant: Option<String>,
    apres: Option<String>,
) -> Result<(), EntDbError> {
    let borne = |t: Option<String>| t.map(|t| t.chars().take(1900).collect::<String>());
    sqlx::query(
        "INSERT INTO ent_agenda_journal (id, etablissement_id, objet, objet_id, action, par_user_id, avant, apres)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(objet)
    .bind(objet_id)
    .bind(action)
    .bind(par)
    .bind(borne(avant))
    .bind(borne(apres))
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

fn nettoyer(t: &str, max: usize, msg: &'static str) -> Result<String, EntDbError> {
    let t = t.trim();
    if t.is_empty() || t.chars().count() > max {
        return Err(invalide(msg));
    }
    Ok(t.to_string())
}

fn optionnel(
    t: &Option<String>,
    max: usize,
    msg: &'static str,
) -> Result<Option<String>, EntDbError> {
    match t.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(v) if v.chars().count() <= max => Ok(Some(v.to_string())),
        Some(_) => Err(invalide(msg)),
    }
}

const CONFIDENTIALITES: [&str; 3] = ["etablissement", "participants", "restreint"];

/// Ligne d'événement relue pour contrôler l'accès et construire les avis.
#[derive(Debug, Clone, sqlx::FromRow)]
struct LigneEvenement {
    id: Uuid,
    type_code: String,
    titre: String,
    description: Option<String>,
    debut: DateTime<Utc>,
    fin: DateTime<Utc>,
    journee_entiere: bool,
    jour_debut: NaiveDate,
    jour_fin: NaiveDate,
    lieu_kind: String,
    salle_id: Option<Uuid>,
    lieu_texte: Option<String>,
    organisateur_user_id: Uuid,
    confidentialite: String,
    statut: String,
    motif_annulation: Option<String>,
    serie_id: Option<Uuid>,
    occurrence_detachee: bool,
    version: i32,
}

const COLONNES_EVT: &str =
    "e.id, e.type_code, e.titre, e.description, e.debut, e.fin, e.journee_entiere,
    e.jour_debut, e.jour_fin, e.lieu_kind, e.salle_id, e.lieu_texte, e.organisateur_user_id,
    e.confidentialite, e.statut, e.motif_annulation, e.serie_id, e.occurrence_detachee, e.version";

async fn charger(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<LigneEvenement, EntDbError> {
    sqlx::query_as(&format!(
        "SELECT {COLONNES_EVT} FROM ent_agenda_evenement e
          WHERE e.etablissement_id = $1 AND e.id = $2{}",
        if verrou { " FOR UPDATE OF e" } else { "" }
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

/// Libellé du lieu (salle de l'établissement ou texte libre).
async fn lieu_de(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    e: &LigneEvenement,
) -> Result<Option<String>, EntDbError> {
    match (e.lieu_kind.as_str(), e.salle_id) {
        ("salle", Some(s)) => Ok(sqlx::query_scalar::<_, String>(
            "SELECT libelle FROM ent_salle WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(s)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?),
        ("texte", _) => Ok(e.lieu_texte.clone()),
        _ => Ok(None),
    }
}

async fn valider_saisie(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    n: &NouvelEvenement,
) -> Result<(), EntDbError> {
    nettoyer(&n.titre, 200, "titre : 1 à 200 caractères")?;
    optionnel(&n.description, 4000, "description trop longue")?;
    optionnel(&n.lieu_texte, 200, "lieu trop long")?;
    if n.salle_id.is_some() && optionnel(&n.lieu_texte, 200, "lieu trop long")?.is_some() {
        return Err(invalide("une salle ou un lieu libre, pas les deux"));
    }
    if let Some(c) = n.confidentialite.as_deref() {
        if !CONFIDENTIALITES.contains(&c) {
            return Err(invalide("confidentialité inconnue"));
        }
    }
    if n.invites.len() > 200 {
        return Err(invalide("trop d'invités nominatifs"));
    }
    if let Some(jf) = n.jour_fin {
        if jf < n.jour {
            return Err(invalide("le dernier jour précède le premier"));
        }
    }
    // Type : intégré, ou type actif de l'établissement.
    let integre = TYPES_INTEGRES.iter().any(|(c, _, _)| *c == n.type_code);
    if !integre {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_agenda_type_local
                             WHERE etablissement_id = $1 AND code = $2 AND actif)",
        )
        .bind(etab)
        .bind(&n.type_code)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if !ok {
            return Err(invalide("type d'événement inconnu"));
        }
    }
    if let Some(s) = n.salle_id {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_salle WHERE etablissement_id = $1 AND id = $2 AND actif)",
        )
        .bind(etab)
        .bind(s)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if !ok {
            return Err(invalide("salle inconnue dans cet établissement"));
        }
    }
    if let Some(c) = &n.coupon {
        nettoyer(&c.question, 300, "question du coupon : 1 à 300 caractères")?;
        if !(2..=3).contains(&c.choix.len()) {
            return Err(invalide("un coupon propose 2 ou 3 choix"));
        }
        for ch in &c.choix {
            nettoyer(ch, 80, "choix du coupon : 1 à 80 caractères")?;
        }
    }
    Ok(())
}

/// Les invités nominatifs sont du **personnel de cet établissement** (jamais un parent ni un élève :
/// un élève ne peut pas en inviter un autre, piège 7 ; les familles passent par l'audience).
async fn controler_invites(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    invites: &[Uuid],
) -> Result<Vec<Uuid>, EntDbError> {
    let distincts: Vec<Uuid> = invites
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if distincts.is_empty() {
        return Ok(distincts);
    }
    let (n,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_membership
          WHERE etablissement_id = $1 AND user_id = ANY($2) AND etat = 'ouvert'
            AND role NOT IN ('parent', 'eleve')",
    )
    .bind(etab)
    .bind(&distincts)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if n as usize != distincts.len() {
        return Err(invalide(
            "invité inconnu : seuls des comptes du personnel de l'établissement sont invitables",
        ));
    }
    Ok(distincts)
}

#[allow(clippy::too_many_arguments)]
async fn inserer_evenement(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    organisateur: Uuid,
    n: &NouvelEvenement,
    jour: NaiveDate,
    serie_id: Option<Uuid>,
    invites: &[Uuid],
) -> Result<Uuid, EntDbError> {
    let jour_fin = if serie_id.is_some() {
        jour
    } else {
        n.jour_fin.unwrap_or(jour)
    };
    let (debut, fin, entiere) =
        instants(conn, etab, jour, n.heure_debut, jour_fin, n.heure_fin).await?;
    let id = Uuid::new_v4();
    let lieu_texte = optionnel(&n.lieu_texte, 200, "lieu trop long")?;
    let kind = match (n.salle_id, &lieu_texte) {
        (Some(_), _) => "salle",
        (None, Some(_)) => "texte",
        _ => "aucun",
    };
    sqlx::query(
        "INSERT INTO ent_agenda_evenement
            (id, etablissement_id, type_code, titre, description, debut, fin, journee_entiere,
             jour_debut, jour_fin, lieu_kind, salle_id, lieu_texte, organisateur_user_id,
             confidentialite, serie_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
    )
    .bind(id)
    .bind(etab)
    .bind(&n.type_code)
    .bind(n.titre.trim())
    .bind(optionnel(&n.description, 4000, "description trop longue")?)
    .bind(debut)
    .bind(fin)
    .bind(entiere)
    .bind(jour)
    .bind(jour_fin)
    .bind(kind)
    .bind(n.salle_id)
    .bind(&lieu_texte)
    .bind(organisateur)
    .bind(n.confidentialite.as_deref().unwrap_or("participants"))
    .bind(serie_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_agenda_participant
            (id, etablissement_id, evenement_id, user_id, role, obligatoire, source, reponse, repondu_le)
         VALUES ($1,$2,$3,$4,'organisateur', true, 'organisateur', 'oui', now())",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(id)
    .bind(organisateur)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    for u in invites.iter().filter(|u| **u != organisateur) {
        sqlx::query(
            "INSERT INTO ent_agenda_participant
                (id, etablissement_id, evenement_id, user_id, role, source)
             VALUES ($1,$2,$3,$4,'invite','nominatif')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(u)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    if let Some(c) = &n.coupon {
        sqlx::query(
            "INSERT INTO ent_agenda_coupon (id, etablissement_id, evenement_id, question, choix_1, choix_2, choix_3)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(id)
        .bind(c.question.trim())
        .bind(c.choix[0].trim())
        .bind(c.choix[1].trim())
        .bind(c.choix.get(2).map(|s| s.trim()))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(id)
}

fn resume(e: &LigneEvenement) -> String {
    format!(
        "{} | {} → {} | {}{}",
        e.titre,
        e.debut.format("%Y-%m-%d %H:%M"),
        e.fin.format("%Y-%m-%d %H:%M"),
        e.confidentialite,
        e.lieu_texte
            .as_deref()
            .map(|l| format!(" | {l}"))
            .unwrap_or_default()
    )
}

/// Résolution de l'audience d'une publication (sous les droits de l'auteur, R-AG8).
struct Resolution {
    /// (responsable, enfant de contexte).
    familles: Vec<(Uuid, Uuid)>,
    nb_eleves: usize,
    exclus: usize,
    personnels: Vec<Uuid>,
    empreinte: String,
}

fn ouvre_hors_etab(kind: &str) -> bool {
    matches!(kind, "etablissement" | "tous_personnels")
}

impl PgEnt {
    async fn exiger_creer(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AgendaEvenementsCreer)
            .await
    }

    // ───── Création (AGD-113, AGD-119, AGD-118) ────────────────────────────────────────────

    /// Crée un événement en brouillon : visible de son seul organisateur tant qu'il n'est pas publié.
    pub async fn agenda_creer_evenement(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        n: NouvelEvenement,
    ) -> Result<EvenementCree, EntDbError> {
        self.exiger_creer(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        valider_saisie(&mut tx, etab, &n).await?;
        let invites = controler_invites(&mut tx, etab, &n.invites).await?;
        let id = inserer_evenement(&mut tx, etab, acteur, &n, n.jour, None, &invites).await?;
        let e = charger(&mut tx, etab, id, false).await?;
        journaliser(
            &mut tx,
            etab,
            "evenement",
            id,
            "creer",
            Some(acteur),
            None,
            Some(resume(&e)),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(EvenementCree {
            id,
            serie_id: None,
            nb_occurrences: 1,
        })
    }

    /// Série hebdomadaire **matérialisée** : une ligne par occurrence (modifiable, annulable une à une).
    pub async fn agenda_creer_serie(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        s: NouvelleSerie,
    ) -> Result<EvenementCree, EntDbError> {
        self.exiger_creer(etab, acteur, role).await?;
        if s.modele.coupon.is_some() {
            return Err(invalide("un coupon ne se pose pas sur une série"));
        }
        if s.modele.jour_fin.is_some_and(|j| j != s.modele.jour) {
            return Err(invalide(
                "une série se compose d'événements d'une seule journée",
            ));
        }
        let jours: BTreeSet<i16> = s.jours_semaine.iter().copied().collect();
        if jours.is_empty() || jours.iter().any(|j| !(1..=7).contains(j)) {
            return Err(invalide("jours de la semaine : 1 (lundi) à 7 (dimanche)"));
        }
        let intervalle = s.intervalle_semaines.unwrap_or(1);
        if !(1..=8).contains(&intervalle) {
            return Err(invalide("intervalle : 1 à 8 semaines"));
        }
        if s.au < s.modele.jour || s.au > s.modele.jour + Duration::days(730) {
            return Err(invalide(
                "la série se termine après son début, dans les deux ans",
            ));
        }
        let lundi0 = lundi_de(s.modele.jour);
        let mut dates = Vec::new();
        let mut d = s.modele.jour;
        while d <= s.au {
            use chrono::Datelike;
            let semaine = (lundi_de(d) - lundi0).num_days() / 7;
            if jours.contains(&(d.weekday().number_from_monday() as i16))
                && semaine % i64::from(intervalle) == 0
            {
                dates.push(d);
            }
            d += Duration::days(1);
        }
        if dates.is_empty() {
            return Err(invalide("aucune occurrence dans cette plage"));
        }
        if dates.len() > OCCURRENCES_MAX {
            return Err(invalide("trop d'occurrences : 150 au plus"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        valider_saisie(&mut tx, etab, &s.modele).await?;
        let invites = controler_invites(&mut tx, etab, &s.modele.invites).await?;
        let serie_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_agenda_serie (id, etablissement_id, frequence, intervalle, du, au, cree_par_user_id)
             VALUES ($1,$2,'hebdomadaire',$3,$4,$5,$6)",
        )
        .bind(serie_id)
        .bind(etab)
        .bind(intervalle)
        .bind(s.modele.jour)
        .bind(s.au)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for j in &jours {
            sqlx::query("INSERT INTO ent_agenda_serie_jour (etablissement_id, serie_id, jour_semaine) VALUES ($1,$2,$3)")
                .bind(etab)
                .bind(serie_id)
                .bind(j)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let mut premier = None;
        for d in &dates {
            let id = inserer_evenement(
                &mut tx,
                etab,
                acteur,
                &s.modele,
                *d,
                Some(serie_id),
                &invites,
            )
            .await?;
            premier.get_or_insert(id);
        }
        journaliser(
            &mut tx,
            etab,
            "serie",
            serie_id,
            "creer_serie",
            Some(acteur),
            None,
            Some(format!(
                "{} occurrences, du {} au {}",
                dates.len(),
                s.modele.jour,
                s.au
            )),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(EvenementCree {
            id: premier.expect("au moins une occurrence"),
            serie_id: Some(serie_id),
            nb_occurrences: dates.len(),
        })
    }

    /// Duplique un événement (sans participants ni audience) à un autre jour, en brouillon.
    pub async fn agenda_dupliquer_evenement(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        jour: NaiveDate,
    ) -> Result<EvenementCree, EntDbError> {
        self.exiger_creer(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e = charger(&mut tx, etab, id, false).await?;
        if e.organisateur_user_id != acteur {
            return Err(introuvable());
        }
        let tz = tz_sql("$1");
        let (hd, hf): (NaiveTime, NaiveTime) = sqlx::query_as(&format!(
            "SELECT (debut AT TIME ZONE {tz})::time, (fin AT TIME ZONE {tz})::time
               FROM ent_agenda_evenement WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let n = NouvelEvenement {
            type_code: e.type_code.clone(),
            titre: e.titre.clone(),
            description: e.description.clone(),
            jour,
            jour_fin: (e.jour_fin > e.jour_debut).then(|| jour + (e.jour_fin - e.jour_debut)),
            heure_debut: (!e.journee_entiere).then_some(hd),
            heure_fin: (!e.journee_entiere).then_some(hf),
            salle_id: e.salle_id,
            lieu_texte: e.lieu_texte.clone(),
            confidentialite: Some(e.confidentialite.clone()),
            invites: vec![],
            coupon: None,
        };
        let nouveau = inserer_evenement(&mut tx, etab, acteur, &n, jour, None, &[]).await?;
        journaliser(
            &mut tx,
            etab,
            "evenement",
            nouveau,
            "dupliquer",
            Some(acteur),
            Some(id.to_string()),
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(EvenementCree {
            id: nouveau,
            serie_id: None,
            nb_occurrences: 1,
        })
    }

    // ───── Publication (AGD-114, R-AG8) ─────────────────────────────────────────────────────

    async fn resoudre_audience(
        &self,
        tx: &mut sqlx::PgConnection,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        evenement_id: Uuid,
        audiences: &[AudienceDto],
    ) -> Result<Resolution, EntDbError> {
        let mut eleves: BTreeSet<Uuid> = BTreeSet::new();
        let mut personnels: BTreeSet<Uuid> = BTreeSet::new();
        let mut exclus = 0usize;
        if !audiences.is_empty() {
            // Droit de publier à une audience, et son périmètre (`None` : tout l'établissement).
            self.exiger_action_quelconque(etab, acteur, role, Action::AgendaEvenementsPublier)
                .await?;
            let perimetre = self
                .eleves_autorises(etab, acteur, role, Action::AgendaEvenementsPublier)
                .await?;
            if audiences.len() > 50 {
                return Err(invalide("trop d'audiences"));
            }
            let jour = self.aujourdhui_etablissement(etab).await?;
            for a in audiences {
                if ouvre_hors_etab(&a.kind) && perimetre.is_some() {
                    // Un rôle limité à ses classes n'écrit pas à tout l'établissement.
                    return Err(EntDbError::Forbidden);
                }
                if a.kind == "tous_personnels" {
                    if a.cible_id.is_some() {
                        return Err(invalide("« tous les personnels » n'a pas de cible"));
                    }
                    let rows: Vec<(Uuid,)> = sqlx::query_as(
                        "SELECT user_id FROM ent_membership
                          WHERE etablissement_id = $1 AND etat = 'ouvert'
                            AND role NOT IN ('parent', 'eleve')",
                    )
                    .bind(etab)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(db)?;
                    personnels.extend(rows.into_iter().map(|(u,)| u));
                    continue;
                }
                let cibles: Vec<Uuid> = a.cible_id.into_iter().collect();
                valider_audience(&mut *tx, etab, &a.kind, &cibles).await?;
                let trouves = eleves_de_audience(&mut *tx, etab, &a.kind, &cibles, jour).await?;
                for e in trouves {
                    match &perimetre {
                        Some(p) if !p.contains(&e) => exclus += 1,
                        _ => {
                            eleves.insert(e);
                        }
                    }
                }
            }
        }
        let liste: Vec<Uuid> = eleves.iter().copied().collect();
        let responsables = responsables_de(&mut *tx, etab, &liste).await?;
        let familles: Vec<(Uuid, Uuid)> = responsables
            .iter()
            .map(|r| (r.user_id, r.eleve_id))
            .collect();
        let mut auds: Vec<String> = audiences
            .iter()
            .map(|a| format!("{}:{:?}", a.kind, a.cible_id))
            .collect();
        auds.sort();
        let ids_fam: BTreeSet<Uuid> = familles.iter().map(|f| f.0).collect();
        let empreinte = sha256_hex(&format!(
            "{evenement_id}|{auds:?}|{ids_fam:?}|{personnels:?}|{exclus}"
        ));
        Ok(Resolution {
            familles,
            nb_eleves: eleves.len(),
            exclus,
            personnels: personnels.into_iter().collect(),
            empreinte,
        })
    }

    /// Aperçu (rien n'est écrit) puis publication avec confirmation par empreinte. Sans audience
    /// (invités nominatifs seulement), la publication n'exige que le droit de créer.
    pub async fn agenda_publier_evenement(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        d: DemandePublication,
    ) -> Result<Result<ApercuPublication, EvenementDetail>, EntDbError> {
        self.exiger_creer(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e = charger(&mut tx, etab, id, true).await?;
        if e.organisateur_user_id != acteur {
            return Err(introuvable());
        }
        if e.statut != "brouillon" {
            return Err(EntDbError::ConflitDossier(
                "l'événement est déjà publié, annulé ou clos",
            ));
        }
        let res = self
            .resoudre_audience(&mut tx, etab, acteur, role, id, &d.audiences)
            .await?;
        let apercu = ApercuPublication {
            nb_familles: res.nb_eleves,
            nb_responsables: res.familles.len(),
            nb_personnels: res.personnels.len(),
            exclus_hors_droits: res.exclus,
            empreinte: res.empreinte.clone(),
        };
        if d.apercu_seulement {
            return Ok(Ok(apercu));
        }
        if !d.audiences.is_empty() {
            match d.empreinte.as_deref() {
                None => {
                    return Err(invalide(
                        "confirmation requise : relisez l'aperçu avant de publier",
                    ))
                }
                Some(emp) if emp != res.empreinte => {
                    return Err(EntDbError::ConflitDossier(
                        "l'audience a changé depuis l'aperçu : relisez-le avant de publier",
                    ))
                }
                _ => {}
            }
        }
        // Série : « toutes » ou « suivantes » publient les occurrences encore en brouillon ; l'avis aux
        // invités n'est envoyé qu'une fois (sur la première), pour ne pas multiplier les messages.
        let cibles = Self::cibles_portee(&mut tx, etab, &e, d.portee.as_deref()).await?;
        let mut premier = true;
        for cid in &cibles {
            let ev = charger(&mut tx, etab, *cid, true).await?;
            if ev.statut != "brouillon" {
                continue;
            }
            self.appliquer_publication(
                &mut tx,
                etab,
                acteur,
                &ev,
                &d.audiences,
                &res,
                premier,
                cibles.len(),
            )
            .await?;
            premier = false;
        }
        tx.commit().await.map_err(db)?;
        Ok(Err(self.agenda_evenement(etab, acteur, role, id).await?))
    }

    /// Publie UNE occurrence : audience, participants, statut, journal, événement, avis (si `notifier`).
    #[allow(clippy::too_many_arguments)]
    async fn appliquer_publication(
        &self,
        tx: &mut sqlx::PgConnection,
        etab: Uuid,
        acteur: Uuid,
        e: &LigneEvenement,
        audiences: &[AudienceDto],
        res: &Resolution,
        notifier: bool,
        nb_occurrences: usize,
    ) -> Result<(), EntDbError> {
        let id = e.id;
        for a in audiences {
            sqlx::query(
                "INSERT INTO ent_agenda_audience (id, etablissement_id, evenement_id, kind, cible_id)
                 VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(e.id)
            .bind(&a.kind)
            .bind(a.cible_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        for (u, eleve) in &res.familles {
            sqlx::query(
                "INSERT INTO ent_agenda_participant
                    (id, etablissement_id, evenement_id, user_id, role, source, eleve_contexte_id)
                 VALUES ($1,$2,$3,$4,'famille','audience',$5) ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(e.id)
            .bind(u)
            .bind(eleve)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        for u in &res.personnels {
            sqlx::query(
                "INSERT INTO ent_agenda_participant
                    (id, etablissement_id, evenement_id, user_id, role, source)
                 VALUES ($1,$2,$3,$4,'invite','audience') ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(e.id)
            .bind(u)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        sqlx::query(
            "UPDATE ent_agenda_evenement SET statut = 'publie', publie_le = now(), version = version + 1,
                    updated_at = now() WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut *tx,
            etab,
            "evenement",
            id,
            "publier",
            Some(acteur),
            Some("brouillon".into()),
            Some(format!(
                "{} familles, {} personnels",
                res.familles.len(),
                res.personnels.len()
            )),
        )
        .await?;
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_agenda::EVENEMENT_PUBLIE.to_string(),
                aggregate: AggregateKind::AgendaEvenement,
                aggregate_id: id,
                eleve_id: None,
                payload: json!({ "evenement_id": id, "type": e.type_code }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        if notifier {
            // Avis aux invités (R-AG7 : un événement restreint ne montre pas son titre).
            let invites: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
                "SELECT user_id, eleve_contexte_id FROM ent_agenda_participant
                  WHERE etablissement_id = $1 AND evenement_id = $2 AND role <> 'organisateur'",
            )
            .bind(etab)
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let tz = tz_sql("$1");
            let (quand,): (String,) = sqlx::query_as(&format!(
                "SELECT to_char(debut AT TIME ZONE {tz}, 'DD/MM/YYYY \"à\" HH24:MI') FROM ent_agenda_evenement
                  WHERE etablissement_id = $1 AND id = $2"
            ))
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let suite = if nb_occurrences > 1 {
                format!(" (première de {nb_occurrences} dates : voir l'agenda pour les suivantes)")
            } else {
                String::new()
            };
            let titre_avis = if e.confidentialite == "restreint" {
                "Événement de l'établissement".to_string()
            } else {
                e.titre.clone()
            };
            let corps = if e.confidentialite == "restreint" {
                format!(
                    "Vous êtes invité(e) à un événement le {quand}.{suite} Ouvrez l'agenda pour le détail."
                )
            } else {
                format!("Vous êtes invité(e) : {} — le {quand}.{suite}", e.titre)
            };
            let maintenant = Utc::now();
            for (u, eleve) in invites {
                notifier_personne(
                    &mut *tx,
                    EnvoiAPlanifier {
                        etablissement_id: etab,
                        user_id: u,
                        kind: "agenda_invitation",
                        titre: &titre_avis,
                        corps: &corps,
                        eleve_id: eleve,
                        notification_id: None,
                        source_type: "evenement",
                        source_id: id,
                        cle_dedup: format!("agenda:{id}:invitation"),
                        droit_requis: eleve.map(|_| "lien"),
                        regroupable: true,
                        canaux_voulus: &["email", "push"],
                    },
                    evt.id,
                    maintenant,
                )
                .await?;
            }
        }
        Ok(())
    }

    // ───── Modification, report, annulation (AGD-115 à AGD-117, AGD-120) ────────────────────

    async fn cibles_portee(
        tx: &mut sqlx::PgConnection,
        etab: Uuid,
        e: &LigneEvenement,
        portee: Option<&str>,
    ) -> Result<Vec<Uuid>, EntDbError> {
        match (portee.unwrap_or("occurrence"), e.serie_id) {
            ("occurrence", _) => Ok(vec![e.id]),
            ("suivantes" | "toutes", None) => {
                Err(invalide("cet événement ne fait pas partie d'une série"))
            }
            (p @ ("suivantes" | "toutes"), Some(serie)) => {
                let rows: Vec<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM ent_agenda_evenement
                      WHERE etablissement_id = $1 AND serie_id = $2 AND statut IN ('brouillon','publie')
                        AND ($3 = 'toutes' OR debut >= $4)
                      ORDER BY debut",
                )
                .bind(etab)
                .bind(serie)
                .bind(p)
                .bind(e.debut)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                Ok(rows.into_iter().map(|(i,)| i).collect())
            }
            _ => Err(invalide("portée : occurrence, suivantes ou toutes")),
        }
    }

    /// Modifie un événement (organisateur seul). Les invités sont prévenus quand la date, l'heure ou
    /// le lieu d'un événement publié changent ; une occurrence de série déplacée est « détachée ».
    pub async fn agenda_modifier_evenement(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        m: ModifEvenement,
    ) -> Result<EvenementDetail, EntDbError> {
        self.exiger_creer(etab, acteur, role).await?;
        let portee = m.portee.clone();
        if portee.as_deref().unwrap_or("occurrence") != "occurrence" && m.jour.is_some() {
            return Err(invalide("le jour ne se change que sur une occurrence"));
        }
        if let Some(c) = m.confidentialite.as_deref() {
            if !CONFIDENTIALITES.contains(&c) {
                return Err(invalide("confidentialité inconnue"));
            }
        }
        let titre = m
            .titre
            .as_deref()
            .map(|t| nettoyer(t, 200, "titre : 1 à 200 caractères"))
            .transpose()?;
        let description = optionnel(&m.description, 4000, "description trop longue")?;
        let lieu_texte = optionnel(&m.lieu_texte, 200, "lieu trop long")?;
        if m.salle_id.is_some() && lieu_texte.is_some() {
            return Err(invalide("une salle ou un lieu libre, pas les deux"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e0 = charger(&mut tx, etab, id, true).await?;
        if e0.organisateur_user_id != acteur {
            return Err(introuvable());
        }
        if !matches!(e0.statut.as_str(), "brouillon" | "publie") {
            return Err(EntDbError::ConflitDossier(
                "un événement annulé ou clos ne se modifie plus",
            ));
        }
        if e0.version != m.version {
            return Err(EntDbError::ConflitDossier(
                "l'événement a été modifié entre-temps : rechargez-le",
            ));
        }
        if let Some(s) = m.salle_id {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_salle WHERE etablissement_id = $1 AND id = $2 AND actif)",
            )
            .bind(etab)
            .bind(s)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(invalide("salle inconnue dans cet établissement"));
            }
        }
        let cibles = Self::cibles_portee(&mut tx, etab, &e0, portee.as_deref()).await?;
        let tz = tz_sql("$1");
        let mut horaire_change = false;
        for cid in &cibles {
            let avant = charger(&mut tx, etab, *cid, true).await?;
            let (hd, hf): (NaiveTime, NaiveTime) = sqlx::query_as(&format!(
                "SELECT (debut AT TIME ZONE {tz})::time, (fin AT TIME ZONE {tz})::time
                   FROM ent_agenda_evenement WHERE etablissement_id = $1 AND id = $2"
            ))
            .bind(etab)
            .bind(cid)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let jour = m.jour.unwrap_or(avant.jour_debut);
            let jour_fin = if m.jour.is_some() {
                jour + (avant.jour_fin - avant.jour_debut)
            } else {
                avant.jour_fin
            };
            let (nd, nf, entiere) =
                if avant.journee_entiere && m.heure_debut.is_none() && m.heure_fin.is_none() {
                    instants(&mut tx, etab, jour, None, jour_fin, None).await?
                } else {
                    instants(
                        &mut tx,
                        etab,
                        jour,
                        Some(m.heure_debut.unwrap_or(hd)),
                        jour_fin,
                        Some(m.heure_fin.unwrap_or(hf)),
                    )
                    .await?
                };
            let lieu_change = m.salle_id.is_some() || lieu_texte.is_some();
            let (kind, salle, texte) = if let Some(s) = m.salle_id {
                ("salle", Some(s), None)
            } else if let Some(t) = &lieu_texte {
                ("texte", None, Some(t.clone()))
            } else {
                (
                    avant.lieu_kind.as_str(),
                    avant.salle_id,
                    avant.lieu_texte.clone(),
                )
            };
            let detache = avant.serie_id.is_some()
                && (portee.as_deref().unwrap_or("occurrence") == "occurrence")
                && (m.jour.is_some() || m.heure_debut.is_some() || m.heure_fin.is_some());
            sqlx::query(
                "UPDATE ent_agenda_evenement
                    SET titre = COALESCE($3, titre), description = COALESCE($4, description),
                        debut = $5, fin = $6, journee_entiere = $7, jour_debut = $8, jour_fin = $9,
                        lieu_kind = $10, salle_id = $11, lieu_texte = $12,
                        confidentialite = COALESCE($13, confidentialite),
                        occurrence_detachee = occurrence_detachee OR $14,
                        version = version + 1, updated_at = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(cid)
            .bind(&titre)
            .bind(&description)
            .bind(nd)
            .bind(nf)
            .bind(entiere)
            .bind(jour)
            .bind(jour_fin)
            .bind(kind)
            .bind(salle)
            .bind(&texte)
            .bind(m.confidentialite.as_deref())
            .bind(detache)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let apres = charger(&mut tx, etab, *cid, false).await?;
            if avant.statut == "publie"
                && (avant.debut != apres.debut || avant.fin != apres.fin || lieu_change)
            {
                horaire_change = true;
            }
            journaliser(
                &mut tx,
                etab,
                "evenement",
                *cid,
                "modifier",
                Some(acteur),
                Some(resume(&avant)),
                Some(resume(&apres)),
            )
            .await?;
        }
        if horaire_change {
            let evt = tx
                .publier(MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types_agenda::EVENEMENT_MODIFIE.to_string(),
                    aggregate: AggregateKind::AgendaEvenement,
                    aggregate_id: id,
                    eleve_id: None,
                    payload: json!({ "evenement_id": id }),
                    actor_user_id: Some(acteur.to_string()),
                })
                .await?;
            self.avis_participants(
                &mut tx,
                etab,
                id,
                evt.id,
                "agenda_modifie",
                "Événement modifié",
                None,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        self.agenda_evenement(etab, acteur, role, id).await
    }

    /// Avis à tous les participants (hors organisateur) d'un événement.
    #[allow(clippy::too_many_arguments)]
    async fn avis_participants(
        &self,
        tx: &mut sqlx::PgConnection,
        etab: Uuid,
        id: Uuid,
        evt_id: Uuid,
        kind: &'static str,
        titre: &str,
        motif: Option<&str>,
    ) -> Result<(), EntDbError> {
        let e = charger(&mut *tx, etab, id, false).await?;
        let tz = tz_sql("$1");
        let (quand,): (String,) = sqlx::query_as(&format!(
            "SELECT to_char(debut AT TIME ZONE {tz}, 'DD/MM/YYYY \"à\" HH24:MI') FROM ent_agenda_evenement
              WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let nom = if e.confidentialite == "restreint" {
            "Un événement de l'établissement".to_string()
        } else {
            format!("« {} »", e.titre)
        };
        let corps = format!(
            "{nom} — {quand}.{}",
            motif.map(|m| format!(" Motif : {m}.")).unwrap_or_default()
        );
        let invites: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT user_id, eleve_contexte_id FROM ent_agenda_participant
              WHERE etablissement_id = $1 AND evenement_id = $2 AND role <> 'organisateur'",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let maintenant = Utc::now();
        for (u, eleve) in invites {
            notifier_personne(
                &mut *tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: u,
                    kind,
                    titre,
                    corps: &corps,
                    eleve_id: eleve,
                    notification_id: None,
                    source_type: "evenement",
                    source_id: id,
                    cle_dedup: format!("agenda:{id}:{kind}:{evt_id}"),
                    droit_requis: eleve.map(|_| "lien"),
                    regroupable: false,
                    canaux_voulus: &["email", "push"],
                },
                evt_id,
                maintenant,
            )
            .await?;
        }
        Ok(())
    }

    /// Annule (motif public obligatoire) : l'événement, ou toute / une partie de sa série.
    pub async fn agenda_annuler_evenement(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        a: AnnulationEvenement,
    ) -> Result<EvenementDetail, EntDbError> {
        self.exiger_creer(etab, acteur, role).await?;
        let motif = nettoyer(&a.motif, 300, "motif obligatoire (300 caractères au plus)")?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e = charger(&mut tx, etab, id, true).await?;
        if e.organisateur_user_id != acteur {
            return Err(introuvable());
        }
        if !matches!(e.statut.as_str(), "brouillon" | "publie") {
            return Err(EntDbError::ConflitDossier("déjà annulé ou clos"));
        }
        let cibles = Self::cibles_portee(&mut tx, etab, &e, a.portee.as_deref()).await?;
        for cid in &cibles {
            let avant = charger(&mut tx, etab, *cid, true).await?;
            sqlx::query(
                "UPDATE ent_agenda_evenement SET statut = 'annule', motif_annulation = $3,
                        version = version + 1, updated_at = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(cid)
            .bind(&motif)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            journaliser(
                &mut tx,
                etab,
                "evenement",
                *cid,
                "annuler",
                Some(acteur),
                Some(avant.statut.clone()),
                Some(motif.clone()),
            )
            .await?;
            if avant.statut == "publie" {
                let evt = tx
                    .publier(MetierEventDraft {
                        etablissement_id: etab,
                        type_name: types_agenda::EVENEMENT_ANNULE.to_string(),
                        aggregate: AggregateKind::AgendaEvenement,
                        aggregate_id: *cid,
                        eleve_id: None,
                        payload: json!({ "evenement_id": cid }),
                        actor_user_id: Some(acteur.to_string()),
                    })
                    .await?;
                self.avis_participants(
                    &mut tx,
                    etab,
                    *cid,
                    evt.id,
                    "agenda_annule",
                    "Événement annulé",
                    Some(&motif),
                )
                .await?;
            }
        }
        tx.commit().await.map_err(db)?;
        self.agenda_evenement(etab, acteur, role, id).await
    }

    // ───── Lecture d'un événement (AGD-131, AGD-147) ────────────────────────────────────────

    /// Détail d'un événement **selon la place du lecteur** (R-AG5) : l'organisateur voit tout ; un
    /// invité voit les décomptes ; la partie publique (`etablissement`) est lisible de tous ; sinon 404.
    pub async fn agenda_evenement(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<EvenementDetail, EntDbError> {
        let _ = role;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e = charger(&mut tx, etab, id, false).await?;
        let moi: Option<(String, String)> = sqlx::query_as(
            "SELECT role, reponse FROM ent_agenda_participant
              WHERE etablissement_id = $1 AND evenement_id = $2 AND user_id = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(user)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let organisateur = e.organisateur_user_id == user;
        let visible = organisateur
            || (moi.is_some() && e.statut != "brouillon")
            || (e.confidentialite == "etablissement" && e.statut != "brouillon");
        if !visible {
            return Err(introuvable());
        }
        let lieu = lieu_de(&mut tx, etab, &e).await?;
        let tz = tz_sql("$1");
        let (hd, hf): (NaiveTime, NaiveTime) = sqlx::query_as(&format!(
            "SELECT (debut AT TIME ZONE {tz})::time, (fin AT TIME ZONE {tz})::time
               FROM ent_agenda_evenement WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let (orga_nom,): (Option<String>,) =
            sqlx::query_as("SELECT display_name FROM ent_user WHERE id = $1")
                .bind(e.organisateur_user_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        let comptes: Vec<(String, i64)> = sqlx::query_as(
            "SELECT reponse, count(*) FROM ent_agenda_participant
              WHERE etablissement_id = $1 AND evenement_id = $2 GROUP BY reponse",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let n = |r: &str| {
            comptes
                .iter()
                .find(|(x, _)| x == r)
                .map(|(_, c)| *c)
                .unwrap_or(0)
        };
        let participants = if organisateur {
            let rows: Vec<(Uuid, Option<String>, String, bool, String, Option<String>)> = sqlx::query_as(
                "SELECT p.user_id, u.display_name, p.role, p.obligatoire, p.reponse, p.motif_excuse
                   FROM ent_agenda_participant p JOIN ent_user u ON u.id = p.user_id
                  WHERE p.etablissement_id = $1 AND p.evenement_id = $2
                  ORDER BY p.role, u.display_name NULLS LAST, p.user_id LIMIT 2000",
            )
            .bind(etab)
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            Some(
                rows.into_iter()
                    .map(|(user_id, nom, role, obligatoire, reponse, motif_excuse)| {
                        ParticipantDto {
                            user_id,
                            nom,
                            role,
                            obligatoire,
                            reponse,
                            motif_excuse,
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };
        let coupon: Option<(Uuid, String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT id, question, choix_1, choix_2, choix_3 FROM ent_agenda_coupon
              WHERE etablissement_id = $1 AND evenement_id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let coupon = match coupon {
            None => None,
            Some((cid, question, c1, c2, c3)) => {
                let mut choix = vec![c1, c2];
                choix.extend(c3);
                let mon: Option<(i16,)> = sqlx::query_as(
                    "SELECT choix FROM ent_agenda_coupon_reponse
                      WHERE etablissement_id = $1 AND coupon_id = $2 AND user_id = $3",
                )
                .bind(etab)
                .bind(cid)
                .bind(user)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                let decompte = if organisateur {
                    let rows: Vec<(i16, i64)> = sqlx::query_as(
                        "SELECT choix, count(*) FROM ent_agenda_coupon_reponse
                          WHERE etablissement_id = $1 AND coupon_id = $2 GROUP BY choix",
                    )
                    .bind(etab)
                    .bind(cid)
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(db)?;
                    Some(
                        (1..=choix.len() as i16)
                            .map(|i| {
                                rows.iter()
                                    .find(|(c, _)| *c == i)
                                    .map(|(_, n)| *n)
                                    .unwrap_or(0)
                            })
                            .collect(),
                    )
                } else {
                    None
                };
                Some(CouponDto {
                    question,
                    choix,
                    decompte,
                    mon_choix: mon.map(|m| m.0),
                })
            }
        };
        Ok(EvenementDetail {
            id: e.id,
            type_code: e.type_code,
            titre: e.titre,
            description: e.description,
            debut: e.debut,
            fin: e.fin,
            jour: e.jour_debut,
            jour_fin: e.jour_fin,
            heure_debut: (!e.journee_entiere).then_some(hd),
            heure_fin: (!e.journee_entiere).then_some(hf),
            journee_entiere: e.journee_entiere,
            lieu,
            confidentialite: e.confidentialite,
            statut: e.statut,
            motif_annulation: e.motif_annulation,
            serie_id: e.serie_id,
            occurrence_detachee: e.occurrence_detachee,
            version: e.version,
            organisateur_user_id: e.organisateur_user_id,
            organisateur_nom: orga_nom,
            je_suis_organisateur: organisateur,
            ma_reponse: moi.map(|m| m.1),
            participants,
            nb_oui: n("oui"),
            nb_non: n("non"),
            nb_peut_etre: n("peut_etre"),
            nb_en_attente: n("en_attente"),
            coupon,
        })
    }

    /// Historique d'un événement : son organisateur, ou qui lit les journaux de l'agenda.
    pub async fn agenda_historique(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Vec<FaitJournal>, EntDbError> {
        let lecteur = self
            .a_action_etablissement(etab, user, role, Action::AgendaJournalLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e = charger(&mut tx, etab, id, false).await?;
        if e.organisateur_user_id != user && !lecteur {
            return Err(introuvable());
        }
        let rows: Vec<(String, Option<Uuid>, DateTime<Utc>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT action, par_user_id, le, avant, apres FROM ent_agenda_journal
              WHERE etablissement_id = $1 AND objet = 'evenement' AND objet_id = $2 ORDER BY le, id",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(action, par_user_id, le, avant, apres)| FaitJournal {
                action,
                par_user_id,
                le,
                avant,
                apres,
            })
            .collect())
    }

    // ───── Réponses (AGD-128, AGD-129, AGD-144) ─────────────────────────────────────────────

    /// L'invité répond (oui / non / peut-être, motif). Passer de « oui » à « non » prévient l'organisateur.
    pub async fn agenda_repondre(
        &self,
        etab: Uuid,
        user: Uuid,
        id: Uuid,
        r: ReponseInvitation,
    ) -> Result<EvenementDetail, EntDbError> {
        if !matches!(r.reponse.as_str(), "oui" | "non" | "peut_etre") {
            return Err(invalide("réponse : oui, non ou peut_etre"));
        }
        let motif = optionnel(&r.motif_excuse, 300, "motif trop long")?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e = charger(&mut tx, etab, id, false).await?;
        let moi: Option<(String, String)> = sqlx::query_as(
            "SELECT role, reponse FROM ent_agenda_participant
              WHERE etablissement_id = $1 AND evenement_id = $2 AND user_id = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .bind(user)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((role_p, avant)) = moi else {
            return Err(introuvable());
        };
        if e.statut == "brouillon" {
            return Err(introuvable());
        }
        if e.statut != "publie" {
            return Err(EntDbError::ConflitDossier(
                "l'événement est annulé ou clos : plus de réponse",
            ));
        }
        sqlx::query(
            "UPDATE ent_agenda_participant SET reponse = $4, repondu_le = now(), motif_excuse = $5
              WHERE etablissement_id = $1 AND evenement_id = $2 AND user_id = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(user)
        .bind(&r.reponse)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            etab,
            "evenement",
            id,
            "reponse",
            Some(user),
            Some(avant.clone()),
            Some(r.reponse.clone()),
        )
        .await?;
        if avant == "oui" && r.reponse == "non" && role_p != "organisateur" {
            let (nom,): (Option<String>,) =
                sqlx::query_as("SELECT display_name FROM ent_user WHERE id = $1")
                    .bind(user)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
            let evt = tx
                .publier(MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types_agenda::REPONSE.to_string(),
                    aggregate: AggregateKind::AgendaEvenement,
                    aggregate_id: id,
                    eleve_id: None,
                    payload: json!({ "evenement_id": id }),
                    actor_user_id: Some(user.to_string()),
                })
                .await?;
            let corps = format!(
                "{} ne pourra finalement pas participer à « {} ».{}",
                nom.unwrap_or_else(|| "Un participant".into()),
                if e.confidentialite == "restreint" {
                    "l'événement"
                } else {
                    e.titre.as_str()
                },
                motif
                    .as_deref()
                    .map(|m| format!(" Motif : {m}."))
                    .unwrap_or_default()
            );
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: e.organisateur_user_id,
                    kind: "agenda_empechement",
                    titre: "Empêchement",
                    corps: &corps,
                    eleve_id: None,
                    notification_id: None,
                    source_type: "evenement",
                    source_id: id,
                    cle_dedup: format!("agenda:{id}:empechement:{user}:{}", evt.id),
                    droit_requis: None,
                    regroupable: false,
                    canaux_voulus: &["push"],
                },
                evt.id,
                Utc::now(),
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        self.agenda_evenement(etab, user, "", id).await
    }

    /// Réponse au coupon (question fermée) : un participant, une réponse, modifiable.
    pub async fn agenda_repondre_coupon(
        &self,
        etab: Uuid,
        user: Uuid,
        id: Uuid,
        r: ReponseCoupon,
    ) -> Result<EvenementDetail, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let e = charger(&mut tx, etab, id, false).await?;
        let (est,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_agenda_participant
                             WHERE etablissement_id = $1 AND evenement_id = $2 AND user_id = $3)",
        )
        .bind(etab)
        .bind(id)
        .bind(user)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !est || e.statut == "brouillon" {
            return Err(introuvable());
        }
        if e.statut != "publie" {
            return Err(EntDbError::ConflitDossier(
                "l'événement est annulé ou clos : plus de réponse",
            ));
        }
        let c: Option<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT id, choix_3 FROM ent_agenda_coupon WHERE etablissement_id = $1 AND evenement_id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (cid, c3) = c.ok_or_else(introuvable)?;
        let max = if c3.is_some() { 3 } else { 2 };
        if !(1..=max).contains(&r.choix) {
            return Err(invalide("choix hors du coupon"));
        }
        sqlx::query(
            "INSERT INTO ent_agenda_coupon_reponse (etablissement_id, coupon_id, user_id, choix)
             VALUES ($1,$2,$3,$4)
             ON CONFLICT (etablissement_id, coupon_id, user_id) DO UPDATE SET choix = $4, repondu_le = now()",
        )
        .bind(etab)
        .bind(cid)
        .bind(user)
        .bind(r.choix)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.agenda_evenement(etab, user, "", id).await
    }

    // ───── Types locaux (AGD, paramétrage) ───────────────────────────────────────────────────

    /// Types disponibles : intégrés puis ceux de l'établissement (actifs).
    pub async fn agenda_types(
        &self,
        etab: Uuid,
    ) -> Result<Vec<(String, String, String)>, EntDbError> {
        let mut v: Vec<(String, String, String)> = TYPES_INTEGRES
            .iter()
            .map(|(c, l, f)| (c.to_string(), l.to_string(), f.to_string()))
            .collect();
        let rows: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT code, libelle, famille FROM ent_agenda_type_local
              WHERE etablissement_id = $1 AND actif ORDER BY libelle",
        )
        .bind(etab)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        v.extend(rows);
        Ok(v)
    }

    pub async fn agenda_creer_type_local(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        code: &str,
        libelle: &str,
        famille: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AgendaParametrer)
            .await?;
        let libelle = nettoyer(libelle, 80, "libellé : 1 à 80 caractères")?;
        if TYPES_INTEGRES.iter().any(|(c, _, _)| *c == code) {
            return Err(EntDbError::ConflitDossier(
                "ce code est celui d'un type intégré",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_agenda_type_local (id, etablissement_id, code, libelle, famille)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id)
        .bind(etab)
        .bind(code)
        .bind(&libelle)
        .bind(famille)
        .execute(&mut *tx)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(d) if d.code().as_deref() == Some("23505") => {
                EntDbError::ConflitDossier("ce code existe déjà")
            }
            sqlx::Error::Database(d) if d.code().as_deref() == Some("23514") => {
                invalide("code (minuscules, chiffres, _) ou famille invalide")
            }
            _ => db(e),
        })?;
        journaliser(
            &mut tx,
            etab,
            "type",
            id,
            "creer_type",
            Some(acteur),
            None,
            Some(format!("{code} — {libelle}")),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ───── Vue agenda calculée (AGD-100 à AGD-104, AGD-111) ──────────────────────────────────

    /// Mon agenda sur une plage bornée, toutes sources. `eleve` : l'agenda d'un enfant (responsable,
    /// élève majeur) ; `sources` : filtre (vide = toutes). Une source en échec est signalée.
    pub async fn agenda_vue(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        du: NaiveDate,
        au: NaiveDate,
        sources: &[String],
        eleve: Option<Uuid>,
    ) -> Result<VueAgenda, EntDbError> {
        if au < du {
            return Err(invalide("la fin précède le début"));
        }
        if (au - du).num_days() >= PLAGE_MAX_JOURS {
            return Err(invalide("plage trop large : 42 jours au plus"));
        }
        if sources.iter().any(|s| !SOURCES.contains(&s.as_str())) {
            return Err(invalide("source inconnue"));
        }
        let voulue = |s: &str| sources.is_empty() || sources.iter().any(|x| x == s);
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let mut elements: Vec<ElementAgenda> = Vec::new();
        let mut absentes: Vec<SourceIndisponible> = Vec::new();

        if voulue("evenements") {
            match self.source_evenements(etab, user, du, au, eleve).await {
                Ok(mut v) => elements.append(&mut v),
                Err(_) => absentes.push(SourceIndisponible {
                    source: "evenements".into(),
                    raison: "lecture impossible".into(),
                }),
            }
        }
        if voulue("rendez_vous") {
            match self.source_rendez_vous(etab, user, du, au, eleve).await {
                Ok(mut v) => elements.append(&mut v),
                Err(_) => absentes.push(SourceIndisponible {
                    source: "rendez_vous".into(),
                    raison: "lecture impossible".into(),
                }),
            }
        }
        if voulue("edt") || voulue("calendrier") {
            match self
                .source_edt(
                    etab,
                    user,
                    role,
                    parsed,
                    du,
                    au,
                    eleve,
                    voulue("edt"),
                    voulue("calendrier"),
                )
                .await
            {
                Ok(mut v) => elements.append(&mut v),
                Err(EntDbError::Forbidden) => {
                    for s in ["edt", "calendrier"] {
                        if voulue(s) {
                            absentes.push(SourceIndisponible {
                                source: s.into(),
                                raison: "droits insuffisants pour cette source".into(),
                            });
                        }
                    }
                }
                Err(_) => {
                    for s in ["edt", "calendrier"] {
                        if voulue(s) {
                            absentes.push(SourceIndisponible {
                                source: s.into(),
                                raison: "lecture impossible".into(),
                            });
                        }
                    }
                }
            }
        }
        if voulue("sorties") {
            // Port du socle 42 : l'adaptateur n'existe pas encore (R-AG1 : on le dit).
            absentes.push(SourceIndisponible {
                source: "sorties".into(),
                raison: "pas encore branchée à l'agenda".into(),
            });
        }
        elements.sort_by(|a, b| a.debut.cmp(&b.debut).then_with(|| a.titre.cmp(&b.titre)));
        Ok(VueAgenda {
            du,
            au,
            elements,
            sources_indisponibles: absentes,
        })
    }

    /// Bornes UTC d'une plage de jours (fuseau de l'établissement).
    async fn bornes(
        &self,
        etab: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<(DateTime<Utc>, DateTime<Utc>), EntDbError> {
        let mut c = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (a, b, _) = instants(&mut c, etab, du, None, au, None).await?;
        Ok((a, b))
    }

    async fn source_evenements(
        &self,
        etab: Uuid,
        user: Uuid,
        du: NaiveDate,
        au: NaiveDate,
        eleve: Option<Uuid>,
    ) -> Result<Vec<ElementAgenda>, EntDbError> {
        let (debut, fin) = self.bornes(etab, du, au).await?;
        type Ligne = (
            Uuid,
            String,
            DateTime<Utc>,
            DateTime<Utc>,
            bool,
            Option<String>,
            Option<String>,
            String,
            String,
            Option<String>,
            Option<Uuid>,
            NaiveDate,
            NaiveDate,
            Option<NaiveTime>,
            Option<NaiveTime>,
        );
        let tz = tz_sql("$1");
        let rows: Vec<Ligne> = sqlx::query_as(&format!(
            "SELECT e.id, e.titre, e.debut, e.fin, e.journee_entiere,
                    COALESCE(s.libelle, e.lieu_texte), e.type_code, e.statut,
                    CASE WHEN e.organisateur_user_id = $3 THEN 'organisateur'
                         WHEN p.user_id IS NOT NULL THEN p.role ELSE 'lecteur' END,
                    p.reponse, p.eleve_contexte_id, e.jour_debut, e.jour_fin,
                    CASE WHEN e.journee_entiere THEN NULL ELSE (e.debut AT TIME ZONE {tz})::time END,
                    CASE WHEN e.journee_entiere THEN NULL ELSE (e.fin AT TIME ZONE {tz})::time END
               FROM ent_agenda_evenement e
               LEFT JOIN ent_agenda_participant p
                 ON p.etablissement_id = e.etablissement_id AND p.evenement_id = e.id AND p.user_id = $3
               LEFT JOIN ent_salle s ON s.etablissement_id = e.etablissement_id AND s.id = e.salle_id
              WHERE e.etablissement_id = $1 AND e.debut < $5 AND e.fin > $4
                AND ((e.organisateur_user_id = $3)
                     OR (e.statut <> 'brouillon' AND p.user_id IS NOT NULL)
                     OR (e.statut <> 'brouillon' AND e.confidentialite = 'etablissement'))
                AND ($2::uuid IS NULL OR p.eleve_contexte_id = $2)
              ORDER BY e.debut LIMIT 1000"
        ))
        .bind(etab)
        .bind(eleve)
        .bind(user)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    titre,
                    d,
                    f,
                    entiere,
                    lieu,
                    nature,
                    statut,
                    role,
                    reponse,
                    eleve_id,
                    jour,
                    jour_fin,
                    hd,
                    hf,
                )| ElementAgenda {
                    source: "evenements".into(),
                    id: Some(id),
                    titre,
                    debut: d,
                    fin: f,
                    jour,
                    jour_fin,
                    heure_debut: hd,
                    heure_fin: hf,
                    journee_entiere: entiere,
                    lieu,
                    statut,
                    nature: nature.unwrap_or_default(),
                    role: Some(role),
                    reponse,
                    eleve_id,
                },
            )
            .collect())
    }

    /// Rendez-vous des rencontres parents-professeurs (192) : ceux du lecteur seulement. L'enseignant
    /// voit l'enfant ; une famille voit l'enseignant ; personne ne voit le rendez-vous d'un autre.
    async fn source_rendez_vous(
        &self,
        etab: Uuid,
        user: Uuid,
        du: NaiveDate,
        au: NaiveDate,
        eleve: Option<Uuid>,
    ) -> Result<Vec<ElementAgenda>, EntDbError> {
        let (debut, fin) = self.bornes(etab, du, au).await?;
        type Ligne = (
            Uuid,
            DateTime<Utc>,
            DateTime<Utc>,
            Uuid,
            Uuid,
            Option<String>,
            Option<String>,
            Option<Uuid>,
            NaiveDate,
            NaiveTime,
            NaiveTime,
        );
        let tz = tz_sql("$1");
        let rows: Vec<Ligne> = sqlx::query_as(&format!(
            "SELECT k.id, k.debut, k.fin, k.enseignant_user_id, COALESCE(k.reserve_par_user_id, k.enseignant_user_id),
                    u.display_name, e.prenom, k.eleve_id,
                    (k.debut AT TIME ZONE {tz})::date, (k.debut AT TIME ZONE {tz})::time,
                    (k.fin AT TIME ZONE {tz})::time
               FROM ent_creneau_rdv k
               JOIN ent_user u ON u.id = k.enseignant_user_id
               LEFT JOIN ent_eleve e ON e.etablissement_id = k.etablissement_id AND e.id = k.eleve_id
              WHERE k.etablissement_id = $1 AND k.statut = 'reserve'
                AND k.debut < $4 AND k.fin > $3
                AND (k.enseignant_user_id = $2 OR k.reserve_par_user_id = $2)
                AND ($5::uuid IS NULL OR k.eleve_id = $5)
              ORDER BY k.debut LIMIT 500"
        ))
        .bind(etab)
        .bind(user)
        .bind(debut)
        .bind(fin)
        .bind(eleve)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(
                |(id, d, f, prof, _, prof_nom, prenom, eleve_id, jour, hd, hf)| {
                    let en_tant_que_prof = prof == user;
                    ElementAgenda {
                        source: "rendez_vous".into(),
                        id: Some(id),
                        titre: if en_tant_que_prof {
                            format!("Rendez-vous parent — {}", prenom.unwrap_or_default())
                        } else {
                            format!(
                                "Rendez-vous avec {}",
                                prof_nom.unwrap_or_else(|| "l'enseignant".into())
                            )
                        },
                        debut: d,
                        fin: f,
                        jour,
                        jour_fin: jour,
                        heure_debut: Some(hd),
                        heure_fin: Some(hf),
                        journee_entiere: false,
                        lieu: None,
                        statut: "confirme".into(),
                        nature: "rencontre".into(),
                        role: Some(if en_tant_que_prof {
                            "organisateur".into()
                        } else {
                            "famille".into()
                        }),
                        reponse: None,
                        // Un enseignant ne reçoit pas d'identifiant d'enfant dans sa vue personnelle.
                        eleve_id: if en_tant_que_prof { None } else { eleve_id },
                    }
                },
            )
            .collect())
    }

    #[allow(clippy::too_many_arguments)]
    async fn source_edt(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        parsed: Role,
        du: NaiveDate,
        au: NaiveDate,
        eleve: Option<Uuid>,
        avec_edt: bool,
        avec_calendrier: bool,
    ) -> Result<Vec<ElementAgenda>, EntDbError> {
        let acteur = Acteur {
            etablissement_id: etab,
            user_id: user,
            role,
        };
        // Cibles : le personnel lit son propre emploi du temps ; un élève le sien ; un responsable
        // celui de l'enfant demandé (le contrôle du lien est fait par `edt_semaine`).
        let mut cibles: Vec<(CibleEdt, Option<Uuid>)> = Vec::new();
        match (parsed, eleve) {
            (Role::Parent, Some(e)) => cibles.push((CibleEdt::Eleve(e), Some(e))),
            (Role::Parent, None) => {
                let rows: Vec<(Uuid,)> = sqlx::query_as(
                    "SELECT DISTINCT eleve_id FROM ent_eleve_lien_sollicitable
                      WHERE etablissement_id = $1 AND user_id = $2 AND autorite_parentale",
                )
                .bind(etab)
                .bind(user)
                .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
                .await
                .map_err(db)?;
                cibles.extend(rows.into_iter().map(|(e,)| (CibleEdt::Eleve(e), Some(e))));
            }
            (Role::Eleve, _) => {
                if let Some(e) = self.eleve_id_for_compte(etab, user).await? {
                    cibles.push((CibleEdt::Eleve(e), None));
                }
            }
            (_, Some(e)) => cibles.push((CibleEdt::Eleve(e), None)),
            (_, None) => cibles.push((CibleEdt::Personnel(user), None)),
        }
        let mut out = Vec::new();
        let mut lundi = lundi_de(du);
        let tz = tz_sql("$1");
        let mut deja_ferme: BTreeSet<NaiveDate> = BTreeSet::new();
        while lundi <= au {
            for (cible, eleve_id) in &cibles {
                let sem = self.edt_semaine(&acteur, cible.clone(), lundi).await?;
                if avec_edt {
                    for s in sem.seances.iter().filter(|s| s.jour >= du && s.jour <= au) {
                        if matches!(s.statut.as_str(), "annulee" | "deplacee") {
                            continue;
                        }
                        let (d, f): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(&format!(
                            "SELECT ($2::date + $3::time) AT TIME ZONE {tz}, ($2::date + $4::time) AT TIME ZONE {tz}"
                        ))
                        .bind(etab)
                        .bind(s.jour)
                        .bind(s.heure_debut)
                        .bind(s.heure_fin)
                        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
                        .await
                        .map_err(db)?;
                        out.push(ElementAgenda {
                            source: "edt".into(),
                            id: Some(s.creneau_id),
                            titre: format!("{} — {}", s.matiere_libelle, s.classe),
                            debut: d,
                            fin: f,
                            jour: s.jour,
                            jour_fin: s.jour,
                            heure_debut: Some(s.heure_debut),
                            heure_fin: Some(s.heure_fin),
                            journee_entiere: false,
                            lieu: s.salle.clone(),
                            statut: s.statut.clone(),
                            nature: "cours".into(),
                            role: Some("lecteur".into()),
                            reponse: None,
                            eleve_id: *eleve_id,
                        });
                    }
                }
                if avec_calendrier {
                    for j in sem
                        .jours_fermes
                        .iter()
                        .filter(|j| j.jour >= du && j.jour <= au)
                    {
                        if !deja_ferme.insert(j.jour) {
                            continue;
                        }
                        let (d, f, _) = instants(
                            &mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?,
                            etab,
                            j.jour,
                            None,
                            j.jour,
                            None,
                        )
                        .await?;
                        out.push(ElementAgenda {
                            source: "calendrier".into(),
                            id: None,
                            titre: j.libelle.clone(),
                            debut: d,
                            fin: f,
                            jour: j.jour,
                            jour_fin: j.jour,
                            heure_debut: None,
                            heure_fin: None,
                            journee_entiere: true,
                            lieu: None,
                            statut: "ferme".into(),
                            nature: "fermeture".into(),
                            role: Some("lecteur".into()),
                            reponse: None,
                            eleve_id: None,
                        });
                    }
                }
            }
            lundi += Duration::days(7);
        }
        Ok(out)
    }

    /// Agenda d'une classe (AGD-104), **non nominatif** : séances de la classe et événements publiés
    /// qui la visent, sauf ceux en confidentialité restreinte. Droit `agenda.lire_structure`.
    pub async fn agenda_classe(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        classe_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<VueAgenda, EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::AgendaLireStructure)
            .await?;
        if au < du || (au - du).num_days() >= PLAGE_MAX_JOURS {
            return Err(invalide("plage invalide : 42 jours au plus"));
        }
        let (debut, fin) = self.bornes(etab, du, au).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(classe_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if !ok {
            return Err(introuvable());
        }
        let tz = tz_sql("$1");
        let rows: Vec<(Uuid, String, DateTime<Utc>, DateTime<Utc>, bool, Option<String>, String, NaiveDate, NaiveDate, Option<NaiveTime>, Option<NaiveTime>)> = sqlx::query_as(&format!(
            "SELECT e.id, e.titre, e.debut, e.fin, e.journee_entiere, e.type_code, e.statut,
                    e.jour_debut, e.jour_fin,
                    CASE WHEN e.journee_entiere THEN NULL ELSE (e.debut AT TIME ZONE {tz})::time END,
                    CASE WHEN e.journee_entiere THEN NULL ELSE (e.fin AT TIME ZONE {tz})::time END
               FROM ent_agenda_evenement e
              WHERE e.etablissement_id = $1 AND e.statut IN ('publie', 'annule') AND e.confidentialite <> 'restreint'
                AND e.debut < $4 AND e.fin > $3
                AND EXISTS (SELECT 1 FROM ent_agenda_audience a
                             WHERE a.etablissement_id = e.etablissement_id AND a.evenement_id = e.id
                               AND ((a.kind = 'classe' AND a.cible_id = $2) OR a.kind = 'etablissement'
                                    OR (a.kind = 'niveau' AND a.cible_id IN
                                        (SELECT c.niveau_id FROM ent_classe c WHERE c.etablissement_id = $1 AND c.id = $2))))
              ORDER BY e.debut LIMIT 500"
        ))
        .bind(etab)
        .bind(classe_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        drop(tx);
        let mut elements: Vec<ElementAgenda> = rows
            .into_iter()
            .map(
                |(id, titre, d, f, entiere, nature, statut, jour, jour_fin, hd, hf)| {
                    ElementAgenda {
                        source: "evenements".into(),
                        id: Some(id),
                        titre,
                        debut: d,
                        fin: f,
                        jour,
                        jour_fin,
                        heure_debut: hd,
                        heure_fin: hf,
                        journee_entiere: entiere,
                        lieu: None,
                        statut,
                        nature: nature.unwrap_or_default(),
                        role: Some("lecteur".into()),
                        reponse: None,
                        eleve_id: None,
                    }
                },
            )
            .collect();
        let acteur = Acteur {
            etablissement_id: etab,
            user_id: user,
            role,
        };
        let tz = tz_sql("$1");
        let mut absentes = Vec::new();
        let mut lundi = lundi_de(du);
        while lundi <= au {
            match self
                .edt_semaine(&acteur, CibleEdt::Classe(classe_id), lundi)
                .await
            {
                Ok(sem) => {
                    for s in sem.seances.iter().filter(|s| {
                        s.jour >= du
                            && s.jour <= au
                            && s.statut != "annulee"
                            && s.statut != "deplacee"
                    }) {
                        let (d, f): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(&format!(
                            "SELECT ($2::date + $3::time) AT TIME ZONE {tz}, ($2::date + $4::time) AT TIME ZONE {tz}"
                        ))
                        .bind(etab)
                        .bind(s.jour)
                        .bind(s.heure_debut)
                        .bind(s.heure_fin)
                        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
                        .await
                        .map_err(db)?;
                        elements.push(ElementAgenda {
                            source: "edt".into(),
                            id: Some(s.creneau_id),
                            titre: s.matiere_libelle.clone(),
                            debut: d,
                            fin: f,
                            jour: s.jour,
                            jour_fin: s.jour,
                            heure_debut: Some(s.heure_debut),
                            heure_fin: Some(s.heure_fin),
                            journee_entiere: false,
                            lieu: s.salle.clone(),
                            statut: s.statut.clone(),
                            nature: "cours".into(),
                            role: Some("lecteur".into()),
                            reponse: None,
                            eleve_id: None,
                        });
                    }
                }
                Err(_) => {
                    absentes.push(SourceIndisponible {
                        source: "edt".into(),
                        raison: "lecture impossible".into(),
                    });
                    break;
                }
            }
            lundi += Duration::days(7);
        }
        elements.sort_by(|a, b| a.debut.cmp(&b.debut).then_with(|| a.titre.cmp(&b.titre)));
        Ok(VueAgenda {
            du,
            au,
            elements,
            sources_indisponibles: absentes,
        })
    }

    // ───── iCal ponctuel minimisé (AGD-109, R-AG25) ───────────────────────────────────────────

    /// Fichier iCal d'**un** événement visible du lecteur : titre (ou mention générique pour un événement
    /// restreint), plage, lieu. Jamais de participants, ni de nom d'élève, ni de description.
    pub async fn agenda_ics(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<String, EntDbError> {
        let d = self.agenda_evenement(etab, user, role, id).await?;
        Ok(construire_ics(&d))
    }
}

fn echapper_ics(t: &str) -> String {
    t.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace(['\r', '\n'], " ")
}

/// Fichier iCal (RFC 5545) d'un événement, **minimisé** : pas de participants, pas de description.
pub fn construire_ics(d: &EvenementDetail) -> String {
    let titre = if d.confidentialite == "restreint" && !d.je_suis_organisateur {
        "Événement de l'établissement".to_string()
    } else {
        d.titre.clone()
    };
    let fmt = |t: DateTime<Utc>| t.format("%Y%m%dT%H%M%SZ").to_string();
    let mut l: Vec<String> = vec![
        "BEGIN:VCALENDAR".into(),
        "VERSION:2.0".into(),
        "PRODID:-//ScolIA//Agenda//FR".into(),
        "CALSCALE:GREGORIAN".into(),
        "METHOD:PUBLISH".into(),
        "BEGIN:VEVENT".into(),
        format!("UID:{}@scolia", d.id),
        format!("DTSTAMP:{}", fmt(Utc::now())),
        format!("DTSTART:{}", fmt(d.debut)),
        format!("DTEND:{}", fmt(d.fin)),
        format!("SUMMARY:{}", echapper_ics(&titre)),
    ];
    if let Some(lieu) = &d.lieu {
        l.push(format!("LOCATION:{}", echapper_ics(lieu)));
    }
    l.push(format!(
        "STATUS:{}",
        if d.statut == "annule" {
            "CANCELLED"
        } else {
            "CONFIRMED"
        }
    ));
    l.push("END:VEVENT".into());
    l.push("END:VCALENDAR".into());
    // RFC 5545 : lignes de 75 octets au plus, repliées par CRLF + espace.
    let mut out = String::new();
    for ligne in l {
        let mut courant = String::new();
        let mut octets = 0usize;
        for c in ligne.chars() {
            let n = c.len_utf8();
            if octets + n > 73 {
                out.push_str(&courant);
                out.push_str("\r\n ");
                courant.clear();
                octets = 0;
            }
            courant.push(c);
            octets += n;
        }
        out.push_str(&courant);
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detail(confidentialite: &str, organisateur: bool) -> EvenementDetail {
        EvenementDetail {
            id: Uuid::nil(),
            type_code: "reunion".into(),
            titre: "Conseil; de, classe 4B".into(),
            description: Some("Notes secrètes".into()),
            debut: Utc::now(),
            fin: Utc::now() + Duration::hours(1),
            jour: Utc::now().date_naive(),
            jour_fin: Utc::now().date_naive(),
            heure_debut: None,
            heure_fin: None,
            journee_entiere: false,
            lieu: Some("Salle 12".into()),
            confidentialite: confidentialite.into(),
            statut: "publie".into(),
            motif_annulation: None,
            serie_id: None,
            occurrence_detachee: false,
            version: 1,
            organisateur_user_id: Uuid::nil(),
            organisateur_nom: Some("Mme Martin".into()),
            je_suis_organisateur: organisateur,
            ma_reponse: None,
            participants: Some(vec![]),
            nb_oui: 0,
            nb_non: 0,
            nb_peut_etre: 0,
            nb_en_attente: 0,
            coupon: None,
        }
    }

    #[test]
    fn ics_minimise_sans_description_ni_participants_et_echappe() {
        let ics = construire_ics(&detail("participants", false));
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\n") && ics.ends_with("END:VCALENDAR\r\n"));
        assert!(ics.contains("SUMMARY:Conseil\\; de\\, classe 4B"), "{ics}");
        assert!(ics.contains("LOCATION:Salle 12"));
        for interdit in [
            "Notes secr",
            "DESCRIPTION",
            "ATTENDEE",
            "ORGANIZER",
            "Martin",
        ] {
            assert!(
                !ics.contains(interdit),
                "{interdit} ne doit pas figurer dans l'iCal"
            );
        }
    }

    #[test]
    fn ics_d_un_evenement_restreint_masque_le_titre_hors_organisateur() {
        let ics = construire_ics(&detail("restreint", false));
        assert!(ics.contains("SUMMARY:Événement de l'établissement") && !ics.contains("Conseil"));
        assert!(construire_ics(&detail("restreint", true)).contains("Conseil"));
    }

    #[test]
    fn ics_replie_les_longues_lignes_et_marque_l_annulation() {
        let mut d = detail("participants", true);
        d.titre = "é".repeat(120);
        d.statut = "annule".into();
        let ics = construire_ics(&d);
        assert!(ics.contains("STATUS:CANCELLED"));
        for ligne in ics.split("\r\n") {
            assert!(
                ligne.len() <= 75,
                "ligne trop longue : {} octets",
                ligne.len()
            );
        }
    }
}
