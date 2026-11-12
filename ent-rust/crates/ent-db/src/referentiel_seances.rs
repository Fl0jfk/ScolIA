//! Référentiel S2-2 : séances datées (EDT-06) et lien créneau ↔ service (EDT-03).
//!
//! Une **séance** est l'occurrence datée d'un créneau d'emploi du temps : base de l'appel,
//! des absences à la séance (D-020), du cahier de textes et des remplacements. Elle est
//! générée depuis les créneaux de l'établissement, la grille horaire (M1…S4), l'alternance
//! A/B, les exceptions (annulation, remplacement, changement de salle) et le calendrier
//! (vacances, fermetures, journées banalisées : aucune séance ces jours-là).
//!
//! La génération est **idempotente** : relancée, elle met à jour les séances générées,
//! supprime celles qui ne sont plus attendues (nouveau jour férié…) et ne touche jamais une
//! séance saisie à la main (`source <> 'genere'`).
//! Limite : quand l'appel sera rattaché aux séances (S2-4), la suppression d'une séance
//! référencée devra devenir une annulation (`statut = 'annulee'`).

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use domain_eleve::{creneau_actif_semaine, semaine_ab_au_jour, EdtSemaineAb};
use ent_auth::Action;
use ent_events::AggregateKind;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, publier_agg, Acteur};
use crate::referentiel_structure::{index_neutralises, CreneauGrille, JourCalendrier};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Créneau de l'EDT rapproché de la classe de l'année visée.
#[derive(Debug, Clone, FromRow)]
pub struct CreneauSource {
    pub id: Uuid,
    pub classe: String,
    pub classe_id: Uuid,
    pub site_id: Option<Uuid>,
    pub prof_user_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub salle: Option<String>,
    pub semaine_ab: String,
    pub service_id: Option<Uuid>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ExceptionSource {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub kind: String,
    pub remplacant_user_id: Option<Uuid>,
    pub nouvelle_salle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeancePlan {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub classe_id: Uuid,
    pub classe: String,
    pub service_id: Option<Uuid>,
    pub prof_user_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub salle: Option<String>,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub grille_code: Option<String>,
    pub demi_journee: String,
    pub statut: String,
    pub remplacant_user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportSeances {
    pub annee_id: Uuid,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub creees: i64,
    pub mises_a_jour: i64,
    pub supprimees: i64,
    /// Séances « plus attendues » qui portent un contenu de cahier ou un travail : retirées
    /// (état `retiree`), jamais supprimées (socle 10, D1).
    pub retirees: i64,
    /// Séances rattachées à leur placement et à leur version (socle 10, T2) : information, jamais une
    /// modification du plan.
    pub rattachees: i64,
    /// Jours de la plage sans séance parce que neutralisés par le calendrier.
    pub jours_neutralises: i64,
    /// Créneaux A/B ignorés faute de calendrier d'alternance (aucune séance inventée).
    pub creneaux_ab_ignores: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct Seance {
    pub id: Uuid,
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub classe_id: Option<Uuid>,
    pub classe: String,
    pub service_id: Option<Uuid>,
    pub prof_user_id: Uuid,
    pub matiere_libelle: String,
    pub salle: Option<String>,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub grille_code: Option<String>,
    pub demi_journee: Option<String>,
    pub statut: String,
    pub remplacant_user_id: Option<Uuid>,
}

/// Créneau de grille qui commence à `debut` (site d'abord, établissement ensuite).
fn creneau_de_grille<'a>(
    grille: &'a [CreneauGrille],
    site_id: Option<Uuid>,
    debut: NaiveTime,
) -> Option<&'a CreneauGrille> {
    let cherche = |site: Option<Uuid>| {
        grille
            .iter()
            .find(|g| g.site_id == site && g.heure_debut == debut)
    };
    site_id.and_then(|s| cherche(Some(s))).or_else(|| cherche(None))
}

/// Calcule les séances attendues sur `[du, au]` : fonction pure (testée sans base).
/// Renvoie aussi le nombre de jours neutralisés et de créneaux A/B ignorés.
pub fn planifier_seances(
    creneaux: &[CreneauSource],
    du: NaiveDate,
    au: NaiveDate,
    neutralises: &BTreeMap<NaiveDate, Vec<Option<Uuid>>>,
    ab_reference: Option<NaiveDate>,
    exceptions: &[ExceptionSource],
    grille: &[CreneauGrille],
) -> (Vec<SeancePlan>, i64, i64) {
    let exc: HashMap<(Uuid, NaiveDate), &ExceptionSource> =
        exceptions.iter().map(|e| ((e.creneau_id, e.jour), e)).collect();
    let mut plans = Vec::new();
    let mut jours_neutres = 0;
    let mut ab_ignores: HashSet<Uuid> = HashSet::new();
    let mut jour = du;
    while jour <= au {
        let iso = jour.weekday().number_from_monday() as i16;
        let sites_fermes = neutralises.get(&jour);
        if sites_fermes.is_some_and(|s| s.contains(&None)) && iso <= 5 {
            jours_neutres += 1;
        }
        for c in creneaux.iter().filter(|c| c.jour_semaine == iso) {
            if let Some(s) = sites_fermes {
                if s.contains(&None) || (c.site_id.is_some() && s.contains(&c.site_id)) {
                    continue;
                }
            }
            let ab = EdtSemaineAb::parse(&c.semaine_ab).unwrap_or(EdtSemaineAb::Toutes);
            if ab != EdtSemaineAb::Toutes {
                match ab_reference {
                    None => {
                        ab_ignores.insert(c.id);
                        continue;
                    }
                    Some(r) if !creneau_actif_semaine(ab, semaine_ab_au_jour(r, jour)) => continue,
                    Some(_) => {}
                }
            }
            let mut plan = SeancePlan {
                creneau_id: c.id,
                jour,
                classe_id: c.classe_id,
                classe: c.classe.trim().to_string(),
                service_id: c.service_id,
                prof_user_id: c.prof_user_id,
                matiere_id: c.matiere_id,
                matiere_libelle: c.matiere_libelle.clone(),
                salle: c.salle.clone(),
                heure_debut: c.heure_debut,
                heure_fin: c.heure_fin,
                grille_code: None,
                demi_journee: String::new(),
                statut: "prevue".into(),
                remplacant_user_id: None,
            };
            match creneau_de_grille(grille, c.site_id, c.heure_debut) {
                Some(g) => {
                    plan.grille_code = Some(g.code.clone());
                    plan.demi_journee = g.demi_journee.clone();
                }
                None => {
                    let midi = NaiveTime::from_hms_opt(12, 30, 0).expect("heure valide");
                    plan.demi_journee =
                        if c.heure_debut < midi { "matin" } else { "apres_midi" }.into();
                }
            }
            if let Some(e) = exc.get(&(c.id, jour)) {
                match e.kind.as_str() {
                    "annulation" => plan.statut = "annulee".into(),
                    "remplacement" => {
                        plan.statut = "remplacee".into();
                        plan.remplacant_user_id = e.remplacant_user_id;
                        if e.nouvelle_salle.is_some() {
                            plan.salle = e.nouvelle_salle.clone();
                        }
                    }
                    "changement_salle" => {
                        if e.nouvelle_salle.is_some() {
                            plan.salle = e.nouvelle_salle.clone();
                        }
                    }
                    _ => {}
                }
            }
            plans.push(plan);
        }
        jour += Duration::days(1);
    }
    (plans, jours_neutres, ab_ignores.len() as i64)
}

/// D1 (socle 10) : une séance qui porte un contenu de cahier ou un travail n'est jamais supprimée,
/// elle est **retirée** (état `retiree`, date, motif) ; une séance vide est supprimée. Retourne
/// `(retirées, supprimées)`. Toute suppression d'une séance passe par cette fonction.
pub(crate) async fn retirer_ou_supprimer_seances(
    tx: &mut TenantTx<'_>,
    etab: Uuid,
    ids: &[Uuid],
    motif: &str,
) -> Result<(i64, i64), EntDbError> {
    let retirees = sqlx::query(
        "UPDATE ent_edt_seance s
            SET statut = 'retiree', retiree_le = now(), retiree_motif = $3, updated_at = now()
          WHERE s.etablissement_id = $1 AND s.id = ANY($2) AND s.statut <> 'retiree'
            AND (EXISTS (SELECT 1 FROM ent_seance_contenu c
                          WHERE c.etablissement_id = s.etablissement_id AND c.seance_id = s.id)
                 OR EXISTS (SELECT 1 FROM ent_travail t
                             WHERE t.etablissement_id = s.etablissement_id AND t.seance_id = s.id))",
    )
    .bind(etab)
    .bind(ids)
    .bind(motif)
    .execute(&mut **tx)
    .await
    .map_err(db)?
    .rows_affected() as i64;
    let supprimees = sqlx::query(
        "DELETE FROM ent_edt_seance s
          WHERE s.etablissement_id = $1 AND s.id = ANY($2) AND s.statut <> 'retiree'
            AND NOT EXISTS (SELECT 1 FROM ent_seance_contenu c
                             WHERE c.etablissement_id = s.etablissement_id AND c.seance_id = s.id)
            AND NOT EXISTS (SELECT 1 FROM ent_travail t
                             WHERE t.etablissement_id = s.etablissement_id AND t.seance_id = s.id)",
    )
    .bind(etab)
    .bind(ids)
    .execute(&mut **tx)
    .await
    .map_err(db)?
    .rows_affected() as i64;
    Ok((retirees, supprimees))
}

impl PgEnt {
    /// Socle 10 (T2) : rattache les séances générées de `[du, au]` à leur placement et à leur version
    /// (identité stable : la séance du 12 novembre reste la même quand le plan est republié), pose le
    /// cours et le code de créneau, synchronise les lieux depuis la salle de la séance, et le motif
    /// PUBLIC des annulations de lot. Idempotent : ne réécrit que ce qui diffère.
    pub(crate) async fn rattacher_seances(
        tx: &mut sqlx::PgConnection,
        etab: Uuid,
        annee_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
        version_id: Uuid,
    ) -> Result<i64, EntDbError> {
        let rattachees = sqlx::query(
            "UPDATE ent_edt_seance s
                SET placement_id = p.id, version_id = p.version_id, cours_id = c.cours_id,
                    creneau_code = p.creneau_debut_code, calcule_le = now()
               FROM ent_edt_creneau c
               JOIN ent_edt_placement p ON p.creneau_id = c.id AND p.etablissement_id = c.etablissement_id
              WHERE s.etablissement_id = $1 AND s.annee_id = $2 AND s.jour BETWEEN $3 AND $4
                AND s.creneau_id = c.id AND s.etablissement_id = c.etablissement_id
                AND p.version_id = $5 AND s.statut <> 'retiree'
                AND (s.placement_id IS DISTINCT FROM p.id OR s.version_id IS DISTINCT FROM p.version_id
                     OR s.cours_id IS DISTINCT FROM c.cours_id
                     OR s.creneau_code IS DISTINCT FROM p.creneau_debut_code)",
        )
        .bind(etab)
        .bind(annee_id)
        .bind(du)
        .bind(au)
        .bind(version_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected() as i64;
        // Lieux : la salle (texte) de la séance, résolue dans le référentiel des lieux.
        sqlx::query(
            "DELETE FROM ent_edt_seance_lieu l USING ent_edt_seance s
              WHERE l.etablissement_id = $1 AND s.id = l.seance_id AND s.etablissement_id = l.etablissement_id
                AND s.annee_id = $2 AND s.jour BETWEEN $3 AND $4
                AND NOT EXISTS (SELECT 1 FROM ent_salle sa
                                 WHERE sa.id = l.lieu_id AND sa.etablissement_id = s.etablissement_id
                                   AND lower(btrim(sa.code)) = lower(btrim(coalesce(s.salle, ''))))",
        )
        .bind(etab)
        .bind(annee_id)
        .bind(du)
        .bind(au)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_edt_seance_lieu (seance_id, etablissement_id, lieu_id, principal)
             SELECT s.id, s.etablissement_id, sa.id, true
               FROM ent_edt_seance s
               JOIN ent_salle sa ON sa.etablissement_id = s.etablissement_id
                AND lower(btrim(sa.code)) = lower(btrim(s.salle))
              WHERE s.etablissement_id = $1 AND s.annee_id = $2 AND s.jour BETWEEN $3 AND $4
                AND s.statut <> 'retiree' AND s.source IN ('genere', 'deplacement')
             ON CONFLICT DO NOTHING",
        )
        .bind(etab)
        .bind(annee_id)
        .bind(du)
        .bind(au)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Motif PUBLIC d'une annulation de lot (jamais le motif interne de l'exception).
        sqlx::query(
            "UPDATE ent_edt_seance s SET motif_public = l.motif_public
               FROM ent_edt_exception x
               JOIN ent_edt_annulation_masse l ON l.id = x.lot_id AND l.etablissement_id = x.etablissement_id
              WHERE s.etablissement_id = $1 AND s.annee_id = $2 AND s.jour BETWEEN $3 AND $4
                AND x.etablissement_id = s.etablissement_id AND x.creneau_id = s.creneau_id AND x.jour = s.jour
                AND x.kind = 'annulation' AND s.statut = 'annulee'
                AND s.motif_public IS DISTINCT FROM l.motif_public",
        )
        .bind(etab)
        .bind(annee_id)
        .bind(du)
        .bind(au)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_edt_seance SET motif_public = NULL
              WHERE etablissement_id = $1 AND annee_id = $2 AND jour BETWEEN $3 AND $4
                AND statut <> 'annulee' AND motif_public IS NOT NULL",
        )
        .bind(etab)
        .bind(annee_id)
        .bind(du)
        .bind(au)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rattachees)
    }
}

impl PgEnt {
    /// EDT-06 : génère (ou régénère) les séances datées de l'année sur `[du, au]`
    /// (défaut : de aujourd'hui, borné à l'année, à la fin de l'année).
    pub async fn ref_generer_seances(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        du: Option<NaiveDate>,
        au: Option<NaiveDate>,
    ) -> Result<RapportSeances, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await?;
        let annee: Option<(NaiveDate, NaiveDate, String)> = sqlx::query_as(
            "SELECT date_debut, date_fin, statut FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((debut, fin, statut)) = annee else {
            return Err(introuvable());
        };
        if statut == "cloturee" {
            return Err(conflit());
        }
        // D8 (socle 10 T0) : « aujourd'hui » est le jour de l'établissement, pas le jour UTC.
        let aujourdhui = self.aujourdhui_etablissement(e).await?;
        let du = du.unwrap_or_else(|| aujourdhui.clamp(debut, fin)).max(debut);
        let au = au.unwrap_or(fin).min(fin);
        if au < du || (au - du).num_days() > 400 {
            return Err(invalide());
        }

        let creneaux: Vec<CreneauSource> = sqlx::query_as(
            "SELECT c.id, btrim(c.classe) AS classe, cl.id AS classe_id, cl.site_id,
                    c.prof_user_id, c.matiere_id, c.matiere_libelle, c.jour_semaine,
                    c.heure_debut, c.heure_fin, c.salle, c.semaine_ab, s.id AS service_id
               FROM ent_edt_creneau c
               JOIN ent_classe cl
                 ON cl.etablissement_id = c.etablissement_id AND cl.annee_id = $2
                AND cl.code = btrim(c.classe) AND cl.archivee_le IS NULL
               LEFT JOIN ent_service_enseignement s
                 ON s.id = c.service_id AND s.etablissement_id = c.etablissement_id AND s.annee_id = $2
              WHERE c.etablissement_id = $1",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let exceptions: Vec<ExceptionSource> = sqlx::query_as(
            "SELECT creneau_id, jour, kind, remplacant_user_id, nouvelle_salle FROM ent_edt_exception
              WHERE etablissement_id = $1 AND jour BETWEEN $2 AND $3",
        )
        .bind(e)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let ab_reference: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT reference_lundi FROM ent_edt_calendrier_ab WHERE etablissement_id = $1",
        )
        .bind(e)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let grille: Vec<CreneauGrille> = sqlx::query_as(
            "SELECT id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin
               FROM ent_grille_horaire WHERE etablissement_id = $1",
        )
        .bind(e)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let jours: Vec<JourCalendrier> = self.calendrier_plage(e, du, au).await?;
        let neutralises = index_neutralises(&jours);

        let (plans, jours_neutralises, creneaux_ab_ignores) =
            planifier_seances(&creneaux, du, au, &neutralises, ab_reference, &exceptions, &grille);

        let mut rapport = RapportSeances {
            annee_id,
            du: Some(du),
            au: Some(au),
            jours_neutralises,
            creneaux_ab_ignores,
            ..Default::default()
        };
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 1))")
            .bind(e)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        // Socle 10 (T1) : les créneaux sont repris en placements d'une version « reprise » avant de
        // générer ; les séances sont ensuite rattachées à leur placement (T2). Rejouable.
        let reprise = Self::reprendre_creneaux_tx(&mut tx, a, annee_id).await?;
        let attendues: HashSet<(Uuid, NaiveDate)> =
            plans.iter().map(|p| (p.creneau_id, p.jour)).collect();
        for p in &plans {
            let res: Option<(bool,)> = sqlx::query_as(
                "INSERT INTO ent_edt_seance
                   (id, etablissement_id, annee_id, creneau_id, jour, classe_id, classe, service_id,
                    prof_user_id, matiere_id, matiere_libelle, salle, heure_debut, heure_fin,
                    grille_code, demi_journee, statut, remplacant_user_id)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
                 ON CONFLICT (etablissement_id, creneau_id, jour) DO UPDATE SET
                    annee_id = EXCLUDED.annee_id, classe_id = EXCLUDED.classe_id,
                    classe = EXCLUDED.classe, service_id = EXCLUDED.service_id,
                    prof_user_id = EXCLUDED.prof_user_id, matiere_id = EXCLUDED.matiere_id,
                    matiere_libelle = EXCLUDED.matiere_libelle, salle = EXCLUDED.salle,
                    heure_debut = EXCLUDED.heure_debut, heure_fin = EXCLUDED.heure_fin,
                    grille_code = EXCLUDED.grille_code, demi_journee = EXCLUDED.demi_journee,
                    statut = EXCLUDED.statut, remplacant_user_id = EXCLUDED.remplacant_user_id,
                    retiree_le = NULL, retiree_motif = NULL,
                    updated_at = now()
                  WHERE ent_edt_seance.source = 'genere'
                    AND (ent_edt_seance.classe_id, ent_edt_seance.service_id, ent_edt_seance.prof_user_id,
                         ent_edt_seance.matiere_libelle, ent_edt_seance.salle, ent_edt_seance.heure_debut,
                         ent_edt_seance.heure_fin, ent_edt_seance.grille_code, ent_edt_seance.demi_journee,
                         ent_edt_seance.statut, ent_edt_seance.remplacant_user_id, ent_edt_seance.annee_id)
                        IS DISTINCT FROM
                        (EXCLUDED.classe_id, EXCLUDED.service_id, EXCLUDED.prof_user_id,
                         EXCLUDED.matiere_libelle, EXCLUDED.salle, EXCLUDED.heure_debut,
                         EXCLUDED.heure_fin, EXCLUDED.grille_code, EXCLUDED.demi_journee,
                         EXCLUDED.statut, EXCLUDED.remplacant_user_id, EXCLUDED.annee_id)
                 RETURNING (xmax = 0)",
            )
            .bind(Uuid::new_v4())
            .bind(e)
            .bind(annee_id)
            .bind(p.creneau_id)
            .bind(p.jour)
            .bind(p.classe_id)
            .bind(&p.classe)
            .bind(p.service_id)
            .bind(p.prof_user_id)
            .bind(p.matiere_id)
            .bind(&p.matiere_libelle)
            .bind(&p.salle)
            .bind(p.heure_debut)
            .bind(p.heure_fin)
            .bind(&p.grille_code)
            .bind(&p.demi_journee)
            .bind(&p.statut)
            .bind(p.remplacant_user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match res {
                Some((true,)) => rapport.creees += 1,
                Some((false,)) => rapport.mises_a_jour += 1,
                None => {}
            }
        }
        // Séances générées qui ne sont plus attendues (vacances ajoutées, créneau retiré…).
        // D1 : une séance qui porte un contenu de cahier ou un travail n'est JAMAIS supprimée : elle
        // est retirée (état `retiree`, ce qui était attaché reste lisible et rattachable). Les autres,
        // vides, sont supprimées comme avant.
        let existantes: Vec<(Uuid, Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT id, creneau_id, jour FROM ent_edt_seance
              WHERE etablissement_id = $1 AND annee_id = $2 AND source = 'genere'
                AND statut <> 'retiree' AND jour BETWEEN $3 AND $4",
        )
        .bind(e)
        .bind(annee_id)
        .bind(du)
        .bind(au)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let a_retirer: Vec<Uuid> = existantes
            .into_iter()
            .filter(|(_, c, j)| !attendues.contains(&(*c, *j)))
            .map(|(id, _, _)| id)
            .collect();
        if !a_retirer.is_empty() {
            let (retirees, supprimees) =
                retirer_ou_supprimer_seances(&mut tx, e, &a_retirer, "plus attendue par le plan")
                    .await?;
            rapport.retirees = retirees;
            rapport.supprimees = supprimees;
        }
        if let Some(reprise) = reprise {
            rapport.rattachees =
                Self::rattacher_seances(&mut tx, e, annee_id, du, au, reprise.version_id).await?;
        }
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            "edt.seances_generees",
            annee_id,
            json!({ "du": du, "au": au, "creees": rapport.creees,
                    "mises_a_jour": rapport.mises_a_jour, "supprimees": rapport.supprimees,
                    "retirees": rapport.retirees }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(rapport)
    }

    /// Séances d'une plage (≤ 62 jours), filtrables par classe ou professeur.
    pub async fn ref_seances(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
        classe_id: Option<Uuid>,
        prof_user_id: Option<Uuid>,
    ) -> Result<Vec<Seance>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        if au < du || (au - du).num_days() > 62 {
            return Err(invalide());
        }
        sqlx::query_as::<_, Seance>(
            "SELECT id, creneau_id, jour, classe_id, classe, service_id, prof_user_id,
                    matiere_libelle, salle, heure_debut, heure_fin, grille_code, demi_journee,
                    statut, remplacant_user_id
               FROM ent_edt_seance
              WHERE etablissement_id = $1 AND jour BETWEEN $2 AND $3 AND statut <> 'retiree'
                AND ($4::uuid IS NULL OR classe_id = $4)
                AND ($5::uuid IS NULL OR prof_user_id = $5 OR remplacant_user_id = $5)
              ORDER BY jour, heure_debut, classe",
        )
        .bind(a.etablissement_id)
        .bind(du)
        .bind(au)
        .bind(classe_id)
        .bind(prof_user_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
        .await
        .map_err(db)
    }

    /// EDT-03 : lie un créneau à un service d'enseignement (ou le délie avec `None`). Le
    /// service doit être du même établissement, du même professeur et de la même classe.
    pub async fn ref_lier_creneau_service(
        &self,
        a: &Acteur<'_>,
        creneau_id: Uuid,
        service_id: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let creneau: Option<(Uuid, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT prof_user_id, classe_id, btrim(classe) FROM ent_edt_creneau
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(e)
        .bind(creneau_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((prof, _classe_id, code)) = creneau else {
            return Err(introuvable());
        };
        if let Some(sid) = service_id {
            let ok: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_service_enseignement s
                   LEFT JOIN ent_classe c ON c.id = s.classe_id
                   LEFT JOIN ent_groupe g ON g.id = s.groupe_id
                   LEFT JOIN ent_classe cg ON cg.id = g.classe_id
                  WHERE s.etablissement_id = $1 AND s.id = $2 AND s.prof_user_id = $3
                    AND coalesce(c.code, cg.code) = $4",
            )
            .bind(e)
            .bind(sid)
            .bind(prof)
            .bind(&code)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if ok == 0 {
                // Service inconnu, d'un autre établissement, d'un autre professeur ou d'une
                // autre classe : même réponse, rien n'est révélé.
                return Err(introuvable());
            }
        }
        sqlx::query(
            "UPDATE ent_edt_creneau SET service_id = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(creneau_id)
        .bind(service_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            "edt.creneau_modifie",
            creneau_id,
            json!({ "service_id": service_id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn h(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn creneau(jour_semaine: i16, ab: &str, debut: NaiveTime) -> CreneauSource {
        CreneauSource {
            id: Uuid::new_v4(),
            classe: "4B".into(),
            classe_id: Uuid::new_v4(),
            site_id: None,
            prof_user_id: Uuid::new_v4(),
            matiere_id: None,
            matiere_libelle: "Maths".into(),
            jour_semaine,
            heure_debut: debut,
            heure_fin: debut + Duration::hours(1),
            salle: Some("B12".into()),
            semaine_ab: ab.into(),
            service_id: None,
        }
    }

    #[test]
    fn un_creneau_hebdo_donne_une_seance_par_semaine() {
        let c = creneau(1, "toutes", h(8, 0)); // lundi
        let (p, _, _) = planifier_seances(
            &[c],
            d(2026, 11, 2),
            d(2026, 11, 22),
            &BTreeMap::new(),
            None,
            &[],
            &[],
        );
        let jours: Vec<_> = p.iter().map(|s| s.jour).collect();
        assert_eq!(jours, vec![d(2026, 11, 2), d(2026, 11, 9), d(2026, 11, 16)]);
        assert_eq!(p[0].demi_journee, "matin");
    }

    #[test]
    fn les_jours_neutralises_n_ont_pas_de_seance() {
        let c = creneau(1, "toutes", h(14, 0));
        let mut n: BTreeMap<NaiveDate, Vec<Option<Uuid>>> = BTreeMap::new();
        n.insert(d(2026, 11, 9), vec![None]);
        let (p, neutres, _) =
            planifier_seances(&[c], d(2026, 11, 2), d(2026, 11, 16), &n, None, &[], &[]);
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|s| s.jour != d(2026, 11, 9)));
        assert_eq!(neutres, 1);
        assert_eq!(p[0].demi_journee, "apres_midi");
    }

    #[test]
    fn fermeture_d_un_site_ne_touche_que_ses_classes() {
        let site = Uuid::new_v4();
        let mut a = creneau(2, "toutes", h(8, 0));
        a.site_id = Some(site);
        let b = creneau(2, "toutes", h(8, 0));
        let mut n: BTreeMap<NaiveDate, Vec<Option<Uuid>>> = BTreeMap::new();
        n.insert(d(2026, 11, 3), vec![Some(site)]);
        let (p, _, _) = planifier_seances(
            &[a.clone(), b.clone()],
            d(2026, 11, 3),
            d(2026, 11, 3),
            &n,
            None,
            &[],
            &[],
        );
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].creneau_id, b.id);
    }

    #[test]
    fn alternance_ab_et_absence_de_calendrier() {
        let a = creneau(1, "a", h(8, 0));
        let b = creneau(1, "b", h(8, 0));
        let reference = d(2026, 11, 2); // lundi = semaine A
        let (p, _, ignores) = planifier_seances(
            &[a.clone(), b.clone()],
            d(2026, 11, 2),
            d(2026, 11, 9),
            &BTreeMap::new(),
            Some(reference),
            &[],
            &[],
        );
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].creneau_id, p[0].jour), (a.id, d(2026, 11, 2)));
        assert_eq!((p[1].creneau_id, p[1].jour), (b.id, d(2026, 11, 9)));
        assert_eq!(ignores, 0);
        // Sans calendrier A/B : rien n'est inventé, les créneaux ignorés sont comptés.
        let (p, _, ignores) = planifier_seances(
            &[a, b],
            d(2026, 11, 2),
            d(2026, 11, 9),
            &BTreeMap::new(),
            None,
            &[],
            &[],
        );
        assert!(p.is_empty());
        assert_eq!(ignores, 2);
    }

    #[test]
    fn exceptions_et_grille() {
        let c = creneau(3, "toutes", h(10, 15));
        let cid = c.id;
        let rempl = Uuid::new_v4();
        let ex = vec![
            ExceptionSource {
                creneau_id: cid,
                jour: d(2026, 11, 4),
                kind: "annulation".into(),
                remplacant_user_id: None,
                nouvelle_salle: None,
            },
            ExceptionSource {
                creneau_id: cid,
                jour: d(2026, 11, 11),
                kind: "remplacement".into(),
                remplacant_user_id: Some(rempl),
                nouvelle_salle: Some("C3".into()),
            },
        ];
        let grille = vec![CreneauGrille {
            id: Uuid::new_v4(),
            site_id: None,
            code: "M3".into(),
            libelle: "3e heure".into(),
            demi_journee: "matin".into(),
            ordre: 3,
            heure_debut: h(10, 15),
            heure_fin: h(11, 15),
        }];
        let (p, _, _) = planifier_seances(
            &[c],
            d(2026, 11, 4),
            d(2026, 11, 18),
            &BTreeMap::new(),
            None,
            &ex,
            &grille,
        );
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].statut, "annulee");
        assert_eq!(p[1].statut, "remplacee");
        assert_eq!(p[1].remplacant_user_id, Some(rempl));
        assert_eq!(p[1].salle.as_deref(), Some("C3"));
        assert_eq!(p[2].statut, "prevue");
        assert!(p.iter().all(|s| s.grille_code.as_deref() == Some("M3")));
    }
}
