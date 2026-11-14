//! Socle 12, lot AS4 : **décompte versionné** (`docs/socles/12_ASSIDUITE.md` §13.1, §13.2, §22.6,
//! R-AS15 à R-AS17, CA-25, CA-39, CA-48, CA-49, CA-67).
//!
//! Le décompte en demi-journées est calculé **à partir des épisodes** (AS3), pas de l'ancien statut :
//! un jour qui mêle épisodes valables et non valables compte chaque demi-journée selon ses propres
//! épisodes. Trois entrées, toutes datées du **jour concerné** :
//! - la **règle d'assiduité** en vigueur ce jour-là (`ent_asd_regle`, jamais celle d'aujourd'hui : changer
//!   la règle en janvier ne touche pas septembre) ;
//! - le **calendrier** du socle 5 (vacances, fériés, fermetures, demi-journées sans cours) pour la classe
//!   de l'élève, et sa **scolarisation** : une demi-journée sans cours ou hors scolarisation compte 0 ;
//! - la **coupure** : la grille du socle 5 (STR-35) et, à défaut, le repli sur la règle.
//!
//! Le résultat est **matérialisé** (`ent_asd_demi_journee`, recalculable : recalculer tout ne change rien),
//! chaque demi-journée porte ses **raisons** (liste fermée) d'où l'on tire l'**explication** (« mardi matin
//! compte : … »). Un mois est **arrêté** le 1er du mois suivant (`ent_asd_mois_arrete`, ajout seul, règles
//! figées) ; ce qui change ensuite est inscrit en **rectification** à part (R-AS16), jamais dans le mois
//! arrêté. Le jour métier est celui de l'établissement (`ent_aujourdhui_etab`), jamais `current_date`.

use crate::assiduite::{
    bornes_mois, minutes_union, secondes, ventiler, Contribution, Periode,
    MINUTES_DEMI_JOURNEE_ENTIERE,
};
use crate::outbox::{Causalite, Publier};
use crate::service::{EntDbError, PgEnt};
use crate::structure_calendrier::{calendrier, Cible, Jour};
use crate::tenant_tx::TenantTx;
use crate::vie_scolaire::{db, introuvable};
use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEvent, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use sqlx::PgConnection;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use uuid::Uuid;

pub mod evenements {
    pub const MOIS_ARRETE: &str = "assiduite.mois_arrete";
    pub const RECTIFICATION: &str = "assiduite.rectification";
}

/// Règle par défaut du code (établissement sans version) : identifiée par une date lointaine.
fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

const DEFAUT_DU: (i32, u32, u32) = (2000, 1, 1);
/// Fenêtre de chargement du calendrier (le port refuse plus de 800 jours d'un coup).
const FENETRE_CALENDRIER_JOURS: i64 = 700;
/// Plafond d'une période recalculée d'un coup (deux années scolaires).
const PLAGE_MAX_JOURS: i64 = 800;

pub const CAUSES_RECTIFICATION: [&str; 5] = [
    "recalcul",
    "requalification",
    "correction_appel",
    "neutralisation",
    "calendrier",
];

// ───────────────────────────── Calcul pur ─────────────────────────────

/// Définition de la demi-journée (Q-AS3, §13.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Definition {
    /// Au moins une séance manquée.
    UneSeance,
    /// Au moins N minutes manquées (l'existant : 30).
    Minutes,
    /// Toute la demi-journée (toutes les séances attendues).
    Entiere,
}

impl Definition {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "une_seance" => Some(Self::UneSeance),
            "minutes" => Some(Self::Minutes),
            "entiere" => Some(Self::Entiere),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UneSeance => "une_seance",
            Self::Minutes => "minutes",
            Self::Entiere => "entiere",
        }
    }
}

/// Paramètres d'un jour : la règle en vigueur **ce jour-là** et la coupure retenue.
#[derive(Debug, Clone, Copy)]
pub struct ParamsJour {
    pub definition: Definition,
    pub seuil_minutes: i64,
    pub coupure: NaiveTime,
}

/// Une demi-journée est-elle ouverte aux élèves ce jour-là ? Sinon, pourquoi pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ouverture {
    Ouverte,
    SansCours,
    HorsAnnee,
    HorsScolarisation,
}

/// Pourquoi une demi-journée compte ou ne compte pas (liste fermée, miroir de la base).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Raison {
    Compte,
    SousLeSeuil,
    DemiJourneeIncomplete,
    SansCours,
    HorsAnnee,
    HorsScolarisation,
}

impl Raison {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compte => "compte",
            Self::SousLeSeuil => "sous_le_seuil",
            Self::DemiJourneeIncomplete => "demi_journee_incomplete",
            Self::SansCours => "sans_cours",
            Self::HorsAnnee => "hors_annee",
            Self::HorsScolarisation => "hors_scolarisation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemiJourneeCalc {
    pub periode: Periode,
    pub minutes: i64,
    pub minutes_non_justifiees: i64,
    pub seances: i64,
    pub seances_non_justifiees: i64,
    pub minutes_attendues: Option<i64>,
    pub compte: bool,
    pub non_justifiee: bool,
    pub raison: Raison,
}

/// Demi-journées d'un jour pour un élève, **selon la règle du jour et le calendrier**. Fonction pure.
/// `attendues` : créneaux de cours attendus de l'élève ce jour-là (utile à la définition « entière »).
/// `ouverture` : `[matin, après-midi]`. Ne renvoie que les demi-journées qui ont reçu une contribution.
pub fn deriver_jour(
    contributions: &[Contribution],
    attendues: &[(NaiveTime, NaiveTime)],
    p: &ParamsJour,
    ouverture: [Ouverture; 2],
) -> Vec<DemiJourneeCalc> {
    let coupure = secondes(p.coupure);
    let mut out = Vec::new();
    for (i, periode) in [Periode::Matin, Periode::ApresMidi].into_iter().enumerate() {
        let mut tous = Vec::new();
        let mut injustifies = Vec::new();
        let (mut entiere, mut entiere_injustifiee, mut touchee) = (false, false, false);
        let (mut seances, mut seances_nj) = (0i64, 0i64);
        for c in contributions {
            match *c {
                Contribution::Seance {
                    debut,
                    fin,
                    justifiee,
                } => {
                    let (m, a) = ventiler(secondes(debut), secondes(fin), coupure);
                    if let Some(iv) = if periode == Periode::Matin { m } else { a } {
                        touchee = true;
                        seances += 1;
                        tous.push(iv);
                        if !justifiee {
                            seances_nj += 1;
                            injustifies.push(iv);
                        }
                    }
                }
                Contribution::DemiJournee {
                    periode: q,
                    justifiee,
                } if q == periode => {
                    touchee = true;
                    entiere = true;
                    seances += 1;
                    if !justifiee {
                        entiere_injustifiee = true;
                        seances_nj += 1;
                    }
                }
                Contribution::DemiJournee { .. } => {}
            }
        }
        if !touchee {
            continue;
        }
        let mut minutes = minutes_union(tous);
        let mut non_just = minutes_union(injustifies);
        if entiere {
            minutes = minutes.max(MINUTES_DEMI_JOURNEE_ENTIERE);
        }
        if entiere_injustifiee {
            non_just = non_just.max(MINUTES_DEMI_JOURNEE_ENTIERE);
        }
        let attendues_min = (p.definition == Definition::Entiere).then(|| {
            minutes_union(
                attendues
                    .iter()
                    .filter_map(|(d, f)| {
                        let (m, a) = ventiler(secondes(*d), secondes(*f), coupure);
                        if periode == Periode::Matin {
                            m
                        } else {
                            a
                        }
                    })
                    .collect(),
            )
        });
        let (compte, non_justifiee, raison) = match ouverture[i] {
            Ouverture::SansCours => (false, false, Raison::SansCours),
            Ouverture::HorsAnnee => (false, false, Raison::HorsAnnee),
            Ouverture::HorsScolarisation => (false, false, Raison::HorsScolarisation),
            Ouverture::Ouverte => match p.definition {
                Definition::UneSeance => (true, seances_nj > 0, Raison::Compte),
                Definition::Minutes => {
                    let compte = minutes >= p.seuil_minutes;
                    (
                        compte,
                        compte && non_just >= p.seuil_minutes,
                        if compte {
                            Raison::Compte
                        } else {
                            Raison::SousLeSeuil
                        },
                    )
                }
                Definition::Entiere => {
                    let attendues = attendues_min.unwrap_or(0);
                    // Une absence « à la journée » (ancien circuit) vaut la demi-journée entière ;
                    // sinon les minutes manquées doivent couvrir **tout** le cours attendu.
                    let compte = entiere || (attendues > 0 && minutes >= attendues);
                    (
                        compte,
                        compte && (entiere_injustifiee || (attendues > 0 && non_just >= attendues)),
                        if compte {
                            Raison::Compte
                        } else {
                            Raison::DemiJourneeIncomplete
                        },
                    )
                }
            },
        };
        out.push(DemiJourneeCalc {
            periode,
            minutes,
            minutes_non_justifiees: non_just,
            seances,
            seances_non_justifiees: seances_nj,
            minutes_attendues: attendues_min,
            compte,
            non_justifiee,
            raison,
        });
    }
    out
}

fn nom_jour(d: NaiveDate) -> &'static str {
    match d.weekday() {
        Weekday::Mon => "lundi",
        Weekday::Tue => "mardi",
        Weekday::Wed => "mercredi",
        Weekday::Thu => "jeudi",
        Weekday::Fri => "vendredi",
        Weekday::Sat => "samedi",
        Weekday::Sun => "dimanche",
    }
}

fn nom_definition(d: &str) -> &'static str {
    match d {
        "une_seance" => "au moins une séance manquée",
        "entiere" => "toute la demi-journée manquée",
        _ => "au moins le seuil de minutes manquées",
    }
}

fn pluriel(n: i64, singulier: &str, pluriel: &str) -> String {
    format!("{n} {}", if n > 1 { pluriel } else { singulier })
}

/// « Pourquoi cette demi-journée compte » (§13.2) : phrase tirée des **champs codés** de la ligne, sans
/// aucune donnée de motif ni note (R-AS19, R-AS22). Fonction pure.
pub fn expliquer(d: &DemiJourneeCalculee) -> String {
    let quand = format!(
        "{} {}{}",
        nom_jour(d.jour),
        d.jour.format("%d/%m/%Y"),
        if d.periode == Periode::Matin {
            " matin"
        } else {
            " après-midi"
        }
    );
    let regle = format!(
        "règle : {}{}, version du {}",
        nom_definition(&d.definition),
        if d.definition == "minutes" {
            format!(" ({} min)", d.seuil_minutes)
        } else {
            String::new()
        },
        d.regle_valide_du.format("%d/%m/%Y")
    );
    let manquees = format!(
        "{} ({} min)",
        pluriel(d.seances_manquees, "séance manquée", "séances manquées"),
        d.minutes_manquees
    );
    let justif = if d.seances_non_justifiees == 0 {
        "toutes couvertes par un motif valable".to_string()
    } else {
        format!(
            "dont {} non justifiée{}",
            d.seances_non_justifiees,
            if d.seances_non_justifiees > 1 {
                "s"
            } else {
                ""
            }
        )
    };
    match d.raison {
        "compte" if d.non_justifiee => {
            format!("{quand} compte, non justifiée : {manquees}, {justif} ; {regle}.")
        }
        "compte" => format!("{quand} compte, justifiée : {manquees}, {justif} ; {regle}."),
        "sous_le_seuil" => format!("{quand} ne compte pas : {manquees}, sous le seuil ; {regle}."),
        "demi_journee_incomplete" => format!(
            "{quand} ne compte pas : {manquees} sur {} min de cours attendus ; {regle}.",
            d.minutes_attendues.unwrap_or(0)
        ),
        "sans_cours" => {
            format!("{quand} ne compte pas : aucun cours selon le calendrier de l'établissement.")
        }
        "hors_annee" => format!("{quand} ne compte pas : hors de l'année scolaire."),
        _ => format!("{quand} ne compte pas : l'élève n'était pas scolarisé dans l'établissement."),
    }
}

// ───────────────────────────── Données ─────────────────────────────

/// Une demi-journée calculée, avec tout ce qui permet de la **rejouer** et de l'**expliquer**.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DemiJourneeCalculee {
    pub eleve_id: Uuid,
    pub jour: NaiveDate,
    pub periode: Periode,
    pub minutes_manquees: i64,
    pub minutes_non_justifiees: i64,
    pub seances_manquees: i64,
    pub seances_non_justifiees: i64,
    pub minutes_attendues: Option<i64>,
    pub regle_valide_du: NaiveDate,
    pub regle_id: Option<Uuid>,
    pub definition: String,
    pub seuil_minutes: i32,
    pub coupure: NaiveTime,
    pub coupure_source: &'static str,
    pub calendrier_source: &'static str,
    pub compte: bool,
    pub non_justifiee: bool,
    pub raison: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResumeRecalcul {
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// Lignes écrites ou modifiées : 0 quand rien n'a changé (recalculer tout ne change rien).
    pub lignes_modifiees: u64,
    pub lignes_supprimees: u64,
    pub lignes_total: usize,
    pub rectifications: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EleveMoisArrete {
    pub eleve_id: Uuid,
    pub demi_journees: i32,
    pub non_justifiees: i32,
    pub retards: i32,
    pub regle_valide_du: NaiveDate,
    pub seuil_signalement: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArretMois {
    pub mois: NaiveDate,
    /// Faux si le mois était déjà arrêté : rien n'a été réécrit.
    pub arrete_maintenant: bool,
    pub eleves: Vec<EleveMoisArrete>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Rectification {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub mois: NaiveDate,
    pub avant_demi_journees: i32,
    pub avant_non_justifiees: i32,
    pub apres_demi_journees: i32,
    pub apres_non_justifiees: i32,
    pub cause: String,
    pub cause_ref_id: Option<Uuid>,
    pub touche_signalement: bool,
    pub cree_le: chrono::DateTime<chrono::Utc>,
    pub vue_par: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
struct RegleJour {
    id: Uuid,
    valide_du: NaiveDate,
    definition_demi_journee: String,
    seuil_minutes: i32,
    heure_coupure: NaiveTime,
}

fn defaut_du() -> NaiveDate {
    NaiveDate::from_ymd_opt(DEFAUT_DU.0, DEFAUT_DU.1, DEFAUT_DU.2).expect("date de repli")
}

/// Règle en vigueur le `jour` : la dernière version dont la date de début n'est pas postérieure
/// (R-AS15). `regles` est trié par date croissante.
fn regle_a(regles: &[RegleJour], jour: NaiveDate) -> Option<&RegleJour> {
    regles.iter().rev().find(|r| r.valide_du <= jour)
}

fn mois_suivant(mois: NaiveDate) -> NaiveDate {
    bornes_mois(mois).1
}

fn est_premier(mois: NaiveDate) -> bool {
    mois.day() == 1
}

#[derive(sqlx::FromRow)]
struct Constat {
    eleve_id: Uuid,
    jour: NaiveDate,
    kind: String,
    periode: Option<String>,
    heure_debut: Option<NaiveTime>,
    heure_fin: Option<NaiveTime>,
    justifiee: bool,
}

impl PgEnt {
    // ── Calcul ────────────────────────────────────────────────────────────────────────────────

    /// Calcule (sans rien écrire) les demi-journées de `[du, au]` d'après les épisodes. Les absences de
    /// l'ancien circuit sans épisode reçoivent le leur (fonction SQL idempotente, DA-14).
    pub(crate) async fn calculer_demi_journees(
        &self,
        etab: Uuid,
        eleve: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<DemiJourneeCalculee>, EntDbError> {
        if au < du || (au - du).num_days() > PLAGE_MAX_JOURS {
            return Err(invalide("période de décompte invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query("SELECT ent_asd_adopter_absences($1)")
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let regles: Vec<RegleJour> = sqlx::query_as(
            "SELECT id, valide_du, definition_demi_journee, seuil_minutes, heure_coupure
               FROM ent_asd_regle WHERE etablissement_id = $1 AND valide_du <= $2 ORDER BY valide_du",
        )
        .bind(etab)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let constats: Vec<Constat> = sqlx::query_as(
            "SELECT s.eleve_id, s.jour, s.kind, s.periode, s.heure_debut, s.heure_fin,
                    (a.qualification IN ('reputee_legitime', 'excuse_valable')) AS justifiee
               FROM ent_asd_absence_seance s
               JOIN ent_asd_absence a ON a.etablissement_id = s.etablissement_id AND a.id = s.absence_id
              WHERE s.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3
                AND ($4::uuid IS NULL OR s.eleve_id = $4)
                AND s.retiree_le IS NULL AND a.etat <> 'retire'",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut contribs: BTreeMap<(Uuid, NaiveDate), Vec<Contribution>> = BTreeMap::new();
        for Constat {
            eleve_id: e,
            jour,
            kind,
            periode,
            heure_debut: debut,
            heure_fin: fin,
            justifiee,
        } in constats
        {
            let c = match (
                kind.as_str(),
                periode.as_deref().and_then(Periode::parse),
                debut,
                fin,
            ) {
                ("demi_journee", Some(p), _, _) => Contribution::DemiJournee {
                    periode: p,
                    justifiee,
                },
                ("seance", _, Some(d), Some(f)) => Contribution::Seance {
                    debut: d,
                    fin: f,
                    justifiee,
                },
                _ => continue,
            };
            contribs.entry((e, jour)).or_default().push(c);
        }
        if contribs.is_empty() {
            tx.commit().await.map_err(db)?;
            return Ok(Vec::new());
        }
        let eleves: Vec<Uuid> = contribs
            .keys()
            .map(|(e, _)| *e)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let jour_min = contribs.keys().map(|(_, j)| *j).min().unwrap_or(du);
        let jour_max = contribs.keys().map(|(_, j)| *j).max().unwrap_or(au);

        // Créneaux attendus : seulement si une version de règle de la période utilise « entière ».
        let besoin_attendues = regles
            .iter()
            .any(|r| r.definition_demi_journee == "entiere");
        let mut attendues: HashMap<(Uuid, NaiveDate), Vec<(NaiveTime, NaiveTime)>> = HashMap::new();
        if besoin_attendues {
            let lignes: Vec<(Uuid, NaiveDate, Option<NaiveTime>, Option<NaiveTime>)> = sqlx::query_as(
                "SELECT l.eleve_id, a.jour, COALESCE(a.heure_debut, c.heure_debut), COALESCE(a.heure_fin, c.heure_fin)
                   FROM ent_appel_ligne l
                   JOIN ent_appel a ON a.etablissement_id = l.etablissement_id AND a.id = l.appel_id
                   LEFT JOIN ent_edt_creneau c ON c.etablissement_id = a.etablissement_id AND c.id = a.creneau_id
                  WHERE l.etablissement_id = $1 AND a.jour BETWEEN $2 AND $3 AND l.eleve_id = ANY($4)
                    AND a.etat = 'valide' AND a.type_activite = 'cours'
                    AND l.etat IN ('present', 'absent', 'retard', 'non_pointe')",
            )
            .bind(etab)
            .bind(jour_min)
            .bind(jour_max)
            .bind(&eleves)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for (e, j, d, f) in lignes {
                if let (Some(d), Some(f)) = (d, f) {
                    attendues.entry((e, j)).or_default().push((d, f));
                }
            }
        }

        // Scolarisation : hors de ses périodes de scolarisation, rien ne compte (DA-22).
        let scolarisations: Vec<(Uuid, NaiveDate, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT eleve_id, date_entree, date_sortie FROM ent_eleve_scolarisation
              WHERE etablissement_id = $1 AND eleve_id = ANY($2)",
        )
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut scol: HashMap<Uuid, Vec<(NaiveDate, Option<NaiveDate>)>> = HashMap::new();
        for (e, entree, sortie) in scolarisations {
            scol.entry(e).or_default().push((entree, sortie));
        }

        // Classe de l'élève : cible du calendrier et de la grille.
        let classes: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, classe_id FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let classe_de: HashMap<Uuid, Option<Uuid>> = classes.into_iter().collect();

        // Calendrier du socle 5, par classe. Aucune année scolaire définie : repli (lundi au vendredi).
        let (annees,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1)",
        )
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut calendriers: HashMap<Option<Uuid>, HashMap<NaiveDate, Jour>> = HashMap::new();
        if annees {
            let classes_distinctes: BTreeSet<Option<Uuid>> = eleves
                .iter()
                .map(|e| classe_de.get(e).copied().flatten())
                .collect();
            for classe_id in classes_distinctes {
                let cible = Cible {
                    site_id: None,
                    niveau_id: None,
                    classe_id,
                    groupe_id: None,
                };
                let mut jours = HashMap::new();
                let mut debut = jour_min;
                while debut <= jour_max {
                    let fin = (debut + Duration::days(FENETRE_CALENDRIER_JOURS - 1)).min(jour_max);
                    for j in calendrier::jours(&mut tx, etab, debut, fin, &cible).await? {
                        jours.insert(j.date, j);
                    }
                    debut = fin + Duration::days(1);
                }
                calendriers.insert(classe_id, jours);
            }
        }

        // Coupure de la grille du socle 5 (STR-35), par (classe, jour) ; repli sur la règle.
        let mut coupures: HashMap<(Uuid, NaiveDate), Option<NaiveTime>> = HashMap::new();
        for (e, jour) in contribs.keys() {
            let Some(Some(classe)) = classe_de.get(e) else {
                continue;
            };
            if coupures.contains_key(&(*classe, *jour)) {
                continue;
            }
            let c: Option<NaiveTime> = sqlx::query_scalar(
                "SELECT g.coupure_demi_journee
                   FROM ent_classe c
                   LEFT JOIN ent_mef m ON m.id = c.mef_id AND m.etablissement_id = c.etablissement_id
                   JOIN ent_grille g ON g.etablissement_id = c.etablissement_id
                    AND g.id = ent_grille_pour(c.etablissement_id, $3, c.site_id, COALESCE(c.niveau_id, m.niveau_id))
                  WHERE c.etablissement_id = $1 AND c.id = $2",
            )
            .bind(etab)
            .bind(classe)
            .bind(jour)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            coupures.insert((*classe, *jour), c);
        }
        tx.commit().await.map_err(db)?;

        let mut out = Vec::new();
        for ((e, jour), cs) in &contribs {
            let regle = regle_a(&regles, *jour);
            let (definition, seuil, coupure_regle, regle_du, regle_id) = match regle {
                Some(r) => (
                    Definition::parse(&r.definition_demi_journee).unwrap_or(Definition::Minutes),
                    r.seuil_minutes,
                    r.heure_coupure,
                    r.valide_du,
                    Some(r.id),
                ),
                None => (
                    Definition::Minutes,
                    30,
                    NaiveTime::from_hms_opt(12, 0, 0).expect("12:00"),
                    defaut_du(),
                    None,
                ),
            };
            let classe = classe_de.get(e).copied().flatten();
            let grille = classe.and_then(|c| coupures.get(&(c, *jour)).copied().flatten());
            let (coupure, coupure_source) = match grille {
                Some(c) => (c, "grille"),
                None => (coupure_regle, "regle"),
            };
            let inscrit = scol.get(e).is_none_or(|l| {
                l.iter()
                    .any(|(entree, sortie)| *entree <= *jour && sortie.is_none_or(|s| *jour < s))
            });
            let (ouverture, calendrier_source) = if !inscrit {
                (
                    [Ouverture::HorsScolarisation; 2],
                    if annees { "calendrier" } else { "repli" },
                )
            } else if !annees {
                let ouvre = if jour.weekday().number_from_monday() < 6 {
                    Ouverture::Ouverte
                } else {
                    Ouverture::SansCours
                };
                ([ouvre; 2], "repli")
            } else {
                match calendriers.get(&classe).and_then(|m| m.get(jour)) {
                    Some(j) if j.type_jour == "hors_annee" => {
                        ([Ouverture::HorsAnnee; 2], "calendrier")
                    }
                    Some(j) => (
                        [
                            if j.matin.cours || j.matin.eleves {
                                Ouverture::Ouverte
                            } else {
                                Ouverture::SansCours
                            },
                            if j.apres_midi.cours || j.apres_midi.eleves {
                                Ouverture::Ouverte
                            } else {
                                Ouverture::SansCours
                            },
                        ],
                        "calendrier",
                    ),
                    None => ([Ouverture::HorsAnnee; 2], "calendrier"),
                }
            };
            let params = ParamsJour {
                definition,
                seuil_minutes: i64::from(seuil),
                coupure,
            };
            let att = attendues
                .get(&(*e, *jour))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            for d in deriver_jour(cs, att, &params, ouverture) {
                out.push(DemiJourneeCalculee {
                    eleve_id: *e,
                    jour: *jour,
                    periode: d.periode,
                    minutes_manquees: d.minutes,
                    minutes_non_justifiees: d.minutes_non_justifiees,
                    seances_manquees: d.seances,
                    seances_non_justifiees: d.seances_non_justifiees,
                    minutes_attendues: d.minutes_attendues,
                    regle_valide_du: regle_du,
                    regle_id,
                    definition: definition.as_str().to_string(),
                    seuil_minutes: seuil,
                    coupure,
                    coupure_source,
                    calendrier_source,
                    compte: d.compte,
                    non_justifiee: d.non_justifiee,
                    raison: d.raison.as_str(),
                });
            }
        }
        out.sort_by_key(|d| (d.eleve_id, d.jour, d.periode));
        Ok(out)
    }

    // ── Matérialisation ───────────────────────────────────────────────────────────────────────

    /// Recalcule et **écrit** les demi-journées des mois couvrant `[du, au]` (idempotent : une ligne
    /// inchangée n'est pas réécrite), puis, pour les mois **déjà arrêtés**, inscrit les écarts en
    /// rectifications (R-AS16). Le mois arrêté lui-même n'est jamais réécrit.
    pub(crate) async fn recalculer_decompte(
        &self,
        etab: Uuid,
        eleve: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
        cause: &'static str,
        cause_ref: Option<Uuid>,
    ) -> Result<ResumeRecalcul, EntDbError> {
        if !CAUSES_RECTIFICATION.contains(&cause) {
            return Err(invalide("cause de rectification inconnue"));
        }
        let debut = bornes_mois(du).0;
        let fin = bornes_mois(au).1 - Duration::days(1);
        let rows = self.calculer_demi_journees(etab, eleve, debut, fin).await?;
        let mut publies: Vec<MetierEvent> = Vec::new();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut modifiees = 0u64;
        for r in &rows {
            modifiees += sqlx::query(
                "INSERT INTO ent_asd_demi_journee
                    (etablissement_id, eleve_id, jour, periode, minutes_manquees, minutes_non_justifiees,
                     seances_manquees, seances_non_justifiees, minutes_attendues, regle_valide_du, regle_id,
                     definition, seuil_minutes, coupure, coupure_source, calendrier_source, compte,
                     non_justifiee, raison)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
                 ON CONFLICT (etablissement_id, eleve_id, jour, periode) DO UPDATE SET
                    minutes_manquees = EXCLUDED.minutes_manquees,
                    minutes_non_justifiees = EXCLUDED.minutes_non_justifiees,
                    seances_manquees = EXCLUDED.seances_manquees,
                    seances_non_justifiees = EXCLUDED.seances_non_justifiees,
                    minutes_attendues = EXCLUDED.minutes_attendues,
                    regle_valide_du = EXCLUDED.regle_valide_du, regle_id = EXCLUDED.regle_id,
                    definition = EXCLUDED.definition, seuil_minutes = EXCLUDED.seuil_minutes,
                    coupure = EXCLUDED.coupure, coupure_source = EXCLUDED.coupure_source,
                    calendrier_source = EXCLUDED.calendrier_source, compte = EXCLUDED.compte,
                    non_justifiee = EXCLUDED.non_justifiee, raison = EXCLUDED.raison, calcule_le = now()
                  WHERE (ent_asd_demi_journee.minutes_manquees, ent_asd_demi_journee.minutes_non_justifiees,
                         ent_asd_demi_journee.seances_manquees, ent_asd_demi_journee.seances_non_justifiees,
                         ent_asd_demi_journee.minutes_attendues, ent_asd_demi_journee.regle_valide_du,
                         ent_asd_demi_journee.regle_id, ent_asd_demi_journee.definition,
                         ent_asd_demi_journee.seuil_minutes, ent_asd_demi_journee.coupure,
                         ent_asd_demi_journee.coupure_source, ent_asd_demi_journee.calendrier_source,
                         ent_asd_demi_journee.compte, ent_asd_demi_journee.non_justifiee, ent_asd_demi_journee.raison)
                        IS DISTINCT FROM
                        (EXCLUDED.minutes_manquees, EXCLUDED.minutes_non_justifiees, EXCLUDED.seances_manquees,
                         EXCLUDED.seances_non_justifiees, EXCLUDED.minutes_attendues, EXCLUDED.regle_valide_du,
                         EXCLUDED.regle_id, EXCLUDED.definition, EXCLUDED.seuil_minutes, EXCLUDED.coupure,
                         EXCLUDED.coupure_source, EXCLUDED.calendrier_source, EXCLUDED.compte,
                         EXCLUDED.non_justifiee, EXCLUDED.raison)",
            )
            .bind(etab)
            .bind(r.eleve_id)
            .bind(r.jour)
            .bind(r.periode.as_str())
            .bind(r.minutes_manquees as i32)
            .bind(r.minutes_non_justifiees as i32)
            .bind(r.seances_manquees as i32)
            .bind(r.seances_non_justifiees as i32)
            .bind(r.minutes_attendues.map(|m| m as i32))
            .bind(r.regle_valide_du)
            .bind(r.regle_id)
            .bind(&r.definition)
            .bind(r.seuil_minutes)
            .bind(r.coupure)
            .bind(r.coupure_source)
            .bind(r.calendrier_source)
            .bind(r.compte)
            .bind(r.non_justifiee)
            .bind(r.raison)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
        }
        // Lignes devenues sans objet (épisode retiré, séance annulée) : supprimées de la matérialisation
        // (la table est recalculable ; l'historique est dans les épisodes).
        let (e_ids, j_ids, p_ids): (Vec<Uuid>, Vec<NaiveDate>, Vec<String>) = rows
            .iter()
            .map(|r| (r.eleve_id, r.jour, r.periode.as_str().to_string()))
            .fold((vec![], vec![], vec![]), |mut acc, (e, j, p)| {
                acc.0.push(e);
                acc.1.push(j);
                acc.2.push(p);
                acc
            });
        let supprimees = sqlx::query(
            "DELETE FROM ent_asd_demi_journee d
              WHERE d.etablissement_id = $1 AND d.jour BETWEEN $2 AND $3
                AND ($4::uuid IS NULL OR d.eleve_id = $4)
                AND NOT EXISTS (SELECT 1 FROM UNNEST($5::uuid[], $6::date[], $7::text[]) AS n(e, j, p)
                                 WHERE n.e = d.eleve_id AND n.j = d.jour AND n.p = d.periode)",
        )
        .bind(etab)
        .bind(debut)
        .bind(fin)
        .bind(eleve)
        .bind(&e_ids)
        .bind(&j_ids)
        .bind(&p_ids)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();

        // Mois déjà arrêtés : seul l'écart avec le dernier chiffre connu est inscrit.
        let arretes: Vec<(NaiveDate,)> = sqlx::query_as(
            "SELECT mois FROM ent_asd_mois_cloture WHERE etablissement_id = $1 AND mois BETWEEN $2 AND $3 ORDER BY mois",
        )
        .bind(etab)
        .bind(debut)
        .bind(fin)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut rectifications = 0usize;
        for (mois,) in arretes {
            rectifications += self
                .rectifier_mois_tx(
                    &mut tx,
                    etab,
                    eleve,
                    mois,
                    &rows,
                    cause,
                    cause_ref,
                    &mut publies,
                )
                .await?;
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&publies);
        Ok(ResumeRecalcul {
            du: debut,
            au: fin,
            lignes_modifiees: modifiees,
            lignes_supprimees: supprimees,
            lignes_total: rows.len(),
            rectifications,
        })
    }

    /// Compare le décompte recalculé d'un mois arrêté au dernier chiffre connu (figé, ou dernière
    /// rectification) et inscrit chaque écart. Idempotent.
    #[allow(clippy::too_many_arguments)]
    async fn rectifier_mois_tx(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        eleve: Option<Uuid>,
        mois: NaiveDate,
        rows: &[DemiJourneeCalculee],
        cause: &'static str,
        cause_ref: Option<Uuid>,
        publies: &mut Vec<MetierEvent>,
    ) -> Result<usize, EntDbError> {
        let suivant = mois_suivant(mois);
        let mut nouveau: BTreeMap<Uuid, (i32, i32)> = BTreeMap::new();
        for r in rows
            .iter()
            .filter(|r| r.jour >= mois && r.jour < suivant && r.compte)
        {
            let e = nouveau.entry(r.eleve_id).or_default();
            e.0 += 1;
            if r.non_justifiee {
                e.1 += 1;
            }
        }
        let figes: Vec<(Uuid, i32, i32, i32)> = sqlx::query_as(
            "SELECT eleve_id, demi_journees, non_justifiees, seuil_signalement FROM ent_asd_mois_arrete
              WHERE etablissement_id = $1 AND mois = $2 AND ($3::uuid IS NULL OR eleve_id = $3)",
        )
        .bind(etab)
        .bind(mois)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let figes: HashMap<Uuid, (i32, i32, i32)> = figes
            .into_iter()
            .map(|(e, a, b, c)| (e, (a, b, c)))
            .collect();
        let ids: BTreeSet<Uuid> = nouveau.keys().chain(figes.keys()).copied().collect();
        let seuil_defaut: i32 = sqlx::query_scalar(
            "SELECT COALESCE((SELECT seuil_signalement_demi_journees FROM ent_asd_regle
                               WHERE etablissement_id = $1 AND valide_du <= $2
                               ORDER BY valide_du DESC LIMIT 1), 4)",
        )
        .bind(etab)
        .bind(suivant - Duration::days(1))
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut n = 0;
        for e in ids {
            let (apres_dj, apres_nj) = nouveau.get(&e).copied().unwrap_or((0, 0));
            let (fige_dj, fige_nj, seuil) = figes.get(&e).copied().unwrap_or((0, 0, seuil_defaut));
            let derniere: Option<(i32, i32)> = sqlx::query_as(
                "SELECT apres_demi_journees, apres_non_justifiees FROM ent_asd_rectification
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND mois = $3 ORDER BY cree_le DESC, id DESC LIMIT 1",
            )
            .bind(etab)
            .bind(e)
            .bind(mois)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let (avant_dj, avant_nj) = derniere.unwrap_or((fige_dj, fige_nj));
            if (avant_dj, avant_nj) == (apres_dj, apres_nj) {
                continue;
            }
            let touche = (avant_nj >= seuil) != (apres_nj >= seuil);
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_asd_rectification
                    (id, etablissement_id, eleve_id, mois, avant_demi_journees, avant_non_justifiees,
                     apres_demi_journees, apres_non_justifiees, cause, cause_ref_id, touche_signalement)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            )
            .bind(id)
            .bind(etab)
            .bind(e)
            .bind(mois)
            .bind(avant_dj)
            .bind(avant_nj)
            .bind(apres_dj)
            .bind(apres_nj)
            .bind(cause)
            .bind(cause_ref)
            .bind(touche)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            publies.push(
                tx.publier_avec(
                    MetierEventDraft {
                        etablissement_id: etab,
                        type_name: evenements::RECTIFICATION.to_string(),
                        aggregate: AggregateKind::Absence,
                        aggregate_id: id,
                        eleve_id: Some(e),
                        payload: json!({
                            "mois": mois,
                            "avant_non_justifiees": avant_nj,
                            "apres_non_justifiees": apres_nj,
                            "touche_signalement": touche,
                            "cause": cause,
                        }),
                        actor_user_id: None,
                    },
                    Causalite::default(),
                )
                .await?,
            );
            n += 1;
        }
        Ok(n)
    }

    /// Après une requalification ou une correction : si le jour touché est dans un mois arrêté, le
    /// décompte de l'élève est recalculé et l'écart inscrit. Sans mois arrêté : rien (le décompte
    /// courant se recalcule à la lecture). Ne fait jamais échouer l'opération appelante.
    pub(crate) async fn rectifier_apres_changement(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
        jour_debut: NaiveDate,
        jour_fin: NaiveDate,
        cause: &'static str,
        cause_ref: Option<Uuid>,
    ) {
        let (d, f) = (
            bornes_mois(jour_debut).0,
            bornes_mois(jour_fin).1 - Duration::days(1),
        );
        let arrete: Result<Option<(NaiveDate,)>, EntDbError> = async {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query_as(
                "SELECT mois FROM ent_asd_mois_cloture WHERE etablissement_id = $1 AND mois BETWEEN $2 AND $3 LIMIT 1",
            )
            .bind(etab)
            .bind(d)
            .bind(f)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)
        }
        .await;
        if matches!(arrete, Ok(Some(_))) {
            if let Err(e) = self
                .recalculer_decompte(etab, Some(eleve_id), d, f, cause, cause_ref)
                .await
            {
                tracing::warn!(error = %e, "rectification du décompte non inscrite");
            }
        }
    }

    // ── Lecture ───────────────────────────────────────────────────────────────────────────────

    /// Demi-journées d'un élève sur `[du, au]`, **avec leur explication** (AS-170). Mêmes droits que la
    /// lecture des absences (famille : son enfant).
    pub async fn decompte_eleve_explique_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<(DemiJourneeCalculee, String)>, EntDbError> {
        self.exiger_eleve_existe(etab, eleve_id).await?;
        if au < du || (au - du).num_days() > 400 {
            return Err(invalide("période invalide"));
        }
        if !self
            .actor_can_read_absences(etab, acteur, role, eleve_id)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let rows = self
            .calculer_demi_journees(etab, Some(eleve_id), du, au)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let t = expliquer(&r);
                (r, t)
            })
            .collect())
    }

    // ── Arrêt du mois ─────────────────────────────────────────────────────────────────────────

    /// Arrête un mois **terminé** (§13.2, AS-172) : recalcule, fige le décompte de chaque élève, fige
    /// les versions de règle qui ont servi. Idempotent : un mois déjà arrêté n'est jamais réécrit.
    /// Réservé à la direction (`assiduite.regle_gerer`) ; le tick système passe par [`Self::arreter_mois_systeme`].
    pub async fn arreter_mois_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        mois: NaiveDate,
    ) -> Result<ArretMois, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteRegleGerer)
            .await?;
        self.arreter_mois(etab, mois, Some(acteur)).await
    }

    pub(crate) async fn arreter_mois(
        &self,
        etab: Uuid,
        mois: NaiveDate,
        par: Option<Uuid>,
    ) -> Result<ArretMois, EntDbError> {
        if !est_premier(mois) {
            return Err(invalide("le mois est identifié par son premier jour"));
        }
        let suivant = mois_suivant(mois);
        let aujourdhui = self.aujourdhui_etablissement(etab).await?;
        if suivant > aujourdhui {
            // Un mois n'est arrêté que le 1er du mois suivant, au jour métier de l'établissement.
            return Err(EntDbError::Domain(domain_eleve::DomainError::Conflit));
        }
        if let Some(deja) = self.mois_arrete(etab, mois).await? {
            return Ok(ArretMois {
                mois,
                arrete_maintenant: false,
                eleves: deja,
            });
        }
        let dernier = suivant - Duration::days(1);
        let rows = self
            .calculer_demi_journees(etab, None, mois, dernier)
            .await?;
        // La matérialisation du mois est écrite avant le gel (l'explication et le décompte concordent).
        self.recalculer_decompte(etab, None, mois, dernier, "recalcul", None)
            .await?;
        let mut totaux: BTreeMap<Uuid, (i32, i32)> = BTreeMap::new();
        for r in rows.iter().filter(|r| r.compte) {
            let e = totaux.entry(r.eleve_id).or_default();
            e.0 += 1;
            if r.non_justifiee {
                e.1 += 1;
            }
        }
        let mut publies = Vec::new();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let regle: Option<(Uuid, NaiveDate, i32)> = sqlx::query_as(
            "SELECT id, valide_du, seuil_signalement_demi_journees FROM ent_asd_regle
              WHERE etablissement_id = $1 AND valide_du <= $2 ORDER BY valide_du DESC LIMIT 1",
        )
        .bind(etab)
        .bind(dernier)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (regle_id, regle_du, seuil) = match regle {
            Some((id, du, s)) => (Some(id), du, s),
            None => (None, defaut_du(), 4),
        };
        let pose = sqlx::query(
            "INSERT INTO ent_asd_mois_cloture (etablissement_id, mois, arrete_par, eleves)
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        )
        .bind(etab)
        .bind(mois)
        .bind(par)
        .bind(totaux.len() as i32)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if pose == 0 {
            // Un arrêt concurrent est passé avant : on ne réécrit rien.
            tx.commit().await.map_err(db)?;
            let deja = self.mois_arrete(etab, mois).await?.unwrap_or_default();
            return Ok(ArretMois {
                mois,
                arrete_maintenant: false,
                eleves: deja,
            });
        }
        for (e, (dj, nj)) in &totaux {
            sqlx::query(
                "INSERT INTO ent_asd_mois_arrete
                    (etablissement_id, eleve_id, mois, demi_journees, non_justifiees, regle_valide_du, regle_id,
                     seuil_signalement)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
            )
            .bind(etab)
            .bind(e)
            .bind(mois)
            .bind(dj)
            .bind(nj)
            .bind(regle_du)
            .bind(regle_id)
            .bind(seuil)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        // Les versions de règle qui ont servi à ce mois ne changent plus jamais (R-AS15).
        sqlx::query(
            "UPDATE ent_asd_regle SET figee_le = now()
              WHERE etablissement_id = $1 AND figee_le IS NULL AND valide_du <= $3
                AND valide_du >= COALESCE((SELECT max(valide_du) FROM ent_asd_regle
                                            WHERE etablissement_id = $1 AND valide_du <= $2), $2)",
        )
        .bind(etab)
        .bind(mois)
        .bind(dernier)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        publies.push(
            tx.publier_avec(
                MetierEventDraft {
                    etablissement_id: etab,
                    type_name: evenements::MOIS_ARRETE.to_string(),
                    aggregate: AggregateKind::Absence,
                    aggregate_id: Uuid::new_v4(),
                    eleve_id: None,
                    payload: json!({ "mois": mois, "eleves": totaux.len() }),
                    actor_user_id: par.map(|u| u.to_string()),
                },
                Causalite::default(),
            )
            .await?,
        );
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&publies);
        Ok(ArretMois {
            mois,
            arrete_maintenant: true,
            eleves: totaux
                .into_iter()
                .map(|(e, (dj, nj))| EleveMoisArrete {
                    eleve_id: e,
                    demi_journees: dj,
                    non_justifiees: nj,
                    retards: 0,
                    regle_valide_du: regle_du,
                    seuil_signalement: seuil,
                })
                .collect(),
        })
    }

    /// Décompte figé d'un mois arrêté, `None` s'il ne l'est pas.
    pub(crate) async fn mois_arrete(
        &self,
        etab: Uuid,
        mois: NaiveDate,
    ) -> Result<Option<Vec<EleveMoisArrete>>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let existe: Option<(NaiveDate,)> = sqlx::query_as(
            "SELECT mois FROM ent_asd_mois_cloture WHERE etablissement_id = $1 AND mois = $2",
        )
        .bind(etab)
        .bind(mois)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if existe.is_none() {
            return Ok(None);
        }
        let l: Vec<(Uuid, i32, i32, i32, NaiveDate, i32)> = sqlx::query_as(
            "SELECT eleve_id, demi_journees, non_justifiees, retards, regle_valide_du, seuil_signalement
               FROM ent_asd_mois_arrete WHERE etablissement_id = $1 AND mois = $2 ORDER BY eleve_id",
        )
        .bind(etab)
        .bind(mois)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(Some(
            l.into_iter()
                .map(|(e, dj, nj, r, du, s)| EleveMoisArrete {
                    eleve_id: e,
                    demi_journees: dj,
                    non_justifiees: nj,
                    retards: r,
                    regle_valide_du: du,
                    seuil_signalement: s,
                })
                .collect(),
        ))
    }

    /// Tâche de fond : arrête, pour chaque établissement, les mois terminés non arrêtés (au plus trois
    /// derniers mois), au **jour métier** de chaque établissement.
    pub async fn assiduite_arret_mois_tache(&self) -> Result<usize, EntDbError> {
        let etabs: Vec<(Uuid,)> = crate::tenant_tx::lire_par_etablissement(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("assiduite_arret_mois"),
            "SELECT DISTINCT etablissement_id FROM ent_asd_absence_seance",
        )
        .await?;
        let mut n = 0;
        for (etab,) in etabs {
            match self.arreter_mois_systeme(etab).await {
                Ok(k) => n += k,
                Err(e) => tracing::warn!(error = %e, "arrêt du mois d'assiduité non exécuté"),
            }
        }
        Ok(n)
    }

    /// Arrêt automatique (système) des mois terminés d'un établissement, trois mois en arrière au plus.
    pub async fn arreter_mois_systeme(&self, etab: Uuid) -> Result<usize, EntDbError> {
        let aujourdhui = self.aujourdhui_etablissement(etab).await?;
        let courant = bornes_mois(aujourdhui).0;
        let mut n = 0;
        let mut m = courant;
        for _ in 0..3 {
            m = bornes_mois(m - Duration::days(1)).0;
            if self.arreter_mois(etab, m, None).await?.arrete_maintenant {
                n += 1;
            }
        }
        Ok(n)
    }

    // ── Rectifications ────────────────────────────────────────────────────────────────────────

    /// Rectifications postérieures à l'arrêt d'un mois (lecture : `assiduite.signaler`, CPE et direction).
    pub async fn rectifications_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        mois: NaiveDate,
    ) -> Result<Vec<Rectification>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSignaler)
            .await?;
        if !est_premier(mois) {
            return Err(invalide("le mois est identifié par son premier jour"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as(
            "SELECT id, eleve_id, mois, avant_demi_journees, avant_non_justifiees, apres_demi_journees,
                    apres_non_justifiees, cause, cause_ref_id, touche_signalement, cree_le, vue_par
               FROM ent_asd_rectification WHERE etablissement_id = $1 AND mois = $2 ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(mois)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)
    }

    /// La direction ou le CPE note qu'il a **vu** une rectification (une seule fois).
    pub async fn rectification_vue_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        rectification_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AssiduiteSignaler)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = sqlx::query(
            "UPDATE ent_asd_rectification SET vue_par = $3, vue_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND vue_par IS NULL",
        )
        .bind(etab)
        .bind(rectification_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        tx.commit().await.map_err(db)?;
        let _ = Role::Admin; // (le rôle ne sert qu'au contrôle d'action ci-dessus)
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn seance(d: (u32, u32), f: (u32, u32), justifiee: bool) -> Contribution {
        Contribution::Seance {
            debut: t(d.0, d.1),
            fin: t(f.0, f.1),
            justifiee,
        }
    }

    fn params(definition: Definition, seuil: i64) -> ParamsJour {
        ParamsJour {
            definition,
            seuil_minutes: seuil,
            coupure: t(12, 0),
        }
    }

    const OUVERT: [Ouverture; 2] = [Ouverture::Ouverte; 2];

    #[test]
    fn definition_minutes_identique_a_l_existant() {
        // Équivalence avec `deriver_demi_journees` (AS0 à AS3) pour la définition « minutes ».
        let regle = crate::assiduite::RegleAssiduite::default();
        let cas = vec![
            vec![
                seance((8, 0), (9, 0), false),
                seance((9, 0), (10, 0), false),
            ],
            vec![seance((11, 30), (12, 30), false)],
            vec![seance((8, 0), (8, 29), false)],
            vec![
                seance((8, 0), (8, 30), true),
                seance((13, 0), (14, 0), false),
            ],
            vec![Contribution::DemiJournee {
                periode: Periode::Matin,
                justifiee: false,
            }],
        ];
        for c in cas {
            let ancien = crate::assiduite::deriver_demi_journees(&c, &regle);
            let nouveau = deriver_jour(&c, &[], &params(Definition::Minutes, 30), OUVERT);
            assert_eq!(ancien.len(), nouveau.len(), "{c:?}");
            for (a, n) in ancien.iter().zip(&nouveau) {
                assert_eq!(
                    (a.periode, a.minutes, a.minutes_non_justifiees),
                    (n.periode, n.minutes, n.minutes_non_justifiees)
                );
                assert_eq!(
                    (a.comptee, a.non_justifiee),
                    (n.compte, n.non_justifiee),
                    "{c:?}"
                );
            }
        }
    }

    #[test]
    fn une_seance_suffit_quand_la_regle_le_dit() {
        let d = deriver_jour(
            &[seance((8, 0), (8, 10), false)],
            &[],
            &params(Definition::UneSeance, 30),
            OUVERT,
        );
        assert_eq!(d.len(), 1);
        assert!(d[0].compte && d[0].non_justifiee && d[0].raison == Raison::Compte);
        // Même séance de 10 minutes : sous le seuil avec la définition « minutes ».
        let d = deriver_jour(
            &[seance((8, 0), (8, 10), false)],
            &[],
            &params(Definition::Minutes, 30),
            OUVERT,
        );
        assert!(!d[0].compte && d[0].raison == Raison::SousLeSeuil);
    }

    #[test]
    fn jour_mixte_chaque_seance_garde_son_etat() {
        // Une séance justifiée et une non justifiée le matin : la demi-journée compte, non justifiée,
        // car l'une des séances ne l'est pas (et non « refusée » pour tout le jour).
        let d = deriver_jour(
            &[seance((8, 0), (9, 0), true), seance((9, 0), (10, 0), false)],
            &[],
            &params(Definition::UneSeance, 30),
            OUVERT,
        );
        assert!(d[0].compte && d[0].non_justifiee);
        assert_eq!((d[0].seances, d[0].seances_non_justifiees), (2, 1));
        // L'après-midi est séparée : tout justifié → compte, justifiée.
        let d = deriver_jour(
            &[
                seance((8, 0), (9, 0), false),
                seance((14, 0), (15, 0), true),
            ],
            &[],
            &params(Definition::UneSeance, 30),
            OUVERT,
        );
        assert!(d[0].non_justifiee && d[1].compte && !d[1].non_justifiee);
    }

    #[test]
    fn definition_entiere_exige_tout_le_cours_attendu() {
        let attendues = [
            (t(8, 0), t(9, 0)),
            (t(9, 0), t(10, 0)),
            (t(10, 0), t(11, 0)),
        ];
        let p = params(Definition::Entiere, 30);
        let d = deriver_jour(&[seance((8, 0), (9, 0), false)], &attendues, &p, OUVERT);
        assert!(!d[0].compte && d[0].raison == Raison::DemiJourneeIncomplete);
        assert_eq!(d[0].minutes_attendues, Some(180));
        let d = deriver_jour(
            &[
                seance((8, 0), (9, 0), false),
                seance((9, 0), (10, 0), false),
                seance((10, 0), (11, 0), false),
            ],
            &attendues,
            &p,
            OUVERT,
        );
        assert!(d[0].compte && d[0].non_justifiee);
        // Absence à la journée de l'ancien circuit : la demi-journée entière.
        let d = deriver_jour(
            &[Contribution::DemiJournee {
                periode: Periode::ApresMidi,
                justifiee: true,
            }],
            &[],
            &p,
            OUVERT,
        );
        assert!(d[0].compte && !d[0].non_justifiee);
    }

    #[test]
    fn calendrier_ferme_donne_zero() {
        // Des vacances déclarées : la demi-journée est lue mais ne compte pas (CA-25).
        let fermees = [Ouverture::SansCours, Ouverture::SansCours];
        for def in [
            Definition::UneSeance,
            Definition::Minutes,
            Definition::Entiere,
        ] {
            let d = deriver_jour(
                &[seance((8, 0), (11, 0), false)],
                &[],
                &params(def, 30),
                fermees,
            );
            assert_eq!(d.len(), 1);
            assert!(
                !d[0].compte && !d[0].non_justifiee && d[0].raison == Raison::SansCours,
                "{def:?}"
            );
        }
        // Mercredi après-midi sans cours : le matin compte, l'après-midi non.
        let d = deriver_jour(
            &[
                seance((8, 0), (9, 0), false),
                seance((14, 0), (15, 0), false),
            ],
            &[],
            &params(Definition::UneSeance, 30),
            [Ouverture::Ouverte, Ouverture::SansCours],
        );
        assert!(d[0].compte && !d[1].compte);
        let d = deriver_jour(
            &[seance((8, 0), (9, 0), false)],
            &[],
            &params(Definition::UneSeance, 30),
            [Ouverture::HorsAnnee; 2],
        );
        assert_eq!(d[0].raison, Raison::HorsAnnee);
    }

    #[test]
    fn la_coupure_vient_des_parametres_du_jour() {
        let mut p = params(Definition::Minutes, 30);
        p.coupure = t(12, 30);
        let d = deriver_jour(&[seance((11, 30), (12, 30), false)], &[], &p, OUVERT);
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].periode, d[0].minutes), (Periode::Matin, 60));
    }

    #[test]
    fn l_explication_est_codee_et_sans_motif() {
        let d = DemiJourneeCalculee {
            eleve_id: Uuid::nil(),
            jour: NaiveDate::from_ymd_opt(2026, 9, 8).unwrap(),
            periode: Periode::Matin,
            minutes_manquees: 120,
            minutes_non_justifiees: 120,
            seances_manquees: 2,
            seances_non_justifiees: 2,
            minutes_attendues: None,
            regle_valide_du: NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            regle_id: None,
            definition: "une_seance".into(),
            seuil_minutes: 30,
            coupure: t(12, 0),
            coupure_source: "regle",
            calendrier_source: "calendrier",
            compte: true,
            non_justifiee: true,
            raison: "compte",
        };
        let e = expliquer(&d);
        assert_eq!(
            e,
            "mardi 08/09/2026 matin compte, non justifiée : 2 séances manquées (120 min), dont 2 non justifiées ; \
             règle : au moins une séance manquée, version du 01/09/2026."
        );
        let sans = DemiJourneeCalculee {
            compte: false,
            non_justifiee: false,
            raison: "sans_cours",
            ..d
        };
        assert!(expliquer(&sans).contains("aucun cours selon le calendrier"));
    }
}
