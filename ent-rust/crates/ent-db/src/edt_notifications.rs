#![allow(clippy::too_many_arguments, clippy::type_complexity)]
//! Socle 10 (emploi du temps), lot T4 : notifier les changements du réel par le socle 8 (outbox, `ent_notification`
//! et `ent_notification_envoi`) — `docs/socles/10_EMPLOI_DU_TEMPS.md` §9.5, §14.2, §14.4, §18.1.
//!
//! **Une seule voie.** Les événements du §18.1 (`edt.seance_modifiee`, `edt.seance_annulee`, `edt.seance_retablie`,
//! `edt.seance_ajoutee`, `edt.lot_modifications`) sont publiés dans la transaction du fait (T3 et le pont des écrivains
//! historiques) ; ce consommateur les transforme en notifications **par le socle 8** (`notifier_personne` : message dans
//! l'ENT, puis envois selon les préférences). Il ne crée aucune deuxième voie.
//!
//! **Alias des anciens noms.** `edt.exception`, `edt.exception_levee`, `edt.remplacement_organise`,
//! `edt.annulation_masse` (§18.1, dernier alinéa) restent émis le temps de la bascule des consommateurs ; l'ancien
//! consommateur `d4.notifications_edt` ignore désormais ceux qui portent un `modification_id` (le nouveau les a déjà
//! traités) : un changement ne produit jamais deux messages.
//!
//! **Qui est prévenu : le public à la date.** Les élèves de la séance **le jour de la séance** (`ent_public_cours_jour` :
//! inclusions, exclusions, sorties comprises), leurs responsables au lien actif **si l'élève est mineur** et sans
//! interdiction de communication, les intervenants (titulaire, remplaçant, co-intervenants), jamais le motif interne ; le nom
//! du remplaçant n'est pas donné (Q-EDT8 non tranchée). Un seul message par personne et par changement ; un lot, un seul
//! message par personne.
//!
//! **Urgence selon le délai (§9.5).** Une séance d'aujourd'hui, ou de demain avant 10 h : message immédiat (U2) ; une séance
//! plus lointaine : ordinaire, regroupé dans le résumé de la personne. Pour le lendemain matin décidé avant l'heure du
//! message de la veille (`edt.heure_message_veille`, défaut 18 h 30), l'envoi est **différé à cette heure** et **rappelé le
//! matin** (6 h 30). Les familles ne sont prévenues tout de suite que d'un changement qui modifie l'arrivée, le départ ou la
//! journée de leur enfant (R-E36) ; le reste entre dans le résumé du jour. L'élève est prévenu de tout.
//! **Rétablissement** : la levée d'une modification déjà annoncée envoie un message à ceux qui avaient été prévenus, avec
//! la même urgence ; si l'annonce n'est jamais partie, rien ne part.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::edt_modif::{
    EVT_LOT_MODIFICATIONS, EVT_SEANCE_AJOUTEE, EVT_SEANCE_ANNULEE, EVT_SEANCE_MODIFIEE,
    EVT_SEANCE_RETABLIE,
};
use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};

pub const CONSOMMATEUR_EDT_V2: &str = "d4.notifications_edt_t4";

pub struct ConsommateurNotificationsEdtT4;

const TYPES_T4: &[&str] = &[
    EVT_SEANCE_MODIFIEE,
    EVT_SEANCE_ANNULEE,
    EVT_SEANCE_RETABLIE,
    EVT_SEANCE_AJOUTEE,
    EVT_LOT_MODIFICATIONS,
];

/// Anciens noms d'événements conservés en alias (§18.1) et ce qu'ils deviennent. Une table documentaire : le
/// consommateur historique les traite encore quand ils ne portent pas de `modification_id`.
pub const ALIAS_ANCIENS_NOMS: &[(&str, &str)] = &[
    ("edt.exception", "edt.seance_annulee | edt.seance_modifiee"),
    ("edt.exception_levee", "edt.seance_retablie"),
    ("edt.remplacement_organise", "edt.seance_modifiee"),
    ("edt.annulation_masse", "edt.lot_modifications"),
    ("edt.seances_generees", "edt.seances_materialisees"),
    ("edt.creneau_modifie", "edt.seance_modifiee"),
];

/// Heure du message de la veille par défaut (paramètre `edt.heure_message_veille`).
pub const HEURE_VEILLE_DEFAUT: (u32, u32) = (18, 30);
/// Rappel du matin pour un changement annoncé la veille.
pub const HEURE_RAPPEL_MATIN: (u32, u32) = (6, 30);
/// Une séance qui commence avant cette heure le lendemain est urgente (U2).
pub const LIMITE_LENDEMAIN: (u32, u32) = (10, 0);

fn heure(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).expect("heure valide")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgence {
    /// U2 : la séance a lieu aujourd'hui ou demain avant 10 h.
    Immediate,
    /// C0 : plus lointain ; regroupé dans le résumé de la personne.
    Ordinaire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dest {
    Intervenant,
    Eleve,
    Famille,
}

/// Ce que le serveur décide pour un destinataire : urgence, instant local d'envoi (`None` : tout de suite ou selon
/// ses préférences), rappel du matin, regroupement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanEnvoi {
    pub urgence: Urgence,
    pub envoi_local: Option<NaiveDateTime>,
    pub rappel_local: Option<NaiveDateTime>,
    pub regroupable: bool,
    /// L'envoi ignore les heures calmes de la personne (urgence) : il part à `envoi_local` ou maintenant.
    pub ignore_heures_calmes: bool,
}

/// Plan d'envoi d'un changement (§9.5, §14.4 point 1). `maintenant` et `debut` sont en heure murale de l'établissement.
pub fn planifier(
    maintenant: NaiveDateTime,
    debut: NaiveDateTime,
    heure_veille: NaiveTime,
    dest: Dest,
    affecte_la_journee: bool,
) -> PlanEnvoi {
    let aujourdhui = maintenant.date();
    let demain = aujourdhui + Duration::days(1);
    let urgent = debut <= maintenant
        || debut.date() == aujourdhui
        || (debut.date() == demain && debut.time() < heure(LIMITE_LENDEMAIN.0, LIMITE_LENDEMAIN.1));
    // Une famille n'est prévenue tout de suite que d'un changement qui change la journée de son enfant (R-E36).
    let urgent = urgent && (dest != Dest::Famille || affecte_la_journee);
    if !urgent {
        return PlanEnvoi {
            urgence: Urgence::Ordinaire,
            envoi_local: None,
            rappel_local: None,
            regroupable: true,
            ignore_heures_calmes: false,
        };
    }
    // Lendemain matin décidé avant l'heure du soir : message de la veille à heure fixe, rappelé le matin.
    if debut.date() == demain && maintenant.time() < heure_veille {
        return PlanEnvoi {
            urgence: Urgence::Immediate,
            envoi_local: Some(aujourdhui.and_time(heure_veille)),
            rappel_local: Some(demain.and_time(heure(HEURE_RAPPEL_MATIN.0, HEURE_RAPPEL_MATIN.1))),
            regroupable: false,
            ignore_heures_calmes: true,
        };
    }
    PlanEnvoi {
        urgence: Urgence::Immediate,
        envoi_local: None,
        rappel_local: None,
        regroupable: false,
        ignore_heures_calmes: true,
    }
}

fn motif_libelle(code: &str, texte: Option<&str>) -> String {
    let base = match code {
        "absence_professeur" => "absence du professeur",
        "sortie" => "sortie",
        "examen" => "examen",
        "reunion" => "réunion",
        "travaux" => "travaux",
        "intemperies" => "intempéries",
        "mouvement_social" => "mouvement social",
        _ => "",
    };
    match (
        base.is_empty(),
        texte.map(str::trim).filter(|t| !t.is_empty()),
    ) {
        (false, Some(t)) if t.to_lowercase() != base => format!("{base} : {t}"),
        (false, _) => base.to_string(),
        (true, Some(t)) => t.to_string(),
        (true, None) => String::new(),
    }
}

fn jour_fr(j: NaiveDate) -> String {
    j.format("%d/%m/%Y").to_string()
}

/// Texte d'un changement : jamais le motif interne, jamais le nom du remplaçant.
fn message(
    type_code: &str,
    matiere: &str,
    jour: NaiveDate,
    debut: NaiveTime,
    motif: &str,
    apres_jour: Option<NaiveDate>,
    apres_debut: Option<NaiveTime>,
    salle: Option<&str>,
) -> (&'static str, String, String) {
    let quand = format!("du {} à {}", jour_fr(jour), debut.format("%H:%M"));
    let suffixe = if motif.is_empty() {
        String::new()
    } else {
        format!(" ({motif})")
    };
    match type_code {
        "annulation" => (
            "edt_annulation",
            format!("Cours annulé : {matiere}"),
            format!("Le cours de {matiere} {quand} est annulé{suffixe}."),
        ),
        "neutralisation" => (
            "edt_neutralisation",
            format!("Cours remplacé : {matiere}"),
            format!("Le cours de {matiere} {quand} n'a pas lieu : il est remplacé par un événement de l'établissement{suffixe}."),
        ),
        "deplacement" => (
            "edt_deplacement",
            format!("Cours déplacé : {matiere}"),
            match (apres_jour, apres_debut) {
                (Some(j), Some(d)) => {
                    format!("Le cours de {matiere} {quand} est déplacé au {} à {}{suffixe}.", jour_fr(j), d.format("%H:%M"))
                }
                (Some(j), None) => format!("Le cours de {matiere} {quand} est déplacé au {}{suffixe}.", jour_fr(j)),
                _ => format!("Le cours de {matiere} {quand} est déplacé{suffixe}."),
            },
        ),
        "changement_lieu" => (
            "edt_salle",
            format!("Changement de salle : {matiere}"),
            match salle {
                Some(s) => format!("Le cours de {matiere} {quand} aura lieu en salle {s}{suffixe}."),
                None => format!("Le cours de {matiere} {quand} change de salle{suffixe}."),
            },
        ),
        "changement_horaire" => (
            "edt_horaire",
            format!("Changement d'horaire : {matiere}"),
            match apres_debut {
                Some(d) => format!("Le cours de {matiere} {quand} commencera à {}{suffixe}.", d.format("%H:%M")),
                None => format!("Le cours de {matiere} {quand} change d'horaire{suffixe}."),
            },
        ),
        "intervenant" => (
            "edt_remplacement",
            format!("Cours assuré par un autre enseignant : {matiere}"),
            format!("Le cours de {matiere} {quand} sera assuré par un autre enseignant."),
        ),
        "ajout" => (
            "edt_ajout",
            format!("Cours supplémentaire : {matiere}"),
            format!("Un cours de {matiere} est ajouté {quand}{suffixe}."),
        ),
        "fusion" => (
            "edt_fusion",
            format!("Cours regroupé : {matiere}"),
            format!("Le cours de {matiere} {quand} est regroupé avec un autre cours{suffixe}."),
        ),
        _ => (
            "edt_modification",
            format!("Changement d'emploi du temps : {matiere}"),
            format!("Le cours de {matiere} {quand} change{suffixe}."),
        ),
    }
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
struct ModifEvt {
    id: Uuid,
    seance_id: Option<Uuid>,
    creneau_id: Option<Uuid>,
    seance_arrivee_id: Option<Uuid>,
    jour: NaiveDate,
    type_code: String,
    avant_etat: Option<String>,
    apres_jour: Option<NaiveDate>,
    apres_debut: Option<NaiveTime>,
    apres_salle_texte: Option<String>,
    apres_remplacant_user_id: Option<Uuid>,
    motif_public_code: String,
    motif_public: Option<String>,
    passe: bool,
    levee: bool,
}

#[derive(Debug, Clone, FromRow)]
struct SeanceEvt {
    id: Uuid,
    classe: String,
    cours_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
    jour: NaiveDate,
    heure_debut: NaiveTime,
    heure_fin: NaiveTime,
    matiere: String,
    prof_user_id: Uuid,
    remplacant_user_id: Option<Uuid>,
    debut_local: NaiveDateTime,
}

/// Un destinataire résolu.
#[derive(Debug, Clone)]
struct Destinataire {
    user_id: Uuid,
    eleve_id: Option<Uuid>,
    dest: Dest,
    affecte_la_journee: bool,
}

fn err(e: sqlx::Error) -> String {
    e.to_string()
}

/// Maintenant et début de séance en heure murale de l'établissement, et son fuseau.
async fn horloge(
    conn: &mut PgConnection,
    etab: Uuid,
    maintenant: DateTime<Utc>,
) -> Result<(NaiveDateTime, NaiveTime), String> {
    let (local, veille): (NaiveDateTime, Option<NaiveTime>) = sqlx::query_as(
        "SELECT ($2::timestamptz AT TIME ZONE coalesce((SELECT fuseau FROM ent_etablissement_parametre
                                                         WHERE etablissement_id = $1), 'Europe/Paris')),
                (SELECT valeur_heure FROM ent_parametre
                  WHERE etablissement_id = $1 AND cle = 'edt.heure_message_veille')",
    )
    .bind(etab)
    .bind(maintenant)
    .fetch_one(&mut *conn)
    .await
    .map_err(err)?;
    let veille = veille.unwrap_or_else(|| heure(HEURE_VEILLE_DEFAUT.0, HEURE_VEILLE_DEFAUT.1));
    Ok((local, veille))
}

/// Ce qui est modifié, tel que le notifieur le lit : la séance si elle est matérialisée, sinon le créneau du plan à la
/// date (une exception historique peut porter sur un jour dont la séance n'a pas encore été générée).
const SELECT_CIBLE: &str = "SELECT coalesce(s.id, '00000000-0000-0000-0000-000000000000'::uuid) AS id,
        coalesce(s.classe, btrim(c.classe), '') AS classe, coalesce(s.cours_id, c.cours_id) AS cours_id,
        coalesce(s.classe_id, (SELECT cl.id FROM ent_classe cl
                                 JOIN ent_annee_scolaire an ON an.id = cl.annee_id AND an.etablissement_id = cl.etablissement_id
                                WHERE cl.etablissement_id = m.etablissement_id AND cl.code = btrim(c.classe)
                                  AND an.date_debut <= m.jour AND an.date_fin >= m.jour LIMIT 1)) AS classe_id,
        c.groupe_id, m.jour AS jour, coalesce(s.heure_debut, c.heure_debut) AS heure_debut,
        coalesce(s.heure_fin, c.heure_fin) AS heure_fin,
        coalesce(s.matiere_libelle, c.matiere_libelle, '') AS matiere,
        coalesce(s.prof_user_id, c.prof_user_id) AS prof_user_id, s.remplacant_user_id,
        (m.jour + coalesce(s.heure_debut, c.heure_debut)) AS debut_local
   FROM ent_edt_modification m
   LEFT JOIN ent_edt_seance s ON s.id = m.seance_id AND s.etablissement_id = m.etablissement_id
   LEFT JOIN ent_edt_creneau c ON c.id = coalesce(s.creneau_id, m.creneau_id) AND c.etablissement_id = m.etablissement_id";

async fn charger_cible(
    conn: &mut PgConnection,
    etab: Uuid,
    modification: Uuid,
) -> Result<Option<SeanceEvt>, String> {
    sqlx::query_as(&format!(
        "{SELECT_CIBLE} WHERE m.etablissement_id = $1 AND m.id = $2"
    ))
    .bind(etab)
    .bind(modification)
    .fetch_optional(conn)
    .await
    .map_err(err)
}

/// Destinataires d'un changement de la séance : le public **à la date** de chacun des jours concernés.
async fn destinataires(
    conn: &mut PgConnection,
    etab: Uuid,
    s: &SeanceEvt,
    jours: &[NaiveDate],
    intervenants_en_plus: &[Uuid],
) -> Result<Vec<Destinataire>, String> {
    let mut out: BTreeMap<Uuid, Destinataire> = BTreeMap::new();
    let mut ajouter = |d: Destinataire| {
        // Priorité : intervenant > élève > famille ; un seul message par personne.
        let rang = |x: Dest| match x {
            Dest::Intervenant => 0,
            Dest::Eleve => 1,
            Dest::Famille => 2,
        };
        match out.get(&d.user_id) {
            Some(existant) if rang(existant.dest) <= rang(d.dest) => {}
            _ => {
                out.insert(d.user_id, d);
            }
        }
    };
    // Intervenants : titulaire, remplaçant, co-intervenants du cours, nouveau remplaçant. Membres seulement.
    let mut users: BTreeSet<Uuid> = [s.prof_user_id].into_iter().collect();
    users.extend(s.remplacant_user_id);
    users.extend(intervenants_en_plus.iter().copied());
    if let Some(cours) = s.cours_id {
        let co: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT prof_user_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id = $2",
        )
        .bind(etab)
        .bind(cours)
        .fetch_all(&mut *conn)
        .await
        .map_err(err)?;
        users.extend(co);
    }
    let users: Vec<Uuid> = users.into_iter().collect();
    let membres: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT user_id FROM ent_membership WHERE etablissement_id = $1 AND user_id = ANY($2)",
    )
    .bind(etab)
    .bind(&users)
    .fetch_all(&mut *conn)
    .await
    .map_err(err)?;
    for u in membres {
        ajouter(Destinataire {
            user_id: u,
            eleve_id: None,
            dest: Dest::Intervenant,
            affecte_la_journee: true,
        });
    }
    // Public à la date (chaque jour concerné) et « le changement modifie-t-il la journée de l'élève ? » : la séance
    // était la première ou la dernière de sa journée, ou il ne lui reste plus de séance ce jour-là.
    let mut eleves: BTreeMap<Uuid, bool> = BTreeMap::new();
    if s.cours_id.is_none() {
        // Séance reprise d'un créneau sans cours (M1 : « listé, jamais deviné ») : le public est celui de la classe
        // (ou du groupe) du créneau à la date, comme avant le socle 5.
        if let Some(classe) = s.classe_id {
            for jour in jours {
                let ids: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT DISTINCT e.id FROM ent_eleve e
                      WHERE e.etablissement_id = $1 AND (e.date_sortie IS NULL OR e.date_sortie > $2)
                        AND (EXISTS (SELECT 1 FROM ent_affectation a
                                      WHERE a.etablissement_id = e.etablissement_id AND a.eleve_id = e.id
                                        AND a.date_debut <= $2 AND (a.date_fin IS NULL OR a.date_fin >= $2)
                                        AND (($4::uuid IS NULL AND a.classe_id = $3) OR a.groupe_id = $4))
                             OR ($4::uuid IS NULL AND e.classe_id = $3))",
                )
                .bind(etab)
                .bind(jour)
                .bind(classe)
                .bind(s.groupe_id)
                .fetch_all(&mut *conn)
                .await
                .map_err(err)?;
                for e in ids {
                    eleves.insert(e, true);
                }
            }
        }
    }
    let cours_pour_public = s.cours_id;
    for jour in jours.iter().filter(|_| cours_pour_public.is_some()) {
        let cours = cours_pour_public.expect("filtré");
        let rows: Vec<(Uuid, bool)> = sqlx::query_as(
            "WITH pub AS (SELECT DISTINCT eleve_id FROM ent_public_cours_jour($1, $2, $3)),
                  reste AS (SELECT p.eleve_id, min(o.heure_debut) AS d, max(o.heure_fin) AS f
                              FROM pub p
                              JOIN ent_edt_seance o ON o.etablissement_id = $1 AND o.jour = $2 AND o.id <> $4
                               AND o.etat IN ('prevue', 'modifiee', 'ajoutee') AND o.cours_id IS NOT NULL
                              JOIN LATERAL ent_public_cours_jour($1, $2, o.cours_id) q ON q.eleve_id = p.eleve_id
                             GROUP BY p.eleve_id)
             SELECT p.eleve_id,
                    (r.eleve_id IS NULL OR $5 < r.d OR $6 > r.f) AS affecte
               FROM pub p LEFT JOIN reste r ON r.eleve_id = p.eleve_id",
        )
        .bind(etab)
        .bind(jour)
        .bind(cours)
        .bind(s.id)
        .bind(s.heure_debut)
        .bind(s.heure_fin)
        .fetch_all(&mut *conn)
        .await
        .map_err(err)?;
        for (e, a) in rows {
            let v = eleves.entry(e).or_insert(false);
            *v = *v || a;
        }
    }
    let ids: Vec<Uuid> = eleves.keys().copied().collect();
    if ids.is_empty() {
        return Ok(out.into_values().collect());
    }
    // Comptes d'élèves : prévenus de tout.
    let comptes: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT c.user_id, c.eleve_id FROM ent_eleve_compte c
           JOIN ent_membership m ON m.user_id = c.user_id AND m.etablissement_id = c.etablissement_id
          WHERE c.etablissement_id = $1 AND c.eleve_id = ANY($2) AND m.role = 'eleve'",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(err)?;
    for (u, e) in comptes {
        ajouter(Destinataire {
            user_id: u,
            eleve_id: Some(e),
            dest: Dest::Eleve,
            affecte_la_journee: true,
        });
    }
    // Responsables au lien actif, si l'élève est mineur et sans interdiction de communication (R-E32, socle 4).
    let jour_ref = jours.first().copied().unwrap_or(s.jour);
    let parents: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT DISTINCT l.user_id, l.eleve_id FROM ent_eleve_lien_actif l
           JOIN ent_membership m ON m.user_id = l.user_id AND m.etablissement_id = l.etablissement_id
           JOIN ent_eleve e ON e.id = l.eleve_id AND e.etablissement_id = l.etablissement_id
          WHERE l.etablissement_id = $1 AND l.eleve_id = ANY($2) AND m.role = 'parent' AND l.user_id IS NOT NULL
            AND (e.date_naissance IS NULL OR e.date_naissance > ($3::date - interval '18 years'))
            AND NOT EXISTS (SELECT 1 FROM ent_eleve_interdiction i
                             WHERE i.etablissement_id = l.etablissement_id AND i.eleve_id = l.eleve_id
                               AND i.personne_id = l.personne_id AND i.levee_le IS NULL
                               AND i.du <= $3 AND (i.au IS NULL OR i.au >= $3) AND 'communication' = ANY(i.objet))",
    )
    .bind(etab)
    .bind(&ids)
    .bind(jour_ref)
    .fetch_all(&mut *conn)
    .await
    .map_err(err)?;
    for (u, e) in parents {
        let a = eleves.get(&e).copied().unwrap_or(false);
        ajouter(Destinataire {
            user_id: u,
            eleve_id: Some(e),
            dest: Dest::Famille,
            affecte_la_journee: a,
        });
    }
    Ok(out.into_values().collect())
}

/// Envoie un message à une personne par le socle 8, puis applique le plan (message de la veille, rappel, urgence).
async fn envoyer(
    conn: &mut PgConnection,
    etab: Uuid,
    evt: &EvenementDistribue,
    d: &Destinataire,
    kind: &str,
    titre: &str,
    corps: &str,
    plan: &PlanEnvoi,
    maintenant: DateTime<Utc>,
    suffixe_dedup: &str,
) -> Result<(), String> {
    let cle = format!("edt:{}{}", evt.event().id, suffixe_dedup);
    notifier_personne(
        &mut *conn,
        EnvoiAPlanifier {
            etablissement_id: etab,
            user_id: d.user_id,
            kind,
            titre,
            corps,
            eleve_id: d.eleve_id,
            notification_id: None,
            source_type: "evenement",
            source_id: evt.event().id,
            cle_dedup: cle.clone(),
            droit_requis: (d.dest == Dest::Famille).then_some("lien"),
            regroupable: plan.regroupable,
            canaux_voulus: &["email", "push"],
        },
        evt.event().id,
        maintenant,
    )
    .await
    .map_err(|e| e.to_string())?;
    // Urgence : l'envoi ignore les heures calmes (il part maintenant) ou à l'heure du message de la veille.
    if plan.ignore_heures_calmes {
        sqlx::query(
            "UPDATE ent_notification_envoi
                SET planifie_pour = CASE WHEN $5::timestamp IS NULL THEN $4::timestamptz
                         ELSE ($5::timestamp AT TIME ZONE coalesce((SELECT fuseau FROM ent_etablissement_parametre
                                                                      WHERE etablissement_id = $1), 'Europe/Paris')) END
              WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND cle_dedup = $3 AND statut = 'en_attente'",
        )
        .bind(etab)
        .bind(d.user_id)
        .bind(&cle)
        .bind(maintenant)
        .bind(plan.envoi_local)
        .execute(&mut *conn)
        .await
        .map_err(err)?;
    }
    Ok(())
}

/// Rappel du matin d'un message de la veille : un second envoi, à l'heure du rappel, dédoublonné.
async fn rappeler(
    conn: &mut PgConnection,
    etab: Uuid,
    evt: &EvenementDistribue,
    d: &Destinataire,
    kind: &str,
    titre: &str,
    corps: &str,
    plan: &PlanEnvoi,
    maintenant: DateTime<Utc>,
) -> Result<(), String> {
    let Some(rappel) = plan.rappel_local else {
        return Ok(());
    };
    let cle = format!("edt:{}:rappel", evt.event().id);
    let titre_rappel = format!("Rappel : {titre}");
    notifier_personne(
        &mut *conn,
        EnvoiAPlanifier {
            etablissement_id: etab,
            user_id: d.user_id,
            kind,
            titre: &titre_rappel,
            corps,
            eleve_id: d.eleve_id,
            notification_id: None,
            source_type: "evenement",
            source_id: evt.event().id,
            cle_dedup: cle.clone(),
            droit_requis: (d.dest == Dest::Famille).then_some("lien"),
            regroupable: false,
            canaux_voulus: &["email", "push"],
        },
        // Le rappel n'ajoute pas de seconde ligne dans l'ENT : il réutilise la clé de l'événement pour l'ENT.
        evt.event().id,
        maintenant,
    )
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "UPDATE ent_notification_envoi
            SET planifie_pour = ($4::timestamp AT TIME ZONE coalesce((SELECT fuseau FROM ent_etablissement_parametre
                                                                       WHERE etablissement_id = $1), 'Europe/Paris'))
          WHERE etablissement_id = $1 AND destinataire_user_id = $2 AND cle_dedup = $3 AND statut = 'en_attente'",
    )
    .bind(etab)
    .bind(d.user_id)
    .bind(&cle)
    .bind(rappel)
    .execute(&mut *conn)
    .await
    .map_err(err)?;
    Ok(())
}

async fn traiter_modification(
    conn: &mut PgConnection,
    evt: &EvenementDistribue,
) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let p = &evt.event().payload;
    let Some(mid) = p
        .get("modification_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    else {
        return Ok(());
    };
    let retablie = evt.event().type_name == EVT_SEANCE_RETABLIE;
    let m: Option<ModifEvt> = sqlx::query_as(
        "SELECT id, seance_id, creneau_id, seance_arrivee_id, jour, type AS type_code, avant_etat, apres_jour, apres_debut,
                apres_salle_texte, apres_remplacant_user_id, motif_public_code, motif_public, passe,
                (levee_le IS NOT NULL) AS levee
           FROM ent_edt_modification WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(mid)
    .fetch_optional(&mut *conn)
    .await
    .map_err(err)?;
    let Some(m) = m else { return Ok(()) };
    // Une correction du passé ne prévient personne (§9.1 : « personne, sauf choix »).
    if m.passe || m.type_code == "correction" {
        return Ok(());
    }
    // L'état courant prime sur la charge utile : une modification levée entre-temps n'est pas annoncée.
    if (!retablie && m.levee) || (retablie && !m.levee) {
        return Ok(());
    }
    let Some(s) = charger_cible(&mut *conn, etab, m.id).await? else {
        return Ok(());
    };
    let maintenant = evt.event().created_at;
    let (local, veille) = horloge(&mut *conn, etab, maintenant).await?;
    // Jours concernés : le jour de la séance, et le jour d'arrivée d'un déplacement.
    let mut jours = vec![s.jour];
    if let Some(j) = m.apres_jour.filter(|j| *j != s.jour) {
        jours.push(j);
    }
    let en_plus: Vec<Uuid> = m.apres_remplacant_user_id.into_iter().collect();
    let tous = destinataires(&mut *conn, etab, &s, &jours, &en_plus).await?;
    // Rétablissement : seulement ceux qui avaient été prévenus de cette modification.
    let prevenus: Option<BTreeSet<Uuid>> = if retablie {
        // Ceux qui avaient été prévenus d'un changement de CE créneau à CE jour (la modification levée ou une
        // décision antérieure qu'elle avait remplacée) et que personne n'a déjà rétablis depuis : la levée d'un
        // remplacement, devenu annulation jamais annoncée, doit quand même dire « le cours a lieu normalement ».
        let v: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT a.destinataire_user_id
               FROM ent_notification a
               JOIN ent_metier_event ea ON ea.id = a.source_event_id AND ea.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.kind <> 'edt_retabli' AND ea.type <> $4
                AND ea.payload->>'creneau_id' = $2 AND ea.payload->>'jour' = $3
                AND NOT EXISTS (
                    SELECT 1 FROM ent_notification r
                      JOIN ent_metier_event er ON er.id = r.source_event_id AND er.etablissement_id = r.etablissement_id
                     WHERE r.etablissement_id = a.etablissement_id AND r.destinataire_user_id = a.destinataire_user_id
                       AND r.kind = 'edt_retabli' AND r.created_at > a.created_at
                       AND er.payload->>'creneau_id' = $2 AND er.payload->>'jour' = $3)",
        )
        .bind(etab)
        .bind(m.creneau_id.map(|c| c.to_string()).unwrap_or_default())
        .bind(m.jour.to_string())
        .bind(EVT_SEANCE_RETABLIE)
        .fetch_all(&mut *conn)
        .await
        .map_err(err)?;
        Some(v.into_iter().collect())
    } else {
        None
    };
    let motif = motif_libelle(&m.motif_public_code, m.motif_public.as_deref());
    let (kind, titre, corps) = if retablie {
        (
            "edt_retabli",
            format!("Cours rétabli : {}", s.matiere),
            format!(
                "Le cours de {} du {} à {} aura lieu normalement.",
                s.matiere,
                jour_fr(s.jour),
                s.heure_debut.format("%H:%M")
            ),
        )
    } else {
        let (k, t, c) = message(
            &m.type_code,
            &s.matiere,
            s.jour,
            s.heure_debut,
            &motif,
            m.apres_jour,
            m.apres_debut,
            m.apres_salle_texte.as_deref(),
        );
        (k, t, c)
    };
    // Le début à retenir pour l'urgence : le plus proche entre l'ancien et le nouveau créneau.
    let mut debut = s.debut_local;
    if let (Some(j), Some(d)) = (m.apres_jour, m.apres_debut) {
        debut = debut.min(j.and_time(d));
    }
    let _ = (m.avant_etat, m.seance_arrivee_id);
    for d in tous {
        if let Some(pre) = &prevenus {
            if !pre.contains(&d.user_id) {
                continue;
            }
        }
        let plan = planifier(local, debut, veille, d.dest, d.affecte_la_journee);
        // Le remplaçant désigné apprend qu'il assure le cours (jamais le motif de l'absence).
        if !retablie
            && m.type_code == "intervenant"
            && Some(d.user_id) == m.apres_remplacant_user_id
        {
            let classe = s.classe.clone();
            let titre_r = format!("Remplacement : {classe}");
            let corps_r = format!(
                "Vous assurez le cours de {} ({classe}) du {} à {}.",
                s.matiere,
                jour_fr(s.jour),
                s.heure_debut.format("%H:%M")
            );
            envoyer(
                &mut *conn,
                etab,
                evt,
                &d,
                "edt_remplacement_assigne",
                &titre_r,
                &corps_r,
                &plan,
                maintenant,
                "",
            )
            .await?;
            continue;
        }
        envoyer(
            &mut *conn, etab, evt, &d, kind, &titre, &corps, &plan, maintenant, "",
        )
        .await?;
        rappeler(
            &mut *conn, etab, evt, &d, kind, &titre, &corps, &plan, maintenant,
        )
        .await?;
    }
    Ok(())
}

/// Un lot : UN message par personne, avec le nombre de séances qui la concernent.
async fn traiter_lot(conn: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    let p = &evt.event().payload;
    let Some(lot) = p
        .get("lot_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    else {
        return Ok(());
    };
    let levee = p.get("action").and_then(|v| v.as_str()) == Some("levee");
    let l: Option<(NaiveDate, NaiveDate, String, Option<String>, bool)> = sqlx::query_as(
        "SELECT du, au, motif_public_code, motif_public, (leve_le IS NOT NULL) FROM ent_edt_lot
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(lot)
    .fetch_optional(&mut *conn)
    .await
    .map_err(err)?;
    let Some((du, au, motif_code, motif_texte, leve)) = l else {
        return Ok(());
    };
    // L'état courant prime : un lot levé avant le traitement de sa pose n'est pas annoncé.
    if !levee && leve {
        return Ok(());
    }
    // Séances du lot (vivantes pour une pose ; levées pour un rétablissement), à venir seulement.
    let seances: Vec<SeanceEvt> = sqlx::query_as(&format!(
        "{SELECT_CIBLE}
          WHERE m.etablissement_id = $1 AND m.lot_id = $2 AND ($3 OR m.levee_le IS NULL)
            AND (s.fin_utc IS NULL OR s.fin_utc > $4)
          ORDER BY m.jour, coalesce(s.heure_debut, c.heure_debut), m.id"
    ))
    .bind(etab)
    .bind(lot)
    .bind(levee)
    .bind(evt.event().created_at)
    .fetch_all(&mut *conn)
    .await
    .map_err(err)?;
    if seances.is_empty() {
        return Ok(());
    }
    let maintenant = evt.event().created_at;
    let (local, veille) = horloge(&mut *conn, etab, maintenant).await?;
    // Personnes et nombre de séances qui les concernent.
    let mut par_user: BTreeMap<Uuid, (Destinataire, i64)> = BTreeMap::new();
    for s in &seances {
        for d in destinataires(&mut *conn, etab, s, &[s.jour], &[]).await? {
            let e = par_user.entry(d.user_id).or_insert((d, 0));
            e.1 += 1;
        }
    }
    let prevenus: Option<BTreeSet<Uuid>> = if levee {
        let v: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT n.destinataire_user_id FROM ent_notification n
               JOIN ent_metier_event e ON e.id = n.source_event_id
              WHERE n.etablissement_id = $1 AND e.etablissement_id = $1 AND e.payload->>'lot_id' = $2
                AND e.payload->>'action' = 'pose'",
        )
        .bind(etab)
        .bind(lot.to_string())
        .fetch_all(&mut *conn)
        .await
        .map_err(err)?;
        Some(v.into_iter().collect())
    } else {
        None
    };
    let periode = if du == au {
        format!("le {}", jour_fr(du))
    } else {
        format!("du {} au {}", jour_fr(du), jour_fr(au))
    };
    let motif = motif_libelle(&motif_code, motif_texte.as_deref());
    let suffixe = if motif.is_empty() {
        String::new()
    } else {
        format!(" ({motif})")
    };
    let debut = seances.iter().map(|s| s.debut_local).min().unwrap_or(local);
    let nature: String = p
        .get("nature")
        .and_then(|v| v.as_str())
        .unwrap_or("annulation")
        .to_string();
    for (_, (d, n)) in par_user {
        if let Some(pre) = &prevenus {
            if !pre.contains(&d.user_id) {
                continue;
            }
        }
        let (kind, titre, corps) = if levee {
            (
                "edt_retabli",
                "Cours rétablis".to_string(),
                format!("Les cours annulés {periode} sont rétablis."),
            )
        } else if nature == "salle_fermee" {
            (
                "edt_salle",
                "Changement de salle".to_string(),
                format!("{n} cours changent de salle {periode}{suffixe}."),
            )
        } else {
            let verbe = if nature == "neutralisation" {
                "n'ont pas lieu"
            } else {
                "sont annulés"
            };
            let verbe1 = if nature == "neutralisation" {
                "n'a pas lieu"
            } else {
                "est annulé"
            };
            (
                "edt_annulation_masse",
                "Cours annulés".to_string(),
                format!(
                    "{n} {} {periode}{suffixe}.",
                    if n > 1 {
                        format!("cours {verbe}")
                    } else {
                        format!("cours {verbe1}")
                    }
                ),
            )
        };
        // Un lot touche la journée de la famille : conservateur, le message part selon l'urgence du délai.
        let plan = planifier(local, debut, veille, d.dest, true);
        envoyer(
            &mut *conn, etab, evt, &d, kind, &titre, &corps, &plan, maintenant, "",
        )
        .await?;
        rappeler(
            &mut *conn, etab, evt, &d, kind, &titre, &corps, &plan, maintenant,
        )
        .await?;
    }
    Ok(())
}

async fn traiter_t4(conn: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    match evt.event().type_name.as_str() {
        EVT_LOT_MODIFICATIONS => traiter_lot(conn, evt).await,
        EVT_SEANCE_MODIFIEE | EVT_SEANCE_ANNULEE | EVT_SEANCE_RETABLIE | EVT_SEANCE_AJOUTEE => {
            traiter_modification(conn, evt).await
        }
        _ => Ok(()),
    }
}

impl Consommateur for ConsommateurNotificationsEdtT4 {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_EDT_V2
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_T4
    }

    /// Non rejouable : l'état courant (modification levée, séance déplacée) prime sur la charge utile, et l'effet
    /// externe (mail, push) ne se rattrape pas.
    fn rejouable(&self) -> bool {
        false
    }

    fn traiter<'a>(
        &'a self,
        tx: &'a mut PgConnection,
        evt: &'a EvenementDistribue,
    ) -> FutureTraitement<'a> {
        Box::pin(traiter_t4(tx, evt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(j: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, j)
            .unwrap()
            .and_time(NaiveTime::from_hms_opt(h, m, 0).unwrap())
    }

    const VEILLE: NaiveTime = match NaiveTime::from_hms_opt(18, 30, 0) {
        Some(t) => t,
        None => panic!(),
    };

    #[test]
    fn annulation_de_demain_8h_decidee_a_17h_part_a_l_heure_du_message_de_la_veille() {
        // §9.5 : demain 8 h décidée à 17 h → urgente (U2), envoyée à 18 h 30, rappelée le matin.
        let p = planifier(dt(14, 17, 0), dt(15, 8, 0), VEILLE, Dest::Eleve, true);
        assert_eq!(p.urgence, Urgence::Immediate);
        assert_eq!(p.envoi_local, Some(dt(14, 18, 30)));
        assert_eq!(p.rappel_local, Some(dt(15, 6, 30)));
        assert!(!p.regroupable && p.ignore_heures_calmes);
        // Décidée à 21 h : tout de suite (heures calmes ignorées, la veille est passée), pas de rappel.
        let p = planifier(dt(14, 21, 0), dt(15, 8, 0), VEILLE, Dest::Eleve, true);
        assert_eq!(
            (p.urgence, p.envoi_local, p.rappel_local),
            (Urgence::Immediate, None, None)
        );
    }

    #[test]
    fn dans_trois_semaines_c_est_ordinaire_et_regroupe() {
        let p = planifier(
            dt(14, 17, 0),
            dt(14, 8, 0) + Duration::days(21),
            VEILLE,
            Dest::Eleve,
            true,
        );
        assert_eq!(p.urgence, Urgence::Ordinaire);
        assert!(p.regroupable && !p.ignore_heures_calmes && p.envoi_local.is_none());
        // Demain à 10 h pile : plus urgent (la limite est « avant 10 h »).
        let p = planifier(dt(14, 9, 0), dt(15, 10, 0), VEILLE, Dest::Eleve, true);
        assert_eq!(p.urgence, Urgence::Ordinaire);
    }

    #[test]
    fn le_jour_meme_et_la_seance_deja_commencee_sont_immediats() {
        let p = planifier(dt(14, 7, 45), dt(14, 8, 0), VEILLE, Dest::Intervenant, true);
        assert_eq!((p.urgence, p.envoi_local), (Urgence::Immediate, None));
        let p = planifier(dt(14, 8, 30), dt(14, 8, 0), VEILLE, Dest::Eleve, true);
        assert_eq!(p.urgence, Urgence::Immediate);
    }

    #[test]
    fn la_famille_n_est_prevenue_tout_de_suite_que_si_la_journee_change() {
        // R-E36 : un changement de salle en milieu de journée n'entre que dans le résumé.
        let p = planifier(dt(14, 17, 0), dt(15, 8, 0), VEILLE, Dest::Famille, false);
        assert_eq!((p.urgence, p.regroupable), (Urgence::Ordinaire, true));
        let p = planifier(dt(14, 17, 0), dt(15, 8, 0), VEILLE, Dest::Famille, true);
        assert_eq!(p.urgence, Urgence::Immediate);
        // L'élève, lui, est prévenu de tout.
        let p = planifier(dt(14, 17, 0), dt(15, 8, 0), VEILLE, Dest::Eleve, false);
        assert_eq!(p.urgence, Urgence::Immediate);
    }

    #[test]
    fn les_textes_ne_portent_ni_motif_interne_ni_nom() {
        let (k, t, c) = message(
            "intervenant",
            "Maths",
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            &motif_libelle("absence_professeur", None),
            None,
            None,
            None,
        );
        assert_eq!(k, "edt_remplacement");
        assert!(
            t.contains("autre enseignant")
                && c.contains("un autre enseignant")
                && !c.contains("absence")
        );
        let (_, _, c) = message(
            "annulation",
            "Maths",
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            &motif_libelle("mouvement_social", None),
            None,
            None,
            None,
        );
        assert!(c.contains("mouvement social") && c.contains("annulé"));
        assert_eq!(
            motif_libelle("autre", Some("Réunion de parents")),
            "Réunion de parents"
        );
        assert_eq!(motif_libelle("reunion", Some("réunion")), "réunion");
    }

    #[test]
    fn alias_des_anciens_noms_documentes() {
        assert!(ALIAS_ANCIENS_NOMS
            .iter()
            .any(|(a, _)| *a == "edt.exception"));
        assert!(ALIAS_ANCIENS_NOMS
            .iter()
            .any(|(a, n)| *a == "edt.annulation_masse" && n.contains("lot_modifications")));
    }
}
