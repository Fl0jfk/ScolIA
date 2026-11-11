//! Socle 5, lot S5 (1/2) : matières et offre d'enseignements de l'établissement, règles de choix,
//! incompatibilités, prérequis, groupes porteurs, et **port de lecture** pour le socle 3
//! (`05_STRUCTURE.md` §9.5, §9.10, STR-36, STR-39, STR-41, STR-42).
//!
//! Invariants (tenus par la base, voir 338, redits ici pour des erreurs claires) :
//! * l'offre est PAR ANNÉE et PAR ÉTABLISSEMENT ; une offre se ferme, elle ne se supprime pas ;
//! * aucune règle nationale n'est codée : nombres, incompatibilités et prérequis sont des données ;
//! * droits : lecture `referentiel.lire`, écriture `referentiel.gerer` (direction, administratif) ;
//! * chaque écriture est journalisée (avant / après) ; les événements ne portent aucune donnée
//!   personnelle ;
//! * le moteur de vérification [`offre::verifier_choix`] est **la seule** implémentation des
//!   règles de choix : les enseignements suivis (`structure_enseignements`) et le socle 3 l'appellent.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::{journaliser, net, publier, requis};
use crate::tenant_tx::TenantTx;

pub const MODALITES: [&str; 10] = [
    "commun",
    "langue_1",
    "langue_2",
    "langue_3",
    "option_obligatoire",
    "option_facultative",
    "specialite",
    "dispositif",
    "accompagnement",
    "autre",
];
pub const ADMISSIONS: [&str; 4] = ["libre", "sur_avis", "sur_test", "sur_dossier"];
pub const PUBLICS: [&str; 2] = ["classe", "groupe"];
const TYPES_PREREQUIS: [&str; 2] = ["offre_meme_annee", "matiere_annee_precedente"];

async fn tx(e: &PgEnt, a: &Acteur<'_>) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(e.pool(), a.etablissement_id)
        .await
        .map_err(db)
}

/// Code : lettres, chiffres, `_`, `-`, `.` ; mis en majuscules.
fn code_valide(v: &str, max: usize) -> Result<String, EntDbError> {
    let c = requis(v, max)?.to_uppercase();
    if c.chars()
        .all(|x| x.is_alphanumeric() || matches!(x, '_' | '-' | '.'))
    {
        Ok(c)
    } else {
        Err(invalide())
    }
}

fn couleur_valide(v: Option<&str>) -> Result<Option<String>, EntDbError> {
    match net(v, 7)? {
        None => Ok(None),
        Some(c)
            if c.len() == 7
                && c.starts_with('#')
                && c[1..].chars().all(|x| x.is_ascii_hexdigit()) =>
        {
            Ok(Some(c))
        }
        Some(_) => Err(invalide()),
    }
}

fn alnum_optionnel(v: Option<&str>, max: usize) -> Result<Option<String>, EntDbError> {
    match net(v, max)? {
        None => Ok(None),
        Some(c) if c.chars().all(|x| x.is_ascii_alphanumeric()) => Ok(Some(c.to_uppercase())),
        Some(_) => Err(invalide()),
    }
}

// ───────────────────────── Matières (STR-36, STR-39) ─────────────────────────

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MatiereDetail {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub libelle_court: Option<String>,
    pub couleur: Option<String>,
    pub famille: Option<String>,
    pub parent_id: Option<Uuid>,
    pub code_national: Option<String>,
    pub source_nomenclature: Option<String>,
    pub archivee_le: Option<DateTime<Utc>>,
}

const SQL_MATIERE: &str =
    "SELECT id, code, libelle, libelle_court, couleur, famille, parent_id, code_national,
        source_nomenclature, archivee_le FROM ent_matiere WHERE etablissement_id = $1";

#[derive(Debug, Clone, Deserialize)]
pub struct MatiereSaisie {
    pub code: String,
    pub libelle: String,
    pub libelle_court: Option<String>,
    pub couleur: Option<String>,
    pub famille: Option<String>,
    pub parent_id: Option<Uuid>,
    pub code_national: Option<String>,
}

/// Champs absents = inchangés ; chaîne vide = effacer (champs facultatifs).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MatiereModif {
    pub libelle: Option<String>,
    pub libelle_court: Option<String>,
    pub couleur: Option<String>,
    pub famille: Option<String>,
    pub code_national: Option<String>,
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub retirer_parent: bool,
}

fn json_matiere(m: &MatiereDetail) -> Value {
    json!({ "code": m.code, "libelle": m.libelle, "libelle_court": m.libelle_court, "couleur": m.couleur,
            "famille": m.famille, "parent_id": m.parent_id, "code_national": m.code_national,
            "archivee": m.archivee_le.is_some() })
}

async fn matiere_lue(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<MatiereDetail, EntDbError> {
    sqlx::query_as::<_, MatiereDetail>(&format!("{SQL_MATIERE} AND id = $2 FOR UPDATE"))
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

async fn parent_valide(
    conn: &mut PgConnection,
    etab: Uuid,
    parent: Uuid,
    enfant: Option<Uuid>,
) -> Result<(), EntDbError> {
    if enfant == Some(parent) {
        return Err(invalide());
    }
    let ok: Option<(Option<Uuid>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT parent_id, archivee_le FROM ent_matiere WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(parent)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    match ok {
        None => Err(introuvable()),
        Some((Some(_), _)) | Some((_, Some(_))) => Err(invalide()),
        Some(_) => Ok(()),
    }
}

impl PgEnt {
    pub async fn structure_matieres(
        &self,
        a: &Acteur<'_>,
        avec_archivees: bool,
    ) -> Result<Vec<MatiereDetail>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let rows = sqlx::query_as::<_, MatiereDetail>(&format!(
            "{SQL_MATIERE} AND ($2 OR archivee_le IS NULL) ORDER BY code"
        ))
        .bind(a.etablissement_id)
        .bind(avec_archivees)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    /// STR-36 : crée une matière (code unique sans tenir compte de la casse).
    pub async fn structure_matiere_creer(
        &self,
        a: &Acteur<'_>,
        s: MatiereSaisie,
    ) -> Result<MatiereDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let code = code_valide(&s.code, 40)?;
        let libelle = requis(&s.libelle, 120)?;
        let court = net(s.libelle_court.as_deref(), 20)?;
        let couleur = couleur_valide(s.couleur.as_deref())?;
        let famille = net(s.famille.as_deref(), 60)?;
        let national = alnum_optionnel(s.code_national.as_deref(), 20)?;
        let mut t = tx(self, a).await?;
        let dup: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_matiere WHERE etablissement_id = $1 AND upper(code) = $2",
        )
        .bind(a.etablissement_id)
        .bind(&code)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        if dup.is_some() {
            return Err(conflit());
        }
        if let Some(p) = s.parent_id {
            parent_valide(&mut t, a.etablissement_id, p, None).await?;
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_matiere (id, etablissement_id, code, libelle, libelle_court, couleur, famille,
                    parent_id, code_national)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(&code)
        .bind(&libelle)
        .bind(&court)
        .bind(&couleur)
        .bind(&famille)
        .bind(s.parent_id)
        .bind(&national)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let m = matiere_lue(&mut t, a.etablissement_id, id).await?;
        journaliser(
            &mut t,
            a,
            "matiere",
            &id.to_string(),
            "cree",
            None,
            Some(json_matiere(&m)),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.matiere_creee",
            id,
            json!({ "code": m.code }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(m)
    }

    /// STR-36 : modifie une matière (le code ne change jamais : il identifie la matière dans les imports).
    pub async fn structure_matiere_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        m: MatiereModif,
    ) -> Result<MatiereDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let avant = matiere_lue(&mut t, a.etablissement_id, id).await?;
        if avant.archivee_le.is_some() {
            return Err(conflit());
        }
        let libelle = match &m.libelle {
            Some(l) => requis(l, 120)?,
            None => avant.libelle.clone(),
        };
        let remplace = |nouveau: &Option<String>,
                        ancien: &Option<String>,
                        max: usize|
         -> Result<Option<String>, EntDbError> {
            match nouveau {
                None => Ok(ancien.clone()),
                Some(v) => net(Some(v), max),
            }
        };
        let court = remplace(&m.libelle_court, &avant.libelle_court, 20)?;
        let famille = remplace(&m.famille, &avant.famille, 60)?;
        let couleur = match &m.couleur {
            None => avant.couleur.clone(),
            Some(c) => couleur_valide(Some(c))?,
        };
        let national = match &m.code_national {
            None => avant.code_national.clone(),
            Some(c) => alnum_optionnel(Some(c), 20)?,
        };
        let parent = if m.retirer_parent {
            None
        } else if let Some(p) = m.parent_id {
            parent_valide(&mut t, a.etablissement_id, p, Some(id)).await?;
            Some(p)
        } else {
            avant.parent_id
        };
        sqlx::query(
            "UPDATE ent_matiere SET libelle = $3, libelle_court = $4, couleur = $5, famille = $6,
                    code_national = $7, parent_id = $8, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&libelle)
        .bind(&court)
        .bind(&couleur)
        .bind(&famille)
        .bind(&national)
        .bind(parent)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let apres = matiere_lue(&mut t, a.etablissement_id, id).await?;
        journaliser(
            &mut t,
            a,
            "matiere",
            &id.to_string(),
            "modifie",
            Some(json_matiere(&avant)),
            Some(json_matiere(&apres)),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.matiere_modifiee",
            id,
            json!({ "code": apres.code }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// STR-39 : archive une matière. Refusé si un cours ou une offre ouverts (année non terminée)
    /// l'utilisent, ou si elle a des sous-matières actives.
    pub async fn structure_matiere_archiver(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<MatiereDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let avant = matiere_lue(&mut t, a.etablissement_id, id).await?;
        if avant.archivee_le.is_some() {
            t.commit().await.map_err(db)?;
            return Ok(avant);
        }
        let (cours, offres, enfants): (i64, i64, i64) = sqlx::query_as(
            "SELECT
               (SELECT count(*) FROM ent_service_enseignement s
                  JOIN ent_annee_scolaire y ON y.id = s.annee_id AND y.etablissement_id = s.etablissement_id
                 WHERE s.etablissement_id = $1 AND s.matiere_id = $2
                   AND y.cloturee_le IS NULL AND y.date_fin >= ent_aujourdhui_etab($1)),
               (SELECT count(*) FROM ent_offre_enseignement o
                  JOIN ent_annee_scolaire y ON y.id = o.annee_id AND y.etablissement_id = o.etablissement_id
                 WHERE o.etablissement_id = $1 AND o.matiere_id = $2 AND o.ferme_le IS NULL
                   AND y.cloturee_le IS NULL AND y.date_fin >= ent_aujourdhui_etab($1)),
               (SELECT count(*) FROM ent_matiere c
                 WHERE c.etablissement_id = $1 AND c.parent_id = $2 AND c.archivee_le IS NULL)",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if cours + offres + enfants > 0 {
            return Err(conflit());
        }
        sqlx::query(
            "UPDATE ent_matiere SET archivee_le = now(), modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let apres = matiere_lue(&mut t, a.etablissement_id, id).await?;
        journaliser(
            &mut t,
            a,
            "matiere",
            &id.to_string(),
            "archive",
            Some(json_matiere(&avant)),
            Some(json_matiere(&apres)),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.matiere_archivee",
            id,
            json!({ "code": apres.code }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    pub async fn structure_matiere_desarchiver(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<MatiereDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let avant = matiere_lue(&mut t, a.etablissement_id, id).await?;
        sqlx::query(
            "UPDATE ent_matiere SET archivee_le = NULL, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let apres = matiere_lue(&mut t, a.etablissement_id, id).await?;
        journaliser(
            &mut t,
            a,
            "matiere",
            &id.to_string(),
            "desarchive",
            Some(json_matiere(&avant)),
            Some(json_matiere(&apres)),
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(apres)
    }
}

// ───────────────────────── Offre (STR-41) ─────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PrerequisVue {
    pub id: Uuid,
    pub r#type: String,
    pub prerequis_offre_id: Option<Uuid>,
    pub prerequis_matiere_id: Option<Uuid>,
    pub bloquant: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OffreDetail {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub matiere_id: Option<Uuid>,
    pub modalite: String,
    pub code_modalite_election: Option<String>,
    pub admission: String,
    pub hors_contrat: bool,
    pub capacite: Option<i32>,
    pub ouvert_aux_voeux: bool,
    pub public_type: String,
    pub date_limite_choix: Option<NaiveDate>,
    pub ordre: i16,
    pub source: String,
    pub ferme_le: Option<DateTime<Utc>>,
    pub ferme_motif: Option<String>,
    pub niveau_ids: Vec<Uuid>,
    pub mef_ids: Vec<Uuid>,
    pub groupe_ids: Vec<Uuid>,
    pub prerequis: Vec<PrerequisVue>,
    /// Enseignements acceptés et toujours suivis.
    pub nb_suivis: i64,
    /// Choix en attente de décision (admission sur avis, test ou dossier).
    pub nb_en_attente: i64,
}

#[derive(FromRow)]
struct OffreRow {
    id: Uuid,
    annee_id: Uuid,
    code: String,
    libelle: String,
    matiere_id: Option<Uuid>,
    modalite: String,
    code_modalite_election: Option<String>,
    admission: String,
    hors_contrat: bool,
    capacite: Option<i32>,
    ouvert_aux_voeux: bool,
    public_type: String,
    date_limite_choix: Option<NaiveDate>,
    ordre: i16,
    source: String,
    ferme_le: Option<DateTime<Utc>>,
    ferme_motif: Option<String>,
}

const SQL_OFFRE: &str = "SELECT id, annee_id, code, libelle, matiere_id, modalite, code_modalite_election,
        admission, hors_contrat, capacite, ouvert_aux_voeux, public_type, date_limite_choix, ordre, source,
        ferme_le, ferme_motif FROM ent_offre_enseignement WHERE etablissement_id = $1";

#[derive(Debug, Clone, Deserialize)]
pub struct OffreSaisie {
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub matiere_id: Option<Uuid>,
    pub modalite: String,
    pub code_modalite_election: Option<String>,
    pub admission: Option<String>,
    #[serde(default)]
    pub hors_contrat: bool,
    pub capacite: Option<i32>,
    #[serde(default)]
    pub ouvert_aux_voeux: bool,
    pub public_type: Option<String>,
    pub date_limite_choix: Option<NaiveDate>,
    pub ordre: Option<i16>,
    #[serde(default)]
    pub niveau_ids: Vec<Uuid>,
    #[serde(default)]
    pub mef_ids: Vec<Uuid>,
}

/// Champs absents = inchangés. `retirer_*` efface une valeur facultative. Les cibles, quand elles
/// sont fournies, **remplacent** les précédentes.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OffreModif {
    pub libelle: Option<String>,
    pub matiere_id: Option<Uuid>,
    #[serde(default)]
    pub retirer_matiere: bool,
    pub modalite: Option<String>,
    pub code_modalite_election: Option<String>,
    pub admission: Option<String>,
    pub hors_contrat: Option<bool>,
    pub capacite: Option<i32>,
    #[serde(default)]
    pub retirer_capacite: bool,
    pub ouvert_aux_voeux: Option<bool>,
    pub public_type: Option<String>,
    pub date_limite_choix: Option<NaiveDate>,
    #[serde(default)]
    pub retirer_date_limite: bool,
    pub ordre: Option<i16>,
    pub niveau_ids: Option<Vec<Uuid>>,
    pub mef_ids: Option<Vec<Uuid>>,
}

fn json_offre(o: &OffreDetail) -> Value {
    json!({ "code": o.code, "libelle": o.libelle, "matiere_id": o.matiere_id, "modalite": o.modalite,
            "admission": o.admission, "hors_contrat": o.hors_contrat, "capacite": o.capacite,
            "ouvert_aux_voeux": o.ouvert_aux_voeux, "public_type": o.public_type,
            "date_limite_choix": o.date_limite_choix, "niveau_ids": o.niveau_ids, "mef_ids": o.mef_ids,
            "ferme": o.ferme_le.is_some() })
}

/// Charge des offres avec cibles, groupes, prérequis et effectifs (une requête par agrégat).
pub(crate) async fn offres_detail(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Option<Uuid>,
    ids: Option<&[Uuid]>,
) -> Result<Vec<OffreDetail>, EntDbError> {
    let rows = sqlx::query_as::<_, OffreRow>(&format!(
        "{SQL_OFFRE} AND ($2::uuid IS NULL OR annee_id = $2)
            AND ($3::uuid[] IS NULL OR id = ANY($3)) ORDER BY ordre, code"
    ))
    .bind(etab)
    .bind(annee)
    .bind(ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let id_list: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let cibles: Vec<(Uuid, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
        "SELECT offre_id, niveau_id, mef_id FROM ent_offre_cible
          WHERE etablissement_id = $1 AND offre_id = ANY($2)",
    )
    .bind(etab)
    .bind(&id_list)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let groupes: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT offre_id, groupe_id FROM ent_offre_groupe WHERE etablissement_id = $1 AND offre_id = ANY($2)",
    )
    .bind(etab)
    .bind(&id_list)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    type LignePrerequis = (Uuid, Uuid, String, Option<Uuid>, Option<Uuid>, bool);
    let prerequis: Vec<LignePrerequis> = sqlx::query_as(
        "SELECT offre_id, id, type, prerequis_offre_id, prerequis_matiere_id, bloquant FROM ent_offre_prerequis
          WHERE etablissement_id = $1 AND offre_id = ANY($2) ORDER BY created_at, id",
    )
    .bind(etab)
    .bind(&id_list)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let effectifs: Vec<(Uuid, i64, i64)> = sqlx::query_as(
        "SELECT offre_id,
                count(*) FILTER (WHERE etat = 'accepte' AND quitte_le IS NULL),
                count(*) FILTER (WHERE etat = 'choisi')
           FROM ent_eleve_enseignement
          WHERE etablissement_id = $1 AND offre_id = ANY($2) GROUP BY offre_id",
    )
    .bind(etab)
    .bind(&id_list)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let eff: HashMap<Uuid, (i64, i64)> =
        effectifs.into_iter().map(|(o, s, p)| (o, (s, p))).collect();
    Ok(rows
        .into_iter()
        .map(|r| {
            let (nb_suivis, nb_en_attente) = eff.get(&r.id).copied().unwrap_or((0, 0));
            OffreDetail {
                niveau_ids: cibles
                    .iter()
                    .filter(|c| c.0 == r.id)
                    .filter_map(|c| c.1)
                    .collect(),
                mef_ids: cibles
                    .iter()
                    .filter(|c| c.0 == r.id)
                    .filter_map(|c| c.2)
                    .collect(),
                groupe_ids: groupes
                    .iter()
                    .filter(|g| g.0 == r.id)
                    .map(|g| g.1)
                    .collect(),
                prerequis: prerequis
                    .iter()
                    .filter(|p| p.0 == r.id)
                    .map(|p| PrerequisVue {
                        id: p.1,
                        r#type: p.2.clone(),
                        prerequis_offre_id: p.3,
                        prerequis_matiere_id: p.4,
                        bloquant: p.5,
                    })
                    .collect(),
                nb_suivis,
                nb_en_attente,
                id: r.id,
                annee_id: r.annee_id,
                code: r.code,
                libelle: r.libelle,
                matiere_id: r.matiere_id,
                modalite: r.modalite,
                code_modalite_election: r.code_modalite_election,
                admission: r.admission,
                hors_contrat: r.hors_contrat,
                capacite: r.capacite,
                ouvert_aux_voeux: r.ouvert_aux_voeux,
                public_type: r.public_type,
                date_limite_choix: r.date_limite_choix,
                ordre: r.ordre,
                source: r.source,
                ferme_le: r.ferme_le,
                ferme_motif: r.ferme_motif,
            }
        })
        .collect())
}

pub(crate) async fn offre_lue(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<OffreDetail, EntDbError> {
    offres_detail(conn, etab, None, Some(&[id]))
        .await?
        .pop()
        .ok_or_else(introuvable)
}

async fn verrouiller_offre(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<(), EntDbError> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM ent_offre_enseignement WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .map(|_| ())
    .ok_or_else(introuvable)
}

fn valider_liste<'a>(v: &'a str, permises: &[&str]) -> Result<&'a str, EntDbError> {
    if permises.contains(&v) {
        Ok(v)
    } else {
        Err(invalide())
    }
}

async fn cibles_valides(
    conn: &mut PgConnection,
    etab: Uuid,
    niveaux: &[Uuid],
    mefs: &[Uuid],
) -> Result<(), EntDbError> {
    let ln: HashSet<&Uuid> = niveaux.iter().collect();
    let lm: HashSet<&Uuid> = mefs.iter().collect();
    if ln.len() != niveaux.len() || lm.len() != mefs.len() || niveaux.len() + mefs.len() > 100 {
        return Err(invalide());
    }
    let (n, m): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ent_niveau WHERE etablissement_id = $1 AND id = ANY($2)),
                (SELECT count(*) FROM ent_mef WHERE etablissement_id = $1 AND id = ANY($3))",
    )
    .bind(etab)
    .bind(niveaux)
    .bind(mefs)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    if n as usize != niveaux.len() || m as usize != mefs.len() {
        return Err(introuvable());
    }
    Ok(())
}

async fn poser_cibles(
    conn: &mut PgConnection,
    etab: Uuid,
    offre: Uuid,
    niveaux: &[Uuid],
    mefs: &[Uuid],
) -> Result<(), EntDbError> {
    sqlx::query("DELETE FROM ent_offre_cible WHERE etablissement_id = $1 AND offre_id = $2")
        .bind(etab)
        .bind(offre)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    for n in niveaux {
        sqlx::query("INSERT INTO ent_offre_cible (id, etablissement_id, offre_id, niveau_id) VALUES ($1,$2,$3,$4)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(offre)
            .bind(n)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
    }
    for m in mefs {
        sqlx::query("INSERT INTO ent_offre_cible (id, etablissement_id, offre_id, mef_id) VALUES ($1,$2,$3,$4)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(offre)
            .bind(m)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
    }
    Ok(())
}

async fn matiere_active(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<(), EntDbError> {
    let m: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
        "SELECT archivee_le FROM ent_matiere WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    match m {
        None => Err(introuvable()),
        Some((Some(_),)) => Err(invalide()),
        Some(_) => Ok(()),
    }
}

impl PgEnt {
    /// Offre d'une année (ou de toutes), avec cibles, groupes, prérequis et effectifs.
    pub async fn structure_offres(
        &self,
        a: &Acteur<'_>,
        annee_id: Option<Uuid>,
    ) -> Result<Vec<OffreDetail>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let r = offres_detail(&mut t, a.etablissement_id, annee_id, None).await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    pub async fn structure_offre(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<OffreDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let r = offre_lue(&mut t, a.etablissement_id, id).await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    /// STR-41 : ajoute un enseignement à l'offre d'une année. L'année doit exister et ne pas être
    /// clôturée ; rien n'est créé implicitement (matière, niveau et MEF doivent exister).
    pub async fn structure_offre_creer(
        &self,
        a: &Acteur<'_>,
        s: OffreSaisie,
    ) -> Result<OffreDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let code = code_valide(&s.code, 40)?;
        let libelle = requis(&s.libelle, 160)?;
        let modalite = valider_liste(&s.modalite, &MODALITES)?.to_string();
        let admission =
            valider_liste(s.admission.as_deref().unwrap_or("libre"), &ADMISSIONS)?.to_string();
        let public_type =
            valider_liste(s.public_type.as_deref().unwrap_or("groupe"), &PUBLICS)?.to_string();
        let election = alnum_optionnel(s.code_modalite_election.as_deref(), 10)?;
        if s.capacite.is_some_and(|c| c < 0) {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let annee: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            "SELECT cloturee_le FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(s.annee_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        match annee {
            None => return Err(introuvable()),
            Some((Some(_),)) => return Err(conflit()),
            Some(_) => {}
        }
        if let Some(m) = s.matiere_id {
            matiere_active(&mut t, a.etablissement_id, m).await?;
        }
        cibles_valides(&mut t, a.etablissement_id, &s.niveau_ids, &s.mef_ids).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_offre_enseignement (id, etablissement_id, annee_id, code, libelle, matiere_id,
                    modalite, code_modalite_election, admission, hors_contrat, capacite, ouvert_aux_voeux,
                    public_type, date_limite_choix, ordre, created_by)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(s.annee_id)
        .bind(&code)
        .bind(&libelle)
        .bind(s.matiere_id)
        .bind(&modalite)
        .bind(&election)
        .bind(&admission)
        .bind(s.hors_contrat)
        .bind(s.capacite)
        .bind(s.ouvert_aux_voeux)
        .bind(&public_type)
        .bind(s.date_limite_choix)
        .bind(s.ordre.unwrap_or(0))
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        poser_cibles(&mut t, a.etablissement_id, id, &s.niveau_ids, &s.mef_ids).await?;
        let o = offre_lue(&mut t, a.etablissement_id, id).await?;
        journaliser(
            &mut t,
            a,
            "offre",
            &id.to_string(),
            "cree",
            None,
            Some(json_offre(&o)),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.offre_modifiee",
            id,
            json!({ "code": o.code, "action": "creee" }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(o)
    }

    /// STR-41 : modifie une offre ouverte. Baisser la capacité en dessous du nombre d'élèves qui
    /// suivent déjà l'enseignement est permis (aucun n'est retiré) ; la trace est au journal.
    pub async fn structure_offre_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        m: OffreModif,
    ) -> Result<OffreDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        verrouiller_offre(&mut t, a.etablissement_id, id).await?;
        let avant = offre_lue(&mut t, a.etablissement_id, id).await?;
        if avant.ferme_le.is_some() {
            return Err(conflit());
        }
        let libelle = match &m.libelle {
            Some(l) => requis(l, 160)?,
            None => avant.libelle.clone(),
        };
        let modalite = match &m.modalite {
            Some(v) => valider_liste(v, &MODALITES)?.to_string(),
            None => avant.modalite.clone(),
        };
        let admission = match &m.admission {
            Some(v) => valider_liste(v, &ADMISSIONS)?.to_string(),
            None => avant.admission.clone(),
        };
        let public_type = match &m.public_type {
            Some(v) => valider_liste(v, &PUBLICS)?.to_string(),
            None => avant.public_type.clone(),
        };
        let election = match &m.code_modalite_election {
            Some(v) => alnum_optionnel(Some(v), 10)?,
            None => avant.code_modalite_election.clone(),
        };
        let matiere = if m.retirer_matiere {
            None
        } else if let Some(mid) = m.matiere_id {
            matiere_active(&mut t, a.etablissement_id, mid).await?;
            Some(mid)
        } else {
            avant.matiere_id
        };
        let capacite = if m.retirer_capacite {
            None
        } else if let Some(c) = m.capacite {
            if c < 0 {
                return Err(invalide());
            }
            Some(c)
        } else {
            avant.capacite
        };
        let date_limite = if m.retirer_date_limite {
            None
        } else {
            m.date_limite_choix.or(avant.date_limite_choix)
        };
        sqlx::query(
            "UPDATE ent_offre_enseignement
                SET libelle = $3, matiere_id = $4, modalite = $5, code_modalite_election = $6, admission = $7,
                    hors_contrat = $8, capacite = $9, ouvert_aux_voeux = $10, public_type = $11,
                    date_limite_choix = $12, ordre = $13, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&libelle)
        .bind(matiere)
        .bind(&modalite)
        .bind(&election)
        .bind(&admission)
        .bind(m.hors_contrat.unwrap_or(avant.hors_contrat))
        .bind(capacite)
        .bind(m.ouvert_aux_voeux.unwrap_or(avant.ouvert_aux_voeux))
        .bind(&public_type)
        .bind(date_limite)
        .bind(m.ordre.unwrap_or(avant.ordre))
        .execute(&mut *t)
        .await
        .map_err(db)?;
        if m.niveau_ids.is_some() || m.mef_ids.is_some() {
            let niveaux = m
                .niveau_ids
                .clone()
                .unwrap_or_else(|| avant.niveau_ids.clone());
            let mefs = m.mef_ids.clone().unwrap_or_else(|| avant.mef_ids.clone());
            cibles_valides(&mut t, a.etablissement_id, &niveaux, &mefs).await?;
            poser_cibles(&mut t, a.etablissement_id, id, &niveaux, &mefs).await?;
        }
        let apres = offre_lue(&mut t, a.etablissement_id, id).await?;
        journaliser(
            &mut t,
            a,
            "offre",
            &id.to_string(),
            "modifie",
            Some(json_offre(&avant)),
            Some(json_offre(&apres)),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.offre_modifiee",
            id,
            json!({ "code": apres.code, "action": "modifiee" }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// Rouvre une offre fermée par erreur ; les enseignements déjà quittés ou retirés par la
    /// fermeture ne reviennent pas (ils sont dans l'historique).
    pub async fn structure_offre_rouvrir(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<OffreDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        verrouiller_offre(&mut t, a.etablissement_id, id).await?;
        let avant = offre_lue(&mut t, a.etablissement_id, id).await?;
        sqlx::query(
            "UPDATE ent_offre_enseignement SET ferme_le = NULL, ferme_motif = NULL, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let apres = offre_lue(&mut t, a.etablissement_id, id).await?;
        journaliser(
            &mut t,
            a,
            "offre",
            &id.to_string(),
            "rouverte",
            Some(json_offre(&avant)),
            Some(json_offre(&apres)),
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(apres)
    }

    /// Associe un groupe de la même année à une offre organisée par groupe (S6 y ajoutera la
    /// composition). Un groupe ne porte qu'une offre.
    pub async fn structure_offre_groupe_associer(
        &self,
        a: &Acteur<'_>,
        offre_id: Uuid,
        groupe_id: Uuid,
    ) -> Result<OffreDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        verrouiller_offre(&mut t, a.etablissement_id, offre_id).await?;
        let existe: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_groupe WHERE etablissement_id = $1 AND id = $2")
                .bind(a.etablissement_id)
                .bind(groupe_id)
                .fetch_optional(&mut *t)
                .await
                .map_err(db)?;
        if existe.is_none() {
            return Err(introuvable());
        }
        sqlx::query(
            "INSERT INTO ent_offre_groupe (id, etablissement_id, offre_id, groupe_id, created_by)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(offre_id)
        .bind(groupe_id)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            "offre_groupe",
            &offre_id.to_string(),
            "associe",
            None,
            Some(json!({ "groupe_id": groupe_id })),
        )
        .await?;
        let o = offre_lue(&mut t, a.etablissement_id, offre_id).await?;
        t.commit().await.map_err(db)?;
        Ok(o)
    }

    pub async fn structure_offre_groupe_dissocier(
        &self,
        a: &Acteur<'_>,
        offre_id: Uuid,
        groupe_id: Uuid,
    ) -> Result<OffreDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        verrouiller_offre(&mut t, a.etablissement_id, offre_id).await?;
        let n = sqlx::query(
            "DELETE FROM ent_offre_groupe WHERE etablissement_id = $1 AND offre_id = $2 AND groupe_id = $3",
        )
        .bind(a.etablissement_id)
        .bind(offre_id)
        .bind(groupe_id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        journaliser(
            &mut t,
            a,
            "offre_groupe",
            &offre_id.to_string(),
            "dissocie",
            Some(json!({ "groupe_id": groupe_id })),
            None,
        )
        .await?;
        let o = offre_lue(&mut t, a.etablissement_id, offre_id).await?;
        t.commit().await.map_err(db)?;
        Ok(o)
    }
}

// ───────────────────────── Règles de choix, incompatibilités, prérequis (STR-42) ─────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct RegleVue {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub niveau_id: Option<Uuid>,
    pub mef_id: Option<Uuid>,
    pub libelle: String,
    pub offre_ids: Vec<Uuid>,
    pub min: i16,
    pub max: Option<i16>,
    pub bloquant: bool,
    pub archivee_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegleSaisie {
    pub annee_id: Uuid,
    pub niveau_id: Option<Uuid>,
    pub mef_id: Option<Uuid>,
    pub libelle: String,
    pub offre_ids: Vec<Uuid>,
    #[serde(default)]
    pub min: i16,
    pub max: Option<i16>,
    #[serde(default = "vrai")]
    pub bloquant: bool,
}

fn vrai() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct IncompatibiliteVue {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub offre_a: Uuid,
    pub offre_b: Uuid,
    pub motif: Option<String>,
    pub bloquant: bool,
    pub archivee_le: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IncompatibiliteSaisie {
    pub offre_a: Uuid,
    pub offre_b: Uuid,
    pub motif: Option<String>,
    #[serde(default = "vrai")]
    pub bloquant: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrerequisSaisie {
    pub r#type: String,
    pub prerequis_offre_id: Option<Uuid>,
    pub prerequis_matiere_id: Option<Uuid>,
    #[serde(default = "vrai")]
    pub bloquant: bool,
}

async fn regles_lues(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Uuid,
    regle: Option<Uuid>,
) -> Result<Vec<RegleVue>, EntDbError> {
    type Ligne = (
        Uuid,
        Uuid,
        Option<Uuid>,
        Option<Uuid>,
        String,
        i16,
        Option<i16>,
        bool,
        Option<DateTime<Utc>>,
    );
    let rows: Vec<Ligne> = sqlx::query_as(
        "SELECT id, annee_id, niveau_id, mef_id, libelle, min, max, bloquant, archivee_le
           FROM ent_offre_regle_choix
          WHERE etablissement_id = $1 AND annee_id = $2 AND ($3::uuid IS NULL OR id = $3)
          ORDER BY created_at, id",
    )
    .bind(etab)
    .bind(annee)
    .bind(regle)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let membres: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT regle_id, offre_id FROM ent_offre_regle_choix_offre
          WHERE etablissement_id = $1 AND regle_id = ANY($2) ORDER BY offre_id",
    )
    .bind(etab)
    .bind(rows.iter().map(|r| r.0).collect::<Vec<_>>())
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|r| RegleVue {
            offre_ids: membres.iter().filter(|m| m.0 == r.0).map(|m| m.1).collect(),
            id: r.0,
            annee_id: r.1,
            niveau_id: r.2,
            mef_id: r.3,
            libelle: r.4,
            min: r.5,
            max: r.6,
            bloquant: r.7,
            archivee_le: r.8,
        })
        .collect())
}

impl PgEnt {
    pub async fn structure_regles_choix(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<Vec<RegleVue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let r = regles_lues(&mut t, a.etablissement_id, annee_id, None).await?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    /// STR-42 : « LV2 : exactement 1 » = min 1, max 1 sur les offres de langue 2 du niveau.
    pub async fn structure_regle_creer(
        &self,
        a: &Acteur<'_>,
        s: RegleSaisie,
    ) -> Result<RegleVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let libelle = requis(&s.libelle, 160)?;
        let distincts: HashSet<&Uuid> = s.offre_ids.iter().collect();
        if s.offre_ids.is_empty()
            || s.offre_ids.len() > 50
            || distincts.len() != s.offre_ids.len()
            || s.min < 0
            || s.max.is_some_and(|m| m < s.min)
            || s.min as usize > s.offre_ids.len()
            || (s.niveau_id.is_some() && s.mef_id.is_some())
        {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_offre_enseignement
              WHERE etablissement_id = $1 AND annee_id = $2 AND id = ANY($3)",
        )
        .bind(a.etablissement_id)
        .bind(s.annee_id)
        .bind(&s.offre_ids)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if n as usize != s.offre_ids.len() {
            return Err(introuvable());
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_offre_regle_choix (id, etablissement_id, annee_id, niveau_id, mef_id, libelle,
                    min, max, bloquant, created_by)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(s.annee_id)
        .bind(s.niveau_id)
        .bind(s.mef_id)
        .bind(&libelle)
        .bind(s.min)
        .bind(s.max)
        .bind(s.bloquant)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        for o in &s.offre_ids {
            sqlx::query(
                "INSERT INTO ent_offre_regle_choix_offre (etablissement_id, regle_id, offre_id) VALUES ($1,$2,$3)",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(o)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        let r = regles_lues(&mut t, a.etablissement_id, s.annee_id, Some(id))
            .await?
            .pop()
            .ok_or_else(introuvable)?;
        journaliser(
            &mut t,
            a,
            "regle_choix",
            &id.to_string(),
            "creee",
            None,
            Some(
                json!({ "libelle": r.libelle, "min": r.min, "max": r.max, "bloquant": r.bloquant,
                         "offres": r.offre_ids }),
            ),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.offre_modifiee",
            id,
            json!({ "action": "regle_creee" }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(r)
    }

    pub async fn structure_regle_archiver(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let n = sqlx::query(
            "UPDATE ent_offre_regle_choix SET archivee_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND archivee_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        journaliser(
            &mut t,
            a,
            "regle_choix",
            &id.to_string(),
            "archivee",
            None,
            None,
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    pub async fn structure_incompatibilites(
        &self,
        a: &Acteur<'_>,
        annee_id: Uuid,
    ) -> Result<Vec<IncompatibiliteVue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let r = sqlx::query_as::<_, IncompatibiliteVue>(
            "SELECT id, annee_id, offre_a, offre_b, motif, bloquant, archivee_le FROM ent_offre_incompatibilite
              WHERE etablissement_id = $1 AND annee_id = $2 ORDER BY created_at, id",
        )
        .bind(a.etablissement_id)
        .bind(annee_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(r)
    }

    /// STR-42 : deux offres qui ne se suivent pas ensemble. La paire n'est pas ordonnée.
    pub async fn structure_incompatibilite_declarer(
        &self,
        a: &Acteur<'_>,
        s: IncompatibiliteSaisie,
    ) -> Result<IncompatibiliteVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if s.offre_a == s.offre_b {
            return Err(invalide());
        }
        let (x, y) = if s.offre_a < s.offre_b {
            (s.offre_a, s.offre_b)
        } else {
            (s.offre_b, s.offre_a)
        };
        let motif = net(s.motif.as_deref(), 300)?;
        let mut t = tx(self, a).await?;
        let trouvees: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, annee_id FROM ent_offre_enseignement
              WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(a.etablissement_id)
        .bind(vec![x, y])
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        // Une offre inconnue (ou d'un autre établissement) : introuvable ; deux années : invalide.
        if trouvees.len() != 2 {
            return Err(introuvable());
        }
        if trouvees[0].1 != trouvees[1].1 {
            return Err(invalide());
        }
        let annees = [(trouvees[0].1,)];
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_offre_incompatibilite (id, etablissement_id, annee_id, offre_a, offre_b, motif,
                    bloquant, created_by)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(annees[0].0)
        .bind(x)
        .bind(y)
        .bind(&motif)
        .bind(s.bloquant)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let v = sqlx::query_as::<_, IncompatibiliteVue>(
            "SELECT id, annee_id, offre_a, offre_b, motif, bloquant, archivee_le FROM ent_offre_incompatibilite
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            "incompatibilite",
            &id.to_string(),
            "declaree",
            None,
            Some(json!({ "offre_a": x, "offre_b": y, "bloquant": v.bloquant })),
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(v)
    }

    pub async fn structure_incompatibilite_archiver(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let n = sqlx::query(
            "UPDATE ent_offre_incompatibilite SET archivee_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND archivee_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(introuvable());
        }
        journaliser(
            &mut t,
            a,
            "incompatibilite",
            &id.to_string(),
            "archivee",
            None,
            None,
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    /// STR-42 : prérequis d'une offre (autre offre de la même année, ou matière suivie l'année précédente).
    pub async fn structure_prerequis_ajouter(
        &self,
        a: &Acteur<'_>,
        offre_id: Uuid,
        s: PrerequisSaisie,
    ) -> Result<OffreDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        valider_liste(&s.r#type, &TYPES_PREREQUIS)?;
        let forme_ok = match s.r#type.as_str() {
            "offre_meme_annee" => {
                s.prerequis_offre_id.is_some() && s.prerequis_matiere_id.is_none()
            }
            _ => s.prerequis_matiere_id.is_some() && s.prerequis_offre_id.is_none(),
        };
        if !forme_ok || s.prerequis_offre_id == Some(offre_id) {
            return Err(invalide());
        }
        let mut t = tx(self, a).await?;
        verrouiller_offre(&mut t, a.etablissement_id, offre_id).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_offre_prerequis (id, etablissement_id, offre_id, type, prerequis_offre_id,
                    prerequis_matiere_id, bloquant, created_by)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(offre_id)
        .bind(&s.r#type)
        .bind(s.prerequis_offre_id)
        .bind(s.prerequis_matiere_id)
        .bind(s.bloquant)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            "prerequis",
            &id.to_string(),
            "ajoute",
            None,
            Some(json!({ "offre_id": offre_id, "type": s.r#type, "prerequis_offre_id": s.prerequis_offre_id,
                         "prerequis_matiere_id": s.prerequis_matiere_id })),
        )
        .await?;
        let o = offre_lue(&mut t, a.etablissement_id, offre_id).await?;
        t.commit().await.map_err(db)?;
        Ok(o)
    }

    pub async fn structure_prerequis_retirer(
        &self,
        a: &Acteur<'_>,
        prerequis_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let r: Option<(Uuid,)> = sqlx::query_as(
            "DELETE FROM ent_offre_prerequis WHERE etablissement_id = $1 AND id = $2 RETURNING offre_id",
        )
        .bind(a.etablissement_id)
        .bind(prerequis_id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let (offre,) = r.ok_or_else(introuvable)?;
        journaliser(
            &mut t,
            a,
            "prerequis",
            &prerequis_id.to_string(),
            "retire",
            Some(json!({ "offre_id": offre })),
            None,
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }
}

// ───────────────────────── Port de lecture et moteur de vérification ─────────────────────────

/// Port de lecture de l'offre pour les autres socles (socle 3 : options demandées à l'inscription ;
/// socle 6 : composition des groupes). Fonctions **sans acteur** qui prennent la connexion ou la
/// transaction de l'appelant : l'appelant a déjà authentifié et autorisé (jamais d'autorisation ici),
/// le cloisonnement par établissement reste assuré par la RLS de la transaction.
pub mod offre {
    use super::*;

    /// Une offre telle que le socle 3 la propose à une famille.
    #[derive(Debug, Clone, Serialize)]
    pub struct OffreVoeu {
        pub offre_id: Uuid,
        pub code: String,
        pub libelle: String,
        pub matiere_id: Option<Uuid>,
        pub modalite: String,
        pub admission: String,
        pub capacite: Option<i32>,
        /// `None` = capacité non limitée.
        pub places_restantes: Option<i64>,
        pub hors_contrat: bool,
        pub date_limite_choix: Option<NaiveDate>,
    }

    /// Règle de choix applicable, pour l'affichage (« LV2 : exactement 1 »).
    #[derive(Debug, Clone, Serialize)]
    pub struct RegleVoeu {
        pub regle_id: Uuid,
        pub libelle: String,
        pub min: i16,
        pub max: Option<i16>,
        pub bloquant: bool,
        pub offre_ids: Vec<Uuid>,
    }

    #[derive(Debug, Clone, Copy, Default, Serialize)]
    pub struct ContexteChoix {
        pub niveau_id: Option<Uuid>,
        pub mef_id: Option<Uuid>,
    }

    #[derive(Debug, Clone, Serialize, PartialEq, Eq)]
    pub struct Violation {
        /// `offre_inconnue`, `offre_fermee`, `hors_cible`, `capacite_atteinte`, `regle_max`,
        /// `regle_min`, `incompatibilite`, `prerequis_manquant`, `prerequis_non_verifiable`,
        /// `date_limite_depassee`, `contexte_inconnu`.
        pub code: String,
        pub bloquant: bool,
        pub offre_id: Option<Uuid>,
        pub regle_id: Option<Uuid>,
        pub message: String,
    }

    /// Ce qu'on demande au moteur : les `nouveaux` enseignements s'ajoutent à `ensemble` (qui les
    /// contient déjà). Avec `complet`, toutes les règles applicables sont évaluées (contrôle d'un
    /// choix achevé : les minimums sont alors des écarts) ; sinon seules celles qui touchent un
    /// nouveau enseignement le sont, et un minimum non atteint n'est qu'un avertissement.
    pub struct Demande<'a> {
        pub annee_id: Uuid,
        pub contexte: ContexteChoix,
        pub eleve_id: Option<Uuid>,
        pub ensemble: &'a [Uuid],
        pub nouveaux: &'a [Uuid],
        pub jour: NaiveDate,
        pub complet: bool,
        pub forcer_capacite: bool,
    }

    const SQL_OUVERTES: &str =
        "SELECT o.id, o.code, o.libelle, o.matiere_id, o.modalite, o.admission,
            o.capacite, o.hors_contrat, o.date_limite_choix,
            CASE WHEN o.capacite IS NULL THEN NULL ELSE greatest(o.capacite -
              (SELECT count(*) FROM ent_eleve_enseignement e
                WHERE e.etablissement_id = o.etablissement_id AND e.offre_id = o.id
                  AND e.etat = 'accepte' AND e.quitte_le IS NULL), 0)::bigint END
       FROM ent_offre_enseignement o";

    type LigneVoeu = (
        Uuid,
        String,
        String,
        Option<Uuid>,
        String,
        String,
        Option<i32>,
        bool,
        Option<NaiveDate>,
        Option<i64>,
    );

    fn voeu(r: LigneVoeu) -> OffreVoeu {
        OffreVoeu {
            offre_id: r.0,
            code: r.1,
            libelle: r.2,
            matiere_id: r.3,
            modalite: r.4,
            admission: r.5,
            capacite: r.6,
            places_restantes: r.9,
            hors_contrat: r.7,
            date_limite_choix: r.8,
        }
    }

    /// Une offre (ouverte ou non), pour le socle 3.
    pub async fn lire(
        conn: &mut PgConnection,
        etab: Uuid,
        offre_id: Uuid,
    ) -> Result<Option<OffreVoeu>, EntDbError> {
        let r: Option<LigneVoeu> = sqlx::query_as(&format!(
            "{SQL_OUVERTES} WHERE o.etablissement_id = $1 AND o.id = $2"
        ))
        .bind(etab)
        .bind(offre_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?;
        Ok(r.map(voeu))
    }

    /// Offres que l'établissement ouvre aux vœux pour cette année, filtrées par niveau et/ou MEF
    /// quand ils sont connus. Une offre sans cible vaut pour tous ; une cible MEF vaut aussi pour
    /// le niveau de ce MEF quand le MEF de l'élève n'est pas connu. Les offres fermées, non
    /// ouvertes aux vœux ou archivées sont exclues.
    pub async fn offres_ouvertes_aux_voeux(
        conn: &mut PgConnection,
        etab: Uuid,
        annee_id: Uuid,
        niveau_id: Option<Uuid>,
        mef_id: Option<Uuid>,
    ) -> Result<Vec<OffreVoeu>, EntDbError> {
        let rows: Vec<LigneVoeu> = sqlx::query_as(&format!(
            "{SQL_OUVERTES}
              WHERE o.etablissement_id = $1 AND o.annee_id = $2 AND o.ouvert_aux_voeux AND o.ferme_le IS NULL
                AND ($3::uuid IS NULL AND $4::uuid IS NULL
                     OR NOT EXISTS (SELECT 1 FROM ent_offre_cible c WHERE c.offre_id = o.id)
                     OR EXISTS (SELECT 1 FROM ent_offre_cible c
                                  LEFT JOIN ent_mef m ON m.id = c.mef_id AND m.etablissement_id = c.etablissement_id
                                 WHERE c.offre_id = o.id
                                   AND (c.niveau_id = $3 OR c.mef_id = $4
                                        OR ($4::uuid IS NULL AND m.niveau_id = $3))))
              ORDER BY o.ordre, o.code"
        ))
        .bind(etab)
        .bind(annee_id)
        .bind(niveau_id)
        .bind(mef_id)
        .fetch_all(conn)
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(voeu).collect())
    }

    /// Règles de choix applicables à ce contexte (celles de tous, du niveau, du MEF).
    pub async fn regles_applicables(
        conn: &mut PgConnection,
        etab: Uuid,
        annee_id: Uuid,
        ctx: ContexteChoix,
    ) -> Result<Vec<RegleVoeu>, EntDbError> {
        type Ligne = (Uuid, String, i16, Option<i16>, bool);
        let rows: Vec<Ligne> = sqlx::query_as(
            "SELECT r.id, r.libelle, r.min, r.max, r.bloquant FROM ent_offre_regle_choix r
              WHERE r.etablissement_id = $1 AND r.annee_id = $2 AND r.archivee_le IS NULL
                AND ((r.niveau_id IS NULL AND r.mef_id IS NULL)
                     OR r.niveau_id = $3 OR r.mef_id = $4
                     OR (r.mef_id IS NOT NULL AND $4::uuid IS NULL AND $3::uuid IS NOT NULL
                         AND EXISTS (SELECT 1 FROM ent_mef m WHERE m.id = r.mef_id
                                       AND m.etablissement_id = r.etablissement_id AND m.niveau_id = $3)))
              ORDER BY r.created_at, r.id",
        )
        .bind(etab)
        .bind(annee_id)
        .bind(ctx.niveau_id)
        .bind(ctx.mef_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let membres: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT regle_id, offre_id FROM ent_offre_regle_choix_offre
              WHERE etablissement_id = $1 AND regle_id = ANY($2)",
        )
        .bind(etab)
        .bind(rows.iter().map(|r| r.0).collect::<Vec<_>>())
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|r| RegleVoeu {
                offre_ids: membres.iter().filter(|m| m.0 == r.0).map(|m| m.1).collect(),
                regle_id: r.0,
                libelle: r.1,
                min: r.2,
                max: r.3,
                bloquant: r.4,
            })
            .collect())
    }

    fn v(
        code: &str,
        bloquant: bool,
        offre: Option<Uuid>,
        regle: Option<Uuid>,
        msg: String,
    ) -> Violation {
        Violation {
            code: code.to_string(),
            bloquant,
            offre_id: offre,
            regle_id: regle,
            message: msg,
        }
    }

    /// LE moteur de règles de choix : la seule implémentation, appelée par les enseignements suivis
    /// et par le socle 3 pour valider des vœux. Lecture seule.
    pub async fn verifier_choix(
        conn: &mut PgConnection,
        etab: Uuid,
        d: &Demande<'_>,
    ) -> Result<Vec<Violation>, EntDbError> {
        let mut out: Vec<Violation> = Vec::new();
        let ensemble: HashSet<Uuid> = d.ensemble.iter().copied().collect();
        let nouveaux: HashSet<Uuid> = d.nouveaux.iter().copied().collect();
        let portee: HashSet<Uuid> = if d.complet {
            ensemble.clone()
        } else {
            nouveaux.clone()
        };
        let ids: Vec<Uuid> = ensemble.iter().copied().collect();

        type LigneOffre = (
            Uuid,
            String,
            String,
            Option<DateTime<Utc>>,
            String,
            Option<i32>,
            Option<NaiveDate>,
            Option<Uuid>,
        );
        let offres: Vec<LigneOffre> = sqlx::query_as(
            "SELECT id, code, libelle, ferme_le, admission, capacite, date_limite_choix, matiere_id
               FROM ent_offre_enseignement
              WHERE etablissement_id = $1 AND annee_id = $2 AND id = ANY($3)",
        )
        .bind(etab)
        .bind(d.annee_id)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let par_id: HashMap<Uuid, &LigneOffre> = offres.iter().map(|o| (o.0, o)).collect();
        let nom = |id: &Uuid| {
            par_id
                .get(id)
                .map_or_else(|| id.to_string(), |o| o.1.clone())
        };
        for id in &ids {
            if !par_id.contains_key(id) {
                out.push(v(
                    "offre_inconnue",
                    true,
                    Some(*id),
                    None,
                    "enseignement inconnu pour cette année".into(),
                ));
            }
        }
        if d.contexte.niveau_id.is_none() && d.contexte.mef_id.is_none() && d.eleve_id.is_some() {
            out.push(v(
                "contexte_inconnu",
                false,
                None,
                None,
                "niveau de l'élève inconnu : les cibles et les règles par niveau n'ont pas été vérifiées".into(),
            ));
        }

        // Cibles : une offre ciblée doit viser le niveau ou le MEF de l'élève (si connus).
        type LigneCible = (Uuid, Option<Uuid>, Option<Uuid>, Option<Uuid>);
        let cibles: Vec<LigneCible> = sqlx::query_as(
            "SELECT c.offre_id, c.niveau_id, c.mef_id, m.niveau_id FROM ent_offre_cible c
               LEFT JOIN ent_mef m ON m.id = c.mef_id AND m.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.offre_id = ANY($2)",
        )
        .bind(etab)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;

        for id in nouveaux.iter().filter(|i| par_id.contains_key(i)) {
            let o = par_id[id];
            if o.3.is_some() {
                out.push(v(
                    "offre_fermee",
                    true,
                    Some(*id),
                    None,
                    format!("{} est fermé", o.1),
                ));
            }
            if let Some(lim) = o.6 {
                if lim < d.jour {
                    out.push(v(
                        "date_limite_depassee",
                        false,
                        Some(*id),
                        None,
                        format!("la date limite de choix de {} est dépassée", o.1),
                    ));
                }
            }
            let ses_cibles: Vec<_> = cibles.iter().filter(|c| c.0 == *id).collect();
            if !ses_cibles.is_empty()
                && (d.contexte.niveau_id.is_some() || d.contexte.mef_id.is_some())
            {
                let vise = ses_cibles.iter().any(|c| {
                    (c.1.is_some() && c.1 == d.contexte.niveau_id)
                        || (c.2.is_some() && c.2 == d.contexte.mef_id)
                        || (c.2.is_some()
                            && d.contexte.mef_id.is_none()
                            && c.3.is_some()
                            && c.3 == d.contexte.niveau_id)
                });
                if !vise {
                    out.push(v(
                        "hors_cible",
                        true,
                        Some(*id),
                        None,
                        format!("{} n'est pas proposé à ce niveau", o.1),
                    ));
                }
            }
            // Capacité : seules les offres à acceptation immédiate occupent une place au choix.
            if o.4 == "libre" {
                if let Some(cap) = o.5 {
                    let pris: i64 = sqlx::query_scalar(
                        "SELECT count(*) FROM ent_eleve_enseignement
                          WHERE etablissement_id = $1 AND offre_id = $2 AND etat = 'accepte' AND quitte_le IS NULL
                            AND ($3::uuid IS NULL OR eleve_id <> $3)",
                    )
                    .bind(etab)
                    .bind(id)
                    .bind(d.eleve_id)
                    .fetch_one(&mut *conn)
                    .await
                    .map_err(db)?;
                    if pris >= i64::from(cap) {
                        out.push(v(
                            "capacite_atteinte",
                            !d.forcer_capacite,
                            Some(*id),
                            None,
                            format!("{} est complet ({pris}/{cap})", o.1),
                        ));
                    }
                }
            }
        }

        // Règles de choix.
        for r in regles_applicables(&mut *conn, etab, d.annee_id, d.contexte).await? {
            let touche = r.offre_ids.iter().any(|o| portee.contains(o));
            if !d.complet && !touche {
                continue;
            }
            let n = r.offre_ids.iter().filter(|o| ensemble.contains(o)).count() as i64;
            if let Some(max) = r.max {
                if n > i64::from(max) {
                    out.push(v(
                        "regle_max",
                        r.bloquant,
                        None,
                        Some(r.regle_id),
                        format!("{} : {n} choisi(s), {max} au plus", r.libelle),
                    ));
                }
            }
            if n < i64::from(r.min) {
                out.push(v(
                    "regle_min",
                    d.complet && r.bloquant,
                    None,
                    Some(r.regle_id),
                    format!("{} : {n} choisi(s), {} au moins", r.libelle, r.min),
                ));
            }
        }

        // Incompatibilités (paires dont les deux offres sont dans l'ensemble).
        let incompat: Vec<(Uuid, Uuid, Uuid, Option<String>, bool)> = sqlx::query_as(
            "SELECT id, offre_a, offre_b, motif, bloquant FROM ent_offre_incompatibilite
              WHERE etablissement_id = $1 AND annee_id = $2 AND archivee_le IS NULL
                AND offre_a = ANY($3) AND offre_b = ANY($3)",
        )
        .bind(etab)
        .bind(d.annee_id)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (_, x, y, motif, bloquant) in incompat {
            if !(portee.contains(&x) || portee.contains(&y)) {
                continue;
            }
            let suite = motif.map(|m| format!(" ({m})")).unwrap_or_default();
            out.push(v(
                "incompatibilite",
                bloquant,
                Some(x),
                None,
                format!(
                    "{} et {} ne se suivent pas ensemble{suite}",
                    nom(&x),
                    nom(&y)
                ),
            ));
        }

        // Prérequis.
        type LignePrerequisVerif = (Uuid, String, Option<Uuid>, Option<Uuid>, bool);
        let prerequis: Vec<LignePrerequisVerif> = sqlx::query_as(
            "SELECT offre_id, type, prerequis_offre_id, prerequis_matiere_id, bloquant FROM ent_offre_prerequis
              WHERE etablissement_id = $1 AND offre_id = ANY($2)",
        )
        .bind(etab)
        .bind(portee.iter().copied().collect::<Vec<_>>())
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (offre, kind, pre_offre, pre_matiere, bloquant) in prerequis {
            if kind == "offre_meme_annee" {
                if let Some(p) = pre_offre {
                    if !ensemble.contains(&p) {
                        out.push(v(
                            "prerequis_manquant",
                            bloquant,
                            Some(offre),
                            None,
                            format!("{} demande de suivre aussi {}", nom(&offre), nom(&p)),
                        ));
                    }
                }
            } else if let Some(mat) = pre_matiere {
                match d.eleve_id {
                    None => out.push(v(
                        "prerequis_non_verifiable",
                        false,
                        Some(offre),
                        None,
                        format!(
                            "{} demande une matière suivie l'an dernier (élève inconnu)",
                            nom(&offre)
                        ),
                    )),
                    Some(eleve) => {
                        let a_suivi: bool = sqlx::query_scalar(
                            "SELECT EXISTS (
                               SELECT 1 FROM ent_eleve_enseignement e
                                 JOIN ent_offre_enseignement o ON o.id = e.offre_id AND o.etablissement_id = e.etablissement_id
                                WHERE e.etablissement_id = $1 AND e.eleve_id = $2 AND o.matiere_id = $3
                                  AND e.etat IN ('accepte', 'quitte')
                                  AND e.annee_id = (SELECT p.id FROM ent_annee_scolaire p
                                                      WHERE p.etablissement_id = $1
                                                        AND p.date_fin < (SELECT c.date_debut FROM ent_annee_scolaire c
                                                                           WHERE c.id = $4 AND c.etablissement_id = $1)
                                                      ORDER BY p.date_fin DESC LIMIT 1))",
                        )
                        .bind(etab)
                        .bind(eleve)
                        .bind(mat)
                        .bind(d.annee_id)
                        .fetch_one(&mut *conn)
                        .await
                        .map_err(db)?;
                        if !a_suivi {
                            out.push(v(
                                "prerequis_manquant",
                                bloquant,
                                Some(offre),
                                None,
                                format!(
                                    "{} demande la matière suivie l'année précédente",
                                    nom(&offre)
                                ),
                            ));
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}
