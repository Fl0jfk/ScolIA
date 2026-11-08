//! Référentiel S2-2 : sites, niveaux / MEF, périodes, calendrier, grille horaire
//! (REF-04→09, 13, 14). Domaine propriétaire de `ent_site`, `ent_niveau`, `ent_mef`,
//! `ent_periode`, `ent_jour_calendrier`, `ent_grille_horaire` (migration 110).
//!
//! **Droits** : moteur D-017 (`referentiel.lire` / `referentiel.gerer`), jamais un test de
//! rôle en dur. **Cloisonnement** : toute requête filtre `etablissement_id`; une référence
//! d'un autre établissement est « introuvable ». **Événements** : chaque écriture publie son
//! fait dans la même transaction (outbox S2-0).
//!
//! **Nomenclatures nationales (D-019)** : les niveaux portent un code mnémonique stable
//! (6EME…TERM) et un cycle ; les MEF portent le code BCN à 11 caractères, le MEFSTAT11 et la
//! formation/diplôme BCN. `ref_importer_mefs` est le point d'entrée d'un import SIECLE / BCN
//! (idempotent, rejets détaillés) ; aucun fichier national n'est embarqué ici.

use std::collections::{BTreeMap, HashMap};

use chrono::{Duration, NaiveDate, NaiveTime};
use domain_eleve::DomainError;
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, publier_ref, Acteur};
use crate::service::{EntDbError, PgEnt};

// ───────────────────────────── Types ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct Site {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub adresse: Option<String>,
    pub uai: Option<String>,
    pub actif: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct Niveau {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub libelle_court: Option<String>,
    pub ordre: i16,
    pub cycle: Option<i16>,
    pub source: String,
    pub actif: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct Mef {
    pub id: Uuid,
    pub code_mef: String,
    pub mefstat11: Option<String>,
    pub niveau_id: Option<Uuid>,
    pub libelle_long: String,
    pub libelle_edition: Option<String>,
    pub code_formation_diplome: Option<String>,
    pub date_ouverture: Option<NaiveDate>,
    pub date_fermeture: Option<NaiveDate>,
    pub source: String,
    pub actif: bool,
}

/// Ligne d'import MEF (SIECLE / BCN). `niveau_code` : mnémonique de `ent_niveau`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MefImport {
    pub code_mef: String,
    #[serde(default)]
    pub mefstat11: Option<String>,
    #[serde(default)]
    pub niveau_code: Option<String>,
    pub libelle_long: String,
    #[serde(default)]
    pub libelle_edition: Option<String>,
    #[serde(default)]
    pub code_formation_diplome: Option<String>,
    #[serde(default)]
    pub date_ouverture: Option<NaiveDate>,
    #[serde(default)]
    pub date_fermeture: Option<NaiveDate>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RapportImportMef {
    pub crees: i64,
    pub mis_a_jour: i64,
    /// (code MEF, raison) des lignes rejetées ; le reste de l'import est appliqué.
    pub rejets: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct Periode {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub niveau_id: Option<Uuid>,
    pub code: String,
    pub libelle: String,
    pub kind: String,
    pub ordre: i16,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodeSaisie {
    pub code: String,
    pub libelle: String,
    /// `trimestre` | `semestre` | `periode`.
    pub kind: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
}

/// Jour neutralisé (aucune séance) : vacances / férié / fermeture / journée banalisée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JourCalendrier {
    pub jour: NaiveDate,
    pub kind: String,
    pub libelle: String,
    pub site_id: Option<Uuid>,
    /// `jour` (ent_jour_calendrier) ou `vacances` (ent_periode_vacances).
    pub origine: String,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct CreneauGrille {
    pub id: Uuid,
    pub site_id: Option<Uuid>,
    pub code: String,
    pub libelle: String,
    pub demi_journee: String,
    pub ordre: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreneauGrilleSaisie {
    pub code: String,
    pub libelle: String,
    pub demi_journee: String,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
}

/// Niveaux fournis par la nomenclature nationale : (code, libellé, court, ordre, cycle).
pub const NIVEAUX_NATIONAUX: [(&str, &str, &str, i16, Option<i16>); 7] = [
    ("6EME", "Sixième", "6e", 1, Some(3)),
    ("5EME", "Cinquième", "5e", 2, Some(4)),
    ("4EME", "Quatrième", "4e", 3, Some(4)),
    ("3EME", "Troisième", "3e", 4, Some(4)),
    ("2NDE", "Seconde", "2nde", 5, None),
    ("1ERE", "Première", "1re", 6, None),
    ("TERM", "Terminale", "Tle", 7, None),
];

/// Code de niveau national correspondant au texte `niveau` historique (« 4e » → « 4EME »).
pub fn code_niveau_de_texte(niveau: &str) -> Option<&'static str> {
    NIVEAUX_NATIONAUX
        .iter()
        .find(|(_, _, court, _, _)| court.eq_ignore_ascii_case(niveau.trim()))
        .map(|(code, ..)| *code)
}

/// Grille standard (modifiable) : M1…M4 le matin, S1…S4 l'après-midi.
pub fn grille_standard() -> Vec<CreneauGrilleSaisie> {
    let h = |h: u32, m: u32| NaiveTime::from_hms_opt(h, m, 0).expect("heure valide");
    let c = |code: &str, lib: &str, demi: &str, d: NaiveTime, f: NaiveTime| CreneauGrilleSaisie {
        code: code.into(),
        libelle: lib.into(),
        demi_journee: demi.into(),
        heure_debut: d,
        heure_fin: f,
    };
    vec![
        c("M1", "1re heure du matin", "matin", h(8, 0), h(9, 0)),
        c("M2", "2e heure du matin", "matin", h(9, 0), h(10, 0)),
        c("M3", "3e heure du matin", "matin", h(10, 15), h(11, 15)),
        c("M4", "4e heure du matin", "matin", h(11, 15), h(12, 15)),
        c("S1", "1re heure de l'après-midi", "apres_midi", h(13, 30), h(14, 30)),
        c("S2", "2e heure de l'après-midi", "apres_midi", h(14, 30), h(15, 30)),
        c("S3", "3e heure de l'après-midi", "apres_midi", h(15, 45), h(16, 45)),
        c("S4", "4e heure de l'après-midi", "apres_midi", h(16, 45), h(17, 45)),
    ]
}

/// Vérifie qu'un jeu de périodes couvre `[debut, fin]` sans trou ni chevauchement
/// (REF-04). Pure, testable sans base.
pub fn verifier_couverture(
    periodes: &[PeriodeSaisie],
    debut: NaiveDate,
    fin: NaiveDate,
) -> Result<(), &'static str> {
    if periodes.is_empty() {
        return Err("aucune période");
    }
    let mut tri: Vec<&PeriodeSaisie> = periodes.iter().collect();
    tri.sort_by_key(|p| p.date_debut);
    if tri[0].date_debut != debut {
        return Err("la première période doit commencer au début de l'année");
    }
    for p in &tri {
        if p.date_fin < p.date_debut {
            return Err("période dont la fin précède le début");
        }
        if !matches!(p.kind.as_str(), "trimestre" | "semestre" | "periode") {
            return Err("type de période inconnu");
        }
    }
    for w in tri.windows(2) {
        if w[1].date_debut <= w[0].date_fin {
            return Err("périodes qui se chevauchent");
        }
        if w[1].date_debut != w[0].date_fin + Duration::days(1) {
            return Err("trou entre deux périodes");
        }
    }
    if tri[tri.len() - 1].date_fin != fin {
        return Err("la dernière période doit finir à la fin de l'année");
    }
    let mut codes = std::collections::HashSet::new();
    if !tri.iter().all(|p| codes.insert(p.code.trim().to_string())) {
        return Err("code de période en double");
    }
    Ok(())
}

// ───────────────────────────── Accès ─────────────────────────────

const SANS_SITE: Uuid = Uuid::nil();

impl PgEnt {
    // ───── Sites (REF-14) ─────

    pub async fn ref_sites(&self, a: &Acteur<'_>) -> Result<Vec<Site>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        sqlx::query_as::<_, Site>(
            "SELECT id, code, libelle, adresse, uai, actif FROM ent_site
              WHERE etablissement_id = $1 ORDER BY code",
        )
        .bind(a.etablissement_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    pub async fn ref_creer_site(
        &self,
        a: &Acteur<'_>,
        code: &str,
        libelle: &str,
        adresse: Option<&str>,
        uai: Option<&str>,
    ) -> Result<Site, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let (code, libelle) = (code.trim(), libelle.trim());
        if code.is_empty() || code.chars().count() > 40 || libelle.is_empty() || libelle.chars().count() > 120 {
            return Err(invalide());
        }
        // UAI au format national (socle 5, S2) : la contrainte de base le refuse aussi.
        let uai = uai.map(str::trim).filter(|v| !v.is_empty()).map(str::to_ascii_uppercase);
        if uai.as_deref().is_some_and(|u| !crate::structure_etab::uai_valide(u)) {
            return Err(invalide());
        }
        let uai = uai.as_deref();
        let mut tx = self.pool().begin().await.map_err(db)?;
        let s = sqlx::query_as::<_, Site>(
            "INSERT INTO ent_site (id, etablissement_id, code, libelle, adresse, uai)
             VALUES ($1,$2,$3,$4,$5,$6)
             RETURNING id, code, libelle, adresse, uai, actif",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(code)
        .bind(libelle)
        .bind(adresse.map(str::trim).filter(|v| !v.is_empty()))
        .bind(uai.map(str::trim).filter(|v| !v.is_empty()))
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.site_cree",
            s.id,
            json!({ "code": s.code }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(s)
    }

    pub async fn ref_maj_site(
        &self,
        a: &Acteur<'_>,
        site_id: Uuid,
        libelle: Option<&str>,
        adresse: Option<&str>,
        actif: Option<bool>,
    ) -> Result<Site, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let s = sqlx::query_as::<_, Site>(
            "UPDATE ent_site SET
                libelle = coalesce(nullif(btrim($3), ''), libelle),
                adresse = coalesce($4, adresse),
                actif = coalesce($5, actif)
              WHERE etablissement_id = $1 AND id = $2
              RETURNING id, code, libelle, adresse, uai, actif",
        )
        .bind(a.etablissement_id)
        .bind(site_id)
        .bind(libelle)
        .bind(adresse)
        .bind(actif)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.site_modifie",
            s.id,
            json!({ "code": s.code, "actif": s.actif }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(s)
    }

    // ───── Niveaux / MEF (REF-07) ─────

    pub async fn ref_niveaux(&self, a: &Acteur<'_>) -> Result<Vec<Niveau>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        sqlx::query_as::<_, Niveau>(
            "SELECT id, code, libelle, libelle_court, ordre, cycle, source, actif FROM ent_niveau
              WHERE etablissement_id = $1 ORDER BY ordre, code",
        )
        .bind(a.etablissement_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Installe la nomenclature nationale des niveaux (idempotent) et rattache les classes
    /// dont le texte `niveau` correspond (`niveau_id` encore vide). Renvoie (niveaux créés,
    /// classes rattachées).
    pub async fn ref_amorcer_niveaux(&self, a: &Acteur<'_>) -> Result<(i64, i64), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let mut crees = 0;
        for (code, libelle, court, ordre, cycle) in NIVEAUX_NATIONAUX {
            let r = sqlx::query(
                "INSERT INTO ent_niveau
                   (id, etablissement_id, code, libelle, libelle_court, ordre, cycle, source)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'nomenclature') ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(code)
            .bind(libelle)
            .bind(court)
            .bind(ordre)
            .bind(cycle)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            crees += r.rows_affected() as i64;
        }
        let mut lies = 0;
        for (code, _, court, ..) in NIVEAUX_NATIONAUX {
            let r = sqlx::query(
                "UPDATE ent_classe c SET niveau_id = n.id
                   FROM ent_niveau n
                  WHERE c.etablissement_id = $1 AND n.etablissement_id = $1 AND n.code = $2
                    AND c.niveau_id IS NULL AND lower(c.niveau) = lower($3)",
            )
            .bind(a.etablissement_id)
            .bind(code)
            .bind(court)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            lies += r.rows_affected() as i64;
        }
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.niveaux_amorces",
            a.etablissement_id,
            json!({ "niveaux_crees": crees, "classes_rattachees": lies }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok((crees, lies))
    }

    pub async fn ref_creer_niveau(
        &self,
        a: &Acteur<'_>,
        code: &str,
        libelle: &str,
        ordre: i16,
        cycle: Option<i16>,
    ) -> Result<Niveau, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let (code, libelle) = (code.trim().to_uppercase(), libelle.trim());
        if code.is_empty() || code.chars().count() > 20 || libelle.is_empty() {
            return Err(invalide());
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let n = sqlx::query_as::<_, Niveau>(
            "INSERT INTO ent_niveau (id, etablissement_id, code, libelle, ordre, cycle)
             VALUES ($1,$2,$3,$4,$5,$6)
             RETURNING id, code, libelle, libelle_court, ordre, cycle, source, actif",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(&code)
        .bind(libelle)
        .bind(ordre)
        .bind(cycle)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_ref(&mut tx, a, "referentiel.niveau_cree", n.id, json!({ "code": n.code })).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(n)
    }

    pub async fn ref_mefs(&self, a: &Acteur<'_>) -> Result<Vec<Mef>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        sqlx::query_as::<_, Mef>(
            "SELECT id, code_mef, mefstat11, niveau_id, libelle_long, libelle_edition,
                    code_formation_diplome, date_ouverture, date_fermeture, source, actif
               FROM ent_mef WHERE etablissement_id = $1 ORDER BY code_mef",
        )
        .bind(a.etablissement_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Import idempotent de MEF (SIECLE / BCN) : création ou mise à jour par code MEF ;
    /// les lignes invalides sont rejetées une à une, sans bloquer le reste.
    pub async fn ref_importer_mefs(
        &self,
        a: &Acteur<'_>,
        lignes: &[MefImport],
        source: &str,
    ) -> Result<RapportImportMef, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !matches!(source, "import_siecle" | "import_bcn" | "saisie") || lignes.len() > 5000 {
            return Err(invalide());
        }
        let niveaux: HashMap<String, Uuid> = sqlx::query_as::<_, (String, Uuid)>(
            "SELECT code, id FROM ent_niveau WHERE etablissement_id = $1",
        )
        .bind(a.etablissement_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let mut rapport = RapportImportMef::default();
        let mut tx = self.pool().begin().await.map_err(db)?;
        for l in lignes {
            let code = l.code_mef.trim().to_uppercase();
            let rejet = |raison: &str| (code.clone(), raison.to_string());
            if code.len() != 11 || !code.chars().all(|c| c.is_ascii_digit() || c.is_ascii_uppercase()) {
                rapport.rejets.push(rejet("code MEF invalide (11 caractères attendus)"));
                continue;
            }
            if l.libelle_long.trim().is_empty() {
                rapport.rejets.push(rejet("libellé vide"));
                continue;
            }
            if let Some(m) = &l.mefstat11 {
                if m.trim().len() != 11 {
                    rapport.rejets.push(rejet("MEFSTAT11 invalide"));
                    continue;
                }
            }
            if let (Some(o), Some(f)) = (l.date_ouverture, l.date_fermeture) {
                if f < o {
                    rapport.rejets.push(rejet("dates incohérentes"));
                    continue;
                }
            }
            let niveau_id = match l.niveau_code.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
                None => None,
                Some(c) => match niveaux.get(&c.to_uppercase()) {
                    Some(id) => Some(*id),
                    None => {
                        rapport.rejets.push(rejet("niveau inconnu"));
                        continue;
                    }
                },
            };
            let (insere,): (bool,) = sqlx::query_as(
                "INSERT INTO ent_mef
                   (id, etablissement_id, code_mef, mefstat11, niveau_id, libelle_long,
                    libelle_edition, code_formation_diplome, date_ouverture, date_fermeture, source)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                 ON CONFLICT (etablissement_id, code_mef) DO UPDATE SET
                    mefstat11 = EXCLUDED.mefstat11, niveau_id = EXCLUDED.niveau_id,
                    libelle_long = EXCLUDED.libelle_long, libelle_edition = EXCLUDED.libelle_edition,
                    code_formation_diplome = EXCLUDED.code_formation_diplome,
                    date_ouverture = EXCLUDED.date_ouverture, date_fermeture = EXCLUDED.date_fermeture,
                    source = EXCLUDED.source
                 RETURNING (xmax = 0)",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(&code)
            .bind(l.mefstat11.as_deref().map(str::trim))
            .bind(niveau_id)
            .bind(l.libelle_long.trim())
            .bind(l.libelle_edition.as_deref())
            .bind(l.code_formation_diplome.as_deref())
            .bind(l.date_ouverture)
            .bind(l.date_fermeture)
            .bind(source)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if insere {
                rapport.crees += 1;
            } else {
                rapport.mis_a_jour += 1;
            }
        }
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.mef_importes",
            a.etablissement_id,
            json!({ "crees": rapport.crees, "mis_a_jour": rapport.mis_a_jour,
                    "rejets": rapport.rejets.len(), "source": source }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(rapport)
    }

    // ───── Classes : rattachements (REF-08) ─────

    /// Rattache une classe à un site / niveau / MEF (références du même établissement),
    /// change son libellé ou l'archive. Archiver une classe qui a encore des élèves
    /// affectés est refusé (« pas de suppression si données »).
    #[allow(clippy::too_many_arguments)]
    pub async fn ref_modifier_classe(
        &self,
        a: &Acteur<'_>,
        classe_id: Uuid,
        libelle: Option<&str>,
        site_id: Option<Uuid>,
        niveau_id: Option<Uuid>,
        mef_id: Option<Uuid>,
        archiver: Option<bool>,
    ) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let annee: Option<(Uuid, bool)> = sqlx::query_as(
            "SELECT c.annee_id, y.cloturee_le IS NOT NULL FROM ent_classe c
               JOIN ent_annee_scolaire y ON y.id = c.annee_id
              WHERE c.etablissement_id = $1 AND c.id = $2 FOR UPDATE OF c",
        )
        .bind(e)
        .bind(classe_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((_, cloturee)) = annee else {
            return Err(introuvable());
        };
        if cloturee {
            return Err(conflit());
        }
        for (table, id) in [("ent_site", site_id), ("ent_niveau", niveau_id), ("ent_mef", mef_id)] {
            if let Some(id) = id {
                let n: i64 = sqlx::query_scalar(&format!(
                    "SELECT count(*) FROM {table} WHERE etablissement_id = $1 AND id = $2"
                ))
                .bind(e)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if n == 0 {
                    return Err(introuvable());
                }
            }
        }
        if archiver == Some(true) {
            let ouvertes: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_affectation
                  WHERE etablissement_id = $1 AND classe_id = $2 AND date_fin IS NULL",
            )
            .bind(e)
            .bind(classe_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if ouvertes > 0 {
                return Err(conflit());
            }
        }
        sqlx::query(
            "UPDATE ent_classe SET
                libelle = coalesce(nullif(btrim($3), ''), libelle),
                site_id = coalesce($4, site_id),
                niveau_id = coalesce($5, niveau_id),
                mef_id = coalesce($6, mef_id),
                archivee_le = CASE WHEN $7::boolean IS NULL THEN archivee_le
                                   WHEN $7 THEN coalesce(archivee_le, now()) ELSE NULL END
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(classe_id)
        .bind(libelle)
        .bind(site_id)
        .bind(niveau_id)
        .bind(mef_id)
        .bind(archiver)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let type_ = if archiver == Some(true) {
            "referentiel.classe_archivee"
        } else {
            "referentiel.classe_modifiee"
        };
        let ev = publier_ref(&mut tx, a, type_, classe_id, json!({})).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    /// REF-13 : désigne le professeur principal d'une classe (un seul par classe). Le
    /// professeur doit déjà avoir un service d'enseignement dans la classe.
    pub async fn ref_designer_pp(
        &self,
        a: &Acteur<'_>,
        classe_id: Uuid,
        prof_user_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let existe: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(classe_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if existe == 0 {
            return Err(introuvable());
        }
        let service: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_service_enseignement
              WHERE etablissement_id = $1 AND classe_id = $2 AND prof_user_id = $3
              ORDER BY created_at LIMIT 1",
        )
        .bind(e)
        .bind(classe_id)
        .bind(prof_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some(service) = service else {
            return Err(introuvable());
        };
        sqlx::query(
            "UPDATE ent_service_enseignement SET professeur_principal = false
              WHERE etablissement_id = $1 AND classe_id = $2 AND professeur_principal AND id <> $3",
        )
        .bind(e)
        .bind(classe_id)
        .bind(service)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_service_enseignement SET professeur_principal = true
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(e)
        .bind(service)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.pp_designe",
            classe_id,
            json!({ "prof_user_id": prof_user_id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    // ───── Périodes (REF-04) ─────

    pub async fn ref_periodes(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        niveau_id: Option<Uuid>,
    ) -> Result<Vec<Periode>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        sqlx::query_as::<_, Periode>(
            "SELECT id, annee_id, niveau_id, code, libelle, kind, ordre, date_debut, date_fin
               FROM ent_periode
              WHERE etablissement_id = $1 AND annee_id = $2 AND niveau_id IS NOT DISTINCT FROM $3
              ORDER BY date_debut",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .bind(niveau_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Remplace atomiquement les périodes d'une année (globales ou d'un niveau). Elles
    /// doivent couvrir l'année sans trou ni chevauchement. Une période déjà utilisée par la
    /// saisie des notes ne peut être retirée.
    pub async fn ref_definir_periodes(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
        niveau_id: Option<Uuid>,
        periodes: &[PeriodeSaisie],
    ) -> Result<Vec<Periode>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let annee: Option<(NaiveDate, NaiveDate, bool)> = sqlx::query_as(
            "SELECT date_debut, date_fin, cloturee_le IS NOT NULL FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(e)
        .bind(annee_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((debut, fin, cloturee)) = annee else {
            return Err(introuvable());
        };
        if cloturee {
            return Err(conflit());
        }
        if let Some(n) = niveau_id {
            let ok: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_niveau WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(e)
            .bind(n)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if ok == 0 {
                return Err(introuvable());
            }
        }
        if verifier_couverture(periodes, debut, fin).is_err() {
            return Err(invalide());
        }
        let codes: Vec<String> = periodes.iter().map(|p| p.code.trim().to_string()).collect();
        // Retraits : refusés si la saisie de notes s'appuie dessus.
        let utilisees: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_periode p
              WHERE p.etablissement_id = $1 AND p.annee_id = $2
                AND p.niveau_id IS NOT DISTINCT FROM $3 AND NOT (p.code = ANY($4))
                AND EXISTS (SELECT 1 FROM ent_note_periode np
                             WHERE np.etablissement_id = p.etablissement_id AND np.periode_id = p.id)",
        )
        .bind(e)
        .bind(annee_id)
        .bind(niveau_id)
        .bind(&codes)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if utilisees > 0 {
            return Err(conflit());
        }
        sqlx::query(
            "DELETE FROM ent_periode
              WHERE etablissement_id = $1 AND annee_id = $2
                AND niveau_id IS NOT DISTINCT FROM $3 AND NOT (code = ANY($4))",
        )
        .bind(e)
        .bind(annee_id)
        .bind(niveau_id)
        .bind(&codes)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut tri: Vec<&PeriodeSaisie> = periodes.iter().collect();
        tri.sort_by_key(|p| p.date_debut);
        for (i, p) in tri.iter().enumerate() {
            let existant: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM ent_periode
                  WHERE etablissement_id = $1 AND annee_id = $2
                    AND niveau_id IS NOT DISTINCT FROM $3 AND code = $4",
            )
            .bind(e)
            .bind(annee_id)
            .bind(niveau_id)
            .bind(p.code.trim())
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match existant {
                Some(id) => {
                    sqlx::query(
                        "UPDATE ent_periode SET libelle = $3, kind = $4, ordre = $5,
                                date_debut = $6, date_fin = $7
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(e)
                    .bind(id)
                    .bind(p.libelle.trim())
                    .bind(&p.kind)
                    .bind((i + 1) as i16)
                    .bind(p.date_debut)
                    .bind(p.date_fin)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                }
                None => {
                    sqlx::query(
                        "INSERT INTO ent_periode
                           (id, etablissement_id, annee_id, niveau_id, code, libelle, kind, ordre,
                            date_debut, date_fin)
                         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(e)
                    .bind(annee_id)
                    .bind(niveau_id)
                    .bind(p.code.trim())
                    .bind(p.libelle.trim())
                    .bind(&p.kind)
                    .bind((i + 1) as i16)
                    .bind(p.date_debut)
                    .bind(p.date_fin)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                }
            }
        }
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.periodes_definies",
            annee_id,
            json!({ "niveau_id": niveau_id, "periodes": codes }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        self.ref_periodes(a, annee_id, niveau_id).await
    }

    // ───── Calendrier (REF-05) ─────

    /// Jours neutralisés d'une plage : fermetures / banalisées (`ent_jour_calendrier`) et
    /// vacances / fériés historiques (`ent_periode_vacances`, géré par `vacances_repo`).
    pub async fn ref_calendrier(
        &self,
        a: &Acteur<'_>,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<JourCalendrier>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        self.calendrier_plage(a.etablissement_id, debut, fin).await
    }

    pub(crate) async fn calendrier_plage(
        &self,
        etablissement_id: Uuid,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Vec<JourCalendrier>, EntDbError> {
        if fin < debut || (fin - debut).num_days() > 800 {
            return Err(invalide());
        }
        let mut out: Vec<JourCalendrier> = sqlx::query_as::<_, (NaiveDate, String, String, Option<Uuid>)>(
            "SELECT jour, kind, libelle, site_id FROM ent_jour_calendrier
              WHERE etablissement_id = $1 AND jour BETWEEN $2 AND $3",
        )
        .bind(etablissement_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(self.pool())
        .await
        .map_err(db)?
        .into_iter()
        .map(|(jour, kind, libelle, site_id)| JourCalendrier {
            jour,
            kind,
            libelle,
            site_id,
            origine: "jour".into(),
        })
        .collect();
        let vac: Vec<(String, String, NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT libelle, kind, debut, fin FROM ent_periode_vacances
              WHERE etablissement_id = $1 AND debut <= $3 AND fin >= $2",
        )
        .bind(etablissement_id)
        .bind(debut)
        .bind(fin)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        for (libelle, kind, d, f) in vac {
            let kind = if kind == "ferie" { "ferie" } else { "vacances" }.to_string();
            let mut j = d.max(debut);
            while j <= f.min(fin) {
                out.push(JourCalendrier {
                    jour: j,
                    kind: kind.clone(),
                    libelle: libelle.clone(),
                    site_id: None,
                    origine: "vacances".into(),
                });
                j += Duration::days(1);
            }
        }
        out.sort_by(|x, y| (x.jour, &x.origine).cmp(&(y.jour, &y.origine)));
        Ok(out)
    }

    /// Ajoute un jour (ou une plage) neutralisé. Idempotent sur (jour, site).
    pub async fn ref_ajouter_jours(
        &self,
        a: &Acteur<'_>,
        du: NaiveDate,
        au: NaiveDate,
        kind: &str,
        libelle: &str,
        site_id: Option<Uuid>,
    ) -> Result<i64, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !matches!(kind, "ferme" | "banalise" | "ferie" | "vacances")
            || libelle.trim().is_empty()
            || au < du
            || (au - du).num_days() > 60
        {
            return Err(invalide());
        }
        let e = a.etablissement_id;
        let mut tx = self.pool().begin().await.map_err(db)?;
        if let Some(s) = site_id {
            let n: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_site WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(e)
            .bind(s)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if n == 0 {
                return Err(introuvable());
            }
        }
        let mut ajoutes = 0;
        let mut j = du;
        while j <= au {
            let r = sqlx::query(
                "INSERT INTO ent_jour_calendrier
                   (id, etablissement_id, jour, kind, libelle, site_id, created_by_user_id)
                 VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(e)
            .bind(j)
            .bind(kind)
            .bind(libelle.trim())
            .bind(site_id)
            .bind(a.user_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            ajoutes += r.rows_affected() as i64;
            j += Duration::days(1);
        }
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.jour_ferme",
            e,
            json!({ "du": du, "au": au, "kind": kind, "ajoutes": ajoutes, "site_id": site_id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ajoutes)
    }

    pub async fn ref_retirer_jour(
        &self,
        a: &Acteur<'_>,
        jour: NaiveDate,
        site_id: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let r = sqlx::query(
            "DELETE FROM ent_jour_calendrier
              WHERE etablissement_id = $1 AND jour = $2 AND site_id IS NOT DISTINCT FROM $3",
        )
        .bind(a.etablissement_id)
        .bind(jour)
        .bind(site_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(introuvable());
        }
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.jour_rouvert",
            a.etablissement_id,
            json!({ "jour": jour, "site_id": site_id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    /// Crée une période de vacances / un férié dans la table historique
    /// `ent_periode_vacances` (rattachée au référentiel : même droits, même événement).
    pub async fn ref_creer_vacances(
        &self,
        a: &Acteur<'_>,
        libelle: &str,
        kind: &str,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if libelle.trim().is_empty()
            || !matches!(kind, "vacances" | "ferie" | "fermeture")
            || fin < debut
            || (fin - debut).num_days() > 120
        {
            return Err(invalide());
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_periode_vacances
               (id, etablissement_id, libelle, kind, debut, fin, created_by_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(libelle.trim())
        .bind(kind)
        .bind(debut)
        .bind(fin)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.vacances_creees",
            id,
            json!({ "kind": kind, "debut": debut, "fin": fin }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(id)
    }

    // ───── Grille horaire (REF-06) ─────

    pub async fn ref_grille(
        &self,
        a: &Acteur<'_>,
        site_id: Option<Uuid>,
    ) -> Result<Vec<CreneauGrille>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        sqlx::query_as::<_, CreneauGrille>(
            "SELECT id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin
               FROM ent_grille_horaire
              WHERE etablissement_id = $1 AND site_id IS NOT DISTINCT FROM $2
              ORDER BY heure_debut",
        )
        .bind(a.etablissement_id)
        .bind(site_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Remplace la grille d'un site (ou de l'établissement). Les créneaux ne se chevauchent
    /// pas et chacun appartient à une demi-journée cohérente avec ses horaires.
    pub async fn ref_definir_grille(
        &self,
        a: &Acteur<'_>,
        site_id: Option<Uuid>,
        creneaux: &[CreneauGrilleSaisie],
    ) -> Result<Vec<CreneauGrille>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let e = a.etablissement_id;
        let mut tri: Vec<&CreneauGrilleSaisie> = creneaux.iter().collect();
        tri.sort_by_key(|c| c.heure_debut);
        let mut codes = std::collections::HashSet::new();
        for (i, c) in tri.iter().enumerate() {
            if c.code.trim().is_empty()
                || c.heure_fin <= c.heure_debut
                || !matches!(c.demi_journee.as_str(), "matin" | "apres_midi")
                || !codes.insert(c.code.trim().to_string())
                || (i > 0 && c.heure_debut < tri[i - 1].heure_fin)
            {
                return Err(invalide());
            }
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        if let Some(s) = site_id {
            let n: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM ent_site WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(e)
            .bind(s)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if n == 0 {
                return Err(introuvable());
            }
        }
        sqlx::query(
            "DELETE FROM ent_grille_horaire WHERE etablissement_id = $1 AND site_id IS NOT DISTINCT FROM $2",
        )
        .bind(e)
        .bind(site_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (i, c) in tri.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_grille_horaire
                   (id, etablissement_id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            )
            .bind(Uuid::new_v4())
            .bind(e)
            .bind(site_id)
            .bind(c.code.trim())
            .bind(c.libelle.trim())
            .bind(&c.demi_journee)
            .bind((i + 1) as i16)
            .bind(c.heure_debut)
            .bind(c.heure_fin)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let ev = publier_ref(
            &mut tx,
            a,
            "referentiel.grille_horaire_definie",
            site_id.unwrap_or(SANS_SITE),
            json!({ "creneaux": creneaux.len(), "site_id": site_id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        self.ref_grille(a, site_id).await
    }

    /// Installe la grille standard M1…M4 / S1…S4 si l'établissement n'en a pas.
    pub async fn ref_amorcer_grille(&self, a: &Acteur<'_>) -> Result<Vec<CreneauGrille>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let existante = self.ref_grille(a, None).await?;
        if !existante.is_empty() {
            return Ok(existante);
        }
        self.ref_definir_grille(a, None, &grille_standard()).await
    }
}

/// Jours neutralisés indexés par date, pour la génération des séances.
pub(crate) fn index_neutralises(
    jours: &[JourCalendrier],
) -> BTreeMap<NaiveDate, Vec<Option<Uuid>>> {
    let mut m: BTreeMap<NaiveDate, Vec<Option<Uuid>>> = BTreeMap::new();
    for j in jours {
        m.entry(j.jour).or_default().push(j.site_id);
    }
    m
}

#[allow(dead_code)]
fn _erreur_domaine_exemple() -> DomainError {
    DomainError::Conflit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn p(code: &str, du: NaiveDate, au: NaiveDate) -> PeriodeSaisie {
        PeriodeSaisie {
            code: code.into(),
            libelle: code.into(),
            kind: "trimestre".into(),
            date_debut: du,
            date_fin: au,
        }
    }

    #[test]
    fn couverture_des_periodes() {
        let (debut, fin) = (d(2026, 9, 1), d(2027, 8, 31));
        let bon = vec![
            p("T1", d(2026, 9, 1), d(2026, 11, 30)),
            p("T2", d(2026, 12, 1), d(2027, 2, 28)),
            p("T3", d(2027, 3, 1), d(2027, 8, 31)),
        ];
        assert_eq!(verifier_couverture(&bon, debut, fin), Ok(()));
        let trou = vec![p("T1", d(2026, 9, 1), d(2026, 11, 29)), p("T2", d(2026, 12, 1), d(2027, 8, 31))];
        assert!(verifier_couverture(&trou, debut, fin).is_err());
        let chevauche = vec![p("T1", d(2026, 9, 1), d(2026, 12, 5)), p("T2", d(2026, 12, 1), d(2027, 8, 31))];
        assert!(verifier_couverture(&chevauche, debut, fin).is_err());
        let court = vec![p("T1", d(2026, 9, 1), d(2027, 6, 30))];
        assert!(verifier_couverture(&court, debut, fin).is_err());
        let tard = vec![p("T1", d(2026, 9, 2), d(2027, 8, 31))];
        assert!(verifier_couverture(&tard, debut, fin).is_err());
        assert!(verifier_couverture(&[], debut, fin).is_err());
        let doublon = vec![p("T1", d(2026, 9, 1), d(2026, 12, 31)), p("T1", d(2027, 1, 1), d(2027, 8, 31))];
        assert!(verifier_couverture(&doublon, debut, fin).is_err());
    }

    #[test]
    fn niveaux_nationaux_coherents() {
        assert_eq!(code_niveau_de_texte("4e"), Some("4EME"));
        assert_eq!(code_niveau_de_texte(" Tle "), Some("TERM"));
        assert_eq!(code_niveau_de_texte("ULIS"), None);
        let ordres: Vec<i16> = NIVEAUX_NATIONAUX.iter().map(|n| n.3).collect();
        assert!(ordres.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn grille_standard_sans_chevauchement() {
        let g = grille_standard();
        assert_eq!(g.len(), 8);
        for w in g.windows(2) {
            assert!(w[0].heure_fin <= w[1].heure_debut);
        }
        assert!(g.iter().take(4).all(|c| c.demi_journee == "matin"));
    }
}
