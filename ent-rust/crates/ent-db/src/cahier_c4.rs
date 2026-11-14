//! Socle 20, lot C4 : la **charge de travail** (`20_CAHIER_DE_TEXTES.md` §11) : calcul par élève réel, bandeau au moment de
//! donner un travail, dates alternatives, repères à trois positions, calendrier de classe, alertes ignorées comptées.
//!
//! Principes :
//! * la charge se calcule **par élève** (public du cours **au jour de l'établissement**, échéance effective de chacun, travaux
//!   à public restreint pour leurs seuls élèves) : un groupe qui mélange quatre classes n'a pas « la charge d'une classe » ;
//! * ce qui sort est un **nombre ou une nature** : jamais le nom d'un élève, jamais le titre ni le contenu d'un travail d'un
//!   collègue ; la **matière** n'est montrée que pour une échéance de la classe du cours (l'équipe lit déjà son cahier), jamais
//!   hors classe ni pour un groupe transversal (§11.1) ;
//! * les repères sont ceux de l'établissement, à trois positions, par groupe de niveaux ; l'alerte ne bloque rien ;
//! * la dérogation est comptée : agrégée par classe pour la direction et le PP, nominative pour le seul professeur concerné.
//!
//! Le calcul lui-même est pur (`charge_calcul`) ; ici : lectures groupées (une requête par nature de donnée, jamais une par
//! élève) et droits. Port consommé : `evaluations::calendrier` (lecture minimale du socle 21, `evaluations_calendrier`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use ent_auth::{Action, Role};
use ent_types::cahier_c4::*;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::cahier::Cible;
use crate::cahier_c3::{lundi_de, LecteurC3};
use crate::charge_calcul::{
    calculer, distribution, evaluations_semaine, est_long, mediane, reperes, reperes_de, reperes_depasses_echeance, Echeance, Genre,
    Groupe, JoursOuvres, Position,
};
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

/// Élèves au plus d'un aperçu (garde-fou : un cours ne dépasse pas quelques dizaines d'élèves ; plusieurs cours reliés oui).
const MAX_ELEVES_APERCU: usize = 400;
/// Plus longue période du calendrier de classe et de la charge d'un élève (jours).
const MAX_PERIODE_JOURS: i64 = 100;
/// Jours ouvrés cherchés après l'échéance pour proposer une autre date.
const HORIZON_PROPOSITIONS_JOURS: i64 = 14;

fn invalide(message: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Invalide, message)
}

/// Natures qui sont des évaluations annoncées (le reste est un travail à faire).
fn nature_est_evaluation(code: &str) -> bool {
    matches!(code, "evaluation" | "controle" | "interrogation")
}

#[derive(Debug, FromRow)]
struct EleveRow {
    eleve_id: Uuid,
    classe_id: Option<Uuid>,
    niveau: Option<String>,
}

#[derive(Debug, FromRow)]
struct EcheanceRow {
    eleve_id: Uuid,
    jour: NaiveDate,
    minutes: Option<i16>,
    donne_le: NaiveDate,
    nature: String,
    nature_code: String,
    evaluation_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    matiere: String,
}

/// Élèves lus, avec leur groupe de niveaux, et leurs échéances (travaux et évaluations annoncées), sans doublon (CC-55).
struct Base {
    eleves: Vec<EleveRow>,
    echeances: Vec<Echeance>,
    ouvres: JoursOuvres,
}

/// Position des repères de l'établissement pour l'année courante (défaut : normale).
async fn position_de(tx: &mut PgConnection, etab: Uuid) -> Result<Position, EntDbError> {
    let code: Option<String> = sqlx::query_scalar(
        "SELECT r.repere_charge FROM ent_cdt_reglage r
           JOIN ent_annee_scolaire y ON y.etablissement_id = r.etablissement_id AND y.id = r.annee_id
          WHERE r.etablissement_id = $1 AND y.date_debut <= ent_aujourdhui_etab($1) AND y.date_fin >= ent_aujourdhui_etab($1)
          ORDER BY y.date_debut DESC LIMIT 1",
    )
    .bind(etab)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?;
    Ok(code.as_deref().map(Position::depuis).unwrap_or(Position::Normal))
}

/// Élèves du cours (public à `jour`) avec leur classe d'affectation et le code de son niveau.
async fn eleves_des_cours(
    tx: &mut PgConnection,
    etab: Uuid,
    cours: &[Uuid],
    jour: NaiveDate,
) -> Result<Vec<EleveRow>, EntDbError> {
    sqlx::query_as(
        "SELECT DISTINCT ON (p.eleve_id) p.eleve_id, ca.classe_id, n.code AS niveau
           FROM unnest($2::uuid[]) AS c(cours_id)
          CROSS JOIN LATERAL ent_public_cours_jour($1, $3, c.cours_id, NULL, NULL, false) p
           LEFT JOIN LATERAL (SELECT a.classe_id FROM ent_affectation a
                               WHERE a.etablissement_id = $1 AND a.eleve_id = p.eleve_id AND a.classe_id IS NOT NULL
                                 AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
                               ORDER BY a.date_debut DESC LIMIT 1) ca ON true
           LEFT JOIN ent_classe cl ON cl.etablissement_id = $1 AND cl.id = ca.classe_id
           LEFT JOIN ent_niveau n ON n.etablissement_id = $1 AND n.id = cl.niveau_id
          ORDER BY p.eleve_id",
    )
    .bind(etab)
    .bind(cours)
    .bind(jour)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)
}

/// Élèves d'une classe (affectation en cours) avec le code de son niveau.
async fn eleves_de_classe(tx: &mut PgConnection, etab: Uuid, classe: Uuid, jour: NaiveDate) -> Result<Vec<EleveRow>, EntDbError> {
    sqlx::query_as(
        "SELECT a.eleve_id, a.classe_id, n.code AS niveau
           FROM ent_affectation a
           JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
           JOIN ent_classe cl ON cl.etablissement_id = a.etablissement_id AND cl.id = a.classe_id
           LEFT JOIN ent_niveau n ON n.etablissement_id = cl.etablissement_id AND n.id = cl.niveau_id
          WHERE a.etablissement_id = $1 AND a.classe_id = $2 AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
            AND (e.date_sortie IS NULL OR e.date_sortie > $3)
          ORDER BY a.eleve_id",
    )
    .bind(etab)
    .bind(classe)
    .bind(jour)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)
}

/// Échéances de ces élèves dans [du, au] : travaux publiés et visibles (échéance effective de chacun) et évaluations annoncées.
async fn echeances_des_eleves(
    tx: &mut PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
    du: NaiveDate,
    au: NaiveDate,
    aujourdhui: NaiveDate,
) -> Result<Vec<Echeance>, EntDbError> {
    if eleves.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<EcheanceRow> = sqlx::query_as(
        "SELECT e.eleve_id, ent_aujourdhui_etab($1, eff.le) AS jour, t.duree_estimee_min AS minutes, t.donne_le,
                n.libelle AS nature, n.code AS nature_code, t.evaluation_id, c.classe_id, c.libelle AS matiere
           FROM unnest($2::uuid[]) AS e(eleve_id)
          CROSS JOIN LATERAL ent_public_cours_jour($1, $5, NULL, NULL, e.eleve_id, false) p
           JOIN ent_cdt_travail t ON t.etablissement_id = $1 AND t.cours_id = p.cours_id AND t.etat = 'publie'
            AND t.visible_a IS NOT NULL AND t.visible_a <= now()
           JOIN ent_cdt_nature n ON n.etablissement_id = t.etablissement_id AND n.id = t.nature_id
           JOIN ent_cours c ON c.etablissement_id = t.etablissement_id AND c.id = t.cours_id
          CROSS JOIN LATERAL (SELECT ent_cdt_echeance_effective(t.etablissement_id, t.id, e.eleve_id, t.echeance_le) AS le) eff
          WHERE (t.public = 'cours'
                 OR EXISTS (SELECT 1 FROM ent_cdt_travail_cible k
                             WHERE k.etablissement_id = t.etablissement_id AND k.travail_id = t.id AND k.eleve_id = e.eleve_id))
            AND ent_aujourdhui_etab($1, eff.le) BETWEEN $3 AND $4",
    )
    .bind(etab)
    .bind(eleves)
    .bind(du)
    .bind(au)
    .bind(aujourdhui)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let annoncees: HashSet<(Uuid, Uuid)> = rows.iter().filter_map(|r| r.evaluation_id.map(|e| (r.eleve_id, e))).collect();
    let mut out: Vec<Echeance> = rows
        .into_iter()
        .map(|r| {
            let eval = r.evaluation_id.is_some() || nature_est_evaluation(&r.nature_code);
            Echeance {
                eleve_id: r.eleve_id,
                jour: r.jour,
                genre: if eval { Genre::Evaluation } else { Genre::Travail },
                minutes: if eval { 0 } else { i32::from(r.minutes.unwrap_or(0)) },
                donne_le: r.donne_le,
                nature: r.nature,
                classe_id: r.classe_id,
                matiere: r.matiere,
            }
        })
        .collect();
    // Évaluations annoncées du socle 21 : une seule fois, même si un travail les annonce déjà (CC-55).
    for e in PgEnt::eva_calendrier(&mut *tx, etab, eleves, du, au, aujourdhui).await? {
        if annoncees.contains(&(e.eleve_id, e.evaluation.id)) {
            continue;
        }
        out.push(Echeance {
            eleve_id: e.eleve_id,
            jour: e.evaluation.jour,
            genre: Genre::Evaluation,
            minutes: 0,
            donne_le: e.evaluation.jour,
            nature: "Évaluation".to_string(),
            classe_id: e.evaluation.classe_id,
            matiere: e.evaluation.matiere,
        });
    }
    Ok(out)
}

/// Jours ouvrés de l'établissement dans [du, au] (week-ends, vacances, fermetures et fériés exclus).
async fn jours_ouvres(tx: &mut PgConnection, etab: Uuid, du: NaiveDate, au: NaiveDate) -> Result<JoursOuvres, EntDbError> {
    let jours: Vec<(NaiveDate,)> = sqlx::query_as(
        "SELECT d::date FROM generate_series($2::date, $3::date, interval '1 day') AS d
          WHERE NOT ent_cdt_jour_ferme($1, d::date) ORDER BY 1",
    )
    .bind(etab)
    .bind(du)
    .bind(au)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    Ok(JoursOuvres::new(jours.into_iter().map(|(j,)| j)))
}

fn groupe_de(e: &EleveRow) -> Groupe {
    Groupe::depuis_niveau(e.niveau.as_deref())
}

impl PgEnt {
    /// Position des repères et leurs valeurs : toute personne de l'établissement qui lit le cahier (personnel selon ses droits,
    /// élève, responsable).
    pub async fn cdt_reperes(&self, a: &Acteur<'_>) -> Result<ReperesDto, EntDbError> {
        self.cdt_exiger_lecture_charge(a).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        Ok(reperes_de(position_de(&mut tx, a.etablissement_id).await?))
    }

    /// Choisit la position des repères de l'établissement (S20-10) : direction. La position vaut pour l'année courante.
    pub async fn cdt_reperes_regler(&self, a: &Acteur<'_>, position: &str) -> Result<ReperesDto, EntDbError> {
        self.exiger_action_etab(a.etablissement_id, a.user_id, a.role, Action::CahierParametrer).await?;
        if !Position::valide(position) {
            return Err(invalide("position inconnue : souple, normal ou strict"));
        }
        let annee = self.referentiel_annee_courante_assuree(a.etablissement_id).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_cdt_reglage (id, etablissement_id, annee_id, repere_charge, modifie_par_user_id)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (etablissement_id, annee_id) DO UPDATE
                SET repere_charge = EXCLUDED.repere_charge, modifie_par_user_id = EXCLUDED.modifie_par_user_id,
                    modifie_le = now(), version = ent_cdt_reglage.version + 1",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(annee.id)
        .bind(position)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        Ok(reperes_de(position_de(&mut tx, a.etablissement_id).await?))
    }

    /// Lecture de la charge : personnel qui lit le cahier, élève ou responsable (repères publics de l'établissement).
    async fn cdt_exiger_lecture_charge(&self, a: &Acteur<'_>) -> Result<(), EntDbError> {
        match Role::parse(a.role) {
            Some(Role::Eleve | Role::Parent) => Ok(()),
            Some(r) if r.is_staff() => {
                let droits = self.droits_effectifs(a.etablissement_id, a.user_id, a.role).await?;
                if droits.iter().any(|d| matches!(d.action, Action::CahierLire | Action::CahierEcrire)) {
                    Ok(())
                } else {
                    Err(EntDbError::Forbidden)
                }
            }
            _ => Err(EntDbError::Forbidden),
        }
    }

    /// CDT charge, vue du professeur **au moment de donner** : ce que ses élèves réels ont déjà ce jour-là, le soir d'avant,
    /// sur la semaine ; l'alerte éventuelle (un repère dépassé *à cause* de ce travail) ; deux dates alternatives calculées.
    /// Droit : saisie dans chacun des cours, à la date de l'action. Aucun nom, aucun titre, aucune matière hors classe.
    pub async fn cdt_charge_apercu(&self, a: &Acteur<'_>, b: ApercuBody) -> Result<ApercuChargeDto, EntDbError> {
        let etab = a.etablissement_id;
        if b.cours_ids.is_empty() || b.cours_ids.len() > 20 {
            return Err(invalide("indiquez entre 1 et 20 cours"));
        }
        let mut cours = b.cours_ids.clone();
        cours.sort();
        cours.dedup();
        for c in &cours {
            // Le cours d'un autre établissement n'existe pas ; le droit de saisie est relu à la date de l'action.
            self.cdt_cours(etab, *c).await?;
            self.cdt_exiger_saisie(a, *c, None).await?;
        }
        self.cdt_projeter_echus(etab).await?;
        let aujourdhui = self.jour_etab(etab).await?;
        // Échéance : le jour donné, ou la prochaine séance du premier cours (résolue ici, jamais par le client).
        let echeance_jour = match (b.echeance_jour, b.prochaine_seance) {
            (Some(j), _) => j,
            (None, true) => self.cdt_prochaine_seance(etab, cours[0], aujourdhui).await?.ok_or_else(|| {
                EntDbError::Refus(RefusMetier::Conflit, "aucune séance à venir pour ce cours : choisissez une date")
            })?,
            (None, false) => return Err(invalide("indiquez l'échéance")),
        };
        if echeance_jour <= aujourdhui {
            return Err(invalide("l'échéance est après aujourd'hui"));
        }
        let (duree, evaluation) = {
            let code = b.nature.as_deref().unwrap_or("travail");
            let eval = nature_est_evaluation(code);
            let defaut: Option<i16> = if eval { None } else { self.cdt_duree_nature(etab, code).await? };
            (if eval { 0 } else { i32::from(b.duree_estimee_min.or(defaut).unwrap_or(0)).max(0) }, eval)
        };
        let horizon = echeance_jour + Duration::days(HORIZON_PROPOSITIONS_JOURS);
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let position = position_de(&mut tx, etab).await?;
        let eleves = eleves_des_cours(&mut tx, etab, &cours, aujourdhui).await?;
        if eleves.len() > MAX_ELEVES_APERCU {
            return Err(invalide("trop d'élèves pour un aperçu : réduisez le nombre de cours"));
        }
        // Classes des cours demandés : seules leurs échéances montrent leur matière (§11.1).
        let classes_cours: HashSet<Uuid> = {
            let rows: Vec<(Option<Uuid>,)> = sqlx::query_as(
                "SELECT classe_id FROM ent_cours WHERE etablissement_id = $1 AND id = ANY($2)",
            )
            .bind(etab)
            .bind(&cours)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            rows.into_iter().filter_map(|(c,)| c).collect()
        };
        let ids: Vec<Uuid> = eleves.iter().map(|e| e.eleve_id).collect();
        let base_du = lundi_de(aujourdhui);
        let base = Base {
            echeances: echeances_des_eleves(&mut tx, etab, &ids, base_du, horizon + Duration::days(7), aujourdhui).await?,
            ouvres: jours_ouvres(&mut tx, etab, base_du - Duration::days(7), horizon + Duration::days(14)).await?,
            eleves,
        };
        Ok(apercu(&base, &classes_cours, position, aujourdhui, echeance_jour, duree, evaluation))
    }

    /// Jour de la prochaine séance (non annulée) d'un cours, après `apres`.
    async fn cdt_prochaine_seance(&self, etab: Uuid, cours: Uuid, apres: NaiveDate) -> Result<Option<NaiveDate>, EntDbError> {
        sqlx::query_scalar(
            "SELECT min(s.jour) FROM ent_edt_seance s
              WHERE s.etablissement_id = $1 AND s.jour > $3 AND s.statut <> 'annulee'
                AND ent_cdt_cours_de_seance(s.etablissement_id, s.id) = $2",
        )
        .bind(etab)
        .bind(cours)
        .bind(apres)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)
    }

    async fn cdt_duree_nature(&self, etab: Uuid, code: &str) -> Result<Option<i16>, EntDbError> {
        let d: Option<Option<i16>> = sqlx::query_scalar(
            "SELECT duree_defaut_min FROM ent_cdt_nature WHERE etablissement_id = $1 AND code = $2 AND archivee_le IS NULL",
        )
        .bind(etab)
        .bind(code)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await.map_err(db)?)
        .await
        .map_err(db)?;
        Ok(d.flatten())
    }

    /// La charge d'un élève, jour par jour : lui-même ou un responsable habilité (mêmes conditions que l'agenda, C3).
    pub async fn cdt_charge_eleve(
        &self,
        a: &Acteur<'_>,
        eleve_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<ChargeEleveDto, EntDbError> {
        if au < du || (au - du).num_days() > MAX_PERIODE_JOURS {
            return Err(invalide("période invalide"));
        }
        let (eleve, _vue): (Uuid, LecteurC3) = self.cdt_lecteur(a, eleve_id).await?;
        let etab = a.etablissement_id;
        self.cdt_projeter_echus(etab).await?;
        let aujourdhui = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let position = position_de(&mut tx, etab).await?;
        let niveau: Option<String> = sqlx::query_scalar(
            "SELECT n.code FROM ent_affectation a
               JOIN ent_classe cl ON cl.etablissement_id = a.etablissement_id AND cl.id = a.classe_id
               JOIN ent_niveau n ON n.etablissement_id = cl.etablissement_id AND n.id = cl.niveau_id
              WHERE a.etablissement_id = $1 AND a.eleve_id = $2 AND a.classe_id IS NOT NULL
                AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
              ORDER BY a.date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(eleve)
        .bind(aujourdhui)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let repere = reperes(position, Groupe::depuis_niveau(niveau.as_deref()));
        let echeances = echeances_des_eleves(&mut tx, etab, &[eleve], lundi_de(du), au + Duration::days(7), aujourdhui).await?;
        let ouvres = jours_ouvres(&mut tx, etab, lundi_de(du) - Duration::days(14), au + Duration::days(14)).await?;
        let charge = calculer(&echeances, &ouvres);
        let vide = BTreeMap::new();
        let jours = charge.get(&eleve).unwrap_or(&vide);
        let mut out_jours = Vec::new();
        let mut j = du;
        while j <= au {
            if let Some(c) = jours.get(&j) {
                out_jours.push(ChargeJourEleveDto {
                    jour: j,
                    nb_evaluations: c.nb_evaluations,
                    nb_travaux: c.nb_travaux,
                    minutes_soir_precedent: ouvres.veille(j).and_then(|v| jours.get(&v)).map(|x| x.minutes_soir).unwrap_or(0),
                    au_dessus: reperes_depasses_echeance(jours, &ouvres, j, &repere).into_iter().map(String::from).collect(),
                });
            }
            j += Duration::days(1);
        }
        let mut semaines = Vec::new();
        let mut lundi = lundi_de(du);
        while lundi <= au {
            let n = evaluations_semaine(jours, lundi);
            if n > 0 {
                semaines.push(ChargeSemaineEleveDto {
                    lundi,
                    nb_evaluations: n,
                    au_dessus: n > repere.evaluations_semaine,
                });
            }
            lundi += Duration::days(7);
        }
        Ok(ChargeEleveDto {
            eleve_id: eleve,
            du,
            au,
            position: position.code().to_string(),
            repere,
            jours: out_jours,
            semaines,
        })
    }

    /// Calendrier d'une classe : évaluations et travaux de ses cours par jour, avec les matières (l'équipe lit déjà le cahier
    /// de la classe), et, par jour, la charge **réelle** de ses élèves (groupes transversaux compris) agrégée, sans nom.
    /// Droit : `cahier.lire` sur la classe (professeur de la classe, PP, direction).
    pub async fn cdt_calendrier_classe(
        &self,
        a: &Acteur<'_>,
        classe_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<CalendrierClasseDto, EntDbError> {
        if au < du || (au - du).num_days() > MAX_PERIODE_JOURS {
            return Err(invalide("période invalide"));
        }
        let etab = a.etablissement_id;
        let cible = Cible {
            classe_id: Some(classe_id),
            groupe_id: None,
        };
        if !self.cahier_peut(a, Action::CahierLire, cible).await? {
            // Une classe inconnue ou d'un autre établissement n'existe pas pour cet acteur.
            return Err(EntDbError::Forbidden);
        }
        self.cdt_projeter_echus(etab).await?;
        let aujourdhui = self.jour_etab(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let position = position_de(&mut tx, etab).await?;
        let eleves = eleves_de_classe(&mut tx, etab, classe_id, aujourdhui).await?;
        let ids: Vec<Uuid> = eleves.iter().map(|e| e.eleve_id).collect();
        let echeances = echeances_des_eleves(&mut tx, etab, &ids, du, au + Duration::days(7), aujourdhui).await?;
        let ouvres = jours_ouvres(&mut tx, etab, du - Duration::days(14), au + Duration::days(14)).await?;
        let charge = calculer(&echeances, &ouvres);
        let groupes: HashMap<Uuid, Groupe> = eleves.iter().map(|e| (e.eleve_id, groupe_de(e))).collect();
        // Lignes du jour : les échéances **de la classe** (matière et titre), dédoublonnées entre élèves.
        #[derive(FromRow)]
        struct Ligne {
            jour: NaiveDate,
            matiere: String,
            nature: String,
            code: String,
            titre: String,
            minutes: Option<i16>,
            evaluation: bool,
        }
        let lignes_travaux: Vec<Ligne> = sqlx::query_as(
            "SELECT ent_aujourdhui_etab($1, t.echeance_le) AS jour, c.libelle AS matiere, n.libelle AS nature, n.code,
                    t.titre, t.duree_estimee_min AS minutes, (t.evaluation_id IS NOT NULL) AS evaluation
               FROM ent_cdt_travail t
               JOIN ent_cours c ON c.etablissement_id = t.etablissement_id AND c.id = t.cours_id AND c.classe_id = $2
               JOIN ent_cdt_nature n ON n.etablissement_id = t.etablissement_id AND n.id = t.nature_id
              WHERE t.etablissement_id = $1 AND t.etat = 'publie' AND t.visible_a IS NOT NULL AND t.visible_a <= now()
                AND t.public = 'cours' AND ent_aujourdhui_etab($1, t.echeance_le) BETWEEN $3 AND $4
              ORDER BY t.echeance_le, t.titre, t.id",
        )
        .bind(etab)
        .bind(classe_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut par_jour: BTreeMap<NaiveDate, Vec<LigneClasseDto>> = BTreeMap::new();
        for l in lignes_travaux {
            let eval = l.evaluation || nature_est_evaluation(&l.code);
            let long = l.minutes.is_some_and(|m| est_long(i32::from(m), l.jour, l.jour));
            if !eval && !long && l.minutes.is_none_or(|m| i32::from(m) <= 90) {
                // Calendrier : évaluations et travaux longs seulement (les travaux courts restent dans le cahier).
                continue;
            }
            par_jour.entry(l.jour).or_default().push(LigneClasseDto {
                genre: if eval { "evaluation" } else { "travail" }.to_string(),
                matiere: l.matiere,
                nature: l.nature,
                titre: l.titre,
                duree_estimee_min: l.minutes,
            });
        }
        // Évaluations annoncées du socle 21 pour la classe (matière et titre) : même règle.
        let devoirs: Vec<(NaiveDate, String, String)> = sqlx::query_as(
            "SELECT d.date_devoir, COALESCE(m.libelle, 'Évaluation'), d.titre
               FROM ent_devoir d LEFT JOIN ent_matiere m ON m.etablissement_id = d.etablissement_id AND m.id = d.matiere_id
              WHERE d.etablissement_id = $1 AND d.classe_id = $2 AND d.date_devoir BETWEEN $3 AND $4
                AND NOT EXISTS (SELECT 1 FROM ent_cdt_travail t WHERE t.etablissement_id = d.etablissement_id AND t.evaluation_id = d.id)
              ORDER BY d.date_devoir, d.titre, d.id",
        )
        .bind(etab)
        .bind(classe_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (jour, matiere, titre) in devoirs {
            par_jour.entry(jour).or_default().push(LigneClasseDto {
                genre: "evaluation".to_string(),
                matiere,
                nature: "Évaluation".to_string(),
                titre,
                duree_estimee_min: None,
            });
        }
        let mut jours = Vec::new();
        let mut j = du;
        while j <= au {
            let lignes = par_jour.remove(&j).unwrap_or_default();
            let mut au_dessus = 0;
            let mut eval_max = 0;
            let mut soirs = Vec::new();
            for (eleve, g) in &groupes {
                let Some(jours_eleve) = charge.get(eleve) else { continue };
                let r = reperes(position, *g);
                if !reperes_depasses_echeance(jours_eleve, &ouvres, j, &r).is_empty() {
                    au_dessus += 1;
                }
                let c = jours_eleve.get(&j).copied().unwrap_or_default();
                eval_max = eval_max.max(c.nb_evaluations);
                soirs.push(ouvres.veille(j).and_then(|v| jours_eleve.get(&v)).map(|x| x.minutes_soir).unwrap_or(0));
            }
            if !lignes.is_empty() || au_dessus > 0 || eval_max > 0 {
                jours.push(JourClasseDto {
                    jour: j,
                    lignes,
                    nb_eleves_au_dessus: au_dessus,
                    evaluations_max_par_eleve: eval_max,
                    minutes_soir_precedent_mediane: mediane(soirs),
                });
            }
            j += Duration::days(1);
        }
        Ok(CalendrierClasseDto {
            classe_id,
            du,
            au,
            position: position.code().to_string(),
            nb_eleves: eleves.len() as i32,
            jours,
        })
    }

    /// Le professeur passe outre l'alerte pour ce travail : le serveur **recalcule** l'alerte (il ne croit jamais les valeurs
    /// du client) et compte la dérogation, une fois par repère. Sans alerte réelle : rien n'est compté (conflit).
    pub async fn cdt_alerte_ignorer(&self, a: &Acteur<'_>, travail_id: Uuid) -> Result<Vec<AlerteChargeDto>, EntDbError> {
        let etab = a.etablissement_id;
        let t = self.cdt_charger_travail(a, travail_id).await?;
        // Seul l'auteur du travail déroge pour lui-même (la direction ne déroge pas à sa place).
        if t.auteur_user_id != Some(a.user_id) {
            return Err(EntDbError::Forbidden);
        }
        self.cdt_exiger_saisie(a, t.cours_id, None).await?;
        let aujourdhui = self.jour_etab(etab).await?;
        let (echeance_jour, nature_code, duree): (NaiveDate, String, Option<i16>) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            sqlx::query_as(
                "SELECT ent_aujourdhui_etab(t.etablissement_id, t.echeance_le), n.code, t.duree_estimee_min
                   FROM ent_cdt_travail t JOIN ent_cdt_nature n ON n.etablissement_id = t.etablissement_id AND n.id = t.nature_id
                  WHERE t.etablissement_id = $1 AND t.id = $2",
            )
            .bind(etab)
            .bind(travail_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?
        };
        // Alerte recalculée *sans* ce travail puis *avec* lui : le travail est déjà publié, on le retire du calcul de base.
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let position = position_de(&mut tx, etab).await?;
        let eleves = eleves_des_cours(&mut tx, etab, &[t.cours_id], aujourdhui).await?;
        let ids: Vec<Uuid> = eleves.iter().map(|e| e.eleve_id).collect();
        let horizon = echeance_jour + Duration::days(HORIZON_PROPOSITIONS_JOURS);
        let mut echeances = echeances_des_eleves(&mut tx, etab, &ids, lundi_de(aujourdhui), horizon + Duration::days(7), aujourdhui).await?;
        let ouvres = jours_ouvres(&mut tx, etab, lundi_de(aujourdhui) - Duration::days(7), horizon + Duration::days(14)).await?;
        // Retire UNE échéance de ce travail par élève (même jour, même durée, même nature) : la base « sans lui ».
        let eval = nature_est_evaluation(&nature_code);
        let minutes = if eval { 0 } else { i32::from(duree.unwrap_or(0)) };
        for e in &ids {
            if let Some(i) = echeances.iter().position(|x| {
                x.eleve_id == *e && x.jour == echeance_jour && x.minutes == minutes && (x.genre == Genre::Evaluation) == eval
            }) {
                echeances.remove(i);
            }
        }
        let classes_cours = HashSet::new();
        let base = Base {
            echeances,
            ouvres,
            eleves,
        };
        let r = apercu(&base, &classes_cours, position, aujourdhui, echeance_jour, minutes, eval);
        if r.alertes.is_empty() {
            return Err(EntDbError::Refus(RefusMetier::Conflit, "aucune alerte de charge pour ce travail"));
        }
        for al in &r.alertes {
            sqlx::query(
                "INSERT INTO ent_cdt_alerte_charge (etablissement_id, travail_id, repere, valeur, seuil, ignoree)
                 VALUES ($1, $2, $3, $4, $5, true)
                 ON CONFLICT (travail_id, repere) DO UPDATE SET valeur = EXCLUDED.valeur, seuil = EXCLUDED.seuil, ignoree = true",
            )
            .bind(etab)
            .bind(travail_id)
            .bind(&al.repere)
            .bind(al.valeur_max)
            .bind(al.seuil)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        tx.commit().await.map_err(db)?;
        Ok(r.alertes)
    }

    /// Alertes ignorées sur une période (jour de création du travail) : **comptées** par classe pour la direction et le PP,
    /// jamais nominatives ; un professeur retrouve les siennes (titre, repère, valeur).
    pub async fn cdt_alertes_ignorees(&self, a: &Acteur<'_>, du: NaiveDate, au: NaiveDate) -> Result<AlertesIgnoreesDto, EntDbError> {
        if au < du || (au - du).num_days() > 366 {
            return Err(invalide("période invalide"));
        }
        let etab = a.etablissement_id;
        let direction = self.a_action_etablissement(etab, a.user_id, a.role, Action::CahierLire).await?;
        let prof = self.cahier_droit_ecrire_global(a).await?;
        if !direction && !prof {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Par classe : lecture agrégée réservée à qui lit le cahier de tout l'établissement ; un professeur ne voit que les siennes.
        let (total, par_classe): (i32, Vec<AlertesParClasseDto>) = if direction {
            let rows: Vec<(String, i64)> = sqlx::query_as(
                "SELECT COALESCE(cl.code, 'Groupe sans classe'), count(DISTINCT t.id)
                   FROM ent_cdt_alerte_charge x
                   JOIN ent_cdt_travail t ON t.etablissement_id = x.etablissement_id AND t.id = x.travail_id
                   JOIN ent_cours c ON c.etablissement_id = t.etablissement_id AND c.id = t.cours_id
                   LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = c.classe_id
                  WHERE x.etablissement_id = $1 AND x.ignoree AND ent_aujourdhui_etab($1, x.cree_le) BETWEEN $2 AND $3
                  GROUP BY 1 ORDER BY 1",
            )
            .bind(etab)
            .bind(du)
            .bind(au)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let total = rows.iter().map(|(_, n)| *n as i32).sum();
            (
                total,
                rows.into_iter().map(|(classe, n)| AlertesParClasseDto { classe, nb: n as i32 }).collect(),
            )
        } else {
            (0, Vec::new())
        };
        let mes: Vec<(Uuid, String, String, i32, i32)> = sqlx::query_as(
            "SELECT t.id, t.titre, x.repere, x.valeur, x.seuil
               FROM ent_cdt_alerte_charge x
               JOIN ent_cdt_travail t ON t.etablissement_id = x.etablissement_id AND t.id = x.travail_id
              WHERE x.etablissement_id = $1 AND x.ignoree AND t.auteur_user_id = $2
                AND ent_aujourdhui_etab($1, x.cree_le) BETWEEN $3 AND $4
              ORDER BY x.cree_le, t.id, x.repere",
        )
        .bind(etab)
        .bind(a.user_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let total = if direction {
            total
        } else {
            mes.iter().map(|m| m.0).collect::<BTreeSet<_>>().len() as i32
        };
        Ok(AlertesIgnoreesDto {
            du,
            au,
            total,
            par_classe,
            mes_alertes: mes
                .into_iter()
                .map(|(travail_id, titre, repere, valeur, seuil)| MonAlerteIgnoreeDto {
                    travail_id,
                    titre,
                    repere,
                    valeur,
                    seuil,
                })
                .collect(),
        })
    }
}

/// Aperçu de charge pour un travail à donner (`duree` minutes, `evaluation` : annoncé comme évaluation) dû à `echeance`.
fn apercu(
    base: &Base,
    classes_cours: &HashSet<Uuid>,
    position: Position,
    aujourdhui: NaiveDate,
    echeance: NaiveDate,
    duree: i32,
    evaluation: bool,
) -> ApercuChargeDto {
    let groupes: HashMap<Uuid, Groupe> = base.eleves.iter().map(|e| (e.eleve_id, groupe_de(e))).collect();
    let nouveau = |jour: NaiveDate| -> Vec<Echeance> {
        base.eleves
            .iter()
            .map(|e| Echeance {
                eleve_id: e.eleve_id,
                jour,
                genre: if evaluation { Genre::Evaluation } else { Genre::Travail },
                minutes: duree,
                donne_le: aujourdhui,
                nature: String::new(),
                classe_id: None,
                matiere: String::new(),
            })
            .collect()
    };
    let avant = calculer(&base.echeances, &base.ouvres);
    let vide = BTreeMap::new();
    // Repères nouvellement dépassés à cause du travail, par élève, pour une échéance donnée.
    let depassements = |jour: NaiveDate| -> Vec<(Uuid, Vec<(&'static str, i32, i32)>)> {
        let mut tous = base.echeances.clone();
        tous.extend(nouveau(jour));
        let apres = calculer(&tous, &base.ouvres);
        let mut out = Vec::new();
        for e in &base.eleves {
            let r = reperes(position, groupes[&e.eleve_id]);
            let (j0, j1) = (avant.get(&e.eleve_id).unwrap_or(&vide), apres.get(&e.eleve_id).unwrap_or(&vide));
            let mut v: Vec<(&'static str, i32, i32)> = Vec::new();
            // Soirs touchés : tous ceux dont la durée augmente (travail long réparti) ; le pire soir au-dessus du repère.
            let pire_soir = j1
                .iter()
                .filter(|(j, c)| c.minutes_soir > j0.get(j).map(|x| x.minutes_soir).unwrap_or(0) && c.minutes_soir > r.minutes_soir)
                .map(|(_, c)| c.minutes_soir)
                .max();
            if let Some(m) = pire_soir {
                v.push((REPERE_MINUTES, m, r.minutes_soir));
            }
            let (n0, n1) = (
                j0.get(&jour).map(|c| c.nb_evaluations).unwrap_or(0),
                j1.get(&jour).map(|c| c.nb_evaluations).unwrap_or(0),
            );
            if n1 > n0 && n1 > r.evaluations_jour {
                v.push((REPERE_EVALUATIONS, n1, r.evaluations_jour));
            }
            let (s0, s1) = (evaluations_semaine(j0, jour), evaluations_semaine(j1, jour));
            if s1 > s0 && s1 > r.evaluations_semaine {
                v.push((REPERE_SEMAINE, s1, r.evaluations_semaine));
            }
            if !v.is_empty() {
                out.push((e.eleve_id, v));
            }
        }
        out
    };
    let dep = depassements(echeance);
    let mut alertes: Vec<AlerteChargeDto> = Vec::new();
    for repere in [REPERE_MINUTES, REPERE_EVALUATIONS, REPERE_SEMAINE] {
        let concernes: Vec<&(&'static str, i32, i32)> = dep.iter().flat_map(|(_, v)| v.iter()).filter(|x| x.0 == repere).collect();
        if concernes.is_empty() {
            continue;
        }
        alertes.push(AlerteChargeDto {
            repere: repere.to_string(),
            seuil: concernes.iter().map(|x| x.2).min().unwrap_or(0),
            valeur_max: concernes.iter().map(|x| x.1).max().unwrap_or(0),
            nb_eleves: concernes.len() as i32,
        });
    }
    // Soir du travail : veille ouvrée de l'échéance ; durée cumulée AVEC le travail à donner (travail court).
    let soir = base.ouvres.veille(echeance).unwrap_or(echeance - Duration::days(1));
    let mut tous = base.echeances.clone();
    tous.extend(nouveau(echeance));
    let apres = calculer(&tous, &base.ouvres);
    let minutes_soir: Vec<i32> = base
        .eleves
        .iter()
        .map(|e| apres.get(&e.eleve_id).and_then(|j| j.get(&soir)).map(|c| c.minutes_soir).unwrap_or(0))
        .collect();
    let evaluations_ce_jour = distribution(
        base.eleves
            .iter()
            .map(|e| avant.get(&e.eleve_id).and_then(|j| j.get(&echeance)).map(|c| c.nb_evaluations).unwrap_or(0)),
    );
    // Autres échéances du jour : natures et nombres ; la matière seulement pour la classe d'un cours demandé.
    let mut autres: BTreeMap<(String, String, Option<String>), HashSet<Uuid>> = BTreeMap::new();
    for e in base.echeances.iter().filter(|e| e.jour == echeance) {
        let matiere = e.classe_id.filter(|c| classes_cours.contains(c)).map(|_| e.matiere.clone());
        autres.entry((e.genre.code().to_string(), e.nature.clone(), matiere)).or_default().insert(e.eleve_id);
    }
    let autres_echeances = autres
        .into_iter()
        .map(|((genre, nature, matiere), eleves)| EcheanceAutreDto {
            jour: echeance,
            genre,
            nature,
            matiere,
            nb_eleves: eleves.len() as i32,
        })
        .collect();
    // Dates alternatives : en cas d'alerte, les deux jours ouvrés (après aujourd'hui, jusqu'à 14 jours après l'échéance) qui
    // laissent le moins d'élèves au-dessus d'un repère.
    let mut dates_proposees = Vec::new();
    if !alertes.is_empty() {
        let limite = echeance + Duration::days(HORIZON_PROPOSITIONS_JOURS);
        let mut candidats: Vec<(i32, i32, i64, DateProposeeDto)> = Vec::new();
        for jour in base.ouvres.iter_apres(aujourdhui).take_while(|j| *j <= limite) {
            if jour == echeance {
                continue;
            }
            let n = depassements(jour).len() as i32;
            let mut t2 = base.echeances.clone();
            t2.extend(nouveau(jour));
            let ap = calculer(&t2, &base.ouvres);
            let veille = base.ouvres.veille(jour).unwrap_or(jour - Duration::days(1));
            let med = mediane(
                base.eleves
                    .iter()
                    .map(|e| ap.get(&e.eleve_id).and_then(|j| j.get(&veille)).map(|c| c.minutes_soir).unwrap_or(0))
                    .collect(),
            );
            let emax = base
                .eleves
                .iter()
                .map(|e| ap.get(&e.eleve_id).and_then(|j| j.get(&jour)).map(|c| c.nb_evaluations).unwrap_or(0))
                .max()
                .unwrap_or(0);
            let distance = (jour - echeance).num_days().abs();
            candidats.push((
                n,
                med,
                distance,
                DateProposeeDto {
                    jour,
                    nb_eleves_au_dessus: n,
                    evaluations_max: emax,
                    minutes_soir_mediane: med,
                },
            ));
        }
        candidats.sort_by(|a, b| (a.0, a.1, a.2, a.3.jour).cmp(&(b.0, b.1, b.2, b.3.jour)));
        dates_proposees = candidats.into_iter().take(2).map(|c| c.3).collect();
        dates_proposees.sort_by_key(|d| d.jour);
    }
    ApercuChargeDto {
        echeance_jour: echeance,
        soir_jour: soir,
        nb_eleves: base.eleves.len() as i32,
        position: position.code().to_string(),
        evaluations_ce_jour,
        minutes_soir_mediane: mediane(minutes_soir.clone()),
        minutes_soir_max: minutes_soir.into_iter().max().unwrap_or(0),
        autres_echeances,
        alerte: !alertes.is_empty(),
        alertes,
        dates_proposees,
    }
}

#[allow(dead_code)]
fn _introuvable() -> EntDbError {
    introuvable()
}
