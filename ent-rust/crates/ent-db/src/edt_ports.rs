//! Socle 10, lot T2 — ports exposés aux autres socles (`docs/socles/10_EMPLOI_DU_TEMPS.md` §6.2) :
//! séances, séance en cours, occupation d'un lieu, plages libres, séances touchées, et — pour la
//! réservation de salles (D-058) et l'agenda à calques (D-059) — les **salles libérées** par l'emploi du
//! temps réel et les **cours supprimés ou modifiés**.
//!
//! Principes :
//! * **Lecture seule, jamais de copie** : tout est calculé sur `ent_edt_seance` (la séance datée) et
//!   `ent_edt_seance_lieu` ; un autre socle n'écrit pas ici et ne garde pas de double.
//! * **La portée de l'appelant s'applique** (D-022) : la lecture nominative d'un élève passe par le
//!   périmètre unique ; un professeur ne lit que sa propre séance ou ses classes ; un collègue n'est
//!   jamais détaillé. Les ports d'occupation et de disponibilité sont anonymisés (créneau, classe, état :
//!   ni nom d'intervenant, ni motif).
//! * **Le motif interne n'existe pas ici** : seule la colonne `motif_public` (200 caractères, choisie pour
//!   être lue des familles) sort. Une salle « libérée » dit pourquoi (« cours annulé ») sans révéler le
//!   motif d'une absence de professeur (donnée personnelle, D-058).
//! * Les séances `retiree` ne sortent jamais. Occupent un lieu : `prevue`, `modifiee`, `ajoutee` ; ne
//!   l'occupent pas : `annulee`, `deplacee`, `neutralisee`, `non_assuree`, `retiree`.

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use ent_auth::Role;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::ref_commun::{db, invalide, Acteur};
use crate::referentiel_structure::index_neutralises;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Plage maximale d'une lecture (jours), comme les autres lectures d'EDT.
pub const PORT_PLAGE_MAX_JOURS: i64 = 62;

/// États qui occupent un lieu et mobilisent leur public.
const ETATS_OCCUPANTS: [&str; 3] = ["prevue", "modifiee", "ajoutee"];

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct SeanceEdt {
    pub id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub debut_utc: Option<DateTime<Utc>>,
    pub fin_utc: Option<DateTime<Utc>>,
    pub cours_id: Option<Uuid>,
    pub placement_id: Option<Uuid>,
    pub version_id: Option<Uuid>,
    pub creneau_id: Uuid,
    pub creneau_code: Option<String>,
    pub classe_id: Option<Uuid>,
    pub classe: String,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    pub remplacant_user_id: Option<Uuid>,
    /// Salle en texte (projection historique) ; `lieux` est la source.
    pub salle: Option<String>,
    pub etat: String,
    /// Motif PUBLIC seulement.
    pub motif_public: Option<String>,
    pub origine: String,
    /// Lieux, le principal en premier.
    pub lieux: Vec<Uuid>,
}

const SELECT_SEANCE: &str = "SELECT s.id, s.jour, s.heure_debut, s.heure_fin, s.debut_utc, s.fin_utc, s.cours_id,
        s.placement_id, s.version_id, s.creneau_id, s.creneau_code, s.classe_id, s.classe, s.matiere_libelle,
        s.prof_user_id, s.remplacant_user_id, s.salle, s.etat, s.motif_public, s.origine,
        coalesce((SELECT array_agg(l.lieu_id ORDER BY l.principal DESC, l.lieu_id)
                    FROM ent_edt_seance_lieu l
                   WHERE l.seance_id = s.id AND l.etablissement_id = s.etablissement_id), '{}') AS lieux
   FROM ent_edt_seance s";

/// Cible d'une lecture de séances (§6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum CibleSeances {
    Eleve(Uuid),
    Cours(Uuid),
    Classe(Uuid),
    Groupe(Uuid),
    /// Compte de l'intervenant (titulaire ou remplaçant).
    Intervenant(Uuid),
    Lieu(Uuid),
}

/// Ressource dont on cherche les plages libres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum RessourcePlan {
    Lieu(Uuid),
    Intervenant(Uuid),
    Classe(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlageLibre {
    pub jour: NaiveDate,
    pub debut: NaiveTime,
    pub fin: NaiveTime,
}

/// Occupation anonymisée d'un lieu (D-058 : ce qui occupe, pas qui).
#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct OccupationLieu {
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub cours_id: Option<Uuid>,
    pub classe: String,
    pub etat: String,
}

/// Pourquoi une salle est libre (D-058). Jamais le motif d'absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaisonSalleLibre {
    CoursAnnule,
    CoursDeplace,
    CoursNeutralise,
    CoursNonAssure,
}

impl RaisonSalleLibre {
    fn depuis_etat(etat: &str) -> Option<Self> {
        match etat {
            "annulee" => Some(Self::CoursAnnule),
            "deplacee" => Some(Self::CoursDeplace),
            "neutralisee" => Some(Self::CoursNeutralise),
            "non_assuree" => Some(Self::CoursNonAssure),
            _ => None,
        }
    }
}

/// Salle de classe libérée par un cours supprimé (D-058). Le demandeur de réservation voit
/// « cours de 4B annulé », jamais le motif d'absence du professeur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalleLiberee {
    pub lieu_id: Uuid,
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub cours_id: Option<Uuid>,
    pub classe: String,
    pub raison: RaisonSalleLibre,
    pub motif_public: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct SeanceChangee {
    pub seance_id: Uuid,
    pub jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub cours_id: Option<Uuid>,
    pub classe: String,
    pub etat: String,
    pub motif_public: Option<String>,
    pub lieux: Vec<Uuid>,
}

// ───────────────────────────── Logique pure ─────────────────────────────

/// Retire de `fenetre` les intervalles `occupes` et rend les plages restantes d'au moins `duree`.
/// Fonction pure (testée sans base) : les intervalles peuvent se chevaucher ou déborder.
pub fn plages_restantes(
    fenetre: (NaiveTime, NaiveTime),
    occupes: &[(NaiveTime, NaiveTime)],
    duree: Duration,
) -> Vec<(NaiveTime, NaiveTime)> {
    let mut tries: Vec<(NaiveTime, NaiveTime)> = occupes
        .iter()
        .filter(|(d, f)| f > d && *f > fenetre.0 && *d < fenetre.1)
        .map(|(d, f)| ((*d).max(fenetre.0), (*f).min(fenetre.1)))
        .collect();
    tries.sort();
    let mut libres = Vec::new();
    let mut curseur = fenetre.0;
    for (d, f) in tries {
        if d > curseur && d - curseur >= duree {
            libres.push((curseur, d));
        }
        curseur = curseur.max(f);
    }
    if fenetre.1 > curseur && fenetre.1 - curseur >= duree {
        libres.push((curseur, fenetre.1));
    }
    libres
}

fn num_jour(lun_dim: &str) -> Option<u32> {
    match lun_dim {
        "lun" => Some(1),
        "mar" => Some(2),
        "mer" => Some(3),
        "jeu" => Some(4),
        "ven" => Some(5),
        "sam" => Some(6),
        "dim" => Some(7),
        _ => None,
    }
}

fn est_personnel(role: &str) -> bool {
    Role::parse(role).is_some_and(|r| r.is_staff())
}

fn verifier_plage(du: NaiveDate, au: NaiveDate) -> Result<(), EntDbError> {
    if au < du || (au - du).num_days() > PORT_PLAGE_MAX_JOURS {
        Err(invalide())
    } else {
        Ok(())
    }
}

// ───────────────────────────── Ports ─────────────────────────────

impl PgEnt {
    /// `edt::seances(cible, plage)` : séances (hors retirées) de la cible, avec la portée de l'appelant.
    pub async fn edt_seances(
        &self,
        a: &Acteur<'_>,
        cible: CibleSeances,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceEdt>, EntDbError> {
        verifier_plage(du, au)?;
        let e = a.etablissement_id;
        match cible {
            CibleSeances::Eleve(id) => self.edt_acces_eleve(a, id).await?,
            CibleSeances::Intervenant(u) => {
                if u != a.user_id && !Self::role_can_manage_edt(a.role) {
                    return Err(EntDbError::Forbidden);
                }
                if !est_personnel(a.role) {
                    return Err(EntDbError::Forbidden);
                }
            }
            CibleSeances::Lieu(_) => {
                if !Self::role_can_manage_edt(a.role) {
                    return Err(EntDbError::Forbidden);
                }
            }
            CibleSeances::Cours(_) | CibleSeances::Classe(_) | CibleSeances::Groupe(_) => {
                if !est_personnel(a.role) {
                    return Err(EntDbError::Forbidden);
                }
            }
        }
        let mut tx = TenantTx::begin(self.pool(), e).await?;
        let (filtre, id): (&str, Uuid) = match cible {
            CibleSeances::Eleve(id) => (
                "AND (EXISTS (SELECT 1 FROM ent_public_cours_jour(s.etablissement_id, s.jour, s.cours_id, NULL, $4) p)
                      OR (s.cours_id IS NULL AND s.classe_id IN
                            (SELECT classe_id FROM ent_affectation f
                              WHERE f.etablissement_id = s.etablissement_id AND f.eleve_id = $4
                                AND f.classe_id IS NOT NULL AND f.date_debut <= s.jour
                                AND (f.date_fin IS NULL OR f.date_fin >= s.jour))))",
                id,
            ),
            CibleSeances::Cours(id) => ("AND s.cours_id = $4", id),
            CibleSeances::Classe(id) => ("AND s.classe_id = $4", id),
            CibleSeances::Groupe(id) => (
                "AND s.cours_id IN (SELECT c.id FROM ent_cours c
                                     WHERE c.etablissement_id = s.etablissement_id AND c.groupe_id = $4)",
                id,
            ),
            CibleSeances::Intervenant(id) => ("AND (s.prof_user_id = $4 OR s.remplacant_user_id = $4)", id),
            CibleSeances::Lieu(id) => (
                "AND EXISTS (SELECT 1 FROM ent_edt_seance_lieu l
                              WHERE l.seance_id = s.id AND l.etablissement_id = s.etablissement_id AND l.lieu_id = $4)",
                id,
            ),
        };
        let sql = format!(
            "{SELECT_SEANCE}
              WHERE s.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3 AND s.etat <> 'retiree' {filtre}
              ORDER BY s.jour, s.heure_debut, s.classe, s.id"
        );
        sqlx::query_as::<_, SeanceEdt>(&sql)
            .bind(e)
            .bind(du)
            .bind(au)
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)
    }

    /// `edt::seance(id)` : une séance, pour le personnel concerné (le titulaire, le remplaçant, ou ceux
    /// qui gèrent l'emploi du temps). Une famille ou un élève passe par `edt_seances(Eleve)`.
    pub async fn edt_seance(&self, a: &Acteur<'_>, id: Uuid) -> Result<SeanceEdt, EntDbError> {
        if !est_personnel(a.role) {
            return Err(EntDbError::Forbidden);
        }
        let e = a.etablissement_id;
        let s = sqlx::query_as::<_, SeanceEdt>(&format!(
            "{SELECT_SEANCE} WHERE s.etablissement_id = $1 AND s.id = $2 AND s.etat <> 'retiree'"
        ))
        .bind(e)
        .bind(id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), e).await?)
        .await
        .map_err(db)?
        .ok_or(EntDbError::Forbidden)?; // inconnue ou retirée : même refus (pas d'oracle)
        let concerne = s.prof_user_id == a.user_id || s.remplacant_user_id == Some(a.user_id);
        if !concerne && !Self::role_can_manage_edt(a.role) {
            return Err(EntDbError::Forbidden);
        }
        Ok(s)
    }

    /// `edt::seance_en_cours(eleve, instant)` : « maintenant » pour un élève (séance occupante).
    pub async fn edt_seance_en_cours(
        &self,
        a: &Acteur<'_>,
        eleve_id: Uuid,
        instant: DateTime<Utc>,
    ) -> Result<Option<SeanceEdt>, EntDbError> {
        self.edt_acces_eleve(a, eleve_id).await?;
        let e = a.etablissement_id;
        let etats: Vec<&str> = ETATS_OCCUPANTS.to_vec();
        sqlx::query_as::<_, SeanceEdt>(&format!(
            "{SELECT_SEANCE}
              WHERE s.etablissement_id = $1 AND s.debut_utc <= $2 AND $2 < s.fin_utc AND s.etat = ANY($4)
                AND EXISTS (SELECT 1 FROM ent_public_cours_jour(s.etablissement_id, s.jour, s.cours_id, NULL, $3) p)
              ORDER BY s.debut_utc LIMIT 1"
        ))
        .bind(e)
        .bind(instant)
        .bind(eleve_id)
        .bind(&etats)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), e).await?)
        .await
        .map_err(db)
    }

    /// `edt::seances_touchees(intervenant, plage)` : séances qu'une absence de l'intervenant touche
    /// (titulaire, hors annulées et retirées) — pour le socle 15.
    pub async fn edt_seances_touchees(
        &self,
        a: &Acteur<'_>,
        intervenant_user_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceEdt>, EntDbError> {
        let toutes = self
            .edt_seances(a, CibleSeances::Intervenant(intervenant_user_id), du, au)
            .await?;
        Ok(toutes
            .into_iter()
            .filter(|s| {
                s.prof_user_id == intervenant_user_id
                    && !matches!(s.etat.as_str(), "annulee" | "deplacee" | "neutralisee")
            })
            .collect())
    }

    /// Occupation anonymisée d'un lieu sur une plage (D-058).
    pub async fn edt_occupation_lieu(
        &self,
        a: &Acteur<'_>,
        lieu_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<OccupationLieu>, EntDbError> {
        if !est_personnel(a.role) {
            return Err(EntDbError::Forbidden);
        }
        verifier_plage(du, au)?;
        let e = a.etablissement_id;
        let etats: Vec<&str> = ETATS_OCCUPANTS.to_vec();
        sqlx::query_as::<_, OccupationLieu>(
            "SELECT s.id AS seance_id, s.jour, s.heure_debut, s.heure_fin, s.cours_id, s.classe, s.etat
               FROM ent_edt_seance s
               JOIN ent_edt_seance_lieu l ON l.seance_id = s.id AND l.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1 AND l.lieu_id = $2 AND s.jour BETWEEN $3 AND $4 AND s.etat = ANY($5)
              ORDER BY s.jour, s.heure_debut, s.id",
        )
        .bind(e)
        .bind(lieu_id)
        .bind(du)
        .bind(au)
        .bind(&etats)
        .fetch_all(&mut *TenantTx::begin(self.pool(), e).await?)
        .await
        .map_err(db)
    }

    /// `edt::occupe(ressource, plage)` : le lieu est-il occupé par un cours (ou indisponible) sur
    /// `[debut, fin)` ce jour-là ?
    pub async fn edt_occupe(
        &self,
        a: &Acteur<'_>,
        lieu_id: Uuid,
        jour: NaiveDate,
        debut: NaiveTime,
        fin: NaiveTime,
    ) -> Result<bool, EntDbError> {
        if fin <= debut {
            return Err(invalide());
        }
        let occ = self.edt_occupation_lieu(a, lieu_id, jour, jour).await?;
        if occ
            .iter()
            .any(|o| o.heure_debut < fin && debut < o.heure_fin)
        {
            return Ok(true);
        }
        let indispo = self
            .indisponibilites_lieu_locales(a.etablissement_id, lieu_id, jour, jour)
            .await?;
        Ok(indispo.iter().any(|(d, f)| {
            let jd = NaiveDateTime::new(jour, debut);
            let jf = NaiveDateTime::new(jour, fin);
            *d < jf && jd < *f
        }))
    }

    /// Indisponibilités du lieu (travaux, examen…) en heures murales de l'établissement.
    async fn indisponibilites_lieu_locales(
        &self,
        etab: Uuid,
        lieu_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<(NaiveDateTime, NaiveDateTime)>, EntDbError> {
        sqlx::query_as(
            "WITH fz AS (SELECT coalesce((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1),
                                         'Europe/Paris') AS z)
             SELECT (i.du AT TIME ZONE fz.z), (i.au AT TIME ZONE fz.z)
               FROM ent_salle_indisponibilite i, fz
              WHERE i.etablissement_id = $1 AND i.salle_id = $2 AND i.annulee_le IS NULL
                AND (i.au AT TIME ZONE fz.z) > $3::date AND (i.du AT TIME ZONE fz.z) < ($4::date + 1)",
        )
        .bind(etab)
        .bind(lieu_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }

    /// `edt::plages_libres(ressource, plage, durée)` : plages d'au moins `duree_min` minutes où la
    /// ressource est libre, dans la journée de la grille, les jours ouvrés non neutralisés. Une classe
    /// est libre quand aucune de ses séances occupantes n'y est ; un intervenant, idem (titulaire ou
    /// remplaçant) ; un lieu tient aussi compte de ses indisponibilités.
    pub async fn edt_plages_libres(
        &self,
        a: &Acteur<'_>,
        ressource: RessourcePlan,
        du: NaiveDate,
        au: NaiveDate,
        duree_min: i64,
    ) -> Result<Vec<PlageLibre>, EntDbError> {
        if !est_personnel(a.role)
            || (matches!(ressource, RessourcePlan::Intervenant(u) if u != a.user_id)
                && !Self::role_can_manage_edt(a.role))
        {
            return Err(EntDbError::Forbidden);
        }
        verifier_plage(du, au)?;
        if !(5..=600).contains(&duree_min) {
            return Err(invalide());
        }
        let e = a.etablissement_id;
        // Fenêtre de la journée : étendue de la grille (repli 8 h – 18 h).
        let (gd, gf): (Option<NaiveTime>, Option<NaiveTime>) = sqlx::query_as(
            "SELECT min(heure_debut), max(heure_fin) FROM ent_grille_horaire WHERE etablissement_id = $1",
        )
        .bind(e)
        .fetch_one(&mut *TenantTx::begin(self.pool(), e).await?)
        .await
        .map_err(db)?;
        let fenetre = (
            gd.unwrap_or(NaiveTime::from_hms_opt(8, 0, 0).expect("heure")),
            gf.unwrap_or(NaiveTime::from_hms_opt(18, 0, 0).expect("heure")),
        );
        // Jours ouvrés de l'établissement (repli lundi–vendredi) et jours neutralisés du calendrier.
        let jours_txt: Vec<String> = sqlx::query_scalar(
            "SELECT jour FROM ent_etablissement_jour_ouvre WHERE etablissement_id = $1",
        )
        .bind(e)
        .fetch_all(&mut *TenantTx::begin(self.pool(), e).await?)
        .await
        .map_err(db)?;
        let mut ouvres: HashSet<u32> = jours_txt.iter().filter_map(|j| num_jour(j)).collect();
        if ouvres.is_empty() {
            ouvres = (1..=5).collect();
        }
        let neutres = index_neutralises(&self.calendrier_plage(e, du, au).await?);
        let ferme = |j: NaiveDate| {
            neutres
                .get(&j)
                .is_some_and(|sites| sites.iter().any(|s| s.is_none()))
        };

        // Occupation de la ressource sur la plage.
        let seances: Vec<(NaiveDate, NaiveTime, NaiveTime)> = {
            let etats: Vec<&str> = ETATS_OCCUPANTS.to_vec();
            let (filtre, id): (&str, Uuid) = match ressource {
                RessourcePlan::Lieu(id) => (
                    "AND EXISTS (SELECT 1 FROM ent_edt_seance_lieu l WHERE l.seance_id = s.id
                                   AND l.etablissement_id = s.etablissement_id AND l.lieu_id = $5)",
                    id,
                ),
                RessourcePlan::Intervenant(id) => {
                    ("AND (s.prof_user_id = $5 OR s.remplacant_user_id = $5)", id)
                }
                RessourcePlan::Classe(id) => ("AND s.classe_id = $5", id),
            };
            sqlx::query_as(&format!(
                "SELECT s.jour, s.heure_debut, s.heure_fin FROM ent_edt_seance s
                  WHERE s.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3 AND s.etat = ANY($4) {filtre}"
            ))
            .bind(e)
            .bind(du)
            .bind(au)
            .bind(&etats)
            .bind(id)
            .fetch_all(&mut *TenantTx::begin(self.pool(), e).await?)
            .await
            .map_err(db)?
        };
        let mut par_jour: BTreeMap<NaiveDate, Vec<(NaiveTime, NaiveTime)>> = BTreeMap::new();
        for (j, d, f) in seances {
            par_jour.entry(j).or_default().push((d, f));
        }
        if let RessourcePlan::Lieu(lieu) = ressource {
            for (d, f) in self.indisponibilites_lieu_locales(e, lieu, du, au).await? {
                let mut j = d.date();
                while j <= f.date() {
                    let debut = if j == d.date() {
                        d.time()
                    } else {
                        NaiveTime::MIN
                    };
                    let fin = if j == f.date() {
                        f.time()
                    } else {
                        NaiveTime::from_hms_opt(23, 59, 59).expect("heure")
                    };
                    par_jour.entry(j).or_default().push((debut, fin));
                    j += Duration::days(1);
                }
            }
        }

        let mut out = Vec::new();
        let mut j = du;
        while j <= au {
            if ouvres.contains(&j.weekday().number_from_monday()) && !ferme(j) {
                let occ = par_jour.remove(&j).unwrap_or_default();
                for (d, f) in plages_restantes(fenetre, &occ, Duration::minutes(duree_min)) {
                    out.push(PlageLibre {
                        jour: j,
                        debut: d,
                        fin: f,
                    });
                }
            }
            j += Duration::days(1);
        }
        Ok(out)
    }

    /// D-058 : salles (et lieux) libérées par un cours supprimé, déplacé, neutralisé ou non assuré sur la
    /// plage, **sans** les créneaux que remplit un autre cours ou une indisponibilité. Le cours prime : si
    /// un remplaçant est nommé, la séance reste `modifiee` et la salle n'apparaît pas ici.
    pub async fn edt_salles_liberees(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SalleLiberee>, EntDbError> {
        if !est_personnel(a.role) {
            return Err(EntDbError::Forbidden);
        }
        verifier_plage(du, au)?;
        let e = a.etablissement_id;
        let etats: Vec<&str> = ["annulee", "deplacee", "neutralisee", "non_assuree"].to_vec();
        let occupants: Vec<&str> = ETATS_OCCUPANTS.to_vec();
        #[derive(FromRow)]
        struct Ligne {
            lieu_id: Uuid,
            seance_id: Uuid,
            jour: NaiveDate,
            heure_debut: NaiveTime,
            heure_fin: NaiveTime,
            cours_id: Option<Uuid>,
            classe: String,
            etat: String,
            motif_public: Option<String>,
        }
        let lignes: Vec<Ligne> = sqlx::query_as(
            "SELECT l.lieu_id, s.id AS seance_id, s.jour, s.heure_debut, s.heure_fin, s.cours_id, s.classe,
                    s.etat, s.motif_public
               FROM ent_edt_seance s
               JOIN ent_edt_seance_lieu l ON l.seance_id = s.id AND l.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3 AND s.etat = ANY($4)
                AND NOT EXISTS (SELECT 1 FROM ent_edt_seance o
                                  JOIN ent_edt_seance_lieu ol ON ol.seance_id = o.id AND ol.etablissement_id = o.etablissement_id
                                 WHERE o.etablissement_id = s.etablissement_id AND ol.lieu_id = l.lieu_id
                                   AND o.jour = s.jour AND o.etat = ANY($5) AND o.id <> s.id
                                   AND o.heure_debut < s.heure_fin AND s.heure_debut < o.heure_fin)
              ORDER BY s.jour, s.heure_debut, l.lieu_id",
        )
        .bind(e)
        .bind(du)
        .bind(au)
        .bind(&etats)
        .bind(&occupants)
        .fetch_all(&mut *TenantTx::begin(self.pool(), e).await?)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for l in lignes {
            let Some(raison) = RaisonSalleLibre::depuis_etat(&l.etat) else {
                continue;
            };
            // Une indisponibilité (travaux, examen) sur le créneau : la salle n'est pas libre.
            let indispo = self
                .indisponibilites_lieu_locales(e, l.lieu_id, l.jour, l.jour)
                .await?;
            let (jd, jf) = (
                NaiveDateTime::new(l.jour, l.heure_debut),
                NaiveDateTime::new(l.jour, l.heure_fin),
            );
            if indispo.iter().any(|(d, f)| *d < jf && jd < *f) {
                continue;
            }
            out.push(SalleLiberee {
                lieu_id: l.lieu_id,
                seance_id: l.seance_id,
                jour: l.jour,
                heure_debut: l.heure_debut,
                heure_fin: l.heure_fin,
                cours_id: l.cours_id,
                classe: l.classe,
                raison,
                motif_public: l.motif_public,
            });
        }
        Ok(out)
    }

    /// Cours supprimés ou modifiés sur la plage (annulés, remplacés, déplacés, neutralisés, non assurés,
    /// ajoutés), avec le motif PUBLIC seulement : pour l'agenda (D-059), les notifications et le cahier.
    pub async fn edt_cours_supprimes_ou_modifies(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceChangee>, EntDbError> {
        if !est_personnel(a.role) {
            return Err(EntDbError::Forbidden);
        }
        verifier_plage(du, au)?;
        let e = a.etablissement_id;
        sqlx::query_as::<_, SeanceChangee>(
            "SELECT s.id AS seance_id, s.jour, s.heure_debut, s.heure_fin, s.cours_id, s.classe, s.etat,
                    s.motif_public,
                    coalesce((SELECT array_agg(l.lieu_id ORDER BY l.principal DESC, l.lieu_id)
                                FROM ent_edt_seance_lieu l
                               WHERE l.seance_id = s.id AND l.etablissement_id = s.etablissement_id), '{}') AS lieux
               FROM ent_edt_seance s
              WHERE s.etablissement_id = $1 AND s.jour BETWEEN $2 AND $3
                AND s.etat IN ('annulee', 'modifiee', 'deplacee', 'neutralisee', 'non_assuree', 'ajoutee')
              ORDER BY s.jour, s.heure_debut, s.id",
        )
        .bind(e)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *TenantTx::begin(self.pool(), e).await?)
        .await
        .map_err(db)
    }

    /// Recalcul des séances de la fenêtre glissante (à partir de demain jusqu'à `jusqu_a`, ou la fin de
    /// l'année) : idempotent, ne touche que l'avenir (T2, §8.1). À appeler par la tâche de fond et par
    /// les réactions aux événements des socles 5 et 6 (calendrier, fermeture, grille, intervenant).
    pub async fn edt_recalculer_fenetre(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        jusqu_a: Option<NaiveDate>,
    ) -> Result<crate::referentiel_seances::RapportSeances, EntDbError> {
        let demain = self.aujourdhui_etablissement(a.etablissement_id).await? + Duration::days(1);
        self.ref_generer_seances(a, annee_id, Some(demain), jusqu_a)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(a: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(a, m, 0).unwrap()
    }

    #[test]
    fn plages_restantes_decoupe_et_filtre_la_duree() {
        let f = (h(8, 0), h(18, 0));
        // Rien d'occupé : toute la journée.
        assert_eq!(
            plages_restantes(f, &[], Duration::minutes(55)),
            vec![(h(8, 0), h(18, 0))]
        );
        // Deux cours, dont un déborde de la fenêtre et deux se chevauchent.
        let occ = [
            (h(9, 0), h(10, 0)),
            (h(9, 30), h(11, 0)),
            (h(17, 30), h(19, 0)),
            (h(12, 0), h(12, 30)),
        ];
        let l = plages_restantes(f, &occ, Duration::minutes(55));
        assert_eq!(
            l,
            vec![
                (h(8, 0), h(9, 0)),
                (h(11, 0), h(12, 0)),
                (h(12, 30), h(17, 30))
            ]
        );
        // Une durée plus longue écarte les petites plages.
        let l = plages_restantes(f, &occ, Duration::minutes(61));
        assert_eq!(l, vec![(h(12, 30), h(17, 30))]);
        // Journée pleine : aucune plage.
        assert!(plages_restantes(f, &[(h(7, 0), h(19, 0))], Duration::minutes(5)).is_empty());
    }

    #[test]
    fn raison_de_salle_libre_ne_depend_que_de_l_etat() {
        assert_eq!(
            RaisonSalleLibre::depuis_etat("annulee"),
            Some(RaisonSalleLibre::CoursAnnule)
        );
        assert_eq!(
            RaisonSalleLibre::depuis_etat("non_assuree"),
            Some(RaisonSalleLibre::CoursNonAssure)
        );
        // Le cours prime : remplacé (`modifiee`) ou prévu, la salle n'est pas libérée.
        assert_eq!(RaisonSalleLibre::depuis_etat("modifiee"), None);
        assert_eq!(RaisonSalleLibre::depuis_etat("prevue"), None);
    }

    #[test]
    fn plage_maximale_et_jours() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        assert!(verifier_plage(d, d + Duration::days(62)).is_ok());
        assert!(verifier_plage(d, d + Duration::days(63)).is_err());
        assert!(verifier_plage(d, d - Duration::days(1)).is_err());
        assert_eq!(num_jour("lun"), Some(1));
        assert_eq!(num_jour("xxx"), None);
    }
}
