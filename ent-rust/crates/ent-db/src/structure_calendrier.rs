//! Socle 5, lot S3 : calendrier de l'établissement, semaines A/B, jours cibles, fermetures
//! exceptionnelles et port `jour(établissement, date)` (`05_STRUCTURE.md` §9.3, STR-23 à STR-30,
//! STR-93, STR-94, R-S1, R-S2, R-S19).
//!
//! Règles du lot :
//! * **droits côté serveur** : lecture = `referentiel.lire`, écriture = `referentiel.gerer`
//!   (administrateur, direction, administratif) ; **fermeture exceptionnelle : direction et
//!   administrateur seulement** (STR-25 « Dir ») ; l'action dédiée `calendrier.gerer` du §13.2
//!   n'existe pas encore (même raccourci que le lot S2) ;
//! * **cloisonnement** : l'établissement vient de l'acteur ; écritures par `TenantTx` (RLS forcée) ;
//!   toutes les requêtes filtrent aussi sur `etablissement_id` ;
//! * **jours fériés calculés** par le code (`structure_feries`), jamais téléchargés ;
//! * **aucune dépendance réseau** : le calendrier officiel des vacances n'est lu que dans
//!   `ref_calendrier_officiel` (vide tant qu'une source vérifiée n'y est pas saisie) ; la saisie
//!   manuelle fonctionne seule (point ouvert documenté au §24 de 05_STRUCTURE.md) ;
//! * **journal en ajout seul** (`ent_structure_journal`) et historique des fermetures
//!   (`ent_fermeture_historique`) ; un changement de fermeture = un événement = un message ;
//! * le **port** [`calendrier`] est la seule réponse à « y a-t-il cours ce jour-là ? ».
//!
//! Les événements `referentiel.*` portent des identifiants, des codes et des dates : jamais de
//! libellé libre, jamais de motif.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc, Weekday};
use ent_auth::{Action, Role};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::{journaliser, net, publier, requis, structure as port_etab};
use crate::structure_feries::{est_alsace_moselle, feries_entre, ponts_possibles, Territoire};
use crate::tenant_tx::TenantTx;

pub const TYPES_EVENEMENT: [&str; 14] = [
    "vacances",
    "ferie",
    "pont",
    "fermeture",
    "fermeture_exceptionnelle",
    "journee_banalisee",
    "journee_pedagogique",
    "rattrapage",
    "examen",
    "stage",
    "periode_entreprise",
    "rentree",
    "pre_rentree",
    "evenement",
];
pub const EFFETS: [&str; 5] = [
    "sans_cours",
    "sans_eleves",
    "accueil_seulement",
    "cours_modifies",
    "informatif",
];
/// Services d'une fermeture exceptionnelle (R-S1).
pub const SERVICES_FERMETURE: [&str; 6] = [
    "cours",
    "accueil_eleves",
    "restauration",
    "internat",
    "transports",
    "administration",
];
/// Services dont on déclare les jours d'ouverture (STR-93, R-S2).
pub const SERVICES: [&str; 7] = [
    "secretariat",
    "intendance",
    "restauration",
    "internat",
    "accueil",
    "cdi",
    "vie_scolaire",
];
const JOURS: [&str; 7] = ["lun", "mar", "mer", "jeu", "ven", "sam", "dim"];
/// Semaine type par défaut (même valeur que la colonne `jours_ouvres` du profil, migration 332).
const JOURS_OUVRES_DEFAUT: &str = r#"{"lun":["matin","apres_midi"],"mar":["matin","apres_midi"],"mer":["matin"],"jeu":["matin","apres_midi"],"ven":["matin","apres_midi"]}"#;
/// Ordre de présentation du type d'une journée quand plusieurs événements s'appliquent.
const PRIORITE_TYPES: [&str; 14] = [
    "fermeture_exceptionnelle",
    "fermeture",
    "ferie",
    "vacances",
    "pont",
    "journee_banalisee",
    "journee_pedagogique",
    "examen",
    "stage",
    "periode_entreprise",
    "rentree",
    "pre_rentree",
    "rattrapage",
    "evenement",
];
/// Garde-fou : plage maximale demandée d'un coup au port (deux ans).
const PLAGE_MAX_JOURS: i64 = 800;

fn cle_jour(w: Weekday) -> &'static str {
    JOURS[w.num_days_from_monday() as usize]
}

// ───────────────────────── Types publics ─────────────────────────

/// Cible d'une question « y a-t-il cours ? » : ce que l'appelant connaît. Une classe complète
/// son niveau et son site, un groupe sa classe (déduits par le port). Vide = l'établissement.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cible {
    pub site_id: Option<Uuid>,
    pub niveau_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
}

/// Une cible d'événement : exactement un champ renseigné.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CibleEvenement {
    pub site_id: Option<Uuid>,
    pub niveau_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub service: Option<String>,
}

impl CibleEvenement {
    fn est_service(&self) -> bool {
        self.service.is_some()
    }
    fn nb_champs(&self) -> usize {
        usize::from(self.site_id.is_some())
            + usize::from(self.niveau_id.is_some())
            + usize::from(self.classe_id.is_some())
            + usize::from(self.groupe_id.is_some())
            + usize::from(self.service.is_some())
    }
    fn concerne(&self, c: &Cible) -> bool {
        (self.site_id.is_some() && self.site_id == c.site_id)
            || (self.niveau_id.is_some() && self.niveau_id == c.niveau_id)
            || (self.classe_id.is_some() && self.classe_id == c.classe_id)
            || (self.groupe_id.is_some() && self.groupe_id == c.groupe_id)
    }
}

/// Événement du calendrier tel que les lecteurs le voient.
#[derive(Debug, Clone, Serialize)]
pub struct Evenement {
    pub id: Uuid,
    pub annee_id: Uuid,
    #[serde(rename = "type")]
    pub type_evenement: String,
    pub libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub du_demi: Option<String>,
    pub au_demi: Option<String>,
    pub effet: String,
    pub origine: String,
    pub ref_officiel_id: Option<Uuid>,
    pub ref_version: Option<String>,
    pub publie_familles: bool,
    pub publie_personnels: bool,
    pub motif: Option<String>,
    pub jour_reference: Option<String>,
    pub services_touches: Vec<String>,
    pub conditionnelle: bool,
    pub levee_prevue_le: Option<DateTime<Utc>>,
    pub internes_presents: bool,
    pub version: i32,
    pub annule_le: Option<DateTime<Utc>>,
    pub annule_motif: Option<String>,
    pub cree_le: DateTime<Utc>,
    pub cibles: Vec<CibleEvenement>,
}

#[derive(Debug, FromRow)]
struct LigneEvenement {
    id: Uuid,
    annee_id: Uuid,
    type_evenement: String,
    libelle: String,
    du: NaiveDate,
    au: NaiveDate,
    du_demi: Option<String>,
    au_demi: Option<String>,
    effet: String,
    origine: String,
    ref_officiel_id: Option<Uuid>,
    ref_version: Option<String>,
    publie_familles: bool,
    publie_personnels: bool,
    motif: Option<String>,
    jour_reference: Option<String>,
    services_touches: Vec<String>,
    conditionnelle: bool,
    levee_prevue_le: Option<DateTime<Utc>>,
    internes_presents: bool,
    version: i32,
    annule_le: Option<DateTime<Utc>>,
    annule_motif: Option<String>,
    cree_le: DateTime<Utc>,
}

/// Une demi-journée telle que `jour()` la résout.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
pub struct DemiJournee {
    /// Des cours sont assurés.
    pub cours: bool,
    /// Des élèves sont accueillis (cours ou accueil seulement).
    pub eleves: bool,
    /// Les personnels sont attendus.
    pub personnel: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvenementJour {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub type_evenement: String,
    pub libelle: String,
    pub effet: String,
    pub jour_reference: Option<String>,
    pub publie_familles: bool,
    pub publie_personnels: bool,
}

/// Réponse du port : l'état d'un jour pour une cible.
#[derive(Debug, Clone, Serialize)]
pub struct Jour {
    pub date: NaiveDate,
    pub annee_id: Option<Uuid>,
    /// Au moins une demi-journée avec cours.
    pub ouvre: bool,
    /// `cours`, `cours_modifie`, `non_travaille`, `hors_annee` ou le type de l'événement qui
    /// ferme la journée (`vacances`, `ferie`, `pont`, `fermeture_exceptionnelle`…).
    pub type_jour: String,
    pub matin: DemiJournee,
    pub apres_midi: DemiJournee,
    /// Étiquette de la semaine (`A`, `B`…), absente pendant les vacances ou hors règle.
    pub semaine_type: Option<String>,
    /// Rattrapage : la journée suit l'emploi du temps de ce jour de la semaine.
    pub jour_reference: Option<String>,
    /// Services fermés par une fermeture exceptionnelle (R-S1) : `restauration`, `internat`…
    pub services_fermes: Vec<String>,
    pub evenements: Vec<EvenementJour>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct JourService {
    pub date: NaiveDate,
    pub service: String,
    pub ouvert: bool,
    /// `week_end`, `ferie`, `fermeture`, `fermeture_exceptionnelle`, `service_ferme` ou absent.
    pub raison: Option<String>,
}

// ───────────────────────── Contexte et calcul pur ─────────────────────────

#[derive(Debug, Clone)]
pub struct AnneeLue {
    pub id: Uuid,
    pub libelle: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    pub cloturee: bool,
}

/// Tout ce qu'il faut pour répondre à `jour()` sans retourner en base.
#[derive(Debug, Clone)]
pub struct Contexte {
    /// Pour chaque jour de la semaine : (matin, après-midi) avec cours.
    jours_ouvres: HashMap<&'static str, (bool, bool)>,
    pub annees: Vec<AnneeLue>,
    pub evenements: Vec<Evenement>,
    /// (année, lundi) → étiquette.
    semaines: HashMap<(Uuid, NaiveDate), Option<String>>,
}

impl Contexte {
    /// Contexte construit à la main (tests purs, sans base).
    pub fn nouveau(
        jours_ouvres: &Value,
        annees: Vec<AnneeLue>,
        evenements: Vec<Evenement>,
        semaines: Vec<(Uuid, NaiveDate, Option<String>)>,
    ) -> Self {
        Self {
            jours_ouvres: lire_jours_ouvres(jours_ouvres),
            annees,
            evenements,
            semaines: semaines.into_iter().map(|(a, l, e)| ((a, l), e)).collect(),
        }
    }

    fn annee_du(&self, date: NaiveDate) -> Option<&AnneeLue> {
        self.annees
            .iter()
            .filter(|a| a.debut <= date && date <= a.fin)
            .max_by_key(|a| a.debut)
    }
}

fn lire_jours_ouvres(v: &Value) -> HashMap<&'static str, (bool, bool)> {
    let mut m = HashMap::new();
    for j in JOURS {
        let (mut matin, mut pm) = (false, false);
        if let Some(a) = v.get(j).and_then(Value::as_array) {
            for d in a.iter().filter_map(Value::as_str) {
                match d {
                    "matin" => matin = true,
                    "apres_midi" => pm = true,
                    _ => {}
                }
            }
        }
        m.insert(j, (matin, pm));
    }
    m
}

/// Cible complétée (classe → niveau et site, groupe → classe).
pub type CibleResolue = Cible;

/// L'événement touche-t-il la demi-journée du `date` ?
fn touche(e: &Evenement, date: NaiveDate) -> (bool, bool) {
    let matin = !(date == e.du && e.du_demi.as_deref() == Some("apres_midi"));
    let pm = !(date == e.au && e.au_demi.as_deref() == Some("matin"));
    (matin, pm)
}

fn restreint(d: &mut DemiJournee, effet: &str) {
    match effet {
        "sans_cours" => *d = DemiJournee::default(),
        "accueil_seulement" => d.cours = false,
        "sans_eleves" => {
            d.cours = false;
            d.eleves = false;
        }
        _ => {}
    }
}

/// Un événement de portée établissement/site/niveau/classe/groupe concerne-t-il la cible ?
fn s_applique(e: &Evenement, c: &Cible) -> bool {
    if e.cibles.iter().any(CibleEvenement::est_service) {
        return false;
    }
    e.cibles.is_empty() || e.cibles.iter().any(|x| x.concerne(c))
}

/// Calcule l'état d'un jour. **Fonction pure** : c'est elle que les tests exercent.
pub fn calculer_jour(ctx: &Contexte, date: NaiveDate, cible: &Cible) -> Jour {
    let Some(annee) = ctx.annee_du(date) else {
        return Jour {
            date,
            annee_id: None,
            ouvre: false,
            type_jour: "hors_annee".into(),
            matin: DemiJournee::default(),
            apres_midi: DemiJournee::default(),
            semaine_type: None,
            jour_reference: None,
            services_fermes: vec![],
            evenements: vec![],
        };
    };
    let actifs: Vec<&Evenement> = ctx
        .evenements
        .iter()
        .filter(|e| {
            e.annee_id == annee.id
                && e.annule_le.is_none()
                && e.du <= date
                && date <= e.au
                && s_applique(e, cible)
        })
        .collect();
    let (base_m, base_p) = ctx
        .jours_ouvres
        .get(cle_jour(date.weekday()))
        .copied()
        .unwrap_or((false, false));
    let mut matin = DemiJournee {
        cours: base_m,
        eleves: base_m,
        personnel: base_m,
    };
    let mut pm = DemiJournee {
        cours: base_p,
        eleves: base_p,
        personnel: base_p,
    };
    // Rattrapage : la journée suit l'emploi du temps d'un autre jour de la semaine.
    let mut jour_reference = None;
    if let Some(r) = actifs
        .iter()
        .filter(|e| e.type_evenement == "rattrapage")
        .max_by_key(|e| e.cree_le)
    {
        if let Some(reference) = r.jour_reference.as_deref() {
            let (rm, rp) = ctx
                .jours_ouvres
                .get(reference)
                .copied()
                .unwrap_or((false, false));
            let (tm, tp) = touche(r, date);
            if tm {
                matin = DemiJournee {
                    cours: rm,
                    eleves: rm,
                    personnel: rm,
                };
            }
            if tp {
                pm = DemiJournee {
                    cours: rp,
                    eleves: rp,
                    personnel: rp,
                };
            }
            jour_reference = Some(reference.to_string());
        }
    }
    let mut restrictifs: Vec<&str> = Vec::new();
    let mut modifie = false;
    let mut services_fermes: BTreeSet<String> = BTreeSet::new();
    for e in actifs.iter().filter(|e| e.type_evenement != "rattrapage") {
        let (tm, tp) = touche(e, date);
        if e.type_evenement == "fermeture_exceptionnelle" {
            // Source unique : les services touchés (l'effet n'en est que le résumé).
            for (touchee, demi) in [(tm, &mut matin), (tp, &mut pm)] {
                if !touchee {
                    continue;
                }
                if e.services_touches.iter().any(|s| s == "cours") {
                    demi.cours = false;
                }
                if e.services_touches.iter().any(|s| s == "accueil_eleves") {
                    demi.cours = false;
                    demi.eleves = false;
                }
                if e.services_touches.iter().any(|s| s == "administration") {
                    demi.personnel = false;
                }
            }
            for s in &e.services_touches {
                services_fermes.insert(s.clone());
            }
            restrictifs.push("fermeture_exceptionnelle");
            continue;
        }
        match e.effet.as_str() {
            "sans_cours" | "sans_eleves" | "accueil_seulement" => {
                if tm {
                    restreint(&mut matin, &e.effet);
                }
                if tp {
                    restreint(&mut pm, &e.effet);
                }
                restrictifs.push(e.type_evenement.as_str());
            }
            "cours_modifies" => modifie = true,
            _ => {}
        }
    }
    let ouvre = matin.cours || pm.cours;
    let type_jour = if let Some(t) = PRIORITE_TYPES
        .iter()
        .find(|t| restrictifs.contains(t))
        .copied()
    {
        t.to_string()
    } else if jour_reference.is_some() {
        "rattrapage".to_string()
    } else if ouvre && modifie {
        "cours_modifie".to_string()
    } else if ouvre {
        "cours".to_string()
    } else {
        "non_travaille".to_string()
    };
    let lundi = date - Duration::days(i64::from(date.weekday().num_days_from_monday()));
    Jour {
        date,
        annee_id: Some(annee.id),
        ouvre,
        type_jour,
        matin,
        apres_midi: pm,
        semaine_type: ctx.semaines.get(&(annee.id, lundi)).cloned().flatten(),
        jour_reference,
        services_fermes: services_fermes.into_iter().collect(),
        evenements: actifs
            .iter()
            .map(|e| EvenementJour {
                id: e.id,
                type_evenement: e.type_evenement.clone(),
                libelle: e.libelle.clone(),
                effet: e.effet.clone(),
                jour_reference: e.jour_reference.clone(),
                publie_familles: e.publie_familles,
                publie_personnels: e.publie_personnels,
            })
            .collect(),
    }
}

/// Service dont dépend la fermeture exceptionnelle (R-S1).
fn service_de_fermeture(service: &str) -> &'static str {
    match service {
        "restauration" => "restauration",
        "internat" => "internat",
        "accueil" => "accueil_eleves",
        _ => "administration",
    }
}

/// Jour ouvré **administratif** d'un service (R-S2) : distinct des jours de cours. Ouvert du
/// lundi au vendredi, fermé les jours fériés, pendant une fermeture de l'établissement
/// (`fermeture`, sans cible) ou une fermeture exceptionnelle qui touche ce service, et les jours
/// où un événement cible explicitement ce service avec l'effet `sans_cours`. Les vacances, ponts
/// et journées banalisées **ne ferment pas** les services. Seules les journées entières ferment.
pub fn calculer_jour_service(ctx: &Contexte, date: NaiveDate, service: &str) -> JourService {
    let ferme = |raison: &str| JourService {
        date,
        service: service.to_string(),
        ouvert: false,
        raison: Some(raison.to_string()),
    };
    if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
        return ferme("week_end");
    }
    let Some(annee) = ctx.annee_du(date) else {
        return JourService {
            date,
            service: service.to_string(),
            ouvert: true,
            raison: None,
        };
    };
    let mapped = service_de_fermeture(service);
    for e in ctx
        .evenements
        .iter()
        .filter(|e| e.annee_id == annee.id && e.annule_le.is_none() && e.du <= date && date <= e.au)
    {
        let (tm, tp) = touche(e, date);
        if !(tm && tp) {
            continue;
        }
        if e.cibles.iter().any(CibleEvenement::est_service) {
            if e.effet == "sans_cours"
                && e.cibles
                    .iter()
                    .any(|c| c.service.as_deref() == Some(service))
            {
                return ferme("service_ferme");
            }
            continue;
        }
        if !e.cibles.is_empty() {
            continue; // portée partielle (site, niveau…) : n'ouvre ni ne ferme un service
        }
        match e.type_evenement.as_str() {
            "ferie" if e.effet == "sans_cours" => return ferme("ferie"),
            "fermeture" if e.effet == "sans_cours" => return ferme("fermeture"),
            "fermeture_exceptionnelle" if e.services_touches.iter().any(|s| s == mapped) => {
                return ferme("fermeture_exceptionnelle")
            }
            _ => {}
        }
    }
    JourService {
        date,
        service: service.to_string(),
        ouvert: true,
        raison: None,
    }
}

// ───────────────────────── Semaines A/B (calcul pur) ─────────────────────────

/// Règle des types de semaine d'une année.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RegleSemaine {
    pub etiquettes: Vec<String>,
    /// `alternance_continue` : on alterne de semaine de cours en semaine de cours, les vacances
    /// ne comptent pas ; `reprise_apres_vacances` : après chaque vacances on repart de la
    /// première étiquette.
    pub mode: String,
    /// Lundi de la première semaine, qui porte la première étiquette.
    pub premiere_semaine: NaiveDate,
}

/// Étiquettes des semaines données (lundi, « semaine de vacances ? »), dans l'ordre.
pub fn etiquettes_semaines(
    semaines: &[(NaiveDate, bool)],
    regle: &RegleSemaine,
) -> Vec<Option<String>> {
    let n = regle.etiquettes.len().max(1);
    let (mut idx, mut apres_vacances, mut demarre) = (0usize, false, false);
    semaines
        .iter()
        .map(|(lundi, vacances)| {
            if *lundi < regle.premiere_semaine {
                return None;
            }
            if *vacances {
                apres_vacances = true;
                return None;
            }
            if regle.mode == "reprise_apres_vacances" && apres_vacances && demarre {
                idx = 0;
            }
            let e = regle.etiquettes.get(idx % n).cloned();
            idx += 1;
            demarre = true;
            apres_vacances = false;
            e
        })
        .collect()
}

fn lundi_de(d: NaiveDate) -> NaiveDate {
    d - Duration::days(i64::from(d.weekday().num_days_from_monday()))
}

// ───────────────────────── Chargement ─────────────────────────

const SELECT_EVT: &str = "id, annee_id, type AS type_evenement, libelle, du, au, du_demi, au_demi, effet, \
    origine, ref_officiel_id, ref_version, publie_familles, publie_personnels, motif, jour_reference, \
    services_touches, conditionnelle, levee_prevue_le, internes_presents, version, annule_le, \
    annule_motif, cree_le";

async fn lire_evenements(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Option<Uuid>,
    plage: Option<(NaiveDate, NaiveDate)>,
    id: Option<Uuid>,
    avec_annules: bool,
    verrou: bool,
) -> Result<Vec<Evenement>, EntDbError> {
    let sql = format!(
        "SELECT {SELECT_EVT} FROM ent_calendrier_evenement
          WHERE etablissement_id = $1
            AND ($2::uuid IS NULL OR annee_id = $2)
            AND ($3::date IS NULL OR au >= $3)
            AND ($4::date IS NULL OR du <= $4)
            AND ($5::uuid IS NULL OR id = $5)
            AND ($6 OR annule_le IS NULL)
          ORDER BY du, au, cree_le, id{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    let lignes: Vec<LigneEvenement> = sqlx::query_as(&sql)
        .bind(etab)
        .bind(annee)
        .bind(plage.map(|p| p.0))
        .bind(plage.map(|p| p.1))
        .bind(id)
        .bind(avec_annules)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
    let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
    type LigneCible = (
        Uuid,
        Option<Uuid>,
        Option<Uuid>,
        Option<Uuid>,
        Option<Uuid>,
        Option<String>,
    );
    let cibles: Vec<LigneCible> = sqlx::query_as(
        "SELECT evenement_id, site_id, niveau_id, classe_id, groupe_id, service
           FROM ent_calendrier_cible WHERE etablissement_id = $1 AND evenement_id = ANY($2)",
    )
    .bind(etab)
    .bind(&ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut par_evt: HashMap<Uuid, Vec<CibleEvenement>> = HashMap::new();
    for (e, site_id, niveau_id, classe_id, groupe_id, service) in cibles {
        par_evt.entry(e).or_default().push(CibleEvenement {
            site_id,
            niveau_id,
            classe_id,
            groupe_id,
            service,
        });
    }
    Ok(lignes
        .into_iter()
        .map(|l| {
            let mut cibles = par_evt.remove(&l.id).unwrap_or_default();
            cibles.sort();
            Evenement {
                id: l.id,
                annee_id: l.annee_id,
                type_evenement: l.type_evenement,
                libelle: l.libelle,
                du: l.du,
                au: l.au,
                du_demi: l.du_demi,
                au_demi: l.au_demi,
                effet: l.effet,
                origine: l.origine,
                ref_officiel_id: l.ref_officiel_id,
                ref_version: l.ref_version,
                publie_familles: l.publie_familles,
                publie_personnels: l.publie_personnels,
                motif: l.motif,
                jour_reference: l.jour_reference,
                services_touches: l.services_touches,
                conditionnelle: l.conditionnelle,
                levee_prevue_le: l.levee_prevue_le,
                internes_presents: l.internes_presents,
                version: l.version,
                annule_le: l.annule_le,
                annule_motif: l.annule_motif,
                cree_le: l.cree_le,
                cibles,
            }
        })
        .collect())
}

async fn lire_annees(
    conn: &mut PgConnection,
    etab: Uuid,
    plage: Option<(NaiveDate, NaiveDate)>,
) -> Result<Vec<AnneeLue>, EntDbError> {
    let l: Vec<(Uuid, String, NaiveDate, NaiveDate, bool)> = sqlx::query_as(
        "SELECT id, libelle, date_debut, date_fin, cloturee_le IS NOT NULL
           FROM ent_annee_scolaire
          WHERE etablissement_id = $1
            AND ($2::date IS NULL OR date_fin >= $2) AND ($3::date IS NULL OR date_debut <= $3)
          ORDER BY date_debut",
    )
    .bind(etab)
    .bind(plage.map(|p| p.0))
    .bind(plage.map(|p| p.1))
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(l.into_iter()
        .map(|(id, libelle, debut, fin, cloturee)| AnneeLue {
            id,
            libelle,
            debut,
            fin,
            cloturee,
        })
        .collect())
}

async fn lire_jours_ouvres_profil(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<Value, EntDbError> {
    let v: Option<Value> = sqlx::query_scalar(
        "SELECT jours_ouvres FROM ent_etablissement_profil WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    Ok(v.unwrap_or_else(|| serde_json::from_str(JOURS_OUVRES_DEFAUT).expect("défaut valide")))
}

async fn lire_semaines(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Option<Uuid>,
    plage: Option<(NaiveDate, NaiveDate)>,
) -> Result<Vec<(Uuid, NaiveDate, Option<String>)>, EntDbError> {
    sqlx::query_as(
        "SELECT annee_id, lundi, etiquette FROM ent_semaine
          WHERE etablissement_id = $1 AND ($2::uuid IS NULL OR annee_id = $2)
            AND ($3::date IS NULL OR lundi >= $3) AND ($4::date IS NULL OR lundi <= $4)",
    )
    .bind(etab)
    .bind(annee)
    .bind(plage.map(|p| lundi_de(p.0)))
    .bind(plage.map(|p| p.1))
    .fetch_all(conn)
    .await
    .map_err(db)
}

/// Charge le contexte de calcul pour une plage de dates.
pub async fn charger_contexte(
    conn: &mut PgConnection,
    etab: Uuid,
    du: NaiveDate,
    au: NaiveDate,
) -> Result<Contexte, EntDbError> {
    let plage = Some((du, au));
    let jours = lire_jours_ouvres_profil(conn, etab).await?;
    let annees = lire_annees(conn, etab, plage).await?;
    let evenements = lire_evenements(conn, etab, None, plage, None, false, false).await?;
    let semaines = lire_semaines(conn, etab, None, plage).await?;
    Ok(Contexte::nouveau(&jours, annees, evenements, semaines))
}

/// Complète une cible : la classe donne son niveau et son site, le groupe sa classe.
async fn resoudre_cible(
    conn: &mut PgConnection,
    etab: Uuid,
    cible: &Cible,
) -> Result<CibleResolue, EntDbError> {
    let mut c = cible.clone();
    if c.classe_id.is_none() {
        if let Some(g) = c.groupe_id {
            let classe: Option<Option<Uuid>> = sqlx::query_scalar(
                "SELECT classe_id FROM ent_groupe WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(g)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
            c.classe_id = classe.flatten();
        }
    }
    if let Some(cl) = c.classe_id {
        let r: Option<(Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT niveau_id, site_id FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(cl)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if let Some((n, s)) = r {
            c.niveau_id = c.niveau_id.or(n);
            c.site_id = c.site_id.or(s);
        }
    }
    Ok(c)
}

// ───────────────────────── Port lu par les autres socles ─────────────────────────

/// Port `structure::jour` du §6.3 : fonctions de lecture prenant la connexion ou la transaction de
/// l'appelant (`&mut TenantTx`), **sans acteur** (l'appelant a déjà autorisé), toujours filtrées
/// par l'établissement passé. Lues par les socles 10, 12, 24, 40, 41 et 43. Ne créent jamais rien.
pub mod calendrier {
    use super::*;

    /// État du jour `date` pour la cible (vide = l'établissement entier).
    pub async fn jour(
        conn: &mut PgConnection,
        etab: Uuid,
        date: NaiveDate,
        cible: &Cible,
    ) -> Result<Jour, EntDbError> {
        let ctx = charger_contexte(conn, etab, date, date).await?;
        let c = resoudre_cible(conn, etab, cible).await?;
        Ok(calculer_jour(&ctx, date, &c))
    }

    /// États de tous les jours de `[du, au]` (au plus 800 jours).
    pub async fn jours(
        conn: &mut PgConnection,
        etab: Uuid,
        du: NaiveDate,
        au: NaiveDate,
        cible: &Cible,
    ) -> Result<Vec<Jour>, EntDbError> {
        if au < du || (au - du).num_days() >= PLAGE_MAX_JOURS {
            return Err(invalide());
        }
        let ctx = charger_contexte(conn, etab, du, au).await?;
        let c = resoudre_cible(conn, etab, cible).await?;
        Ok(du
            .iter_days()
            .take_while(|d| *d <= au)
            .map(|d| calculer_jour(&ctx, d, &c))
            .collect())
    }

    /// Jours avec cours de `[du, au]`.
    pub async fn jours_ouvres(
        conn: &mut PgConnection,
        etab: Uuid,
        du: NaiveDate,
        au: NaiveDate,
        cible: &Cible,
    ) -> Result<Vec<NaiveDate>, EntDbError> {
        Ok(jours(conn, etab, du, au, cible)
            .await?
            .into_iter()
            .filter(|j| j.ouvre)
            .map(|j| j.date)
            .collect())
    }

    /// Jour courant de l'établissement (fuseau de l'établissement) pour la cible.
    pub async fn aujourdhui(
        conn: &mut PgConnection,
        etab: Uuid,
        cible: &Cible,
    ) -> Result<Jour, EntDbError> {
        let d = port_etab::aujourdhui(conn, etab).await?;
        jour(conn, etab, d, cible).await
    }

    /// Ouverture d'un service ce jour-là (R-S2).
    pub async fn jour_service(
        conn: &mut PgConnection,
        etab: Uuid,
        date: NaiveDate,
        service: &str,
    ) -> Result<JourService, EntDbError> {
        if !SERVICES.contains(&service) {
            return Err(invalide());
        }
        let ctx = charger_contexte(conn, etab, date, date).await?;
        Ok(calculer_jour_service(&ctx, date, service))
    }

    /// Jours ouvrés administratifs d'un service dans `[du, au]` : ce que les délais des socles 3,
    /// 8 et 24 doivent compter (R-S2).
    pub async fn jours_ouvres_administratifs(
        conn: &mut PgConnection,
        etab: Uuid,
        du: NaiveDate,
        au: NaiveDate,
        service: &str,
    ) -> Result<Vec<NaiveDate>, EntDbError> {
        if !SERVICES.contains(&service) || au < du || (au - du).num_days() >= PLAGE_MAX_JOURS {
            return Err(invalide());
        }
        let ctx = charger_contexte(conn, etab, du, au).await?;
        Ok(du
            .iter_days()
            .take_while(|d| *d <= au)
            .filter(|d| calculer_jour_service(&ctx, *d, service).ouvert)
            .collect())
    }

    /// Date d'échéance : `n` jours ouvrés administratifs après `depuis` (non comptée).
    pub async fn echeance_jours_ouvres(
        conn: &mut PgConnection,
        etab: Uuid,
        depuis: NaiveDate,
        n: u32,
        service: &str,
    ) -> Result<NaiveDate, EntDbError> {
        if !SERVICES.contains(&service) || n == 0 || n > 120 {
            return Err(invalide());
        }
        let borne = depuis + Duration::days(i64::from(n) * 3 + 60);
        let ouvres =
            jours_ouvres_administratifs(conn, etab, depuis + Duration::days(1), borne, service)
                .await?;
        ouvres.get(n as usize - 1).copied().ok_or_else(invalide)
    }
}

// ───────────────────────── Aides d'écriture ─────────────────────────

async fn tx(e: &PgEnt, a: &Acteur<'_>) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(e.pool(), a.etablissement_id)
        .await
        .map_err(db)
}

fn est_direction(a: &Acteur<'_>) -> bool {
    matches!(Role::parse(a.role), Some(Role::Direction | Role::Admin))
}

fn valider_demi(v: &Option<String>) -> Result<(), EntDbError> {
    match v.as_deref() {
        None | Some("matin") | Some("apres_midi") => Ok(()),
        _ => Err(invalide()),
    }
}

fn effet_par_defaut(type_evenement: &str) -> &'static str {
    match type_evenement {
        "vacances" | "ferie" | "pont" | "fermeture" | "journee_banalisee" => "sans_cours",
        "journee_pedagogique" | "pre_rentree" => "sans_eleves",
        "rattrapage" => "cours_modifies",
        _ => "informatif",
    }
}

/// Effet résumé d'une fermeture exceptionnelle, déduit des services touchés (R-S1).
pub fn effet_de_fermeture(services: &[String]) -> &'static str {
    let a = |s: &str| services.iter().any(|x| x == s);
    match (a("cours"), a("accueil_eleves")) {
        (true, true) => "sans_cours",
        (true, false) => "accueil_seulement",
        _ => "cours_modifies",
    }
}

fn json_cibles(c: &[CibleEvenement]) -> Value {
    json!(c)
}

fn charge_evenement(e: &Evenement, action: &str) -> Value {
    // Identifiants, codes, dates : jamais de libellé libre ni de motif.
    json!({
        "evenement_id": e.id, "annee_id": e.annee_id, "action": action, "type": e.type_evenement,
        "du": e.du, "au": e.au, "du_demi": e.du_demi, "au_demi": e.au_demi, "effet": e.effet,
        "cibles": json_cibles(&e.cibles),
        "services_touches": e.services_touches, "conditionnelle": e.conditionnelle,
        "levee_prevue_le": e.levee_prevue_le,
        "publie_familles": e.publie_familles, "publie_personnels": e.publie_personnels,
    })
}

fn resume_journal(e: &Evenement) -> Value {
    json!({
        "type": e.type_evenement, "libelle": e.libelle, "du": e.du, "au": e.au,
        "du_demi": e.du_demi, "au_demi": e.au_demi, "effet": e.effet, "origine": e.origine,
        "cibles": e.cibles, "services_touches": e.services_touches,
        "annule": e.annule_le.is_some(), "version": e.version,
    })
}

async fn annee_ecrivable(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
) -> Result<AnneeLue, EntDbError> {
    let annee = lire_annees(conn, etab, None)
        .await?
        .into_iter()
        .find(|a| a.id == annee_id)
        .ok_or_else(introuvable)?;
    if annee.cloturee {
        return Err(conflit());
    }
    Ok(annee)
}

/// Les cibles existent dans l'établissement ; classes et groupes sont ceux de l'année.
async fn valider_cibles(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
    cibles: &[CibleEvenement],
) -> Result<(), EntDbError> {
    let mut vues = BTreeSet::new();
    let a_service = cibles.iter().any(CibleEvenement::est_service);
    for c in cibles {
        if c.nb_champs() != 1 || !vues.insert(c.clone()) {
            return Err(invalide());
        }
        if a_service != c.est_service() {
            return Err(invalide()); // pas de mélange service / portée
        }
        if let Some(s) = &c.service {
            if !SERVICES.contains(&s.as_str()) {
                return Err(invalide());
            }
        }
        let existe: bool = if let Some(id) = c.site_id {
            sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_site WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?
        } else if let Some(id) = c.niveau_id {
            sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_niveau WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?
        } else if let Some(id) = c.classe_id {
            sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2 AND annee_id = $3)",
            )
            .bind(etab)
            .bind(id)
            .bind(annee_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?
        } else if let Some(id) = c.groupe_id {
            sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_groupe WHERE etablissement_id = $1 AND id = $2 AND annee_id = $3)",
            )
            .bind(etab)
            .bind(id)
            .bind(annee_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?
        } else {
            true
        };
        if !existe {
            return Err(introuvable());
        }
    }
    Ok(())
}

async fn inserer_cibles(
    conn: &mut PgConnection,
    etab: Uuid,
    evenement: Uuid,
    cibles: &[CibleEvenement],
) -> Result<(), EntDbError> {
    for c in cibles {
        sqlx::query(
            "INSERT INTO ent_calendrier_cible
               (id, etablissement_id, evenement_id, site_id, niveau_id, classe_id, groupe_id, service)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(evenement)
        .bind(c.site_id)
        .bind(c.niveau_id)
        .bind(c.classe_id)
        .bind(c.groupe_id)
        .bind(&c.service)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

/// Résumé d'un événement qui chevauche (avertissement, jamais un refus).
#[derive(Debug, Clone, Serialize)]
pub struct Chevauchement {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub type_evenement: String,
    pub libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub effet: String,
}

fn cibles_se_recouvrent(a: &[CibleEvenement], b: &[CibleEvenement]) -> bool {
    a.is_empty() || b.is_empty() || a.iter().any(|x| b.contains(x))
}

async fn chevauchements(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Uuid,
    du: NaiveDate,
    au: NaiveDate,
    cibles: &[CibleEvenement],
    sauf: Option<Uuid>,
) -> Result<Vec<Chevauchement>, EntDbError> {
    Ok(
        lire_evenements(conn, etab, Some(annee), Some((du, au)), None, false, false)
            .await?
            .into_iter()
            .filter(|e| Some(e.id) != sauf && cibles_se_recouvrent(&e.cibles, cibles))
            .filter(|e| e.effet != "informatif")
            .map(|e| Chevauchement {
                id: e.id,
                type_evenement: e.type_evenement,
                libelle: e.libelle,
                du: e.du,
                au: e.au,
                effet: e.effet,
            })
            .collect(),
    )
}

/// Recalcule les semaines A/B d'une année (si une règle existe) ; garde les semaines forcées.
/// Retourne `true` si une étiquette a changé.
async fn recalculer_semaines(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
    par: Uuid,
) -> Result<bool, EntDbError> {
    let regle: Option<RegleSemaine> = sqlx::query_as(
        "SELECT etiquettes, mode, premiere_semaine FROM ent_semaine_type_regle
          WHERE etablissement_id = $1 AND annee_id = $2",
    )
    .bind(etab)
    .bind(annee_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some(regle) = regle else {
        return Ok(false);
    };
    let annee = lire_annees(conn, etab, None)
        .await?
        .into_iter()
        .find(|a| a.id == annee_id)
        .ok_or_else(introuvable)?;
    let jours = lire_jours_ouvres_profil(conn, etab).await?;
    let evenements = lire_evenements(
        conn,
        etab,
        Some(annee_id),
        Some((annee.debut, annee.fin)),
        None,
        false,
        false,
    )
    .await?;
    let ctx = Contexte::nouveau(&jours, vec![annee.clone()], evenements, vec![]);
    let vide = Cible::default();
    let mut lundis: Vec<(NaiveDate, bool)> = Vec::new();
    let mut lundi = lundi_de(annee.debut);
    while lundi <= annee.fin {
        let vacances = !(0..7).any(|i| {
            let d = lundi + Duration::days(i);
            d >= annee.debut && d <= annee.fin && calculer_jour(&ctx, d, &vide).ouvre
        });
        lundis.push((lundi, vacances));
        lundi += Duration::days(7);
    }
    let etiquettes = etiquettes_semaines(&lundis, &regle);
    let avant: HashMap<NaiveDate, Option<String>> = lire_semaines(conn, etab, Some(annee_id), None)
        .await?
        .into_iter()
        .map(|(_, l, e)| (l, e))
        .collect();
    sqlx::query(
        "DELETE FROM ent_semaine WHERE etablissement_id = $1 AND annee_id = $2 AND source = 'calcule'",
    )
    .bind(etab)
    .bind(annee_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    for ((lundi, _), e) in lundis.iter().zip(&etiquettes) {
        sqlx::query(
            "INSERT INTO ent_semaine (etablissement_id, annee_id, lundi, etiquette, source, modifie_par)
             VALUES ($1,$2,$3,$4,'calcule',$5) ON CONFLICT (annee_id, lundi) DO NOTHING",
        )
        .bind(etab)
        .bind(annee_id)
        .bind(lundi)
        .bind(e)
        .bind(par)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    let apres: HashMap<NaiveDate, Option<String>> = lire_semaines(conn, etab, Some(annee_id), None)
        .await?
        .into_iter()
        .map(|(_, l, e)| (l, e))
        .collect();
    Ok(avant != apres)
}

/// Après une écriture du calendrier : recalcule les semaines et publie `semaines_definies` si
/// une étiquette a changé.
async fn apres_ecriture(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    annee_id: Uuid,
) -> Result<(), EntDbError> {
    if recalculer_semaines(conn, a.etablissement_id, annee_id, a.user_id).await? {
        publier(
            conn,
            a,
            "referentiel.semaines_definies",
            annee_id,
            json!({ "annee_id": annee_id, "origine": "recalcul" }),
        )
        .await?;
    }
    Ok(())
}

// ───────────────────────── Événements du calendrier (STR-24, 27, 93) ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct EvenementSaisie {
    pub annee_id: Uuid,
    #[serde(rename = "type")]
    pub type_evenement: String,
    pub libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub du_demi: Option<String>,
    pub au_demi: Option<String>,
    /// Absent : effet par défaut du type.
    pub effet: Option<String>,
    #[serde(default)]
    pub cibles: Vec<CibleEvenement>,
    #[serde(default)]
    pub publie_familles: bool,
    #[serde(default)]
    pub publie_personnels: bool,
    pub motif: Option<String>,
    /// Rattrapage : jour de la semaine dont la journée suit l'emploi du temps.
    pub jour_reference: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EvenementModif {
    pub libelle: Option<String>,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub du_demi: Option<String>,
    pub au_demi: Option<String>,
    /// `""` retire la demi-journée.
    pub effet: Option<String>,
    pub cibles: Option<Vec<CibleEvenement>>,
    pub publie_familles: Option<bool>,
    pub publie_personnels: Option<bool>,
    pub motif: Option<String>,
    pub jour_reference: Option<String>,
    pub version_attendue: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultatEvenement {
    pub evenement: Evenement,
    pub chevauchements: Vec<Chevauchement>,
}

impl PgEnt {
    /// Événements d'une année (filtrables par plage), annulés exclus sauf demande.
    pub async fn structure_calendrier_evenements(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        plage: Option<(NaiveDate, NaiveDate)>,
        avec_annules: bool,
    ) -> Result<Vec<Evenement>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let v = lire_evenements(
            &mut t,
            a.etablissement_id,
            Some(annee_id),
            plage,
            None,
            avec_annules,
            false,
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    pub async fn structure_calendrier_evenement(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Evenement, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let e = lire_evenements(
            &mut t,
            a.etablissement_id,
            None,
            None,
            Some(id),
            true,
            false,
        )
        .await?
        .pop()
        .ok_or_else(introuvable)?;
        t.commit().await.map_err(db)?;
        Ok(e)
    }

    /// STR-24 / STR-27 / STR-93 : ajoute un pont, une journée banalisée, un rattrapage, un
    /// événement… avec cibles et effet. Les chevauchements sont expliqués, jamais bloquants.
    /// Une fermeture exceptionnelle se déclare par [`PgEnt::structure_fermeture_declarer`].
    pub async fn structure_calendrier_evenement_creer(
        &self,
        a: &Acteur<'_>,
        s: EvenementSaisie,
    ) -> Result<ResultatEvenement, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if s.type_evenement == "fermeture_exceptionnelle" {
            return Err(invalide());
        }
        let r = validations_saisie(&s)?;
        let mut t = tx(self, a).await?;
        let annee = annee_ecrivable(&mut t, a.etablissement_id, s.annee_id).await?;
        if s.au < annee.debut || s.du > annee.fin {
            return Err(invalide());
        }
        valider_cibles(&mut t, a.etablissement_id, s.annee_id, &s.cibles).await?;
        let chev = chevauchements(
            &mut t,
            a.etablissement_id,
            s.annee_id,
            s.du,
            s.au,
            &s.cibles,
            None,
        )
        .await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_calendrier_evenement
               (id, etablissement_id, annee_id, type, libelle, du, au, du_demi, au_demi, effet, origine,
                publie_familles, publie_personnels, motif, jour_reference, cree_par, modifie_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'saisie',$11,$12,$13,$14,$15,$15)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(s.annee_id)
        .bind(&s.type_evenement)
        .bind(&r.libelle)
        .bind(s.du)
        .bind(s.au)
        .bind(&s.du_demi)
        .bind(&s.au_demi)
        .bind(&r.effet)
        .bind(s.publie_familles)
        .bind(s.publie_personnels)
        .bind(&r.motif)
        .bind(&s.jour_reference)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        inserer_cibles(&mut t, a.etablissement_id, id, &s.cibles).await?;
        let e = lire_evenements(
            &mut t,
            a.etablissement_id,
            None,
            None,
            Some(id),
            true,
            false,
        )
        .await?
        .pop()
        .ok_or_else(introuvable)?;
        journaliser(
            &mut t,
            a,
            "calendrier_evenement",
            &id.to_string(),
            "creer",
            None,
            Some(resume_journal(&e)),
        )
        .await?;
        publier(
            &mut t,
            a,
            "referentiel.calendrier_modifie",
            id,
            charge_evenement(&e, "cree"),
        )
        .await?;
        apres_ecriture(&mut t, a, s.annee_id).await?;
        t.commit().await.map_err(db)?;
        Ok(ResultatEvenement {
            evenement: e,
            chevauchements: chev,
        })
    }

    /// Modifie un événement saisi (les événements officiels ou calculés s'annulent, ils ne se
    /// modifient pas ; une fermeture exceptionnelle a ses propres commandes).
    pub async fn structure_calendrier_evenement_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        m: EvenementModif,
    ) -> Result<ResultatEvenement, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let avant = lire_evenements(&mut t, a.etablissement_id, None, None, Some(id), true, true)
            .await?
            .pop()
            .ok_or_else(introuvable)?;
        if avant.type_evenement == "fermeture_exceptionnelle"
            || avant.origine == "officiel"
            || avant.origine == "calcule"
            || avant.annule_le.is_some()
        {
            return Err(conflit());
        }
        if m.version_attendue.is_some_and(|v| v != avant.version) {
            return Err(conflit());
        }
        annee_ecrivable(&mut t, a.etablissement_id, avant.annee_id).await?;
        let s = EvenementSaisie {
            annee_id: avant.annee_id,
            type_evenement: avant.type_evenement.clone(),
            libelle: m.libelle.clone().unwrap_or_else(|| avant.libelle.clone()),
            du: m.du.unwrap_or(avant.du),
            au: m.au.unwrap_or(avant.au),
            du_demi: match m.du_demi.as_deref() {
                Some("") => None,
                Some(v) => Some(v.to_string()),
                None => avant.du_demi.clone(),
            },
            au_demi: match m.au_demi.as_deref() {
                Some("") => None,
                Some(v) => Some(v.to_string()),
                None => avant.au_demi.clone(),
            },
            effet: Some(m.effet.clone().unwrap_or_else(|| avant.effet.clone())),
            cibles: m.cibles.clone().unwrap_or_else(|| avant.cibles.clone()),
            publie_familles: m.publie_familles.unwrap_or(avant.publie_familles),
            publie_personnels: m.publie_personnels.unwrap_or(avant.publie_personnels),
            motif: m.motif.clone().or_else(|| avant.motif.clone()),
            jour_reference: m
                .jour_reference
                .clone()
                .or_else(|| avant.jour_reference.clone()),
        };
        let r = validations_saisie(&s)?;
        let annee = annee_ecrivable(&mut t, a.etablissement_id, avant.annee_id).await?;
        if s.au < annee.debut || s.du > annee.fin {
            return Err(invalide());
        }
        valider_cibles(&mut t, a.etablissement_id, avant.annee_id, &s.cibles).await?;
        let chev = chevauchements(
            &mut t,
            a.etablissement_id,
            avant.annee_id,
            s.du,
            s.au,
            &s.cibles,
            Some(id),
        )
        .await?;
        sqlx::query(
            "UPDATE ent_calendrier_evenement SET libelle=$3, du=$4, au=$5, du_demi=$6, au_demi=$7, effet=$8,
                    publie_familles=$9, publie_personnels=$10, motif=$11, jour_reference=$12,
                    version = version + 1, modifie_par=$13, modifie_le=now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&r.libelle)
        .bind(s.du)
        .bind(s.au)
        .bind(&s.du_demi)
        .bind(&s.au_demi)
        .bind(&r.effet)
        .bind(s.publie_familles)
        .bind(s.publie_personnels)
        .bind(&r.motif)
        .bind(&s.jour_reference)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        if m.cibles.is_some() {
            sqlx::query("DELETE FROM ent_calendrier_cible WHERE etablissement_id = $1 AND evenement_id = $2")
                .bind(a.etablissement_id)
                .bind(id)
                .execute(&mut *t)
                .await
                .map_err(db)?;
            inserer_cibles(&mut t, a.etablissement_id, id, &s.cibles).await?;
        }
        let e = lire_evenements(
            &mut t,
            a.etablissement_id,
            None,
            None,
            Some(id),
            true,
            false,
        )
        .await?
        .pop()
        .ok_or_else(introuvable)?;
        journaliser(
            &mut t,
            a,
            "calendrier_evenement",
            &id.to_string(),
            "modifier",
            Some(resume_journal(&avant)),
            Some(resume_journal(&e)),
        )
        .await?;
        publier(
            &mut t,
            a,
            "referentiel.calendrier_modifie",
            id,
            charge_evenement(&e, "modifie"),
        )
        .await?;
        apres_ecriture(&mut t, a, avant.annee_id).await?;
        t.commit().await.map_err(db)?;
        Ok(ResultatEvenement {
            evenement: e,
            chevauchements: chev,
        })
    }

    /// Annule un événement (jamais supprimé) : un férié travaillé, un pont renoncé. Une
    /// fermeture exceptionnelle se lève par [`PgEnt::structure_fermeture_lever`].
    pub async fn structure_calendrier_evenement_annuler(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        motif: &str,
    ) -> Result<Evenement, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let motif = requis(motif, 500)?;
        self.basculer_annulation(a, id, Some(motif)).await
    }

    /// Rétablit un événement annulé (par exemple un férié calculé annulé par erreur).
    pub async fn structure_calendrier_evenement_retablir(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Evenement, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        self.basculer_annulation(a, id, None).await
    }

    async fn basculer_annulation(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        motif: Option<String>,
    ) -> Result<Evenement, EntDbError> {
        let mut t = tx(self, a).await?;
        let avant = lire_evenements(&mut t, a.etablissement_id, None, None, Some(id), true, true)
            .await?
            .pop()
            .ok_or_else(introuvable)?;
        if avant.type_evenement == "fermeture_exceptionnelle"
            || avant.annule_le.is_some() != motif.is_none()
        {
            return Err(conflit());
        }
        annee_ecrivable(&mut t, a.etablissement_id, avant.annee_id).await?;
        sqlx::query(
            "UPDATE ent_calendrier_evenement
                SET annule_le = CASE WHEN $3::text IS NULL THEN NULL ELSE now() END,
                    annule_par = CASE WHEN $3::text IS NULL THEN NULL ELSE $4::uuid END,
                    annule_motif = $3, version = version + 1, modifie_par = $4, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let e = lire_evenements(
            &mut t,
            a.etablissement_id,
            None,
            None,
            Some(id),
            true,
            false,
        )
        .await?
        .pop()
        .ok_or_else(introuvable)?;
        let action = if motif.is_some() {
            "annuler"
        } else {
            "retablir"
        };
        journaliser(
            &mut t,
            a,
            "calendrier_evenement",
            &id.to_string(),
            action,
            Some(resume_journal(&avant)),
            Some(resume_journal(&e)),
        )
        .await?;
        publier(
            &mut t,
            a,
            "referentiel.calendrier_modifie",
            id,
            charge_evenement(&e, if motif.is_some() { "annule" } else { "retabli" }),
        )
        .await?;
        apres_ecriture(&mut t, a, avant.annee_id).await?;
        t.commit().await.map_err(db)?;
        Ok(e)
    }
}

struct Valide {
    libelle: String,
    effet: String,
    motif: Option<String>,
}

fn validations_saisie(s: &EvenementSaisie) -> Result<Valide, EntDbError> {
    if !TYPES_EVENEMENT.contains(&s.type_evenement.as_str()) {
        return Err(invalide());
    }
    let libelle = requis(&s.libelle, 160)?;
    let motif = net(s.motif.as_deref(), 500)?;
    if s.au < s.du {
        return Err(invalide());
    }
    valider_demi(&s.du_demi)?;
    valider_demi(&s.au_demi)?;
    if s.du == s.au
        && s.du_demi.as_deref() == Some("apres_midi")
        && s.au_demi.as_deref() == Some("matin")
    {
        return Err(invalide());
    }
    let effet = s
        .effet
        .clone()
        .unwrap_or_else(|| effet_par_defaut(&s.type_evenement).to_string());
    if !EFFETS.contains(&effet.as_str()) {
        return Err(invalide());
    }
    if s.publie_familles && s.publie_personnels {
        return Err(invalide()); // R-S19 : « aux personnels seulement »
    }
    let rattrapage = s.type_evenement == "rattrapage";
    match (&s.jour_reference, rattrapage) {
        (Some(j), true) if JOURS.contains(&j.as_str()) && s.du == s.au => {}
        (None, false) => {}
        _ => return Err(invalide()),
    }
    if s.cibles.len() > 200 {
        return Err(invalide());
    }
    Ok(Valide {
        libelle,
        effet,
        motif,
    })
}

// ───────────────────────── Semaines A/B (STR-28) ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct RegleSemaineSaisie {
    pub etiquettes: Vec<String>,
    pub mode: String,
    pub premiere_semaine: NaiveDate,
}

#[derive(Debug, Clone, Serialize)]
pub struct SemaineVue {
    pub lundi: NaiveDate,
    pub etiquette: Option<String>,
    pub source: String,
    pub motif: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SemainesVue {
    pub regle: Option<RegleSemaine>,
    pub semaines: Vec<SemaineVue>,
}

impl PgEnt {
    pub async fn structure_semaines(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<SemainesVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let regle: Option<RegleSemaine> = sqlx::query_as(
            "SELECT etiquettes, mode, premiere_semaine FROM ent_semaine_type_regle
              WHERE etablissement_id = $1 AND annee_id = $2",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let l: Vec<(NaiveDate, Option<String>, String, Option<String>)> = sqlx::query_as(
            "SELECT lundi, etiquette, source, motif FROM ent_semaine
              WHERE etablissement_id = $1 AND annee_id = $2 ORDER BY lundi",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(SemainesVue {
            regle,
            semaines: l
                .into_iter()
                .map(|(lundi, etiquette, source, motif)| SemaineVue {
                    lundi,
                    etiquette,
                    source,
                    motif,
                })
                .collect(),
        })
    }

    /// STR-28 : définit les types de semaine d'une année et matérialise les semaines.
    pub async fn structure_semaines_definir(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        s: RegleSemaineSaisie,
    ) -> Result<SemainesVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let etiquettes: Vec<String> = s
            .etiquettes
            .iter()
            .map(|e| e.trim().to_uppercase())
            .collect();
        let distinctes: BTreeSet<&String> = etiquettes.iter().collect();
        if !(2..=4).contains(&etiquettes.len())
            || distinctes.len() != etiquettes.len()
            || etiquettes.iter().any(|e| {
                e.is_empty() || e.chars().count() > 8 || !e.chars().all(char::is_alphanumeric)
            })
            || !["alternance_continue", "reprise_apres_vacances"].contains(&s.mode.as_str())
            || s.premiere_semaine.weekday() != Weekday::Mon
        {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let annee = annee_ecrivable(&mut t, a.etablissement_id, annee_id).await?;
        if s.premiere_semaine < lundi_de(annee.debut) || s.premiere_semaine > annee.fin {
            return Err(invalide());
        }
        let avant: Option<RegleSemaine> = sqlx::query_as(
            "SELECT etiquettes, mode, premiere_semaine FROM ent_semaine_type_regle
              WHERE etablissement_id = $1 AND annee_id = $2 FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_semaine_type_regle (etablissement_id, annee_id, etiquettes, mode, premiere_semaine, modifie_par)
             VALUES ($1,$2,$3,$4,$5,$6)
             ON CONFLICT (annee_id) DO UPDATE SET etiquettes = EXCLUDED.etiquettes, mode = EXCLUDED.mode,
                  premiere_semaine = EXCLUDED.premiere_semaine, modifie_par = EXCLUDED.modifie_par, modifie_le = now()",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .bind(&etiquettes)
        .bind(&s.mode)
        .bind(s.premiere_semaine)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        // Une étiquette forcée qui n'existe plus dans la règle est rendue au calcul.
        sqlx::query(
            "DELETE FROM ent_semaine WHERE etablissement_id = $1 AND annee_id = $2 AND source = 'force'
                AND etiquette IS NOT NULL AND NOT (etiquette = ANY($3))",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .bind(&etiquettes)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        recalculer_semaines(&mut t, a.etablissement_id, annee_id, a.user_id).await?;
        journaliser(
            &mut t,
            a,
            "semaines",
            &annee_id.to_string(),
            "definir",
            avant.map(|r| json!({"etiquettes": r.etiquettes, "mode": r.mode, "premiere_semaine": r.premiere_semaine})),
            Some(json!({"etiquettes": etiquettes, "mode": s.mode, "premiere_semaine": s.premiere_semaine})),
        )
        .await?;
        publier(
            &mut t,
            a,
            "referentiel.semaines_definies",
            annee_id,
            json!({"annee_id": annee_id, "origine": "regle", "mode": s.mode, "etiquettes": etiquettes}),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.structure_semaines(a, annee_id).await
    }

    /// STR-28 : force l'étiquette d'une semaine (`None` = semaine sans étiquette), motif obligatoire.
    pub async fn structure_semaine_forcer(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        lundi: NaiveDate,
        etiquette: Option<String>,
        motif: &str,
    ) -> Result<SemainesVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let motif = requis(motif, 300)?;
        let etiquette = etiquette.map(|e| e.trim().to_uppercase());
        if lundi.weekday() != Weekday::Mon {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let annee = annee_ecrivable(&mut t, a.etablissement_id, annee_id).await?;
        let regle: RegleSemaine = sqlx::query_as(
            "SELECT etiquettes, mode, premiere_semaine FROM ent_semaine_type_regle
              WHERE etablissement_id = $1 AND annee_id = $2",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(conflit)?;
        if lundi < lundi_de(annee.debut) || lundi > annee.fin {
            return Err(invalide());
        }
        if etiquette
            .as_ref()
            .is_some_and(|e| !regle.etiquettes.contains(e))
        {
            return Err(invalide());
        }
        let avant: Option<(Option<String>, String)> = sqlx::query_as(
            "SELECT etiquette, source FROM ent_semaine WHERE etablissement_id = $1 AND annee_id = $2 AND lundi = $3",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .bind(lundi)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_semaine (etablissement_id, annee_id, lundi, etiquette, source, motif, modifie_par)
             VALUES ($1,$2,$3,$4,'force',$5,$6)
             ON CONFLICT (annee_id, lundi) DO UPDATE SET etiquette = EXCLUDED.etiquette, source = 'force',
                  motif = EXCLUDED.motif, modifie_par = EXCLUDED.modifie_par, modifie_le = now()",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .bind(lundi)
        .bind(&etiquette)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            "semaine",
            &format!("{annee_id}:{lundi}"),
            "forcer",
            avant.map(|(e, s)| json!({"etiquette": e, "source": s})),
            Some(json!({"etiquette": etiquette, "source": "force", "motif": motif})),
        )
        .await?;
        publier(
            &mut t,
            a,
            "referentiel.semaines_definies",
            annee_id,
            json!({"annee_id": annee_id, "origine": "force", "lundi": lundi, "etiquette": etiquette}),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.structure_semaines(a, annee_id).await
    }

    /// Rend une semaine forcée au calcul.
    pub async fn structure_semaine_liberer(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        lundi: NaiveDate,
    ) -> Result<SemainesVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        annee_ecrivable(&mut t, a.etablissement_id, annee_id).await?;
        let n = sqlx::query(
            "DELETE FROM ent_semaine WHERE etablissement_id = $1 AND annee_id = $2 AND lundi = $3 AND source = 'force'",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .bind(lundi)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        recalculer_semaines(&mut t, a.etablissement_id, annee_id, a.user_id).await?;
        journaliser(
            &mut t,
            a,
            "semaine",
            &format!("{annee_id}:{lundi}"),
            "liberer",
            None,
            None,
        )
        .await?;
        publier(
            &mut t,
            a,
            "referentiel.semaines_definies",
            annee_id,
            json!({"annee_id": annee_id, "origine": "libere", "lundi": lundi}),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.structure_semaines(a, annee_id).await
    }
}

// ───────────────────────── Adoption du calendrier officiel (STR-23) ─────────────────────────

/// Ligne de référence officielle (saisie de plateforme ou import vérifié).
#[derive(Debug, Clone, Deserialize)]
pub struct LigneOfficielle {
    pub zone: String,
    pub annee_libelle: String,
    #[serde(rename = "type")]
    pub type_ligne: String,
    pub libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub source: String,
    pub source_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LigneAdoption {
    #[serde(rename = "type")]
    pub type_evenement: String,
    pub libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub origine: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Difference {
    pub evenement_id: Uuid,
    pub libelle: String,
    pub du_actuel: NaiveDate,
    pub au_actuel: NaiveDate,
    pub du_officiel: NaiveDate,
    pub au_officiel: NaiveDate,
    pub version_officielle: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PontPossible {
    pub ferie: NaiveDate,
    pub libelle_ferie: String,
    pub pont: NaiveDate,
}

#[derive(Debug, Clone, Serialize)]
pub struct RapportAdoption {
    pub annee_id: Uuid,
    pub zone: Option<String>,
    /// Des lignes officielles existent pour cette zone et cette année.
    pub officiel_disponible: bool,
    pub ajoutes: Vec<LigneAdoption>,
    pub feries_ajoutes: Vec<LigneAdoption>,
    pub differences: Vec<Difference>,
    pub differences_acceptees: bool,
    pub inchanges: usize,
    pub ponts_possibles: Vec<PontPossible>,
    pub avertissements: Vec<String>,
    pub ecrit: bool,
}

/// Saisie guidée d'un calendrier de vacances à la main (préréglage documenté) : les cinq
/// périodes de l'année scolaire française, à dater.
#[derive(Debug, Clone, Serialize)]
pub struct ModeleSaisie {
    pub zone: Option<String>,
    pub periodes: Vec<&'static str>,
    pub aide: &'static str,
}

pub fn modele_saisie(zone: Option<&str>) -> ModeleSaisie {
    ModeleSaisie {
        zone: zone.map(str::to_string),
        periodes: vec![
            "Vacances de la Toussaint",
            "Vacances de Noël",
            "Vacances d'hiver",
            "Vacances de printemps",
            "Vacances d'été",
        ],
        aide: "Dates à recopier du calendrier officiel publié par le ministère de l'Éducation \
               nationale pour votre zone (A, B, C, Corse ou territoire). Créez un événement de type \
               « vacances » par période ; les jours fériés sont calculés automatiquement.",
    }
}

fn type_officiel(t: &str) -> &'static str {
    match t {
        "vacances" => "vacances",
        "ferie" => "ferie",
        _ => "pont",
    }
}

impl PgEnt {
    /// **Fonction de plateforme** (aucune route d'établissement) : enregistre ou met à jour des
    /// lignes de `ref_calendrier_officiel`. Une ligne existante (même zone, année, type, libellé)
    /// reçoit les nouvelles dates et la nouvelle version de source : les établissements verront
    /// des « différences à accepter » à leur prochaine adoption.
    pub async fn calendrier_officiel_referencer(
        &self,
        lignes: &[LigneOfficielle],
    ) -> Result<usize, EntDbError> {
        let mut t = self.pool().begin().await.map_err(db)?;
        for l in lignes {
            if !["vacances", "ferie", "pont_national"].contains(&l.type_ligne.as_str())
                || l.au < l.du
            {
                return Err(invalide());
            }
            sqlx::query(
                "INSERT INTO ref_calendrier_officiel
                   (id, zone, annee_libelle, type, libelle, du, au, source, source_version)
                 VALUES ($1,lower($2),$3,$4,$5,$6,$7,$8,$9)
                 ON CONFLICT (zone, annee_libelle, type, libelle) DO UPDATE SET
                   du = EXCLUDED.du, au = EXCLUDED.au, source = EXCLUDED.source,
                   source_version = EXCLUDED.source_version, importe_le = now()",
            )
            .bind(Uuid::new_v4())
            .bind(requis(&l.zone, 32)?)
            .bind(requis(&l.annee_libelle, 32)?)
            .bind(&l.type_ligne)
            .bind(requis(&l.libelle, 120)?)
            .bind(l.du)
            .bind(l.au)
            .bind(requis(&l.source, 200)?)
            .bind(requis(&l.source_version, 64)?)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        t.commit().await.map_err(db)?;
        Ok(lignes.len())
    }

    /// STR-23 : adopte le calendrier officiel de la zone pour une année et génère les jours
    /// fériés calculés. `apercu` : rien n'est écrit. Une nouvelle version de la source donne des
    /// **différences** appliquées seulement avec `accepter_differences`. Idempotent.
    pub async fn structure_calendrier_adopter(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        apercu: bool,
        accepter_differences: bool,
    ) -> Result<RapportAdoption, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let annee = annee_ecrivable(&mut t, a.etablissement_id, annee_id).await?;
        let profil: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT zone_calendrier, departement_code, regime_local FROM ent_etablissement_profil
              WHERE etablissement_id = $1",
        )
        .bind(a.etablissement_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (zone, dep, regime) = profil.unwrap_or((None, None, None));
        let zone = zone
            .map(|z| z.trim().to_lowercase())
            .filter(|z| !z.is_empty());
        let mut avertissements = Vec::new();
        let mut r = RapportAdoption {
            annee_id,
            zone: zone.clone(),
            officiel_disponible: false,
            ajoutes: vec![],
            feries_ajoutes: vec![],
            differences: vec![],
            differences_acceptees: false,
            inchanges: 0,
            ponts_possibles: vec![],
            avertissements: vec![],
            ecrit: false,
        };
        // 1. Calendrier officiel de la zone (s'il a été saisi).
        let existants = lire_evenements(
            &mut t,
            a.etablissement_id,
            Some(annee_id),
            None,
            None,
            true,
            true,
        )
        .await?;
        let mut a_ajouter: Vec<(Uuid, String, String, NaiveDate, NaiveDate, String)> = Vec::new();
        let mut a_mettre_a_jour: Vec<(Uuid, String, NaiveDate, NaiveDate, String)> = Vec::new();
        if let Some(z) = &zone {
            let refs: Vec<(Uuid, String, String, NaiveDate, NaiveDate, String)> = sqlx::query_as(
                "SELECT id, type, libelle, du, au, source_version FROM ref_calendrier_officiel
                  WHERE zone = $1 AND annee_libelle = $2 ORDER BY du, libelle",
            )
            .bind(z)
            .bind(&annee.libelle)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
            r.officiel_disponible = !refs.is_empty();
            for (ref_id, type_ref, libelle, du, au, version) in refs {
                match existants.iter().find(|e| e.ref_officiel_id == Some(ref_id)) {
                    None => a_ajouter.push((ref_id, type_ref, libelle, du, au, version)),
                    Some(e)
                        if e.du == du
                            && e.au == au
                            && e.ref_version.as_deref() == Some(version.as_str()) =>
                    {
                        r.inchanges += 1
                    }
                    Some(e) if e.du == du && e.au == au => {
                        // Même dates, nouvelle version de source : rien à accepter.
                        a_mettre_a_jour.push((e.id, libelle, du, au, version));
                    }
                    Some(e) => r.differences.push(Difference {
                        evenement_id: e.id,
                        libelle,
                        du_actuel: e.du,
                        au_actuel: e.au,
                        du_officiel: du,
                        au_officiel: au,
                        version_officielle: version,
                    }),
                }
            }
            if !r.officiel_disponible {
                avertissements.push(format!(
                    "aucun calendrier officiel enregistré pour la zone « {z} » et l'année « {} » : \
                     saisie manuelle des vacances (voir le modèle de saisie)",
                    annee.libelle
                ));
            }
        } else {
            avertissements.push(
                "zone de calendrier non renseignée dans le profil : seuls les jours fériés calculés sont proposés"
                    .into(),
            );
        }
        // 2. Jours fériés calculés (Pâques, Ascension, Pentecôte, régimes locaux).
        let territoire = Territoire::depuis_departement(dep.as_deref());
        let feries = feries_entre(
            annee.debut,
            annee.fin,
            territoire,
            est_alsace_moselle(regime.as_deref(), dep.as_deref()),
        );
        r.ponts_possibles = ponts_possibles(&feries)
            .into_iter()
            .filter_map(|(f, p)| {
                feries.iter().find(|x| x.date == f).map(|x| PontPossible {
                    ferie: f,
                    libelle_ferie: x.libelle.to_string(),
                    pont: p,
                })
            })
            .filter(|p| p.pont >= annee.debut && p.pont <= annee.fin)
            .collect();
        let deja: BTreeSet<(String, NaiveDate)> = existants
            .iter()
            .filter(|e| e.origine == "calcule")
            .map(|e| (e.libelle.clone(), e.du))
            .collect();
        let feries_nouveaux: Vec<_> = feries
            .iter()
            .filter(|f| !deja.contains(&(f.libelle.to_string(), f.date)))
            .collect();
        for f in &feries_nouveaux {
            r.feries_ajoutes.push(LigneAdoption {
                type_evenement: "ferie".into(),
                libelle: f.libelle.to_string(),
                du: f.date,
                au: f.date,
                origine: "calcule".into(),
            });
        }
        for (_, tr, lib, du, au, _) in &a_ajouter {
            r.ajoutes.push(LigneAdoption {
                type_evenement: type_officiel(tr).into(),
                libelle: lib.clone(),
                du: *du,
                au: *au,
                origine: "officiel".into(),
            });
        }
        r.avertissements = avertissements;
        if apercu {
            return Ok(r);
        }
        // 3. Écriture.
        for (ref_id, tr, lib, du, au, version) in &a_ajouter {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_calendrier_evenement
                   (id, etablissement_id, annee_id, type, libelle, du, au, effet, origine, ref_officiel_id,
                    ref_version, cree_par, modifie_par)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'sans_cours','officiel',$8,$9,$10,$10)",
            )
            .bind(id)
            .bind(a.etablissement_id)
            .bind(annee_id)
            .bind(type_officiel(tr))
            .bind(lib)
            .bind(du)
            .bind(au)
            .bind(ref_id)
            .bind(version)
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        for f in &feries_nouveaux {
            sqlx::query(
                "INSERT INTO ent_calendrier_evenement
                   (id, etablissement_id, annee_id, type, libelle, du, au, effet, origine, cree_par, modifie_par)
                 VALUES ($1,$2,$3,'ferie',$4,$5,$5,'sans_cours','calcule',$6,$6)
                 ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(annee_id)
            .bind(f.libelle)
            .bind(f.date)
            .bind(a.user_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        for (id, _, du, au, version) in &a_mettre_a_jour {
            sqlx::query(
                "UPDATE ent_calendrier_evenement SET du = $3, au = $4, ref_version = $5, modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(du)
            .bind(au)
            .bind(version)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        if accepter_differences {
            for d in &r.differences {
                sqlx::query(
                    "UPDATE ent_calendrier_evenement
                        SET du = $3, au = $4, ref_version = $5, version = version + 1, modifie_par = $6, modifie_le = now()
                      WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NULL",
                )
                .bind(a.etablissement_id)
                .bind(d.evenement_id)
                .bind(d.du_officiel)
                .bind(d.au_officiel)
                .bind(&d.version_officielle)
                .bind(a.user_id)
                .execute(&mut *t)
                .await
                .map_err(db)?;
            }
            r.differences_acceptees = true;
        }
        r.ecrit = true;
        journaliser(
            &mut t,
            a,
            "calendrier_adoption",
            &annee_id.to_string(),
            "adopter",
            None,
            Some(json!({
                "zone": r.zone, "ajoutes": r.ajoutes.len(), "feries": r.feries_ajoutes.len(),
                "differences": r.differences.len(), "differences_acceptees": r.differences_acceptees,
            })),
        )
        .await?;
        publier(
            &mut t,
            a,
            "referentiel.calendrier_adopte",
            annee_id,
            json!({
                "annee_id": annee_id, "zone": r.zone, "evenements_ajoutes": r.ajoutes.len(),
                "feries_ajoutes": r.feries_ajoutes.len(),
                "differences_acceptees": if r.differences_acceptees { r.differences.len() } else { 0 },
            }),
        )
        .await?;
        apres_ecriture(&mut t, a, annee_id).await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }
}

// ───────────────────────── Lecture du jour, comparaison (STR-30) ─────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SyntheseAnnee {
    pub annee_id: Uuid,
    pub libelle: String,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    /// Jours avec cours (établissement entier).
    pub jours_cours: usize,
    pub jours_ouvres_theoriques: usize,
    /// Semaines avec au moins un jour de cours.
    pub semaines_cours: usize,
    pub semaines_par_etiquette: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Comparaison {
    pub annee: SyntheseAnnee,
    pub reference: SyntheseAnnee,
    pub ecart_jours_cours: i64,
}

async fn synthese(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: &AnneeLue,
) -> Result<SyntheseAnnee, EntDbError> {
    let ctx = charger_contexte(conn, etab, annee.debut, annee.fin).await?;
    let vide = Cible::default();
    let mut jours_cours = 0;
    let mut theoriques = 0;
    let mut semaines: BTreeMap<NaiveDate, Option<String>> = BTreeMap::new();
    for d in annee.debut.iter_days().take_while(|d| *d <= annee.fin) {
        let j = calculer_jour(&ctx, d, &vide);
        let (bm, bp) = ctx
            .jours_ouvres
            .get(cle_jour(d.weekday()))
            .copied()
            .unwrap_or((false, false));
        if bm || bp {
            theoriques += 1;
        }
        if j.ouvre {
            jours_cours += 1;
            semaines.entry(lundi_de(d)).or_insert(j.semaine_type);
        }
    }
    let mut par_etiquette: BTreeMap<String, usize> = BTreeMap::new();
    for e in semaines.values().flatten() {
        *par_etiquette.entry(e.clone()).or_default() += 1;
    }
    Ok(SyntheseAnnee {
        annee_id: annee.id,
        libelle: annee.libelle.clone(),
        debut: annee.debut,
        fin: annee.fin,
        jours_cours,
        jours_ouvres_theoriques: theoriques,
        semaines_cours: semaines.len(),
        semaines_par_etiquette: par_etiquette,
    })
}

impl PgEnt {
    /// Jour d'une date pour une cible (même calcul que le port).
    pub async fn structure_calendrier_jour(
        &self,
        a: &Acteur<'_>,
        date: NaiveDate,
        cible: &Cible,
    ) -> Result<Jour, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let j = calendrier::jour(&mut t, a.etablissement_id, date, cible).await?;
        t.commit().await.map_err(db)?;
        Ok(j)
    }

    pub async fn structure_calendrier_jours(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
        cible: &Cible,
    ) -> Result<Vec<Jour>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let j = calendrier::jours(&mut t, a.etablissement_id, du, au, cible).await?;
        t.commit().await.map_err(db)?;
        Ok(j)
    }

    pub async fn structure_calendrier_jour_service(
        &self,
        a: &Acteur<'_>,
        date: NaiveDate,
        service: &str,
    ) -> Result<JourService, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let j = calendrier::jour_service(&mut t, a.etablissement_id, date, service).await?;
        t.commit().await.map_err(db)?;
        Ok(j)
    }

    /// STR-30 : compare le calendrier d'une année avec une autre (jours de cours, semaines A/B).
    pub async fn structure_calendrier_comparer(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        reference_id: Uuid,
    ) -> Result<Comparaison, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let annees = lire_annees(&mut t, a.etablissement_id, None).await?;
        let trouver = |id: Uuid| {
            annees
                .iter()
                .find(|x| x.id == id)
                .cloned()
                .ok_or_else(introuvable)
        };
        let (x, y) = (trouver(annee_id)?, trouver(reference_id)?);
        let (sx, sy) = (
            synthese(&mut t, a.etablissement_id, &x).await?,
            synthese(&mut t, a.etablissement_id, &y).await?,
        );
        t.commit().await.map_err(db)?;
        Ok(Comparaison {
            ecart_jours_cours: sx.jours_cours as i64 - sy.jours_cours as i64,
            annee: sx,
            reference: sy,
        })
    }
}

// ───────────────────────── Fermeture exceptionnelle (STR-25, 26, 94, R-S1) ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct FermetureSaisie {
    pub libelle: Option<String>,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub du_demi: Option<String>,
    pub au_demi: Option<String>,
    /// Absent : tous les services, **sauf** restauration et internat si des internes sont présents.
    pub services_touches: Option<Vec<String>>,
    #[serde(default)]
    pub internes_presents: bool,
    /// Vide = tout l'établissement. Portées seulement (pas de service).
    #[serde(default)]
    pub cibles: Vec<CibleEvenement>,
    pub motif: String,
    #[serde(default)]
    pub conditionnelle: bool,
    pub levee_prevue_le: Option<DateTime<Utc>>,
    #[serde(default = "vrai")]
    pub publie_familles: bool,
    #[serde(default)]
    pub publie_personnels: bool,
}

fn vrai() -> bool {
    true
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FermetureModif {
    /// Nouvelle dernière date (prolongation ou raccourcissement).
    pub au: Option<NaiveDate>,
    /// `""` retire la demi-journée de fin.
    pub au_demi: Option<String>,
    pub services_touches: Option<Vec<String>>,
    pub levee_prevue_le: Option<DateTime<Utc>>,
    #[serde(default)]
    pub retirer_levee_programmee: bool,
    pub motif: String,
    pub version_attendue: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApercuFermeture {
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub effet: String,
    pub services_touches: Vec<String>,
    /// Services qui restent ouverts (R-S1).
    pub services_maintenus: Vec<String>,
    /// Jours de cours supprimés par la fermeture (selon les cibles).
    pub jours_cours_touches: Vec<NaiveDate>,
    pub chevauchements: Vec<Chevauchement>,
    /// Socles à prévenir, selon les services touchés (numéros du plan).
    pub socles_a_prevenir: Vec<&'static str>,
    pub avertissements: Vec<String>,
    /// Ce que ce lot ne sait pas calculer, dit franchement.
    pub non_calculable: Vec<&'static str>,
    pub levee_prevue_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultatFermeture {
    pub apercu: ApercuFermeture,
    pub evenement: Option<Evenement>,
    pub ecrit: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct HistoriqueFermeture {
    pub id: Uuid,
    pub changement: String,
    pub origine: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub services_touches: Vec<String>,
    pub levee_prevue_le: Option<DateTime<Utc>>,
    pub par: Option<Uuid>,
    pub le: DateTime<Utc>,
}

fn services_valides(v: &[String]) -> Result<Vec<String>, EntDbError> {
    let mut s: Vec<String> = v.iter().map(|x| x.trim().to_string()).collect();
    s.sort();
    s.dedup();
    if s.is_empty() || s.iter().any(|x| !SERVICES_FERMETURE.contains(&x.as_str())) {
        return Err(invalide());
    }
    Ok(s)
}

fn services_par_defaut(internes_presents: bool) -> Vec<String> {
    let mut v: Vec<String> = SERVICES_FERMETURE
        .iter()
        .filter(|s| !(internes_presents && matches!(**s, "restauration" | "internat")))
        .map(|s| s.to_string())
        .collect();
    v.sort(); // même forme que les services saisis (triés), pour comparer les déclarations
    v
}

fn socles_a_prevenir(services: &[String]) -> Vec<&'static str> {
    let mut v = vec!["8 (familles et personnels)"];
    let a = |s: &str| services.iter().any(|x| x == s);
    if a("cours") {
        v.extend(["10 (séances)", "12 (appels non attendus)", "24 (échéances)"]);
    }
    if a("restauration") {
        v.push("40 (repas)");
    }
    if a("internat") {
        v.push("41 (internat)");
    }
    if a("transports") {
        v.push("43 (transports)");
    }
    v
}

async fn apercu_fermeture(
    conn: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
    s: &FermetureSaisie,
    services: Vec<String>,
    sauf: Option<Uuid>,
) -> Result<ApercuFermeture, EntDbError> {
    let ctx = charger_contexte(conn, etab, s.du, s.au).await?;
    let mut jours: BTreeSet<NaiveDate> = BTreeSet::new();
    let cibles: Vec<Cible> = if s.cibles.is_empty() {
        vec![Cible::default()]
    } else {
        s.cibles
            .iter()
            .map(|c| Cible {
                site_id: c.site_id,
                niveau_id: c.niveau_id,
                classe_id: c.classe_id,
                groupe_id: c.groupe_id,
            })
            .collect()
    };
    for c in &cibles {
        let c = resoudre_cible(conn, etab, c).await?;
        for d in s.du.iter_days().take_while(|d| *d <= s.au) {
            let (tm, tp) = (
                !(d == s.du && s.du_demi.as_deref() == Some("apres_midi")),
                !(d == s.au && s.au_demi.as_deref() == Some("matin")),
            );
            let j = calculer_jour(&ctx, d, &c);
            // Compte seulement ce que la fermeture supprime réellement.
            let ferme_cours = services.iter().any(|x| x == "cours");
            if ferme_cours && ((tm && j.matin.cours) || (tp && j.apres_midi.cours)) {
                jours.insert(d);
            }
        }
    }
    let chev = chevauchements(conn, etab, annee_id, s.du, s.au, &s.cibles, sauf).await?;
    let mut avertissements = Vec::new();
    if s.internes_presents {
        let touche_i = services
            .iter()
            .any(|x| x == "internat" || x == "restauration");
        if touche_i {
            avertissements.push(
                "des internes sont présents : l'internat ou la restauration sont fermés par choix explicite".into(),
            );
        } else {
            avertissements.push(
                "des internes sont présents : internat et restauration restent ouverts (R-S1)"
                    .into(),
            );
        }
    }
    if !chev.is_empty() {
        avertissements.push(format!(
            "{} événement(s) du calendrier se chevauchent",
            chev.len()
        ));
    }
    if s.conditionnelle && s.levee_prevue_le.is_none() {
        avertissements
            .push("fermeture conditionnelle sans levée programmée : à lever à la main".into());
    }
    if s.levee_prevue_le
        .is_some_and(|l| l.date_naive() > s.au + Duration::days(1))
    {
        avertissements.push("la levée programmée tombe après la fin de la fermeture".into());
    }
    Ok(ApercuFermeture {
        du: s.du,
        au: s.au,
        effet: effet_de_fermeture(&services).into(),
        services_maintenus: SERVICES_FERMETURE
            .iter()
            .filter(|x| !services.iter().any(|s| s == **x))
            .map(|x| x.to_string())
            .collect(),
        socles_a_prevenir: socles_a_prevenir(&services),
        services_touches: services,
        jours_cours_touches: jours.into_iter().collect(),
        chevauchements: chev,
        avertissements,
        non_calculable: vec![
            "internes présents et repas maintenus : l'internat et la restauration ne sont pas encore \
             dans l'ENT Rust (socles 40 et 41), la déclaration porte l'indication « internes présents »",
            "séances annulées et appels non attendus : les socles 10 et 12 lisent le port jour() et \
             l'événement referentiel.fermeture_exceptionnelle",
            "familles à prévenir : le socle 8 consomme l'événement ; aucun message n'est envoyé par ce lot",
        ],
        levee_prevue_le: s.levee_prevue_le,
    })
}

async fn historiser(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    e: &Evenement,
    changement: &str,
    origine: &str,
    motif: Option<&str>,
) -> Result<Uuid, EntDbError> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_fermeture_historique
           (id, etablissement_id, evenement_id, changement, origine, du, au, services_touches,
            levee_prevue_le, motif, par)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
    )
    .bind(id)
    .bind(a.etablissement_id)
    .bind(e.id)
    .bind(changement)
    .bind(origine)
    .bind(e.du)
    .bind(e.au)
    .bind(&e.services_touches)
    .bind(e.levee_prevue_le)
    .bind(motif)
    .bind((origine == "manuel").then_some(a.user_id))
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(id)
}

fn charge_fermeture(e: &Evenement, changement: &str, historique_id: Uuid) -> Value {
    let mut v = charge_evenement(e, changement);
    v["changement"] = json!(changement);
    v["historique_id"] = json!(historique_id);
    v["internes_presents"] = json!(e.internes_presents);
    v
}

impl PgEnt {
    /// STR-25 / STR-94 : déclare une fermeture exceptionnelle. `apercu_seul` : calcule les effets
    /// sans rien écrire. **Direction et administrateur seulement.** Possible le jour même.
    pub async fn structure_fermeture_declarer(
        &self,
        a: &Acteur<'_>,
        s: FermetureSaisie,
        apercu_seul: bool,
    ) -> Result<ResultatFermeture, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !est_direction(a) {
            return Err(EntDbError::Forbidden);
        }
        let motif = requis(&s.motif, 500)?;
        let libelle =
            net(s.libelle.as_deref(), 160)?.unwrap_or_else(|| "Fermeture exceptionnelle".into());
        valider_demi(&s.du_demi)?;
        valider_demi(&s.au_demi)?;
        if s.au < s.du
            || (s.du == s.au
                && s.du_demi.as_deref() == Some("apres_midi")
                && s.au_demi.as_deref() == Some("matin"))
            || s.cibles.iter().any(|c| c.service.is_some())
            || (s.levee_prevue_le.is_some() && !s.conditionnelle)
            || (s.publie_familles && s.publie_personnels)
        {
            return Err(invalide());
        }
        let services = match &s.services_touches {
            Some(v) => services_valides(v)?,
            None => services_par_defaut(s.internes_presents),
        };
        let mut t = tx(self, a).await?;
        let aujourdhui = port_etab::aujourdhui(&mut t, a.etablissement_id).await?;
        if s.au < aujourdhui {
            return Err(invalide()); // rien à fermer dans le passé
        }
        if s.levee_prevue_le.is_some_and(|l| l <= Utc::now()) {
            return Err(invalide());
        }
        // L'année est celle qui contient le premier jour fermé.
        let annee = lire_annees(&mut t, a.etablissement_id, Some((s.du, s.du)))
            .await?
            .into_iter()
            .max_by_key(|x| x.debut)
            .ok_or_else(introuvable)?;
        annee_ecrivable(&mut t, a.etablissement_id, annee.id).await?;
        valider_cibles(&mut t, a.etablissement_id, annee.id, &s.cibles).await?;
        let apercu = apercu_fermeture(
            &mut t,
            a.etablissement_id,
            annee.id,
            &s,
            services.clone(),
            None,
        )
        .await?;
        if apercu_seul {
            t.commit().await.map_err(db)?;
            return Ok(ResultatFermeture {
                apercu,
                evenement: None,
                ecrit: false,
            });
        }
        // Une déclaration identique et encore active est refusée (pas de double message).
        let doublon = lire_evenements(
            &mut t,
            a.etablissement_id,
            Some(annee.id),
            Some((s.du, s.au)),
            None,
            false,
            false,
        )
        .await?
        .into_iter()
        .any(|e| {
            e.type_evenement == "fermeture_exceptionnelle"
                && e.du == s.du
                && e.au == s.au
                && e.du_demi == s.du_demi
                && e.au_demi == s.au_demi
                && e.services_touches == services
                && {
                    let mut x = e.cibles.clone();
                    let mut y = s.cibles.clone();
                    x.sort();
                    y.sort();
                    x == y
                }
        });
        if doublon {
            return Err(conflit());
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_calendrier_evenement
               (id, etablissement_id, annee_id, type, libelle, du, au, du_demi, au_demi, effet, origine,
                publie_familles, publie_personnels, motif, services_touches, conditionnelle,
                levee_prevue_le, internes_presents, cree_par, modifie_par)
             VALUES ($1,$2,$3,'fermeture_exceptionnelle',$4,$5,$6,$7,$8,$9,'saisie',$10,$11,$12,$13,$14,$15,$16,$17,$17)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(annee.id)
        .bind(&libelle)
        .bind(s.du)
        .bind(s.au)
        .bind(&s.du_demi)
        .bind(&s.au_demi)
        .bind(effet_de_fermeture(&services))
        .bind(s.publie_familles)
        .bind(s.publie_personnels)
        .bind(&motif)
        .bind(&services)
        .bind(s.conditionnelle)
        .bind(s.levee_prevue_le)
        .bind(s.internes_presents)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        inserer_cibles(&mut t, a.etablissement_id, id, &s.cibles).await?;
        let e = lire_evenements(
            &mut t,
            a.etablissement_id,
            None,
            None,
            Some(id),
            true,
            false,
        )
        .await?
        .pop()
        .ok_or_else(introuvable)?;
        let h = historiser(&mut t, a, &e, "declaree", "manuel", Some(&motif)).await?;
        journaliser(
            &mut t,
            a,
            "fermeture",
            &id.to_string(),
            "declarer",
            None,
            Some(resume_journal(&e)),
        )
        .await?;
        publier(
            &mut t,
            a,
            "referentiel.fermeture_exceptionnelle",
            id,
            charge_fermeture(&e, "declaree", h),
        )
        .await?;
        apres_ecriture(&mut t, a, annee.id).await?;
        t.commit().await.map_err(db)?;
        Ok(ResultatFermeture {
            apercu,
            evenement: Some(e),
            ecrit: true,
        })
    }

    /// STR-94 : prolonge ou raccourcit une fermeture, change ses services, programme ou retire
    /// sa levée. **Un seul événement par changement.**
    pub async fn structure_fermeture_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        m: FermetureModif,
    ) -> Result<ResultatFermeture, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !est_direction(a) {
            return Err(EntDbError::Forbidden);
        }
        let motif = requis(&m.motif, 500)?;
        let mut t = tx(self, a).await?;
        let avant = lire_evenements(&mut t, a.etablissement_id, None, None, Some(id), true, true)
            .await?
            .pop()
            .filter(|e| e.type_evenement == "fermeture_exceptionnelle")
            .ok_or_else(introuvable)?;
        if avant.annule_le.is_some() || m.version_attendue.is_some_and(|v| v != avant.version) {
            return Err(conflit());
        }
        annee_ecrivable(&mut t, a.etablissement_id, avant.annee_id).await?;
        let aujourdhui = port_etab::aujourdhui(&mut t, a.etablissement_id).await?;
        let au = m.au.unwrap_or(avant.au);
        let au_demi = match m.au_demi.as_deref() {
            Some("") => None,
            Some(v) => Some(v.to_string()),
            None => avant.au_demi.clone(),
        };
        valider_demi(&au_demi)?;
        if au < avant.du
            || au < aujourdhui
            || (au == avant.du
                && avant.du_demi.as_deref() == Some("apres_midi")
                && au_demi.as_deref() == Some("matin"))
            || (m.levee_prevue_le.is_some() && m.retirer_levee_programmee)
        {
            return Err(invalide());
        }
        let services = match &m.services_touches {
            Some(v) => services_valides(v)?,
            None => avant.services_touches.clone(),
        };
        let levee = if m.retirer_levee_programmee {
            None
        } else {
            m.levee_prevue_le.or(avant.levee_prevue_le)
        };
        if levee.is_some_and(|l| m.levee_prevue_le.is_some() && l <= Utc::now()) {
            return Err(invalide());
        }
        if levee.is_some() && !avant.conditionnelle {
            return Err(invalide());
        }
        let dates_changent = au != avant.au || au_demi != avant.au_demi;
        let services_changent = services != avant.services_touches;
        let levee_change = levee != avant.levee_prevue_le;
        if !dates_changent && !services_changent && !levee_change {
            return Err(conflit()); // rien à changer : pas de message
        }
        let changement = if au > avant.au {
            "prolongee"
        } else if dates_changent {
            "raccourcie"
        } else if services_changent {
            "services_modifies"
        } else if avant.levee_prevue_le.is_some() && levee.is_some() {
            "levee_programmee_modifiee"
        } else {
            "levee_programmee"
        };
        sqlx::query(
            "UPDATE ent_calendrier_evenement SET au = $3, au_demi = $4, services_touches = $5, effet = $6,
                    levee_prevue_le = $7, version = version + 1, modifie_par = $8, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(au)
        .bind(&au_demi)
        .bind(&services)
        .bind(effet_de_fermeture(&services))
        .bind(levee)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let e = lire_evenements(
            &mut t,
            a.etablissement_id,
            None,
            None,
            Some(id),
            true,
            false,
        )
        .await?
        .pop()
        .ok_or_else(introuvable)?;
        let saisie = FermetureSaisie {
            libelle: None,
            du: e.du,
            au: e.au,
            du_demi: e.du_demi.clone(),
            au_demi: e.au_demi.clone(),
            services_touches: Some(e.services_touches.clone()),
            internes_presents: e.internes_presents,
            cibles: e.cibles.clone(),
            motif: motif.clone(),
            conditionnelle: e.conditionnelle,
            levee_prevue_le: e.levee_prevue_le,
            publie_familles: e.publie_familles,
            publie_personnels: e.publie_personnels,
        };
        let apercu = apercu_fermeture(
            &mut t,
            a.etablissement_id,
            e.annee_id,
            &saisie,
            e.services_touches.clone(),
            Some(id),
        )
        .await?;
        let h = historiser(&mut t, a, &e, changement, "manuel", Some(&motif)).await?;
        journaliser(
            &mut t,
            a,
            "fermeture",
            &id.to_string(),
            changement,
            Some(resume_journal(&avant)),
            Some(resume_journal(&e)),
        )
        .await?;
        publier(
            &mut t,
            a,
            "referentiel.fermeture_exceptionnelle",
            id,
            charge_fermeture(&e, changement, h),
        )
        .await?;
        apres_ecriture(&mut t, a, e.annee_id).await?;
        t.commit().await.map_err(db)?;
        Ok(ResultatFermeture {
            apercu,
            evenement: Some(e),
            ecrit: true,
        })
    }

    /// STR-26 : lève une fermeture (entièrement, ou à partir d'une date : les jours déjà fermés
    /// le restent). `a_partir_du` absent : à partir d'aujourd'hui (jour de l'établissement).
    pub async fn structure_fermeture_lever(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        motif: &str,
        a_partir_du: Option<NaiveDate>,
    ) -> Result<Evenement, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !est_direction(a) {
            return Err(EntDbError::Forbidden);
        }
        let motif = requis(motif, 500)?;
        let mut t = tx(self, a).await?;
        let aujourdhui = port_etab::aujourdhui(&mut t, a.etablissement_id).await?;
        let e = lever_dans(
            &mut t,
            a,
            id,
            a_partir_du.unwrap_or(aujourdhui),
            None,
            "manuel",
            Some(&motif),
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(e)
    }

    /// Historique des changements d'une fermeture (un événement par ligne).
    pub async fn structure_fermeture_historique(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<HistoriqueFermeture>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let v = sqlx::query_as(
            "SELECT id, changement, origine, du, au, services_touches, levee_prevue_le, par, le
               FROM ent_fermeture_historique WHERE etablissement_id = $1 AND evenement_id = $2
              ORDER BY le, id",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Exécute les levées programmées échues d'un établissement : **une levée = un événement**,
    /// même si la tâche est relancée (la levée programmée est effacée dans la même transaction).
    /// Retourne le nombre de fermetures levées.
    pub async fn structure_fermetures_levees_executer(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        let mut n = 0;
        loop {
            let mut t = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let due: Option<(Uuid, DateTime<Utc>)> = sqlx::query_as(
                "SELECT id, levee_prevue_le FROM ent_calendrier_evenement
                  WHERE etablissement_id = $1 AND type = 'fermeture_exceptionnelle' AND annule_le IS NULL
                    AND levee_prevue_le IS NOT NULL AND levee_prevue_le <= $2
                  ORDER BY levee_prevue_le LIMIT 1 FOR UPDATE SKIP LOCKED",
            )
            .bind(etab)
            .bind(maintenant)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?;
            let Some((id, instant)) = due else {
                t.commit().await.map_err(db)?;
                return Ok(n);
            };
            // Le jour métier et la demi-journée de la levée viennent du fuseau de l'établissement.
            let local = port_etab::heure_locale(&mut t, etab, instant).await?;
            let coupure =
                port_etab::parametre(&mut t, etab, "structure.demi_journee_coupure").await?;
            let coupure = coupure
                .as_str()
                .and_then(|s| NaiveTime::parse_from_str(s, "%H:%M").ok())
                .unwrap_or_else(|| NaiveTime::from_hms_opt(12, 30, 0).expect("heure valide"));
            let systeme = Acteur {
                etablissement_id: etab,
                user_id: Uuid::nil(),
                role: "systeme",
            };
            // Levée le matin : ouvert dès ce jour ; levée l'après-midi : le matin reste fermé.
            let (jour, apres_midi) = (local.date(), local.time() >= coupure);
            lever_dans(
                &mut t,
                &systeme,
                id,
                jour,
                Some(apres_midi),
                "programme",
                None,
            )
            .await?;
            t.commit().await.map_err(db)?;
            n += 1;
        }
    }

    /// Tâche de fond : exécute les levées échues de tous les établissements.
    pub async fn structure_fermetures_levees_tache(&self) -> Result<usize, EntDbError> {
        let etabs: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT etablissement_id FROM ent_calendrier_evenement
              WHERE type = 'fermeture_exceptionnelle' AND annule_le IS NULL
                AND levee_prevue_le IS NOT NULL AND levee_prevue_le <= now()",
        )
        .fetch_all(self.pool())
        .await?;
        let mut n = 0;
        for (etab,) in etabs {
            match self
                .structure_fermetures_levees_executer(etab, Utc::now())
                .await
            {
                Ok(k) => n += k,
                Err(e) => tracing::warn!(error = %e, "levée programmée de fermeture non exécutée"),
            }
        }
        Ok(n)
    }
}

/// Lève une fermeture à partir d'un jour. `apres_midi_seulement` (levée programmée) :
/// `Some(true)` = le matin du jour reste fermé. Toute levée retire la levée programmée.
async fn lever_dans(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    id: Uuid,
    jour: NaiveDate,
    apres_midi_seulement: Option<bool>,
    origine: &str,
    motif: Option<&str>,
) -> Result<Evenement, EntDbError> {
    let avant = lire_evenements(conn, a.etablissement_id, None, None, Some(id), true, true)
        .await?
        .pop()
        .filter(|e| e.type_evenement == "fermeture_exceptionnelle")
        .ok_or_else(introuvable)?;
    if avant.annule_le.is_some() {
        return Err(conflit());
    }
    // Nouvelle fin : la veille du jour de levée, ou le matin de ce jour (levée l'après-midi).
    let (nouveau_au, nouveau_demi): (NaiveDate, Option<&str>) =
        if apres_midi_seulement == Some(true) {
            (jour, Some("matin"))
        } else {
            (jour - Duration::days(1), None)
        };
    // Rien à lever si la fermeture est déjà finie à cette date (fin actuelle <= nouvelle fin).
    let rang = |demi: Option<&str>| if demi == Some("matin") { 1 } else { 2 };
    let deja_finie = (avant.au, rang(avant.au_demi.as_deref())) <= (nouveau_au, rang(nouveau_demi));
    if deja_finie {
        // Levée programmée devenue sans objet : on l'efface sans message.
        if origine == "programme" {
            sqlx::query(
                "UPDATE ent_calendrier_evenement SET levee_prevue_le = NULL, modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            return lire_evenements(conn, a.etablissement_id, None, None, Some(id), true, false)
                .await?
                .pop()
                .ok_or_else(introuvable);
        }
        return Err(conflit());
    }
    let entiere = nouveau_au < avant.du
        || (nouveau_au == avant.du
            && nouveau_demi == Some("matin")
            && avant.du_demi.as_deref() == Some("apres_midi"));
    if entiere {
        sqlx::query(
            "UPDATE ent_calendrier_evenement SET annule_le = now(), annule_par = $3, annule_motif = $4,
                    levee_prevue_le = NULL, version = version + 1, modifie_par = $3, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind((origine == "manuel").then_some(a.user_id))
        .bind(motif.map(str::to_string).unwrap_or_else(|| "levée programmée".to_string()))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    } else {
        sqlx::query(
            "UPDATE ent_calendrier_evenement SET au = $3, au_demi = $4, levee_prevue_le = NULL,
                    version = version + 1, modifie_par = $5, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(nouveau_au)
        .bind(nouveau_demi)
        .bind(a.user_id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    let e = lire_evenements(conn, a.etablissement_id, None, None, Some(id), true, false)
        .await?
        .pop()
        .ok_or_else(introuvable)?;
    let h = historiser(conn, a, &e, "levee", origine, motif).await?;
    journaliser(
        conn,
        a,
        "fermeture",
        &id.to_string(),
        "lever",
        Some(resume_journal(&avant)),
        Some(resume_journal(&e)),
    )
    .await?;
    let mut charge = charge_fermeture(&e, "levee", h);
    charge["a_partir_du"] = json!(jour);
    charge["apres_midi_seulement"] = json!(apres_midi_seulement == Some(true));
    charge["complete"] = json!(entiere);
    charge["origine"] = json!(origine);
    publier(conn, a, "referentiel.fermeture_levee", id, charge).await?;
    apres_ecriture(conn, a, e.annee_id).await?;
    Ok(e)
}

#[cfg(test)]
mod tests_purs {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    #[test]
    fn effet_de_fermeture_suit_les_services() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            effet_de_fermeture(&s(&["cours", "accueil_eleves"])),
            "sans_cours"
        );
        assert_eq!(effet_de_fermeture(&s(&["cours"])), "accueil_seulement");
        assert_eq!(effet_de_fermeture(&s(&["restauration"])), "cours_modifies");
    }

    #[test]
    fn services_par_defaut_gardent_internat_et_restauration_ouverts() {
        let sans = services_par_defaut(true);
        assert!(
            !sans.contains(&"internat".to_string()) && !sans.contains(&"restauration".to_string())
        );
        assert!(sans.contains(&"cours".to_string()));
        assert_eq!(services_par_defaut(false).len(), 6);
    }

    #[test]
    fn etiquettes_modes_continue_et_reprise() {
        let regle = |mode: &str| RegleSemaine {
            etiquettes: vec!["A".into(), "B".into()],
            mode: mode.into(),
            premiere_semaine: d(2026, 9, 7),
        };
        let l = |n: i64, v: bool| (d(2026, 8, 31) + Duration::days(7 * n), v);
        // 31/8 (avant la première), 7/9 A, 14/9 B, 21/9 vacances, 28/9 ?, 5/10 ?
        let semaines = vec![
            l(0, false),
            l(1, false),
            l(2, false),
            l(3, true),
            l(4, false),
            l(5, false),
        ];
        let c: Vec<_> = etiquettes_semaines(&semaines, &regle("alternance_continue"));
        assert_eq!(
            c,
            vec![
                None,
                Some("A".into()),
                Some("B".into()),
                None,
                Some("A".into()),
                Some("B".into())
            ]
        );
        let r: Vec<_> = etiquettes_semaines(&semaines, &regle("reprise_apres_vacances"));
        assert_eq!(
            r,
            vec![
                None,
                Some("A".into()),
                Some("B".into()),
                None,
                Some("A".into()),
                Some("B".into())
            ]
        );
        // Une seule semaine avant les vacances : la reprise repart de A, l'alternance continue de B.
        let court = vec![l(1, false), l(2, true), l(3, false)];
        let r = etiquettes_semaines(&court, &regle("reprise_apres_vacances"));
        assert_eq!(r[2], Some("A".into()));
        let c = etiquettes_semaines(&court, &regle("alternance_continue"));
        assert_eq!(c[2], Some("B".into()));
    }

    fn ctx(evenements: Vec<Evenement>) -> (Contexte, Uuid) {
        let annee = Uuid::new_v4();
        let evts = evenements
            .into_iter()
            .map(|mut e| {
                e.annee_id = annee;
                e
            })
            .collect();
        (
            Contexte::nouveau(
                &serde_json::from_str(JOURS_OUVRES_DEFAUT).unwrap(),
                vec![AnneeLue {
                    id: annee,
                    libelle: "2026-2027".into(),
                    debut: d(2026, 9, 1),
                    fin: d(2027, 8, 31),
                    cloturee: false,
                }],
                evts,
                vec![],
            ),
            annee,
        )
    }

    fn evt(type_e: &str, du: NaiveDate, au: NaiveDate, effet: &str) -> Evenement {
        Evenement {
            id: Uuid::new_v4(),
            annee_id: Uuid::nil(),
            type_evenement: type_e.into(),
            libelle: type_e.into(),
            du,
            au,
            du_demi: None,
            au_demi: None,
            effet: effet.into(),
            origine: "saisie".into(),
            ref_officiel_id: None,
            ref_version: None,
            publie_familles: false,
            publie_personnels: false,
            motif: None,
            jour_reference: None,
            services_touches: vec![],
            conditionnelle: false,
            levee_prevue_le: None,
            internes_presents: false,
            version: 1,
            annule_le: None,
            annule_motif: None,
            cree_le: Utc::now(),
            cibles: vec![],
        }
    }

    #[test]
    fn jour_de_base_demi_journees_et_hors_annee() {
        let (c, _) = ctx(vec![]);
        let v = Cible::default();
        // Mercredi 2 septembre 2026 : matin seulement.
        let j = calculer_jour(&c, d(2026, 9, 2), &v);
        assert!(j.ouvre && j.matin.cours && !j.apres_midi.cours);
        assert_eq!(j.type_jour, "cours");
        // Dimanche : non travaillé. Août 2026 : hors année.
        assert_eq!(
            calculer_jour(&c, d(2026, 9, 6), &v).type_jour,
            "non_travaille"
        );
        assert_eq!(
            calculer_jour(&c, d(2026, 8, 15), &v).type_jour,
            "hors_annee"
        );
    }

    #[test]
    fn demi_journee_et_priorite_des_types() {
        let mut v = evt("vacances", d(2026, 10, 19), d(2026, 10, 30), "sans_cours");
        v.du_demi = Some("apres_midi".into());
        let f = evt("ferie", d(2026, 11, 2), d(2026, 11, 2), "sans_cours");
        let mut f2 = evt("ferie", d(2026, 10, 20), d(2026, 10, 20), "sans_cours");
        f2.cree_le = Utc::now();
        let (c, _) = ctx(vec![v, f, f2]);
        let vide = Cible::default();
        // Lundi 19 : vacances à partir de l'après-midi → matin avec cours, après-midi sans.
        let j = calculer_jour(&c, d(2026, 10, 19), &vide);
        assert!(j.matin.cours && !j.apres_midi.cours && j.ouvre);
        // Mardi 20 : vacances + férié → le férié prime dans le type.
        let j = calculer_jour(&c, d(2026, 10, 20), &vide);
        assert!(!j.ouvre);
        assert_eq!(j.type_jour, "ferie");
        assert_eq!(j.evenements.len(), 2);
    }

    #[test]
    fn cibles_niveau_classe_et_rattrapage() {
        let (niveau, classe) = (Uuid::new_v4(), Uuid::new_v4());
        let mut banalise = evt(
            "journee_banalisee",
            d(2026, 9, 10),
            d(2026, 9, 10),
            "sans_cours",
        );
        banalise.cibles = vec![CibleEvenement {
            niveau_id: Some(niveau),
            ..Default::default()
        }];
        let mut rat = evt(
            "rattrapage",
            d(2026, 9, 12),
            d(2026, 9, 12),
            "cours_modifies",
        );
        rat.jour_reference = Some("lun".into());
        let (c, _) = ctx(vec![banalise, rat]);
        let n = Cible {
            niveau_id: Some(niveau),
            ..Default::default()
        };
        let autre = Cible {
            classe_id: Some(classe),
            ..Default::default()
        };
        assert!(!calculer_jour(&c, d(2026, 9, 10), &n).ouvre);
        assert!(calculer_jour(&c, d(2026, 9, 10), &autre).ouvre);
        assert!(calculer_jour(&c, d(2026, 9, 10), &Cible::default()).ouvre);
        // Samedi 12 septembre rattrapé comme un lundi : matin et après-midi.
        let j = calculer_jour(&c, d(2026, 9, 12), &Cible::default());
        assert!(j.ouvre && j.matin.cours && j.apres_midi.cours);
        assert_eq!(j.type_jour, "rattrapage");
        assert_eq!(j.jour_reference.as_deref(), Some("lun"));
    }

    #[test]
    fn fermeture_effets_differencies_et_services() {
        let mut f = evt(
            "fermeture_exceptionnelle",
            d(2026, 11, 9),
            d(2026, 11, 10),
            "accueil_seulement",
        );
        f.services_touches = vec!["cours".into(), "transports".into()];
        let (c, _) = ctx(vec![f]);
        let j = calculer_jour(&c, d(2026, 11, 9), &Cible::default());
        assert!(
            !j.ouvre && j.matin.eleves && j.matin.personnel,
            "accueil et personnels maintenus"
        );
        assert_eq!(j.type_jour, "fermeture_exceptionnelle");
        assert_eq!(
            j.services_fermes,
            vec!["cours".to_string(), "transports".to_string()]
        );
        let s = calculer_jour_service(&c, d(2026, 11, 9), "restauration");
        assert!(s.ouvert, "la restauration n'est pas touchée");
    }

    #[test]
    fn services_jours_ouvres_administratifs() {
        let vacances = evt("vacances", d(2026, 10, 19), d(2026, 10, 30), "sans_cours");
        let ferie = evt("ferie", d(2026, 11, 11), d(2026, 11, 11), "sans_cours");
        let mut ferme = evt("fermeture", d(2027, 7, 20), d(2027, 8, 20), "sans_cours");
        ferme.cibles = vec![CibleEvenement {
            service: Some("secretariat".into()),
            ..Default::default()
        }];
        let (c, _) = ctx(vec![vacances, ferie, ferme]);
        // Les vacances ne ferment pas le secrétariat ; un férié, si ; un samedi, si.
        assert!(calculer_jour_service(&c, d(2026, 10, 21), "secretariat").ouvert);
        assert_eq!(
            calculer_jour_service(&c, d(2026, 11, 11), "secretariat")
                .raison
                .as_deref(),
            Some("ferie")
        );
        assert_eq!(
            calculer_jour_service(&c, d(2026, 11, 14), "secretariat")
                .raison
                .as_deref(),
            Some("week_end")
        );
        // Fermeture estivale du secrétariat seulement.
        assert!(!calculer_jour_service(&c, d(2027, 8, 4), "secretariat").ouvert);
        assert!(calculer_jour_service(&c, d(2027, 8, 4), "intendance").ouvert);
    }
}
