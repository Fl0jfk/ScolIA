//! Socle 10, lot T5a — **matérialisation des séances depuis les versions et les placements**
//! (`docs/socles/10_EMPLOI_DU_TEMPS.md` §8.1, §22 M3).
//!
//! Avant T5a, les séances venaient des créneaux historiques (`ent_edt_creneau`). Elles viennent désormais des
//! **placements** de la version en vigueur à chaque date :
//!
//! * `version(d)` = la version de **base** publiée dont la plage contient `d` ; une version **temporaire** publiée
//!   (§8.4) remplace, pour les cours de son périmètre, le plan de base sur sa plage, puis le plan de base revient seul ;
//! * la version « reprise » (M2) garde ses créneaux historiques comme source exacte (aucune régression des
//!   établissements qui n'ont jamais publié de version) ;
//! * une version publiée par T5 donne ses séances depuis ses placements : heures de la grille à la date, lieux du
//!   placement, intervenant du service du cours (socle 6) ;
//! * **identité stable** : un placement identique à celui d'une autre version (même cours, jour, plage, semaine type,
//!   lieux) réutilise le **même créneau** — la séance du 12 novembre reste la même (même identifiant : appel, cahier,
//!   travail à faire) quand on republie le 3 novembre ;
//! * **pont** : `ent_edt_seance.creneau_id` reste `NOT NULL` jusqu'au lot T11 ; un placement sans créneau historique
//!   reçoit donc un créneau **projeté** (`projete = true`, ignoré par la reprise). Les journaux du réel (T3) sont
//!   rattachés à la séance, qui ne change pas : les modifications ponctuelles sont conservées.
//! * les créneaux historiques **sans cours** (« listés, jamais devinés », M1) continuent de produire leurs séances
//!   comme avant ; les cours **vacants** (aucun intervenant) ne sont pas projetés : leurs séances n'existent pas
//!   encore (l'état `non_assuree` est posé par les réactions aux événements du socle 6, non faites) — ils sont comptés
//!   dans `RapportSeances::non_projetes` et listés par le contrôle de publication.

use std::collections::{HashMap, HashSet};

use chrono::{NaiveDate, NaiveTime};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::edt_versions::fenetre_en_grille;
use crate::ref_commun::db;
use crate::referentiel_seances::CreneauSource;
use crate::referentiel_structure::CreneauGrille;
use crate::service::EntDbError;

/// Un jeu de créneaux valable sur une plage (une version, ou les créneaux sans cours).
#[derive(Debug, Clone)]
pub(crate) struct Tranche {
    pub version_id: Option<Uuid>,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub creneaux: Vec<CreneauSource>,
    /// Version temporaire (les séances qu'elle donne l'emportent sur la base pour ses cours).
    pub temporaire: bool,
}

/// Cours du périmètre d'une version temporaire, sur sa plage : le plan de base ne s'y applique pas.
#[derive(Debug, Clone)]
pub(crate) struct Exclusion {
    pub cours_id: Uuid,
    pub du: NaiveDate,
    pub au: NaiveDate,
}

/// Raison pour laquelle un placement n'a pas (encore) de séance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonProjete {
    pub placement_id: Uuid,
    pub cours_id: Uuid,
    /// `sans_intervenant` (cours vacant), `sans_classe`, `creneau_inconnu`.
    pub raison: &'static str,
}

#[derive(Debug, Default)]
pub(crate) struct SourcesPlan {
    pub tranches: Vec<Tranche>,
    pub exclusions: Vec<Exclusion>,
    pub cours_du_creneau: HashMap<Uuid, Uuid>,
    pub non_projetes: Vec<NonProjete>,
}

/// Intersection de deux plages de dates (incluses).
pub(crate) fn intersection(
    a: (NaiveDate, NaiveDate),
    b: (NaiveDate, NaiveDate),
) -> Option<(NaiveDate, NaiveDate)> {
    let du = a.0.max(b.0);
    let au = a.1.min(b.1);
    (du <= au).then_some((du, au))
}

/// Semaine type d'un placement vers la colonne historique `semaine_ab` : `toutes`, `a` ou `b` (une autre étiquette
/// n'a pas d'équivalent historique : `toutes`).
pub(crate) fn semaine_ab_historique(semaine_type: &str) -> &'static str {
    match semaine_type.trim().to_ascii_lowercase().as_str() {
        "a" => "a",
        "b" => "b",
        _ => "toutes",
    }
}

/// Plans à garder : ceux d'un cours de la plage d'une version temporaire ne viennent que de cette temporaire.
pub(crate) fn exclu_par_temporaire(
    cours_du_creneau: &HashMap<Uuid, Uuid>,
    exclusions: &[Exclusion],
    creneau_id: Uuid,
    jour: NaiveDate,
) -> bool {
    cours_du_creneau.get(&creneau_id).is_some_and(|c| {
        exclusions
            .iter()
            .any(|x| x.cours_id == *c && x.du <= jour && jour <= x.au)
    })
}

#[derive(Debug, FromRow)]
struct LignePlacement {
    placement_id: Uuid,
    creneau_id: Option<Uuid>,
    cours_id: Uuid,
    jour_semaine: i16,
    creneau_debut_code: Option<String>,
    duree_creneaux: Option<i16>,
    heure_debut_libre: Option<NaiveTime>,
    heure_fin_libre: Option<NaiveTime>,
    semaine_type: String,
    libelle: String,
    matiere_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
    salle: Option<String>,
    cl_id: Option<Uuid>,
    cl_code: Option<String>,
    site_id: Option<Uuid>,
    service_id: Option<Uuid>,
    prof_user_id: Option<Uuid>,
}

/// Placements d'une version avec ce qu'il faut pour en faire un créneau : cours, classe (ou classe d'origine du
/// groupe), site, premier lieu, service ouvert avec intervenant.
const SELECT_PLACEMENTS: &str = "
SELECT p.id AS placement_id, p.creneau_id, p.cours_id, p.jour_semaine, p.creneau_debut_code, p.duree_creneaux,
       p.heure_debut_libre, p.heure_fin_libre, p.semaine_type,
       c.libelle, c.matiere_id, c.groupe_id,
       (SELECT sa.code FROM ent_edt_placement_lieu pl
          JOIN ent_salle sa ON sa.id = pl.lieu_id AND sa.etablissement_id = pl.etablissement_id
         WHERE pl.placement_id = p.id AND pl.etablissement_id = p.etablissement_id
         ORDER BY pl.principal DESC, sa.code LIMIT 1) AS salle,
       cl.id AS cl_id, cl.code AS cl_code, cl.site_id,
       sv.id AS service_id, sv.prof_user_id
  FROM ent_edt_placement p
  JOIN ent_cours c ON c.id = p.cours_id AND c.etablissement_id = p.etablissement_id
  LEFT JOIN ent_classe cl ON cl.etablissement_id = c.etablissement_id AND cl.id = COALESCE(
         c.classe_id,
         (SELECT g.classe_id FROM ent_groupe g WHERE g.id = c.groupe_id AND g.etablissement_id = c.etablissement_id),
         (SELECT gc.classe_id FROM ent_groupe_classe gc
           WHERE gc.groupe_id = c.groupe_id AND gc.etablissement_id = c.etablissement_id
           ORDER BY gc.classe_id LIMIT 1))
  LEFT JOIN LATERAL (
         SELECT s.id, s.prof_user_id FROM ent_service_enseignement s
          WHERE s.etablissement_id = c.etablissement_id AND s.cours_id = c.id
            AND s.etat = 'ouvert' AND s.prof_user_id IS NOT NULL
          ORDER BY s.created_at, s.id LIMIT 1) sv ON true
 WHERE p.etablissement_id = $1 AND p.version_id = $2";

/// Heures d'un placement : la grille du site (à défaut, de l'établissement) ou les heures libres.
fn heures_du_placement(
    grille: &[CreneauGrille],
    site: Option<Uuid>,
    l: &LignePlacement,
) -> Option<(NaiveTime, NaiveTime)> {
    match (&l.creneau_debut_code, l.duree_creneaux) {
        (Some(code), Some(d)) => {
            let f = fenetre_en_grille(grille, site, code, d)
                .or_else(|| fenetre_en_grille(grille, None, code, d))?;
            Some((f.debut, f.fin))
        }
        _ => Some((l.heure_debut_libre?, l.heure_fin_libre?)),
    }
}

/// Donne un créneau projeté à chaque placement de la version qui n'en a pas : **réutilise** celui d'un placement
/// identique d'une autre version (identité stable), sinon en crée un. Rejouable. Retourne les placements non projetés.
pub(crate) async fn assurer_creneaux_projetes(
    tx: &mut PgConnection,
    etab: Uuid,
    version_id: Uuid,
    grille: &[CreneauGrille],
) -> Result<Vec<NonProjete>, EntDbError> {
    let lignes: Vec<LignePlacement> = sqlx::query_as(&format!(
        "{SELECT_PLACEMENTS} AND p.creneau_id IS NULL ORDER BY p.cree_le, p.id"
    ))
    .bind(etab)
    .bind(version_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let mut non = Vec::new();
    for l in lignes {
        // Identité stable : un placement identique d'une autre version (mêmes lieux compris) donne son créneau.
        let reutilisable: Option<Uuid> = sqlx::query_scalar(
            "SELECT q.creneau_id FROM ent_edt_placement q
              WHERE q.etablissement_id = $1 AND q.version_id <> $2 AND q.creneau_id IS NOT NULL
                AND q.cours_id = $3 AND q.jour_semaine = $4
                AND q.creneau_debut_code IS NOT DISTINCT FROM $5 AND q.duree_creneaux IS NOT DISTINCT FROM $6
                AND q.heure_debut_libre IS NOT DISTINCT FROM $7 AND q.heure_fin_libre IS NOT DISTINCT FROM $8
                AND q.semaine_type = $9
                AND (SELECT coalesce(array_agg(x.lieu_id ORDER BY x.lieu_id), '{}')
                       FROM ent_edt_placement_lieu x WHERE x.placement_id = q.id AND x.etablissement_id = q.etablissement_id)
                  = (SELECT coalesce(array_agg(y.lieu_id ORDER BY y.lieu_id), '{}')
                       FROM ent_edt_placement_lieu y WHERE y.placement_id = $10 AND y.etablissement_id = $1)
                AND NOT EXISTS (SELECT 1 FROM ent_edt_placement r
                                 WHERE r.etablissement_id = $1 AND r.version_id = $2 AND r.creneau_id = q.creneau_id)
              ORDER BY q.cree_le, q.id LIMIT 1",
        )
        .bind(etab)
        .bind(version_id)
        .bind(l.cours_id)
        .bind(l.jour_semaine)
        .bind(&l.creneau_debut_code)
        .bind(l.duree_creneaux)
        .bind(l.heure_debut_libre)
        .bind(l.heure_fin_libre)
        .bind(&l.semaine_type)
        .bind(l.placement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let creneau = match reutilisable {
            Some(c) => c,
            None => {
                let (Some(cl_id), Some(cl_code)) = (l.cl_id, l.cl_code.clone()) else {
                    non.push(NonProjete {
                        placement_id: l.placement_id,
                        cours_id: l.cours_id,
                        raison: "sans_classe",
                    });
                    continue;
                };
                let (Some(service_id), Some(prof)) = (l.service_id, l.prof_user_id) else {
                    non.push(NonProjete {
                        placement_id: l.placement_id,
                        cours_id: l.cours_id,
                        raison: "sans_intervenant",
                    });
                    continue;
                };
                let Some((debut, fin)) = heures_du_placement(grille, l.site_id, &l) else {
                    non.push(NonProjete {
                        placement_id: l.placement_id,
                        cours_id: l.cours_id,
                        raison: "creneau_inconnu",
                    });
                    continue;
                };
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_edt_creneau
                       (id, etablissement_id, classe, matiere_id, matiere_libelle, prof_user_id, jour_semaine,
                        heure_debut, heure_fin, salle, semaine_ab, classe_id, service_id, cours_id, groupe_id, projete)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,true)",
                )
                .bind(id)
                .bind(etab)
                .bind(cl_code)
                .bind(l.matiere_id)
                .bind(&l.libelle)
                .bind(prof)
                .bind(l.jour_semaine)
                .bind(debut)
                .bind(fin)
                .bind(&l.salle)
                .bind(semaine_ab_historique(&l.semaine_type))
                .bind(cl_id)
                .bind(service_id)
                .bind(l.cours_id)
                .bind(l.groupe_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                id
            }
        };
        sqlx::query(
            "UPDATE ent_edt_placement SET creneau_id = $3, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(l.placement_id)
        .bind(creneau)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    }
    Ok(non)
}

/// Créneaux d'une version **issue d'une reprise** : les créneaux historiques eux-mêmes (source exacte).
async fn creneaux_de_reprise(
    tx: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
    version_id: Uuid,
) -> Result<Vec<CreneauSource>, EntDbError> {
    sqlx::query_as(
        "SELECT c.id, btrim(c.classe) AS classe, cl.id AS classe_id, cl.site_id,
                c.prof_user_id, c.matiere_id, c.matiere_libelle, c.jour_semaine,
                c.heure_debut, c.heure_fin, c.salle, c.semaine_ab, s.id AS service_id
           FROM ent_edt_creneau c
           JOIN ent_classe cl
             ON cl.etablissement_id = c.etablissement_id AND cl.annee_id = $2
            AND cl.code = btrim(c.classe) AND cl.archivee_le IS NULL
           LEFT JOIN ent_service_enseignement s
             ON s.id = c.service_id AND s.etablissement_id = c.etablissement_id AND s.annee_id = $2
          WHERE c.etablissement_id = $1 AND NOT c.projete
            AND EXISTS (SELECT 1 FROM ent_edt_placement p
                         WHERE p.etablissement_id = c.etablissement_id AND p.version_id = $3 AND p.creneau_id = c.id)",
    )
    .bind(etab)
    .bind(annee_id)
    .bind(version_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)
}

/// Créneaux d'une version **publiée par T5** : tirés des placements (cours, grille, lieux, service du socle 6).
async fn creneaux_de_placements(
    tx: &mut PgConnection,
    etab: Uuid,
    version_id: Uuid,
    grille: &[CreneauGrille],
) -> Result<(Vec<CreneauSource>, Vec<(Uuid, Uuid)>), EntDbError> {
    let lignes: Vec<LignePlacement> =
        sqlx::query_as(&format!("{SELECT_PLACEMENTS} AND p.creneau_id IS NOT NULL"))
            .bind(etab)
            .bind(version_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
    let mut out = Vec::new();
    let mut cours = Vec::new();
    for l in lignes {
        let (Some(creneau), Some(cl_id), Some(cl_code), Some(prof)) =
            (l.creneau_id, l.cl_id, l.cl_code.clone(), l.prof_user_id)
        else {
            continue;
        };
        let Some((debut, fin)) = heures_du_placement(grille, l.site_id, &l) else {
            continue;
        };
        cours.push((creneau, l.cours_id));
        out.push(CreneauSource {
            id: creneau,
            classe: cl_code,
            classe_id: cl_id,
            site_id: l.site_id,
            prof_user_id: prof,
            matiere_id: l.matiere_id,
            matiere_libelle: l.libelle.clone(),
            jour_semaine: l.jour_semaine,
            heure_debut: debut,
            heure_fin: fin,
            salle: l.salle.clone(),
            semaine_ab: semaine_ab_historique(&l.semaine_type).to_string(),
            service_id: l.service_id,
        });
    }
    Ok((out, cours))
}

/// Cours du périmètre d'une version temporaire : cours visés, cours des services d'un agent visé, cours des
/// placements de base qui occupent un lieu visé.
async fn cours_du_perimetre(
    tx: &mut PgConnection,
    etab: Uuid,
    version_id: Uuid,
    base_id: Option<Uuid>,
) -> Result<HashSet<Uuid>, EntDbError> {
    let v: Vec<Uuid> = sqlx::query_scalar(
        "SELECT cours_id FROM ent_edt_version_perimetre WHERE etablissement_id = $1 AND version_id = $2 AND cours_id IS NOT NULL
         UNION
         SELECT s.cours_id FROM ent_edt_version_perimetre vp
           JOIN ent_service_intervenant i ON i.agent_id = vp.agent_id AND i.etablissement_id = vp.etablissement_id
                AND i.annule_le IS NULL
           JOIN ent_service_enseignement s ON s.id = i.service_id AND s.etablissement_id = i.etablissement_id
          WHERE vp.etablissement_id = $1 AND vp.version_id = $2 AND vp.agent_id IS NOT NULL
            AND s.etat = 'ouvert' AND s.cours_id IS NOT NULL
         UNION
         SELECT p.cours_id FROM ent_edt_version_perimetre vp
           JOIN ent_edt_placement_lieu pl ON pl.lieu_id = vp.lieu_id AND pl.etablissement_id = vp.etablissement_id
           JOIN ent_edt_placement p ON p.id = pl.placement_id AND p.etablissement_id = pl.etablissement_id
          WHERE vp.etablissement_id = $1 AND vp.version_id = $2 AND vp.lieu_id IS NOT NULL AND p.version_id = $3",
    )
    .bind(etab)
    .bind(version_id)
    .bind(base_id.unwrap_or_else(Uuid::nil))
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    Ok(v.into_iter().collect())
}

/// Tout ce qu'il faut pour planifier `[du, au]` : une tranche par version en vigueur, les créneaux sans cours, et les
/// exclusions du plan de base dues aux versions temporaires.
pub(crate) async fn sources_du_plan(
    tx: &mut PgConnection,
    etab: Uuid,
    annee_id: Uuid,
    annee_fin: NaiveDate,
    du: NaiveDate,
    au: NaiveDate,
    grille: &[CreneauGrille],
) -> Result<SourcesPlan, EntDbError> {
    let mut sources = SourcesPlan::default();
    let versions: Vec<(Uuid, String, String, Option<Uuid>, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT id, origine, nature, base_version_id, valable_du, coalesce(valable_au, $3)
           FROM ent_edt_version
          WHERE etablissement_id = $1 AND annee_id = $2 AND etat = 'publiee' AND valable_du IS NOT NULL
          ORDER BY nature, valable_du, id",
    )
    .bind(etab)
    .bind(annee_id)
    .bind(annee_fin)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    for (version_id, origine, nature, base_id, v_du, v_au) in versions {
        let Some((t_du, t_au)) = intersection((v_du, v_au), (du, au)) else {
            continue;
        };
        let temporaire = nature == "temporaire";
        let creneaux = if origine == "reprise" {
            let c = creneaux_de_reprise(tx, etab, annee_id, version_id).await?;
            let liens: Vec<(Uuid, Uuid)> = sqlx::query_as(
                "SELECT p.creneau_id, p.cours_id FROM ent_edt_placement p
                  WHERE p.etablissement_id = $1 AND p.version_id = $2 AND p.creneau_id IS NOT NULL",
            )
            .bind(etab)
            .bind(version_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            sources.cours_du_creneau.extend(liens);
            c
        } else {
            sources
                .non_projetes
                .extend(assurer_creneaux_projetes(tx, etab, version_id, grille).await?);
            let (c, liens) = creneaux_de_placements(tx, etab, version_id, grille).await?;
            sources.cours_du_creneau.extend(liens.iter().copied());
            if temporaire {
                let mut cours = cours_du_perimetre(tx, etab, version_id, base_id).await?;
                cours.extend(liens.iter().map(|(_, c)| *c));
                sources
                    .exclusions
                    .extend(cours.into_iter().map(|cours_id| Exclusion {
                        cours_id,
                        du: t_du,
                        au: t_au,
                    }));
            }
            c
        };
        sources.tranches.push(Tranche {
            version_id: Some(version_id),
            du: t_du,
            au: t_au,
            creneaux,
            temporaire,
        });
    }
    // Créneaux historiques sans cours : jamais devinés, mais ils produisent leurs séances comme avant (M1).
    let sans_cours: Vec<CreneauSource> = sqlx::query_as(
        "SELECT c.id, btrim(c.classe) AS classe, cl.id AS classe_id, cl.site_id,
                c.prof_user_id, c.matiere_id, c.matiere_libelle, c.jour_semaine,
                c.heure_debut, c.heure_fin, c.salle, c.semaine_ab, s.id AS service_id
           FROM ent_edt_creneau c
           JOIN ent_classe cl
             ON cl.etablissement_id = c.etablissement_id AND cl.annee_id = $2
            AND cl.code = btrim(c.classe) AND cl.archivee_le IS NULL
           LEFT JOIN ent_service_enseignement s
             ON s.id = c.service_id AND s.etablissement_id = c.etablissement_id AND s.annee_id = $2
          WHERE c.etablissement_id = $1 AND NOT c.projete
            AND NOT EXISTS (SELECT 1 FROM ent_edt_placement p
                             WHERE p.etablissement_id = c.etablissement_id AND p.creneau_id = c.id)
            AND (c.cours_id IS NULL
                 OR NOT EXISTS (SELECT 1 FROM ent_cours k
                                 WHERE k.etablissement_id = c.etablissement_id AND k.annee_id = $2 AND k.id = c.cours_id))",
    )
    .bind(etab)
    .bind(annee_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    if !sans_cours.is_empty() {
        sources.tranches.push(Tranche {
            version_id: None,
            du,
            au,
            creneaux: sans_cours,
            temporaire: false,
        });
    }
    Ok(sources)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, j).unwrap()
    }

    #[test]
    fn intersection_de_plages() {
        assert_eq!(
            intersection((d(9, 1), d(12, 31)), (d(11, 3), d(11, 30))),
            Some((d(11, 3), d(11, 30)))
        );
        assert_eq!(
            intersection((d(9, 1), d(10, 31)), (d(11, 3), d(11, 30))),
            None
        );
        // Un seul jour commun.
        assert_eq!(
            intersection((d(9, 1), d(11, 3)), (d(11, 3), d(11, 30))),
            Some((d(11, 3), d(11, 3)))
        );
    }

    #[test]
    fn semaine_type_vers_semaine_historique() {
        assert_eq!(semaine_ab_historique("toutes"), "toutes");
        assert_eq!(semaine_ab_historique("A"), "a");
        assert_eq!(semaine_ab_historique(" b "), "b");
        // Une étiquette sans équivalent historique n'invente pas d'alternance.
        assert_eq!(semaine_ab_historique("C"), "toutes");
    }

    #[test]
    fn la_temporaire_exclut_le_plan_de_base_sur_sa_plage_seulement() {
        let (cr, cours) = (Uuid::new_v4(), Uuid::new_v4());
        let liens: HashMap<Uuid, Uuid> = [(cr, cours)].into_iter().collect();
        let ex = [Exclusion {
            cours_id: cours,
            du: d(11, 10),
            au: d(11, 20),
        }];
        assert!(exclu_par_temporaire(&liens, &ex, cr, d(11, 10)));
        assert!(exclu_par_temporaire(&liens, &ex, cr, d(11, 20)));
        assert!(!exclu_par_temporaire(&liens, &ex, cr, d(11, 9)));
        assert!(!exclu_par_temporaire(&liens, &ex, cr, d(11, 21)));
        // Un créneau inconnu (sans cours) n'est jamais exclu.
        assert!(!exclu_par_temporaire(
            &liens,
            &ex,
            Uuid::new_v4(),
            d(11, 15)
        ));
    }
}
