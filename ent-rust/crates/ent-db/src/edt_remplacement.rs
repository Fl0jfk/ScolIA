//! S2-5 — absences de personnel, remplacements, annulations, déplacements (EDT-04, EDT-05,
//! EDT-08, RH-04) et vue des changements pour les élèves et les familles.
//!
//! Chaîne : absence validée (RH-03) → séances touchées listées automatiquement → remplaçants
//! proposés (disponibles, compétents, sans conflit) → remplacement validé par un humain
//! (`edt.gerer`) → délégation datée du remplaçant (ADM-04) + événement `edt.remplacement_organise`
//! → consommateur `d4.notifications_edt` : élèves et familles prévenus, **sans motif** (RH-05).
//!
//! Tout fait (exception, séance, délégation, événement) s'écrit dans une seule transaction
//! (outbox S2-0). Chaque opération est idempotente : la répéter à l'identique ne produit ni
//! second événement ni seconde notification. Le motif libre des exceptions n'est jamais copié
//! dans un événement ni dans une notification ; seul le motif **public** d'une annulation en
//! masse (saisi comme tel par la direction) est diffusé.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use ent_auth::{Action, Portee, Role};
use ent_events::AggregateKind;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::edt_admin::{CreneauConflit, PlanEdt, conflits_entre};
use crate::ref_commun::{Acteur, conflit, db, introuvable, invalide, publier_agg};
use crate::service::{EntDbError, PgEnt};

/// Droits donnés au remplaçant pour le jour du remplacement, sur la classe (ou le groupe) :
/// ce qu'il lui faut pour connaître les absents du jour [H : paramétrable plus tard].
pub const DROITS_REMPLACEMENT: [Action; 1] = [Action::AbsencesLire];

// ───────────────────────────── Types ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceTouchee {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub classe: String,
    pub classe_id: Uuid,
    pub groupe_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub salle: Option<String>,
    /// `prevue` (à traiter), `annulee`, `remplacee`, `deplacee`.
    pub statut: String,
    /// `titulaire` ou `remplacant` (l'absent assurait ce cours comme remplaçant).
    pub role_absent: String,
    pub remplacant_user_id: Option<Uuid>,
    pub remplacant_nom: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceVue {
    pub id: Uuid,
    pub user_id: Uuid,
    pub nom: Option<String>,
    pub debut: NaiveDate,
    pub fin: NaiveDate,
    /// `signalee` ou `validee` ; le motif n'est jamais renvoyé ici.
    pub statut: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbsenceSeances {
    pub absence: AbsenceVue,
    pub seances: Vec<SeanceTouchee>,
    pub a_traiter: usize,
    /// Plage de séances calculée (une absence très longue est tronquée à 62 jours).
    pub du: NaiveDate,
    pub au: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidat {
    pub user_id: Uuid,
    pub nom: Option<String>,
    /// Enseigne déjà cette matière dans l'établissement.
    pub competent: bool,
    /// Séances déjà assurées ce jour-là (charge).
    pub seances_ce_jour: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CmdRemplacement {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub remplacant_user_id: Uuid,
    #[serde(default)]
    pub absence_rh_id: Option<Uuid>,
    #[serde(default)]
    pub nouvelle_salle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultatException {
    pub exception_id: Uuid,
    /// `true` si l'état demandé existait déjà : rien n'a été écrit ni publié.
    pub deja_fait: bool,
    pub delegation_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CmdDeplacement {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub nouveau_jour: NaiveDate,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default)]
    pub salle: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CmdAnnulationMasse {
    pub du: NaiveDate,
    pub au: NaiveDate,
    /// `etablissement`, `niveau` (cibles = [niveau_id]) ou `classes` (cibles = classes).
    pub portee: String,
    #[serde(default)]
    pub cibles: Vec<Uuid>,
    #[serde(default)]
    pub heure_debut: Option<NaiveTime>,
    #[serde(default)]
    pub heure_fin: Option<NaiveTime>,
    pub motif_public: String,
    /// Clé fournie par le client : la répéter ne crée pas un second lot.
    #[serde(default)]
    pub cle: Option<String>,
    #[serde(default = "vrai")]
    pub apercu: bool,
}

fn vrai() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClasseTouchee {
    pub classe_id: Uuid,
    pub classe: String,
    pub nb_seances: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportMasse {
    pub apercu: bool,
    /// `true` : le lot existait déjà (même clé) ; rien de nouveau n'a été écrit.
    pub deja_applique: bool,
    pub lot_id: Option<Uuid>,
    pub nb_seances: usize,
    pub nb_classes: usize,
    pub classes: Vec<ClasseTouchee>,
    /// Séances de la plage déjà annulées ou déplacées (ignorées).
    pub deja_annulees: usize,
}

/// Changement d'emploi du temps visible d'un élève ou de sa famille. Aucun motif libre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangementEdt {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    /// `annulation`, `remplacement`, `changement_salle`, `deplacement`.
    pub nature: String,
    pub matiere_libelle: String,
    pub classe: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub nouvelle_salle: Option<String>,
    pub remplacant_nom: Option<String>,
    pub nouveau_jour: Option<NaiveDate>,
    pub nouvelle_heure_debut: Option<NaiveTime>,
    pub nouvelle_heure_fin: Option<NaiveTime>,
    /// Motif **public** d'une annulation en masse (grève, fermeture…), s'il y en a un.
    pub motif_public: Option<String>,
    pub enregistre_le: DateTime<Utc>,
}

// ───────────────────────────── Aides ─────────────────────────────

fn peut_se_chevaucher(a: (NaiveTime, NaiveTime), b: (NaiveTime, NaiveTime)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

impl PgEnt {
    /// Plan (séance attendue) d'un créneau à un jour donné.
    async fn edt_plan_du_jour(
        &self,
        etab: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<PlanEdt>, EntDbError> {
        Ok(self
            .edt_plans(etab, jour, jour)
            .await?
            .into_iter()
            .find(|p| p.plan.creneau_id == creneau_id))
    }

    async fn deplacement_de(
        &self,
        etab: Uuid,
        creneau_id: Uuid,
        jour_origine: NaiveDate,
    ) -> Result<Option<Uuid>, EntDbError> {
        sqlx::query_scalar(
            "SELECT id FROM ent_edt_deplacement
              WHERE etablissement_id = $1 AND creneau_id = $2 AND jour_origine = $3",
        )
        .bind(etab)
        .bind(creneau_id)
        .bind(jour_origine)
        .fetch_optional(self.pool())
        .await
        .map_err(db)
    }

    /// Occupations d'un personnel un jour donné (titulaire non remplacé, remplaçant, cours
    /// déplacés arrivant ce jour), hors cours annulés ou déplacés ailleurs.
    async fn occupations_jour(
        &self,
        etab: Uuid,
        user_id: Uuid,
        jour: NaiveDate,
        plans: &[PlanEdt],
    ) -> Result<Vec<(NaiveTime, NaiveTime)>, EntDbError> {
        let deplaces_origine: HashSet<Uuid> = sqlx::query_scalar(
            "SELECT creneau_id FROM ent_edt_deplacement
              WHERE etablissement_id = $1 AND jour_origine = $2",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let mut out = Vec::new();
        for p in plans {
            let pl = &p.plan;
            if pl.statut == "annulee" || deplaces_origine.contains(&pl.creneau_id) {
                continue;
            }
            let assure = match pl.statut.as_str() {
                "remplacee" => pl.remplacant_user_id == Some(user_id),
                _ => pl.prof_user_id == user_id,
            };
            if assure {
                out.push((pl.heure_debut, pl.heure_fin));
            }
        }
        // Cours déplacés vers ce jour, assurés par ce personnel.
        let arrivants: Vec<(NaiveTime, NaiveTime)> = sqlx::query_as(
            "SELECT d.nouvelle_heure_debut, d.nouvelle_heure_fin
               FROM ent_edt_deplacement d
               JOIN ent_edt_creneau c ON c.id = d.creneau_id AND c.etablissement_id = d.etablissement_id
              WHERE d.etablissement_id = $1 AND d.nouveau_jour = $2 AND c.prof_user_id = $3",
        )
        .bind(etab)
        .bind(jour)
        .bind(user_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        out.extend(arrivants);
        Ok(out)
    }

    /// Indisponibilité déclarée (`prof_indispo`) couvrant le créneau horaire ce jour-là ?
    async fn prof_indisponible(
        &self,
        etab: Uuid,
        user_id: Uuid,
        jour: NaiveDate,
        debut: NaiveTime,
        fin: NaiveTime,
    ) -> Result<bool, EntDbError> {
        let iso = crate::edt_repo::jour_semaine_iso(jour);
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_edt_contrainte
              WHERE etablissement_id = $1 AND kind = 'prof_indispo' AND prof_user_id = $2
                AND (jour_semaine IS NULL OR jour_semaine = $3)
                AND coalesce(heure_debut, time '00:00') < $5
                AND coalesce(heure_fin, time '23:59:59') > $4",
        )
        .bind(etab)
        .bind(user_id)
        .bind(iso)
        .bind(debut)
        .bind(fin)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(n > 0)
    }

    /// Le personnel est-il absent (déclarée ou validée) ce jour-là ?
    async fn absent_le(
        &self,
        etab: Uuid,
        user_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_absence_rh
              WHERE etablissement_id = $1 AND user_id = $2 AND statut IN ('signalee', 'validee')
                AND debut <= $3 AND fin >= $3",
        )
        .bind(etab)
        .bind(user_id)
        .bind(jour)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(n > 0)
    }

    /// Un personnel peut-il assurer ce cours ? (membre prof, présent, libre, sans indisponibilité).
    async fn peut_remplacer(
        &self,
        etab: Uuid,
        user_id: Uuid,
        plan: &PlanEdt,
        plans_jour: &[PlanEdt],
    ) -> Result<bool, EntDbError> {
        let pl = &plan.plan;
        if user_id == pl.prof_user_id {
            return Ok(false);
        }
        let role = self.auth_repo().role_in(etab, user_id).await?;
        if role.as_deref().and_then(Role::parse) != Some(Role::Prof) {
            return Ok(false);
        }
        if self.absent_le(etab, user_id, pl.jour).await? {
            return Ok(false);
        }
        let occ = self
            .occupations_jour(etab, user_id, pl.jour, plans_jour)
            .await?;
        if occ
            .iter()
            .any(|o| peut_se_chevaucher(*o, (pl.heure_debut, pl.heure_fin)))
        {
            return Ok(false);
        }
        Ok(!self
            .prof_indisponible(etab, user_id, pl.jour, pl.heure_debut, pl.heure_fin)
            .await?)
    }

    /// Reflète l'état d'une exception sur la séance datée, si elle existe (séance générée).
    async fn sync_seance(
        tx: &mut PgConnection,
        etab: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
        statut: &str,
        remplacant: Option<Uuid>,
        salle: Option<&str>,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "UPDATE ent_edt_seance SET statut = $4, remplacant_user_id = $5,
                    salle = coalesce($6, salle), updated_at = now()
              WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3 AND source = 'genere'",
        )
        .bind(etab)
        .bind(creneau_id)
        .bind(jour)
        .bind(statut)
        .bind(remplacant)
        .bind(salle)
        .execute(tx)
        .await
        .map_err(db)?;
        Ok(())
    }

    /// Révoque les délégations de remplacement liées à une exception (annulation, levée,
    /// changement de remplaçant). Renvoie le nombre de délégations touchées.
    async fn revoquer_delegations_exception(
        tx: &mut PgConnection,
        etab: Uuid,
        exception_id: Uuid,
        sauf_delegue: Option<Uuid>,
        par: Uuid,
    ) -> Result<u64, EntDbError> {
        Ok(sqlx::query(
            "UPDATE ent_delegation SET statut = 'revoquee', revoquee_le = now(), revoquee_par_user_id = $4
              WHERE etablissement_id = $1 AND reference_id = $2 AND source = 'remplacement'
                AND statut = 'accordee' AND ($3::uuid IS NULL OR delegue_user_id <> $3)",
        )
        .bind(etab)
        .bind(exception_id)
        .bind(sauf_delegue)
        .bind(par)
        .execute(tx)
        .await
        .map_err(db)?
        .rows_affected())
    }

    async fn exception_existante(
        tx: &mut PgConnection,
        etab: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<(Uuid, String, Option<Uuid>, Option<String>)>, EntDbError> {
        sqlx::query_as(
            "SELECT id, kind, remplacant_user_id, nouvelle_salle FROM ent_edt_exception
              WHERE etablissement_id = $1 AND creneau_id = $2 AND jour = $3 FOR UPDATE",
        )
        .bind(etab)
        .bind(creneau_id)
        .bind(jour)
        .fetch_optional(tx)
        .await
        .map_err(db)
    }

    #[allow(clippy::too_many_arguments)]
    async fn ecrire_exception(
        tx: &mut PgConnection,
        etab: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
        kind: &str,
        nature: &str,
        remplacant: Option<Uuid>,
        salle: Option<&str>,
        par: Uuid,
        lot_id: Option<Uuid>,
        absence_rh_id: Option<Uuid>,
    ) -> Result<Uuid, EntDbError> {
        let (id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO ent_edt_exception
               (id, etablissement_id, creneau_id, jour, kind, remplacant_user_id, nouvelle_salle,
                motif, created_by_user_id, created_at, lot_id, absence_rh_id, nature)
             VALUES ($1,$2,$3,$4,$5,$6,$7,'',$8,now(),$9,$10,$11)
             ON CONFLICT (etablissement_id, creneau_id, jour) DO UPDATE SET
                kind = EXCLUDED.kind, remplacant_user_id = EXCLUDED.remplacant_user_id,
                nouvelle_salle = EXCLUDED.nouvelle_salle, created_by_user_id = EXCLUDED.created_by_user_id,
                created_at = now(), lot_id = EXCLUDED.lot_id, absence_rh_id = EXCLUDED.absence_rh_id,
                nature = EXCLUDED.nature
             RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(creneau_id)
        .bind(jour)
        .bind(kind)
        .bind(remplacant)
        .bind(salle)
        .bind(par)
        .bind(lot_id)
        .bind(absence_rh_id)
        .bind(nature)
        .fetch_one(tx)
        .await
        .map_err(db)?;
        Ok(id)
    }
}

// ───────────────────────────── Absences → séances touchées (RH-04) ─────────────────────────────

fn seance_touchee(
    p: &PlanEdt,
    role: &str,
    deplace: bool,
    noms: &HashMap<Uuid, String>,
) -> SeanceTouchee {
    let pl = &p.plan;
    SeanceTouchee {
        creneau_id: pl.creneau_id,
        jour: pl.jour,
        classe: pl.classe.clone(),
        classe_id: pl.classe_id,
        groupe_id: p.groupe_id,
        matiere_libelle: pl.matiere_libelle.clone(),
        heure_debut: pl.heure_debut,
        heure_fin: pl.heure_fin,
        salle: pl.salle.clone(),
        statut: if deplace {
            "deplacee".to_string()
        } else {
            pl.statut.clone()
        },
        role_absent: role.to_string(),
        remplacant_user_id: pl.remplacant_user_id,
        remplacant_nom: pl.remplacant_user_id.and_then(|r| noms.get(&r).cloned()),
    }
}

#[derive(Debug, FromRow)]
struct LigneAbsence {
    id: Uuid,
    user_id: Uuid,
    nom: Option<String>,
    debut: NaiveDate,
    fin: NaiveDate,
    statut: String,
}

impl LigneAbsence {
    fn vue(self) -> AbsenceVue {
        AbsenceVue {
            id: self.id,
            user_id: self.user_id,
            nom: self.nom,
            debut: self.debut,
            fin: self.fin,
            statut: self.statut,
        }
    }
}

const SELECT_ABSENCE: &str = "SELECT a.id, a.user_id,
        coalesce(nullif(btrim(u.display_name), ''), u.email) AS nom, a.debut, a.fin, a.statut
   FROM ent_absence_rh a JOIN ent_user u ON u.id = a.user_id
  WHERE a.etablissement_id = $1";

impl PgEnt {
    /// Séances d'un personnel entre deux dates (titulaire ou remplaçant), états compris.
    async fn seances_du_personnel(
        &self,
        etab: Uuid,
        user_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceTouchee>, EntDbError> {
        let plans = self.edt_plans(etab, du, au).await?;
        let deplaces: HashSet<(Uuid, NaiveDate)> = sqlx::query_as::<_, (Uuid, NaiveDate)>(
            "SELECT creneau_id, jour_origine FROM ent_edt_deplacement
              WHERE etablissement_id = $1 AND jour_origine BETWEEN $2 AND $3",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .fetch_all(self.pool())
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let mut ids: Vec<Uuid> = plans
            .iter()
            .filter_map(|p| p.plan.remplacant_user_id)
            .collect();
        ids.sort();
        ids.dedup();
        let noms = self.noms(etab, &ids).await?;
        let mut out: Vec<SeanceTouchee> = plans
            .iter()
            .filter_map(|p| {
                let pl = &p.plan;
                let role = if pl.prof_user_id == user_id {
                    "titulaire"
                } else if pl.remplacant_user_id == Some(user_id) {
                    "remplacant"
                } else {
                    return None;
                };
                Some(seance_touchee(
                    p,
                    role,
                    deplaces.contains(&(pl.creneau_id, pl.jour)),
                    &noms,
                ))
            })
            .collect();
        out.sort_by(|a, b| {
            (a.jour, a.heure_debut, &a.classe).cmp(&(b.jour, b.heure_debut, &b.classe))
        });
        Ok(out)
    }

    /// RH-04 : les séances touchées par une absence, listées automatiquement.
    pub async fn edt_absence_seances(
        &self,
        a: &Acteur<'_>,
        absence_id: Uuid,
    ) -> Result<AbsenceSeances, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let sql = format!("{SELECT_ABSENCE} AND a.id = $2");
        let ligne = sqlx::query_as::<_, LigneAbsence>(&sql)
            .bind(a.etablissement_id)
            .bind(absence_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        let absence = ligne.vue();
        let au = absence.fin.min(absence.debut + Duration::days(62));
        let seances = self
            .seances_du_personnel(a.etablissement_id, absence.user_id, absence.debut, au)
            .await?;
        let a_traiter = seances.iter().filter(|s| s.statut == "prevue").count();
        Ok(AbsenceSeances {
            du: absence.debut,
            au,
            absence,
            seances,
            a_traiter,
        })
    }

    /// Absences de personnel de la période avec le nombre de séances restant à traiter.
    pub async fn edt_absences(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<AbsenceSeances>, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        if au < du || (au - du).num_days() > 62 {
            return Err(invalide());
        }
        let sql = format!(
            "{SELECT_ABSENCE} AND a.statut IN ('signalee', 'validee')
                AND a.debut <= $3 AND a.fin >= $2 ORDER BY a.debut, a.id"
        );
        let lignes = sqlx::query_as::<_, LigneAbsence>(&sql)
            .bind(a.etablissement_id)
            .bind(du)
            .bind(au)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        let mut out = Vec::new();
        for l in lignes {
            let absence = l.vue();
            let (d, f) = (absence.debut.max(du), absence.fin.min(au));
            let seances = if absence.statut == "validee" {
                self.seances_du_personnel(a.etablissement_id, absence.user_id, d, f)
                    .await?
            } else {
                Vec::new() // pas de traitement avant validation
            };
            let a_traiter = seances.iter().filter(|s| s.statut == "prevue").count();
            out.push(AbsenceSeances {
                absence,
                seances,
                a_traiter,
                du: d,
                au: f,
            });
        }
        Ok(out)
    }

    /// Remplaçants proposés pour une séance : disponibles, sans conflit ; compétents d'abord
    /// (enseignent déjà la matière) puis les moins chargés. `tous` ajoute les non compétents.
    pub async fn edt_candidats_remplacants(
        &self,
        a: &Acteur<'_>,
        creneau_id: Uuid,
        jour: NaiveDate,
        tous: bool,
    ) -> Result<Vec<Candidat>, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let etab = a.etablissement_id;
        let plans = self.edt_plans(etab, jour, jour).await?;
        let plan = plans
            .iter()
            .find(|p| p.plan.creneau_id == creneau_id)
            .cloned()
            .ok_or_else(introuvable)?;
        let competents: HashSet<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT prof_user_id FROM ent_edt_creneau
              WHERE etablissement_id = $1 AND lower(btrim(matiere_libelle)) = lower(btrim($2))",
        )
        .bind(etab)
        .bind(&plan.plan.matiere_libelle)
        .fetch_all(self.pool())
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let profs: Vec<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM ent_membership WHERE etablissement_id = $1 AND role = 'prof'",
        )
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let noms = self.noms(etab, &profs).await?;
        let mut out = Vec::new();
        for uid in profs {
            if !self.peut_remplacer(etab, uid, &plan, &plans).await? {
                continue;
            }
            let competent = competents.contains(&uid);
            if !competent && !tous {
                continue;
            }
            let charge = self.occupations_jour(etab, uid, jour, &plans).await?.len();
            out.push(Candidat {
                user_id: uid,
                nom: noms.get(&uid).cloned(),
                competent,
                seances_ce_jour: charge,
            });
        }
        out.sort_by(|x, y| {
            (!x.competent, x.seances_ce_jour, &x.nom).cmp(&(
                !y.competent,
                y.seances_ce_jour,
                &y.nom,
            ))
        });
        Ok(out)
    }
}

// ───────────────────────────── Remplacement (EDT-05) ─────────────────────────────

impl PgEnt {
    /// EDT-05 / RH-04 : valide le remplacement d'une séance. Exception + délégation datée du
    /// remplaçant + événement dans une seule transaction ; idempotent.
    pub async fn edt_organiser_remplacement(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdRemplacement,
    ) -> Result<ResultatException, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let etab = a.etablissement_id;
        let plans = self.edt_plans(etab, cmd.jour, cmd.jour).await?;
        let plan = plans
            .iter()
            .find(|p| p.plan.creneau_id == cmd.creneau_id)
            .cloned()
            .ok_or_else(introuvable)?;
        if plan.plan.statut == "annulee" {
            return Err(conflit());
        }
        if self
            .deplacement_de(etab, cmd.creneau_id, cmd.jour)
            .await?
            .is_some()
        {
            return Err(conflit()); // le cours a été déplacé : on ne le remplace pas ce jour-là
        }
        if let Some(abs_id) = cmd.absence_rh_id {
            let ligne: Option<(Uuid, NaiveDate, NaiveDate, String)> = sqlx::query_as(
                "SELECT user_id, debut, fin, statut FROM ent_absence_rh
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(abs_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
            let (absent, debut, fin, statut) = ligne.ok_or_else(introuvable)?;
            let concerne =
                plan.plan.prof_user_id == absent || plan.plan.remplacant_user_id == Some(absent);
            if statut != "validee" || !concerne || cmd.jour < debut || cmd.jour > fin {
                return Err(conflit());
            }
        }
        let salle = cmd
            .nouvelle_salle
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(code) = salle {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_salle WHERE etablissement_id = $1
                                  AND lower(code) = lower($2) AND actif)",
            )
            .bind(etab)
            .bind(code)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        // Le remplaçant : personnel prof de l'établissement, disponible, sans conflit.
        if self
            .auth_repo()
            .role_in(etab, cmd.remplacant_user_id)
            .await?
            .is_none()
        {
            return Err(introuvable());
        }
        // Le remplaçant déjà en place sur ce cours reste valide (idempotence) ; sinon contrôle.
        let deja_ce_remplacant = plan.plan.statut == "remplacee"
            && plan.plan.remplacant_user_id == Some(cmd.remplacant_user_id);
        if !deja_ce_remplacant
            && !self
                .peut_remplacer(etab, cmd.remplacant_user_id, &plan, &plans)
                .await?
        {
            return Err(conflit());
        }
        let cible = plan.groupe_id.unwrap_or(plan.plan.classe_id);
        let droits: Vec<crate::delegation::DroitDelegue> = DROITS_REMPLACEMENT
            .iter()
            .map(|d| crate::delegation::DroitDelegue {
                action: d.code().to_string(),
                portee: Portee::Designes.code().to_string(),
                cibles: vec![cible],
            })
            .collect();

        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 6))")
            .bind(format!("{etab}:{}", cmd.creneau_id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let existante = Self::exception_existante(&mut tx, etab, cmd.creneau_id, cmd.jour).await?;
        if let Some((id, kind, rempl, sal)) = &existante {
            if kind == "remplacement"
                && *rempl == Some(cmd.remplacant_user_id)
                && (salle.is_none() || sal.as_deref() == salle)
            {
                let delegation: Option<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM ent_delegation WHERE etablissement_id = $1 AND reference_id = $2
                      AND delegue_user_id = $3 AND source = 'remplacement' AND statut = 'accordee'",
                )
                .bind(etab)
                .bind(id)
                .bind(cmd.remplacant_user_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                return Ok(ResultatException {
                    exception_id: *id,
                    deja_fait: true,
                    delegation_id: delegation,
                });
            }
        }
        let exc_id = Self::ecrire_exception(
            &mut tx,
            etab,
            cmd.creneau_id,
            cmd.jour,
            "remplacement",
            "remplacement",
            Some(cmd.remplacant_user_id),
            salle,
            a.user_id,
            None,
            cmd.absence_rh_id,
        )
        .await?;
        Self::sync_seance(
            &mut tx,
            etab,
            cmd.creneau_id,
            cmd.jour,
            "remplacee",
            Some(cmd.remplacant_user_id),
            salle,
        )
        .await?;
        // Remplaçant précédent : sa délégation est révoquée ; le nouveau reçoit la sienne.
        Self::revoquer_delegations_exception(
            &mut tx,
            etab,
            exc_id,
            Some(cmd.remplacant_user_id),
            a.user_id,
        )
        .await?;
        let (delegation_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO ent_delegation
               (id, etablissement_id, delegue_user_id, delegant_user_id, droits, debut, fin, motif,
                source, reference_id)
             VALUES ($1,$2,$3,$4,$5,$6,$6,'Remplacement','remplacement',$7)
             ON CONFLICT (etablissement_id, reference_id, delegue_user_id)
               WHERE source = 'remplacement' AND reference_id IS NOT NULL
             DO UPDATE SET statut = 'accordee', revoquee_le = NULL, revoquee_par_user_id = NULL,
                           expiree_le = NULL, droits = EXCLUDED.droits, delegant_user_id = EXCLUDED.delegant_user_id
             RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(cmd.remplacant_user_id)
        .bind(a.user_id)
        .bind(json!(droits))
        .bind(cmd.jour)
        .bind(exc_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            "edt.remplacement_organise",
            cmd.creneau_id,
            json!({ "exception_id": exc_id, "creneau_id": cmd.creneau_id, "jour": cmd.jour,
                    "remplacant_user_id": cmd.remplacant_user_id, "absence_rh_id": cmd.absence_rh_id }),
        )
        .await?;
        let ev2 = publier_agg(
            &mut tx,
            a,
            AggregateKind::Personne,
            "delegation.accordee",
            delegation_id,
            json!({ "delegation_id": delegation_id, "delegue_user_id": cmd.remplacant_user_id,
                    "actions": DROITS_REMPLACEMENT.iter().map(|d| d.code()).collect::<Vec<_>>(),
                    "debut": cmd.jour, "fin": cmd.jour, "source": "remplacement" }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev, ev2]);
        Ok(ResultatException {
            exception_id: exc_id,
            deja_fait: false,
            delegation_id: Some(delegation_id),
        })
    }

    /// EDT-04 : annule une séance (idempotent). Une délégation de remplacement liée est révoquée.
    pub async fn edt_annuler_seance(
        &self,
        a: &Acteur<'_>,
        creneau_id: Uuid,
        jour: NaiveDate,
        absence_rh_id: Option<Uuid>,
    ) -> Result<ResultatException, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let etab = a.etablissement_id;
        let plan = self
            .edt_plan_du_jour(etab, creneau_id, jour)
            .await?
            .ok_or_else(introuvable)?;
        if self.deplacement_de(etab, creneau_id, jour).await?.is_some() {
            return Err(conflit());
        }
        // RS3-04 : l'absence RH référencée doit appartenir à l'établissement de
        // l'acteur (refus identique à « introuvable » : pas d'oracle d'UUID).
        if let Some(abs_id) = absence_rh_id {
            let existe: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_absence_rh WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(abs_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
            if existe.is_none() {
                return Err(introuvable());
            }
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 6))")
            .bind(format!("{etab}:{creneau_id}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if let Some((id, kind, _, _)) =
            Self::exception_existante(&mut tx, etab, creneau_id, jour).await?
        {
            if kind == "annulation" {
                return Ok(ResultatException {
                    exception_id: id,
                    deja_fait: true,
                    delegation_id: None,
                });
            }
        }
        let exc_id = Self::ecrire_exception(
            &mut tx,
            etab,
            creneau_id,
            jour,
            "annulation",
            "annulation",
            None,
            None,
            a.user_id,
            None,
            absence_rh_id,
        )
        .await?;
        Self::sync_seance(&mut tx, etab, creneau_id, jour, "annulee", None, None).await?;
        Self::revoquer_delegations_exception(&mut tx, etab, exc_id, None, a.user_id).await?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            ent_events::types::EDT_EXCEPTION,
            creneau_id,
            json!({ "exception_id": exc_id, "creneau_id": creneau_id, "jour": jour, "nature": "annulation" }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        let _ = plan;
        Ok(ResultatException {
            exception_id: exc_id,
            deja_fait: false,
            delegation_id: None,
        })
    }

    /// EDT-04 : change la salle d'une séance (conflit de salle refusé).
    pub async fn edt_changer_salle(
        &self,
        a: &Acteur<'_>,
        creneau_id: Uuid,
        jour: NaiveDate,
        salle: &str,
    ) -> Result<ResultatException, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let etab = a.etablissement_id;
        let salle = salle.trim();
        if salle.is_empty() {
            return Err(invalide());
        }
        let plans = self.edt_plans(etab, jour, jour).await?;
        let plan = plans
            .iter()
            .find(|p| p.plan.creneau_id == creneau_id)
            .cloned()
            .ok_or_else(introuvable)?;
        if plan.plan.statut == "annulee"
            || self.deplacement_de(etab, creneau_id, jour).await?.is_some()
        {
            return Err(conflit());
        }
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_salle WHERE etablissement_id = $1 AND lower(code) = lower($2) AND actif)",
        )
        .bind(etab)
        .bind(salle)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        if !ok {
            return Err(introuvable());
        }
        // Salle libre à ce créneau horaire ce jour-là ?
        let occupee = plans.iter().any(|p| {
            p.plan.creneau_id != creneau_id
                && p.plan.statut != "annulee"
                && p.plan
                    .salle
                    .as_deref()
                    .is_some_and(|s| s.trim().eq_ignore_ascii_case(salle))
                && peut_se_chevaucher(
                    (p.plan.heure_debut, p.plan.heure_fin),
                    (plan.plan.heure_debut, plan.plan.heure_fin),
                )
        });
        if occupee {
            return Err(conflit());
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 6))")
            .bind(format!("{etab}:{creneau_id}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let existante = Self::exception_existante(&mut tx, etab, creneau_id, jour).await?;
        if let Some((id, kind, _, sal)) = &existante {
            if kind == "changement_salle" && sal.as_deref() == Some(salle) {
                return Ok(ResultatException {
                    exception_id: *id,
                    deja_fait: true,
                    delegation_id: None,
                });
            }
            if kind != "changement_salle" && kind != "remplacement" {
                return Err(conflit());
            }
        }
        // Un remplacement existant garde son remplaçant : seule la salle change.
        let (kind, rempl) = match &existante {
            Some((_, k, r, _)) if k == "remplacement" => ("remplacement", *r),
            _ => ("changement_salle", None),
        };
        let exc_id = Self::ecrire_exception(
            &mut tx,
            etab,
            creneau_id,
            jour,
            kind,
            "changement_salle",
            rempl,
            Some(salle),
            a.user_id,
            None,
            None,
        )
        .await?;
        Self::sync_seance(
            &mut tx,
            etab,
            creneau_id,
            jour,
            if rempl.is_some() {
                "remplacee"
            } else {
                "prevue"
            },
            rempl,
            Some(salle),
        )
        .await?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            ent_events::types::EDT_EXCEPTION,
            creneau_id,
            json!({ "exception_id": exc_id, "creneau_id": creneau_id, "jour": jour, "nature": "changement_salle" }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ResultatException {
            exception_id: exc_id,
            deja_fait: false,
            delegation_id: None,
        })
    }

    /// Lève une exception (annulation, remplacement, changement de salle, déplacement) : la
    /// séance redevient normale ; élèves et familles en sont informés.
    pub async fn edt_lever_exception(
        &self,
        a: &Acteur<'_>,
        creneau_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let etab = a.etablissement_id;
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 6))")
            .bind(format!("{etab}:{creneau_id}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let Some((exc_id, _, _, _)) =
            Self::exception_existante(&mut tx, etab, creneau_id, jour).await?
        else {
            return Ok(false);
        };
        let nature: Option<String> = sqlx::query_scalar(
            "SELECT coalesce(nature, kind) FROM ent_edt_exception WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(exc_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Self::revoquer_delegations_exception(&mut tx, etab, exc_id, None, a.user_id).await?;
        // Déplacement : la séance arrivée à la nouvelle date disparaît avec lui.
        let depl: Option<(Uuid, NaiveDate)> = sqlx::query_as(
            "DELETE FROM ent_edt_deplacement WHERE etablissement_id = $1 AND creneau_id = $2 AND jour_origine = $3
             RETURNING id, nouveau_jour",
        )
        .bind(etab)
        .bind(creneau_id)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((_, nouveau)) = depl {
            sqlx::query(
                "DELETE FROM ent_edt_seance WHERE etablissement_id = $1 AND creneau_id = $2
                    AND jour = $3 AND source = 'deplacement'",
            )
            .bind(etab)
            .bind(creneau_id)
            .bind(nouveau)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        sqlx::query("DELETE FROM ent_edt_exception WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(exc_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "UPDATE ent_edt_seance s SET statut = 'prevue', remplacant_user_id = NULL,
                    salle = c.salle, updated_at = now()
               FROM ent_edt_creneau c
              WHERE s.etablissement_id = $1 AND s.creneau_id = $2 AND s.jour = $3 AND s.source = 'genere'
                AND c.id = s.creneau_id AND c.etablissement_id = s.etablissement_id",
        )
        .bind(etab)
        .bind(creneau_id)
        .bind(jour)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            "edt.exception_levee",
            creneau_id,
            json!({ "exception_id": exc_id, "creneau_id": creneau_id, "jour": jour,
                    "nature": nature.unwrap_or_default() }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(true)
    }

    /// EDT-04 : déplace un cours à une autre date / heure. Le cours d'origine est annulé, un
    /// cours est ajouté à la nouvelle date (séance de source `deplacement`) si le professeur,
    /// la classe et la salle y sont libres et le jour ouvré.
    #[allow(clippy::type_complexity)]
    pub async fn edt_deplacer_seance(
        &self,
        a: &Acteur<'_>,
        cmd: &CmdDeplacement,
    ) -> Result<ResultatException, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let etab = a.etablissement_id;
        if cmd.heure_fin <= cmd.heure_debut
            || (cmd.nouveau_jour == cmd.jour && cmd.heure_debut == NaiveTime::MIN)
            || (cmd.nouveau_jour - cmd.jour).num_days().abs() > 60
        {
            return Err(invalide());
        }
        // Déjà déplacé à l'identique : idempotent (le cours d'origine est alors « annulé »).
        let identique: Option<(Uuid,)> = sqlx::query_as(
            "SELECT x.id FROM ent_edt_deplacement d
               JOIN ent_edt_exception x ON x.etablissement_id = d.etablissement_id
                AND x.creneau_id = d.creneau_id AND x.jour = d.jour_origine
              WHERE d.etablissement_id = $1 AND d.creneau_id = $2 AND d.jour_origine = $3
                AND d.nouveau_jour = $4 AND d.nouvelle_heure_debut = $5 AND d.nouvelle_heure_fin = $6",
        )
        .bind(etab)
        .bind(cmd.creneau_id)
        .bind(cmd.jour)
        .bind(cmd.nouveau_jour)
        .bind(cmd.heure_debut)
        .bind(cmd.heure_fin)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        if let Some((id,)) = identique {
            return Ok(ResultatException {
                exception_id: id,
                deja_fait: true,
                delegation_id: None,
            });
        }
        let plan = self
            .edt_plan_du_jour(etab, cmd.creneau_id, cmd.jour)
            .await?
            .ok_or_else(introuvable)?;
        if plan.plan.statut != "prevue" {
            return Err(conflit());
        }
        let salle = cmd
            .salle
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(code) = salle {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_salle WHERE etablissement_id = $1 AND lower(code) = lower($2) AND actif)",
            )
            .bind(etab)
            .bind(code)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        // Jour d'arrivée ouvré : le calendrier (vacances, fermetures) l'exclut, sinon refus.
        let site = plan.plan_site(self, etab).await?;
        let ferme = self
            .calendrier_plage(etab, cmd.nouveau_jour, cmd.nouveau_jour)
            .await?
            .iter()
            .any(|j| j.site_id.is_none() || (site.is_some() && j.site_id == site));
        if ferme {
            return Err(conflit());
        }
        // Conflits à l'arrivée : professeur, classe (groupe compris), salle.
        let arrivee = self
            .edt_plans(etab, cmd.nouveau_jour, cmd.nouveau_jour)
            .await?;
        let candidat = CreneauConflit {
            id: None,
            classe: plan.plan.classe.clone(),
            groupe_id: plan.groupe_id,
            prof_user_id: plan.plan.prof_user_id,
            salle: salle.map(str::to_string).or(plan.plan.salle.clone()),
            site: String::new(),
            jour_semaine: crate::edt_repo::jour_semaine_iso(cmd.nouveau_jour),
            heure_debut: cmd.heure_debut,
            heure_fin: cmd.heure_fin,
            semaine_ab: domain_eleve::EdtSemaineAb::Toutes,
        };
        let deplaces_origine: HashSet<(Uuid, NaiveDate)> = sqlx::query_as::<_, (Uuid, NaiveDate)>(
            "SELECT creneau_id, jour_origine FROM ent_edt_deplacement WHERE etablissement_id = $1 AND jour_origine = $2",
        )
        .bind(etab)
        .bind(cmd.nouveau_jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        for p in &arrivee {
            let pl = &p.plan;
            if pl.statut == "annulee"
                || (pl.creneau_id == cmd.creneau_id && cmd.nouveau_jour == cmd.jour)
                || deplaces_origine.contains(&(pl.creneau_id, pl.jour))
            {
                continue;
            }
            let autre = CreneauConflit {
                id: Some(pl.creneau_id),
                classe: pl.classe.clone(),
                groupe_id: p.groupe_id,
                prof_user_id: if pl.statut == "remplacee" {
                    pl.remplacant_user_id.unwrap_or(pl.prof_user_id)
                } else {
                    pl.prof_user_id
                },
                salle: pl.salle.clone(),
                site: String::new(),
                jour_semaine: candidat.jour_semaine,
                heure_debut: pl.heure_debut,
                heure_fin: pl.heure_fin,
                semaine_ab: domain_eleve::EdtSemaineAb::Toutes,
            };
            if !conflits_entre(&candidat, &autre).is_empty() {
                return Err(conflit());
            }
        }
        // Cours déjà déplacés vers ce jour : eux aussi occupent professeur, classe et salle.
        let arrivants: Vec<(Uuid, String, Option<Uuid>, Uuid, Option<String>, NaiveTime, NaiveTime)> = sqlx::query_as(
            "SELECT c.id, btrim(c.classe), c.groupe_id, c.prof_user_id,
                    coalesce(d.nouvelle_salle, c.salle), d.nouvelle_heure_debut, d.nouvelle_heure_fin
               FROM ent_edt_deplacement d
               JOIN ent_edt_creneau c ON c.id = d.creneau_id AND c.etablissement_id = d.etablissement_id
              WHERE d.etablissement_id = $1 AND d.nouveau_jour = $2 AND d.creneau_id <> $3",
        )
        .bind(etab)
        .bind(cmd.nouveau_jour)
        .bind(cmd.creneau_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        for (id, classe, groupe_id, prof, salle_a, debut, fin) in arrivants {
            let autre = CreneauConflit {
                id: Some(id),
                classe,
                groupe_id,
                prof_user_id: prof,
                salle: salle_a,
                site: String::new(),
                jour_semaine: candidat.jour_semaine,
                heure_debut: debut,
                heure_fin: fin,
                semaine_ab: domain_eleve::EdtSemaineAb::Toutes,
            };
            if !conflits_entre(&candidat, &autre).is_empty() {
                return Err(conflit());
            }
        }
        if self
            .prof_indisponible(
                etab,
                plan.plan.prof_user_id,
                cmd.nouveau_jour,
                cmd.heure_debut,
                cmd.heure_fin,
            )
            .await?
            || self
                .absent_le(etab, plan.plan.prof_user_id, cmd.nouveau_jour)
                .await?
        {
            return Err(conflit());
        }

        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 6))")
            .bind(format!("{etab}:{}", cmd.creneau_id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let existante = Self::exception_existante(&mut tx, etab, cmd.creneau_id, cmd.jour).await?;
        if let Some((id, _, _, _)) = &existante {
            // Déjà déplacé à l'identique : idempotent ; toute autre exception bloque.
            let meme: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_edt_deplacement
                  WHERE etablissement_id = $1 AND creneau_id = $2 AND jour_origine = $3
                    AND nouveau_jour = $4 AND nouvelle_heure_debut = $5 AND nouvelle_heure_fin = $6",
            )
            .bind(etab)
            .bind(cmd.creneau_id)
            .bind(cmd.jour)
            .bind(cmd.nouveau_jour)
            .bind(cmd.heure_debut)
            .bind(cmd.heure_fin)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if meme.is_some() {
                return Ok(ResultatException {
                    exception_id: *id,
                    deja_fait: true,
                    delegation_id: None,
                });
            }
            return Err(conflit());
        }
        let exc_id = Self::ecrire_exception(
            &mut tx,
            etab,
            cmd.creneau_id,
            cmd.jour,
            "annulation",
            "deplacement",
            None,
            None,
            a.user_id,
            None,
            None,
        )
        .await?;
        sqlx::query(
            "INSERT INTO ent_edt_deplacement
               (id, etablissement_id, creneau_id, jour_origine, nouveau_jour, nouvelle_heure_debut,
                nouvelle_heure_fin, nouvelle_salle, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(cmd.creneau_id)
        .bind(cmd.jour)
        .bind(cmd.nouveau_jour)
        .bind(cmd.heure_debut)
        .bind(cmd.heure_fin)
        .bind(salle)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Self::sync_seance(
            &mut tx,
            etab,
            cmd.creneau_id,
            cmd.jour,
            "annulee",
            None,
            None,
        )
        .await?;
        if cmd.nouveau_jour != cmd.jour {
            let pl = &plan.plan;
            let annee: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2",
            )
            .bind(etab)
            .bind(cmd.nouveau_jour)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some(annee_id) = annee {
                sqlx::query(
                    "INSERT INTO ent_edt_seance
                       (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, service_id,
                        prof_user_id, matiere_id, matiere_libelle, salle, heure_debut, heure_fin,
                        demi_journee, statut, source)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,'prevue','deplacement')
                     ON CONFLICT (etablissement_id, creneau_id, jour) DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(annee_id)
                .bind(pl.creneau_id)
                .bind(cmd.nouveau_jour)
                .bind(pl.classe_id)
                .bind(&pl.classe)
                .bind(pl.service_id)
                .bind(pl.prof_user_id)
                .bind(pl.matiere_id)
                .bind(&pl.matiere_libelle)
                .bind(salle.map(str::to_string).or(pl.salle.clone()))
                .bind(cmd.heure_debut)
                .bind(cmd.heure_fin)
                .bind(if cmd.heure_debut < NaiveTime::from_hms_opt(12, 30, 0).expect("heure") { "matin" } else { "apres_midi" })
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            ent_events::types::EDT_EXCEPTION,
            cmd.creneau_id,
            json!({ "exception_id": exc_id, "creneau_id": cmd.creneau_id, "jour": cmd.jour,
                    "nature": "deplacement", "nouveau_jour": cmd.nouveau_jour,
                    "heure_debut": cmd.heure_debut, "heure_fin": cmd.heure_fin }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ResultatException {
            exception_id: exc_id,
            deja_fait: false,
            delegation_id: None,
        })
    }
}

impl PlanEdt {
    /// Site (`site_id` de la classe) du créneau, pour la fermeture par site.
    async fn plan_site(&self, ent: &PgEnt, etab: Uuid) -> Result<Option<Uuid>, EntDbError> {
        sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT site_id FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(self.plan.classe_id)
        .fetch_optional(ent.pool())
        .await
        .map(|o| o.flatten())
        .map_err(db)
    }
}
