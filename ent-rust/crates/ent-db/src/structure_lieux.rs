//! Socle 5, lot S2 : sites, bâtiments, salles typées, lieux extérieurs, disponibilités et
//! indisponibilités, import et modification de salles en lot (`05_STRUCTURE.md` §9.1, §9.10,
//! STR-05 à STR-09, STR-109, STR-110).
//!
//! Invariants (tenus par la base, voir la migration 337, et redits ici pour des erreurs claires) :
//! * une salle appartient à un site **de son établissement** ; aucun site, bâtiment ou lieu n'est
//!   jamais créé implicitement (ni par une salle, ni par un import) ;
//! * un bâtiment appartient au site de la salle ; une salle de lieu extérieur est de type
//!   `exterieur` ;
//! * fermer, c'est dater (`ferme_le`), jamais supprimer ; les indisponibilités s'annulent ;
//! * droits : lecture `referentiel.lire`, écriture `referentiel.gerer` ; événements sans donnée
//!   personnelle ; chaque écriture est journalisée avec avant/après.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::{journaliser, net, publier, requis, structure, uai_valide, DEGRES};
use crate::tenant_tx::TenantTx;

pub const TYPES_SALLE: [&str; 11] = [
    "banalisee",
    "laboratoire",
    "informatique",
    "gymnase",
    "atelier",
    "cdi",
    "musique",
    "arts",
    "amphitheatre",
    "exterieur",
    "autre",
];
pub const MOTIFS_INDISPO: [&str; 6] = ["travaux", "evenement", "examen", "requisition", "securite", "autre"];
const MODES_TRAJET: [&str; 4] = ["pied", "car", "transport_commun", "autre"];

/// Lignes maximales et taille maximale d'un import de salles (borne d'abus).
pub const IMPORT_LIGNES_MAX: usize = 2000;
pub const IMPORT_OCTETS_MAX: usize = 512 * 1024;

async fn tx(e: &PgEnt, a: &Acteur<'_>) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(e.pool(), a.etablissement_id).await.map_err(db)
}

// ───────────────────────── Sites (STR-05) ─────────────────────────

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SiteDetail {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub adresse: Option<String>,
    pub uai: Option<String>,
    pub degres: Vec<String>,
    pub couleur: Option<String>,
    pub fuseau_horaire: Option<String>,
    pub actif: bool,
    pub ferme_le: Option<NaiveDate>,
    pub nb_salles_ouvertes: i64,
}

const SQL_SITE: &str = "SELECT s.id, s.code, s.libelle, s.adresse, s.uai, s.degres, s.couleur, s.fuseau_horaire,
        s.actif, s.ferme_le,
        (SELECT count(*) FROM ent_salle r WHERE r.site_id = s.id AND r.etablissement_id = s.etablissement_id
            AND r.ferme_le IS NULL) AS nb_salles_ouvertes
   FROM ent_site s";

/// Modification d'un site : chaque champ absent est conservé ; une chaîne vide efface (sauf libellé).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SiteModif {
    pub libelle: Option<String>,
    pub adresse: Option<String>,
    pub uai: Option<String>,
    pub degres: Option<Vec<String>>,
    pub couleur: Option<String>,
    pub fuseau_horaire: Option<String>,
}

fn couleur_valide(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

async fn fuseau_connu(conn: &mut PgConnection, f: &str) -> bool {
    sqlx::query("SELECT timezone($1, TIMESTAMPTZ '2000-01-01 00:00:00+00')")
        .bind(f)
        .fetch_one(conn)
        .await
        .is_ok()
}

async fn lire_site(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<SiteDetail, EntDbError> {
    sqlx::query_as::<_, SiteDetail>(&format!("{SQL_SITE} WHERE s.etablissement_id = $1 AND s.id = $2"))
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

fn json_site(s: &SiteDetail) -> Value {
    json!({ "code": s.code, "libelle": s.libelle, "adresse": s.adresse, "uai": s.uai, "degres": s.degres,
            "couleur": s.couleur, "fuseau_horaire": s.fuseau_horaire, "ferme_le": s.ferme_le })
}

impl PgEnt {
    pub async fn structure_sites(&self, a: &Acteur<'_>) -> Result<Vec<SiteDetail>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let rows = sqlx::query_as::<_, SiteDetail>(&format!("{SQL_SITE} WHERE s.etablissement_id = $1 ORDER BY s.code"))
            .bind(a.etablissement_id)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    /// STR-05 : modifie un site (UAI au format national, degrés, couleur, exception de fuseau).
    pub async fn structure_site_modifier(
        &self,
        a: &Acteur<'_>,
        site_id: Uuid,
        m: SiteModif,
    ) -> Result<SiteDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let libelle = m.libelle.as_deref().map(|l| requis(l, 120)).transpose()?;
        let adresse = m.adresse.as_deref().map(|v| net(Some(v), 400)).transpose()?;
        let uai = m
            .uai
            .as_deref()
            .map(|v| net(Some(v), 8).map(|o| o.map(|u| u.to_ascii_uppercase())))
            .transpose()?;
        if uai.as_ref().and_then(|o| o.as_deref()).is_some_and(|u| !uai_valide(u)) {
            return Err(invalide());
        }
        let degres = match &m.degres {
            Some(d) => {
                if d.iter().any(|x| !DEGRES.contains(&x.as_str())) {
                    return Err(invalide());
                }
                Some(d.iter().cloned().collect::<HashSet<_>>().into_iter().collect::<Vec<_>>())
            }
            None => None,
        };
        let couleur = m.couleur.as_deref().map(|v| net(Some(v), 7)).transpose()?;
        if couleur.as_ref().and_then(|o| o.as_deref()).is_some_and(|c| !couleur_valide(c)) {
            return Err(invalide());
        }
        let fuseau = m.fuseau_horaire.as_deref().map(|v| net(Some(v), 64)).transpose()?;
        let mut t = tx(self, a).await?;
        if let Some(Some(f)) = &fuseau {
            if !fuseau_connu(&mut t, f).await {
                return Err(invalide());
            }
        }
        let avant = lire_site(&mut t, a.etablissement_id, site_id).await?;
        sqlx::query(
            "UPDATE ent_site SET
                libelle = COALESCE($3, libelle),
                adresse = CASE WHEN $4 THEN $5 ELSE adresse END,
                uai = CASE WHEN $6 THEN $7 ELSE uai END,
                degres = COALESCE($8, degres),
                couleur = CASE WHEN $9 THEN $10 ELSE couleur END,
                fuseau_horaire = CASE WHEN $11 THEN $12 ELSE fuseau_horaire END
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(site_id)
        .bind(&libelle)
        .bind(adresse.is_some())
        .bind(adresse.clone().flatten())
        .bind(uai.is_some())
        .bind(uai.clone().flatten())
        .bind(&degres)
        .bind(couleur.is_some())
        .bind(couleur.clone().flatten())
        .bind(fuseau.is_some())
        .bind(fuseau.clone().flatten())
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let apres = lire_site(&mut t, a.etablissement_id, site_id).await?;
        journaliser(&mut t, a, "site", &site_id.to_string(), "modifie", Some(json_site(&avant)), Some(json_site(&apres)))
            .await?;
        let ev = publier(&mut t, a, "referentiel.site_modifie", site_id, json!({ "code": apres.code })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// STR-05 : ferme un site (date). Refusé s'il reste des classes ouvertes, des bâtiments ou
    /// des salles ouverts : on les ferme ou on les déplace d'abord.
    pub async fn structure_site_fermer(
        &self,
        a: &Acteur<'_>,
        site_id: Uuid,
        ferme_le: NaiveDate,
    ) -> Result<SiteDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let avant = lire_site(&mut t, a.etablissement_id, site_id).await?;
        if avant.ferme_le.is_some() {
            return Err(conflit());
        }
        let bloquants: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM ent_classe c
                       JOIN ent_annee_scolaire y ON y.id = c.annee_id
                      WHERE c.etablissement_id = $1 AND c.site_id = $2
                        AND c.archivee_le IS NULL AND y.cloturee_le IS NULL)
                  + (SELECT count(*) FROM ent_salle r WHERE r.etablissement_id = $1 AND r.site_id = $2
                        AND r.ferme_le IS NULL)
                  + (SELECT count(*) FROM ent_batiment b WHERE b.etablissement_id = $1 AND b.site_id = $2
                        AND b.ferme_le IS NULL)",
        )
        .bind(a.etablissement_id)
        .bind(site_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if bloquants > 0 {
            return Err(conflit());
        }
        sqlx::query("UPDATE ent_site SET ferme_le = $3, actif = false WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(site_id)
            .bind(ferme_le)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        let apres = lire_site(&mut t, a.etablissement_id, site_id).await?;
        journaliser(&mut t, a, "site", &site_id.to_string(), "ferme", Some(json_site(&avant)), Some(json_site(&apres)))
            .await?;
        let ev = publier(&mut t, a, "referentiel.site_ferme", site_id, json!({ "code": apres.code })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// Rouvre un site fermé (correction d'une fermeture saisie par erreur).
    pub async fn structure_site_rouvrir(&self, a: &Acteur<'_>, site_id: Uuid) -> Result<SiteDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let avant = lire_site(&mut t, a.etablissement_id, site_id).await?;
        if avant.ferme_le.is_none() {
            return Err(conflit());
        }
        sqlx::query("UPDATE ent_site SET ferme_le = NULL, actif = true WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(site_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        let apres = lire_site(&mut t, a.etablissement_id, site_id).await?;
        journaliser(&mut t, a, "site", &site_id.to_string(), "rouvert", Some(json_site(&avant)), Some(json_site(&apres)))
            .await?;
        t.commit().await.map_err(db)?;
        Ok(apres)
    }
}

// ───────────────────────── Bâtiments (STR-06) ─────────────────────────

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Batiment {
    pub id: Uuid,
    pub site_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub ferme_le: Option<NaiveDate>,
}

impl PgEnt {
    pub async fn structure_batiments(&self, a: &Acteur<'_>, site_id: Option<Uuid>) -> Result<Vec<Batiment>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let rows = sqlx::query_as::<_, Batiment>(
            "SELECT id, site_id, code, libelle, ferme_le FROM ent_batiment
              WHERE etablissement_id = $1 AND ($2::uuid IS NULL OR site_id = $2) ORDER BY site_id, code",
        )
        .bind(a.etablissement_id)
        .bind(site_id)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    pub async fn structure_batiment_creer(
        &self,
        a: &Acteur<'_>,
        site_id: Uuid,
        code: &str,
        libelle: &str,
    ) -> Result<Batiment, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let (code, libelle) = (requis(code, 40)?.to_ascii_uppercase(), requis(libelle, 120)?);
        let mut t = tx(self, a).await?;
        let site = lire_site(&mut t, a.etablissement_id, site_id).await?;
        if site.ferme_le.is_some() {
            return Err(conflit());
        }
        let b = sqlx::query_as::<_, Batiment>(
            "INSERT INTO ent_batiment (id, etablissement_id, site_id, code, libelle, created_by)
             VALUES ($1,$2,$3,$4,$5,$6) RETURNING id, site_id, code, libelle, ferme_le",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(site_id)
        .bind(&code)
        .bind(&libelle)
        .bind(a.user_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        journaliser(&mut t, a, "batiment", &b.id.to_string(), "cree", None, Some(json!({ "site": site.code, "code": b.code })))
            .await?;
        let ev = publier(&mut t, a, "referentiel.batiment_cree", b.id, json!({ "code": b.code })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(b)
    }

    /// Ferme (date) un bâtiment ; refusé s'il reste des salles ouvertes dedans.
    pub async fn structure_batiment_fermer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        ferme_le: NaiveDate,
    ) -> Result<Batiment, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let ouvertes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_salle WHERE etablissement_id = $1 AND batiment_id = $2 AND ferme_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if ouvertes > 0 {
            return Err(conflit());
        }
        let b = sqlx::query_as::<_, Batiment>(
            "UPDATE ent_batiment SET ferme_le = COALESCE(ferme_le, $3)
              WHERE etablissement_id = $1 AND id = $2 RETURNING id, site_id, code, libelle, ferme_le",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(ferme_le)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        journaliser(&mut t, a, "batiment", &id.to_string(), "ferme", None, Some(json!({ "ferme_le": b.ferme_le }))).await?;
        let ev = publier(&mut t, a, "referentiel.batiment_ferme", id, json!({ "code": b.code })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(b)
    }
}

// ───────────────────────── Lieux extérieurs (STR-109) ─────────────────────────

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Lieu {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub adresse: Option<String>,
    pub proprietaire: Option<String>,
    pub convention_document_id: Option<Uuid>,
    pub trajet_minutes: Option<i16>,
    pub mode_trajet: Option<String>,
    pub ferme_le: Option<NaiveDate>,
}

const SQL_LIEU: &str = "SELECT id, code, libelle, adresse, proprietaire, convention_document_id, trajet_minutes,
        mode_trajet, ferme_le FROM ent_lieu";

#[derive(Debug, Clone, Deserialize)]
pub struct LieuSaisie {
    pub code: String,
    pub libelle: String,
    pub adresse: Option<String>,
    pub proprietaire: Option<String>,
    pub convention_document_id: Option<Uuid>,
    pub trajet_minutes: Option<i16>,
    pub mode_trajet: Option<String>,
}

fn json_lieu(l: &Lieu) -> Value {
    json!({ "code": l.code, "libelle": l.libelle, "adresse": l.adresse, "proprietaire": l.proprietaire,
            "trajet_minutes": l.trajet_minutes, "mode_trajet": l.mode_trajet, "ferme_le": l.ferme_le })
}

fn valider_lieu(s: &LieuSaisie) -> Result<(String, String, Option<String>, Option<String>, Option<String>), EntDbError> {
    if s.trajet_minutes.is_some_and(|m| !(0..=240).contains(&m)) {
        return Err(invalide());
    }
    let mode = net(s.mode_trajet.as_deref(), 20)?;
    if mode.as_deref().is_some_and(|m| !MODES_TRAJET.contains(&m)) {
        return Err(invalide());
    }
    Ok((
        requis(&s.code, 40)?.to_ascii_uppercase(),
        requis(&s.libelle, 120)?,
        net(s.adresse.as_deref(), 400)?,
        net(s.proprietaire.as_deref(), 200)?,
        mode,
    ))
}

impl PgEnt {
    pub async fn structure_lieux(&self, a: &Acteur<'_>) -> Result<Vec<Lieu>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let rows = sqlx::query_as::<_, Lieu>(&format!("{SQL_LIEU} WHERE etablissement_id = $1 ORDER BY code"))
            .bind(a.etablissement_id)
            .fetch_all(&mut *t)
            .await
            .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    pub async fn structure_lieu_creer(&self, a: &Acteur<'_>, s: LieuSaisie) -> Result<Lieu, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let (code, libelle, adresse, proprio, mode) = valider_lieu(&s)?;
        let mut t = tx(self, a).await?;
        let l = sqlx::query_as::<_, Lieu>(&format!(
            "INSERT INTO ent_lieu (id, etablissement_id, code, libelle, adresse, proprietaire,
                    convention_document_id, trajet_minutes, mode_trajet, created_by)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             RETURNING id, code, libelle, adresse, proprietaire, convention_document_id, trajet_minutes,
                       mode_trajet, ferme_le"
        ))
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(&code)
        .bind(&libelle)
        .bind(&adresse)
        .bind(&proprio)
        .bind(s.convention_document_id)
        .bind(s.trajet_minutes)
        .bind(&mode)
        .bind(a.user_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        journaliser(&mut t, a, "lieu", &l.id.to_string(), "cree", None, Some(json_lieu(&l))).await?;
        let ev = publier(&mut t, a, "referentiel.lieu_cree", l.id, json!({ "code": l.code })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(l)
    }

    /// Remplace les champs d'un lieu (le code ne change pas : il identifie le lieu dans les imports).
    pub async fn structure_lieu_modifier(&self, a: &Acteur<'_>, id: Uuid, s: LieuSaisie) -> Result<Lieu, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let (code, libelle, adresse, proprio, mode) = valider_lieu(&s)?;
        let mut t = tx(self, a).await?;
        let avant = sqlx::query_as::<_, Lieu>(&format!("{SQL_LIEU} WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"))
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)?;
        if avant.code != code {
            return Err(invalide());
        }
        let l = sqlx::query_as::<_, Lieu>(
            "UPDATE ent_lieu SET libelle = $3, adresse = $4, proprietaire = $5, convention_document_id = $6,
                    trajet_minutes = $7, mode_trajet = $8
              WHERE etablissement_id = $1 AND id = $2
              RETURNING id, code, libelle, adresse, proprietaire, convention_document_id, trajet_minutes,
                        mode_trajet, ferme_le",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&libelle)
        .bind(&adresse)
        .bind(&proprio)
        .bind(s.convention_document_id)
        .bind(s.trajet_minutes)
        .bind(&mode)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        journaliser(&mut t, a, "lieu", &id.to_string(), "modifie", Some(json_lieu(&avant)), Some(json_lieu(&l))).await?;
        let ev = publier(&mut t, a, "referentiel.lieu_modifie", id, json!({ "code": l.code })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(l)
    }

    /// Ferme un lieu ; refusé s'il reste des salles ouvertes qui en dépendent.
    pub async fn structure_lieu_fermer(&self, a: &Acteur<'_>, id: Uuid, ferme_le: NaiveDate) -> Result<Lieu, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let ouvertes: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_salle WHERE etablissement_id = $1 AND lieu_id = $2 AND ferme_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if ouvertes > 0 {
            return Err(conflit());
        }
        let l = sqlx::query_as::<_, Lieu>(
            "UPDATE ent_lieu SET ferme_le = COALESCE(ferme_le, $3) WHERE etablissement_id = $1 AND id = $2
              RETURNING id, code, libelle, adresse, proprietaire, convention_document_id, trajet_minutes,
                        mode_trajet, ferme_le",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(ferme_le)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        journaliser(&mut t, a, "lieu", &id.to_string(), "ferme", None, Some(json!({ "ferme_le": l.ferme_le }))).await?;
        let ev = publier(&mut t, a, "referentiel.lieu_ferme", id, json!({ "code": l.code })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(l)
    }
}

// ───────────────────────── Salles (STR-07) ─────────────────────────

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SalleDetail {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub site_id: Option<Uuid>,
    pub site_code: Option<String>,
    pub batiment_id: Option<Uuid>,
    pub batiment_code: Option<String>,
    pub lieu_id: Option<Uuid>,
    pub lieu_code: Option<String>,
    pub etage: Option<String>,
    pub type_salle: String,
    pub capacite: Option<i32>,
    pub equipements: Vec<String>,
    pub accessible_pmr: Option<bool>,
    pub reservable: bool,
    pub actif: bool,
    pub ferme_le: Option<NaiveDate>,
    /// Salle historique jamais rattachée à un site : à corriger par la gestion.
    pub sans_site: bool,
}

const SQL_SALLE: &str = "SELECT r.id, r.code, r.libelle, r.site_id, s.code AS site_code, r.batiment_id,
        b.code AS batiment_code, r.lieu_id, l.code AS lieu_code, r.etage, r.type AS type_salle, r.capacite,
        r.equipements, r.accessible_pmr, r.reservable, r.actif, r.ferme_le, (r.site_id IS NULL) AS sans_site
   FROM ent_salle r
   LEFT JOIN ent_site s ON s.id = r.site_id AND s.etablissement_id = r.etablissement_id
   LEFT JOIN ent_batiment b ON b.id = r.batiment_id AND b.etablissement_id = r.etablissement_id
   LEFT JOIN ent_lieu l ON l.id = r.lieu_id AND l.etablissement_id = r.etablissement_id";

/// Création ou remplacement d'une salle.
#[derive(Debug, Clone, Deserialize)]
pub struct SalleSaisie {
    pub code: String,
    pub libelle: String,
    pub site_id: Uuid,
    pub batiment_id: Option<Uuid>,
    pub lieu_id: Option<Uuid>,
    pub etage: Option<String>,
    #[serde(default = "type_defaut")]
    pub type_salle: String,
    pub capacite: Option<i32>,
    #[serde(default)]
    pub equipements: Vec<String>,
    pub accessible_pmr: Option<bool>,
    /// Absent : valeur du paramètre `structure.salles_reservables_par_defaut`.
    pub reservable: Option<bool>,
}

fn type_defaut() -> String {
    "banalisee".into()
}

struct SalleValide {
    code: String,
    libelle: String,
    etage: Option<String>,
    type_salle: String,
    capacite: Option<i32>,
    equipements: Vec<String>,
}

fn valider_salle(s: &SalleSaisie) -> Result<SalleValide, EntDbError> {
    let type_salle = s.type_salle.trim().to_string();
    if !TYPES_SALLE.contains(&type_salle.as_str()) || s.capacite.is_some_and(|c| !(0..=500).contains(&c)) {
        return Err(invalide());
    }
    // Un lieu extérieur n'accueille que des salles « exterieur ».
    if s.lieu_id.is_some() && type_salle != "exterieur" {
        return Err(invalide());
    }
    let mut equipements: Vec<String> = Vec::new();
    for e in &s.equipements {
        let Some(e) = net(Some(e), 60)? else { continue };
        if !equipements.contains(&e) {
            equipements.push(e);
        }
    }
    if equipements.len() > 40 {
        return Err(invalide());
    }
    Ok(SalleValide {
        code: requis(&s.code, 32)?.to_ascii_uppercase(),
        libelle: requis(&s.libelle, 120)?,
        etage: net(s.etage.as_deref(), 20)?,
        type_salle,
        capacite: s.capacite,
        equipements,
    })
}

fn json_salle(s: &SalleDetail) -> Value {
    json!({ "code": s.code, "libelle": s.libelle, "site": s.site_code, "batiment": s.batiment_code,
            "lieu": s.lieu_code, "etage": s.etage, "type": s.type_salle, "capacite": s.capacite,
            "equipements": s.equipements, "accessible_pmr": s.accessible_pmr, "reservable": s.reservable,
            "ferme_le": s.ferme_le })
}

async fn lire_salle(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<SalleDetail, EntDbError> {
    sqlx::query_as::<_, SalleDetail>(&format!("{SQL_SALLE} WHERE r.etablissement_id = $1 AND r.id = $2"))
        .bind(etab)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
}

/// Le site existe dans l'établissement et est ouvert ; le bâtiment est de ce site et ouvert ; le
/// lieu existe et est ouvert. Sinon refus (jamais de création implicite).
async fn verifier_rattachements(
    conn: &mut PgConnection,
    etab: Uuid,
    site_id: Uuid,
    batiment_id: Option<Uuid>,
    lieu_id: Option<Uuid>,
) -> Result<(), EntDbError> {
    let site: Option<Option<NaiveDate>> =
        sqlx::query_scalar("SELECT ferme_le FROM ent_site WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(site_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
    match site {
        None => return Err(introuvable()),
        Some(Some(_)) => return Err(conflit()),
        Some(None) => {}
    }
    if let Some(b) = batiment_id {
        let r: Option<Option<NaiveDate>> = sqlx::query_scalar(
            "SELECT ferme_le FROM ent_batiment WHERE etablissement_id = $1 AND id = $2 AND site_id = $3",
        )
        .bind(etab)
        .bind(b)
        .bind(site_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        match r {
            None => return Err(invalide()),
            Some(Some(_)) => return Err(conflit()),
            Some(None) => {}
        }
    }
    if let Some(l) = lieu_id {
        let r: Option<Option<NaiveDate>> =
            sqlx::query_scalar("SELECT ferme_le FROM ent_lieu WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(l)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
        match r {
            None => return Err(introuvable()),
            Some(Some(_)) => return Err(conflit()),
            Some(None) => {}
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FiltreSalles {
    pub site_id: Option<Uuid>,
    #[serde(default)]
    pub inclure_fermees: bool,
}

impl PgEnt {
    pub async fn structure_salles(&self, a: &Acteur<'_>, f: &FiltreSalles) -> Result<Vec<SalleDetail>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let rows = sqlx::query_as::<_, SalleDetail>(&format!(
            "{SQL_SALLE} WHERE r.etablissement_id = $1 AND ($2::uuid IS NULL OR r.site_id = $2)
                AND ($3 OR r.ferme_le IS NULL) ORDER BY r.code"
        ))
        .bind(a.etablissement_id)
        .bind(f.site_id)
        .bind(f.inclure_fermees)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    /// STR-07 : crée une salle typée, liée à un site existant de l'établissement.
    pub async fn structure_salle_creer(&self, a: &Acteur<'_>, s: SalleSaisie) -> Result<SalleDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let v = valider_salle(&s)?;
        let mut t = tx(self, a).await?;
        verifier_rattachements(&mut t, a.etablissement_id, s.site_id, s.batiment_id, s.lieu_id).await?;
        let reservable = match s.reservable {
            Some(r) => r,
            None => structure::parametre(&mut t, a.etablissement_id, "structure.salles_reservables_par_defaut")
                .await?
                .as_bool()
                .unwrap_or(false),
        };
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_salle (id, etablissement_id, code, libelle, capacite, actif, created_by_user_id,
                    site_id, batiment_id, lieu_id, etage, type, equipements, accessible_pmr, reservable)
             VALUES ($1,$2,$3,$4,$5,true,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(&v.code)
        .bind(&v.libelle)
        .bind(v.capacite)
        .bind(a.user_id)
        .bind(s.site_id)
        .bind(s.batiment_id)
        .bind(s.lieu_id)
        .bind(&v.etage)
        .bind(&v.type_salle)
        .bind(&v.equipements)
        .bind(s.accessible_pmr)
        .bind(reservable)
        .execute(&mut *t)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref d) if d.is_unique_violation() => conflit(),
            e => db(e),
        })?;
        let apres = lire_salle(&mut t, a.etablissement_id, id).await?;
        journaliser(&mut t, a, "salle", &id.to_string(), "cree", None, Some(json_salle(&apres))).await?;
        let ev = publier(&mut t, a, "referentiel.salle_creee", id, json!({ "code": apres.code, "type": apres.type_salle }))
            .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// STR-07 : remplace les attributs d'une salle (le code ne change pas). Changer de site
    /// exige de redire le bâtiment.
    pub async fn structure_salle_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        s: SalleSaisie,
    ) -> Result<SalleDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let v = valider_salle(&s)?;
        let mut t = tx(self, a).await?;
        let avant = lire_salle(&mut t, a.etablissement_id, id).await?;
        if avant.code != v.code {
            return Err(invalide());
        }
        // Ne revérifie l'ouverture que des rattachements qui changent : une salle peut garder
        // un site/bâtiment fermé depuis (elle est alors elle-même à fermer).
        if avant.site_id != Some(s.site_id) || avant.batiment_id != s.batiment_id || avant.lieu_id != s.lieu_id {
            verifier_rattachements(&mut t, a.etablissement_id, s.site_id, s.batiment_id, s.lieu_id).await?;
        }
        sqlx::query(
            "UPDATE ent_salle SET libelle = $3, capacite = $4, site_id = $5, batiment_id = $6, lieu_id = $7,
                    etage = $8, type = $9, equipements = $10, accessible_pmr = $11,
                    reservable = COALESCE($12, reservable)
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&v.libelle)
        .bind(v.capacite)
        .bind(s.site_id)
        .bind(s.batiment_id)
        .bind(s.lieu_id)
        .bind(&v.etage)
        .bind(&v.type_salle)
        .bind(&v.equipements)
        .bind(s.accessible_pmr)
        .bind(s.reservable)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let apres = lire_salle(&mut t, a.etablissement_id, id).await?;
        journaliser(&mut t, a, "salle", &id.to_string(), "modifie", Some(json_salle(&avant)), Some(json_salle(&apres)))
            .await?;
        let ev = publier(&mut t, a, "referentiel.salle_modifiee", id, json!({ "code": apres.code })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// STR-07 : ferme une salle (date). Refusé si des créneaux hebdomadaires l'utilisent encore :
    /// il faut les déplacer d'abord (le socle 10 proposera le déplacement).
    pub async fn structure_salle_fermer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        ferme_le: NaiveDate,
    ) -> Result<SalleDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let avant = lire_salle(&mut t, a.etablissement_id, id).await?;
        if avant.ferme_le.is_some() {
            return Err(conflit());
        }
        let creneaux: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_edt_creneau WHERE etablissement_id = $1 AND lower(salle) = lower($2)",
        )
        .bind(a.etablissement_id)
        .bind(&avant.code)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        if creneaux > 0 {
            return Err(conflit());
        }
        sqlx::query("UPDATE ent_salle SET ferme_le = $3, actif = false WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(id)
            .bind(ferme_le)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        let apres = lire_salle(&mut t, a.etablissement_id, id).await?;
        journaliser(&mut t, a, "salle", &id.to_string(), "ferme", Some(json_salle(&avant)), Some(json_salle(&apres)))
            .await?;
        let ev = publier(&mut t, a, "referentiel.salle_fermee", id, json!({ "code": apres.code })).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// Rattache une salle historique (sans site) à un site existant. Refuse si elle en a déjà un.
    pub async fn structure_salle_rattacher(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        site_id: Uuid,
    ) -> Result<SalleDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let avant = lire_salle(&mut t, a.etablissement_id, id).await?;
        if avant.site_id.is_some() {
            return Err(conflit());
        }
        verifier_rattachements(&mut t, a.etablissement_id, site_id, None, None).await?;
        sqlx::query("UPDATE ent_salle SET site_id = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(id)
            .bind(site_id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        let apres = lire_salle(&mut t, a.etablissement_id, id).await?;
        journaliser(&mut t, a, "salle", &id.to_string(), "rattachee", Some(json_salle(&avant)), Some(json_salle(&apres)))
            .await?;
        t.commit().await.map_err(db)?;
        Ok(apres)
    }
}

// ───────────────────────── Disponibilités et indisponibilités ─────────────────────────

/// Cible d'une disponibilité ou d'une indisponibilité.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum Cible {
    Salle(Uuid),
    Lieu(Uuid),
}

impl Cible {
    fn colonnes(self) -> (Option<Uuid>, Option<Uuid>) {
        match self {
            Cible::Salle(i) => (Some(i), None),
            Cible::Lieu(i) => (None, Some(i)),
        }
    }
}

async fn cible_existe(conn: &mut PgConnection, etab: Uuid, c: Cible) -> Result<(), EntDbError> {
    let (table, id) = match c {
        Cible::Salle(i) => ("ent_salle", i),
        Cible::Lieu(i) => ("ent_lieu", i),
    };
    let ok: bool = sqlx::query_scalar(&format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE etablissement_id = $1 AND id = $2)"))
        .bind(etab)
        .bind(id)
        .fetch_one(conn)
        .await
        .map_err(db)?;
    if ok {
        Ok(())
    } else {
        Err(introuvable())
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Disponibilite {
    pub id: Uuid,
    pub salle_id: Option<Uuid>,
    pub lieu_id: Option<Uuid>,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub libelle: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DisponibiliteSaisie {
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub du: Option<NaiveDate>,
    pub au: Option<NaiveDate>,
    pub libelle: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Indisponibilite {
    pub id: Uuid,
    pub salle_id: Option<Uuid>,
    pub lieu_id: Option<Uuid>,
    pub du: DateTime<Utc>,
    pub au: DateTime<Utc>,
    pub motif_code: String,
    pub motif: Option<String>,
    pub lot_id: Option<Uuid>,
    pub evenement_ref: Option<Uuid>,
    pub annulee_le: Option<DateTime<Utc>>,
}

const SQL_INDISPO: &str = "SELECT id, salle_id, lieu_id, du, au, motif_code, motif, lot_id, evenement_ref, annulee_le
    FROM ent_salle_indisponibilite";

/// Résultat d'une indisponibilité déclarée : l'enregistrement et les séances à revoir.
#[derive(Debug, Clone, Serialize)]
pub struct RapportIndisponibilite {
    pub lot_id: Option<Uuid>,
    pub indisponibilites: Vec<Indisponibilite>,
    /// Créneaux hebdomadaires qui occupent ces salles un jour de la période : à signaler au
    /// socle 10 (emploi du temps) pour déplacement. Informatif, aucune séance n'est modifiée ici.
    pub creneaux_touches: i64,
}

fn motif_valide(code: &str, motif: Option<&str>) -> Result<Option<String>, EntDbError> {
    if !MOTIFS_INDISPO.contains(&code) {
        return Err(invalide());
    }
    net(motif, 300)
}

impl PgEnt {
    pub async fn structure_disponibilites(&self, a: &Acteur<'_>, c: Cible) -> Result<Vec<Disponibilite>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let (salle, lieu) = c.colonnes();
        let mut t = tx(self, a).await?;
        let rows = sqlx::query_as::<_, Disponibilite>(
            "SELECT id, salle_id, lieu_id, jour_semaine, heure_debut, heure_fin, du, au, libelle
               FROM ent_disponibilite
              WHERE etablissement_id = $1 AND annulee_le IS NULL
                AND salle_id IS NOT DISTINCT FROM $2 AND lieu_id IS NOT DISTINCT FROM $3
              ORDER BY jour_semaine, heure_debut",
        )
        .bind(a.etablissement_id)
        .bind(salle)
        .bind(lieu)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    /// STR-109 : ajoute une fenêtre de disponibilité hebdomadaire (salle ou lieu).
    pub async fn structure_disponibilite_ajouter(
        &self,
        a: &Acteur<'_>,
        c: Cible,
        s: DisponibiliteSaisie,
    ) -> Result<Disponibilite, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if !(1..=7).contains(&s.jour_semaine)
            || s.heure_fin <= s.heure_debut
            || matches!((s.du, s.au), (Some(d), Some(f)) if f < d)
        {
            return Err(invalide());
        }
        let libelle = net(s.libelle.as_deref(), 120)?;
        let (salle, lieu) = c.colonnes();
        let mut t = tx(self, a).await?;
        cible_existe(&mut t, a.etablissement_id, c).await?;
        let d = sqlx::query_as::<_, Disponibilite>(
            "INSERT INTO ent_disponibilite (id, etablissement_id, salle_id, lieu_id, jour_semaine, heure_debut,
                    heure_fin, du, au, libelle, created_by)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
             RETURNING id, salle_id, lieu_id, jour_semaine, heure_debut, heure_fin, du, au, libelle",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(salle)
        .bind(lieu)
        .bind(s.jour_semaine)
        .bind(s.heure_debut)
        .bind(s.heure_fin)
        .bind(s.du)
        .bind(s.au)
        .bind(&libelle)
        .bind(a.user_id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        journaliser(
            &mut t,
            a,
            "disponibilite",
            &d.id.to_string(),
            "cree",
            None,
            Some(json!({ "cible": c, "jour": d.jour_semaine, "debut": d.heure_debut, "fin": d.heure_fin })),
        )
        .await?;
        t.commit().await.map_err(db)?;
        Ok(d)
    }

    pub async fn structure_disponibilite_annuler(&self, a: &Acteur<'_>, id: Uuid) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let n = sqlx::query(
            "UPDATE ent_disponibilite SET annulee_le = COALESCE(annulee_le, now())
              WHERE etablissement_id = $1 AND id = $2",
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
        journaliser(&mut t, a, "disponibilite", &id.to_string(), "annule", None, None).await?;
        t.commit().await.map_err(db)?;
        Ok(())
    }

    pub async fn structure_indisponibilites(
        &self,
        a: &Acteur<'_>,
        c: Cible,
        depuis: DateTime<Utc>,
    ) -> Result<Vec<Indisponibilite>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let (salle, lieu) = c.colonnes();
        let mut t = tx(self, a).await?;
        let rows = sqlx::query_as::<_, Indisponibilite>(&format!(
            "{SQL_INDISPO} WHERE etablissement_id = $1 AND salle_id IS NOT DISTINCT FROM $2
                AND lieu_id IS NOT DISTINCT FROM $3 AND au > $4 ORDER BY du"
        ))
        .bind(a.etablissement_id)
        .bind(salle)
        .bind(lieu)
        .bind(depuis)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    /// STR-09 / STR-109 : déclare une indisponibilité datée (travaux du gymnase municipal, salle
    /// en travaux). Une seule cible ; pour plusieurs salles, voir [`Self::structure_indisponibilite_lot`].
    pub async fn structure_indisponibilite_declarer(
        &self,
        a: &Acteur<'_>,
        c: Cible,
        du: DateTime<Utc>,
        au: DateTime<Utc>,
        motif_code: &str,
        motif: Option<&str>,
    ) -> Result<RapportIndisponibilite, EntDbError> {
        let ids = match c {
            Cible::Salle(i) => (vec![i], None),
            Cible::Lieu(i) => (vec![], Some(i)),
        };
        self.indisponibilite_commune(a, ids.0, ids.1, du, au, motif_code, motif, None, false).await
    }

    /// STR-110 : rend plusieurs salles indisponibles en un geste (examens, réquisition), liées
    /// entre elles par un même `lot_id` et, si on la donne, par la référence de l'événement du
    /// calendrier. Tout ou rien : une salle inconnue annule le lot.
    #[allow(clippy::too_many_arguments)]
    pub async fn structure_indisponibilite_lot(
        &self,
        a: &Acteur<'_>,
        salles: Vec<Uuid>,
        du: DateTime<Utc>,
        au: DateTime<Utc>,
        motif_code: &str,
        motif: Option<&str>,
        evenement_ref: Option<Uuid>,
    ) -> Result<RapportIndisponibilite, EntDbError> {
        self.indisponibilite_commune(a, salles, None, du, au, motif_code, motif, evenement_ref, true).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn indisponibilite_commune(
        &self,
        a: &Acteur<'_>,
        salles: Vec<Uuid>,
        lieu: Option<Uuid>,
        du: DateTime<Utc>,
        au: DateTime<Utc>,
        motif_code: &str,
        motif: Option<&str>,
        evenement_ref: Option<Uuid>,
        en_lot: bool,
    ) -> Result<RapportIndisponibilite, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let motif = motif_valide(motif_code, motif)?;
        let distinctes: HashSet<Uuid> = salles.iter().copied().collect();
        if au <= du || (distinctes.is_empty() && lieu.is_none()) || distinctes.len() > 200 {
            return Err(invalide());
        }
        let lot_id = en_lot.then(Uuid::new_v4);
        let mut t = tx(self, a).await?;
        let mut cibles: Vec<Cible> = distinctes.iter().map(|i| Cible::Salle(*i)).collect();
        cibles.sort_by_key(|c| format!("{c:?}"));
        if let Some(l) = lieu {
            cibles.push(Cible::Lieu(l));
        }
        let mut crees = Vec::new();
        for c in &cibles {
            cible_existe(&mut t, a.etablissement_id, *c).await?;
            let (s, l) = c.colonnes();
            let i = sqlx::query_as::<_, Indisponibilite>(
                "INSERT INTO ent_salle_indisponibilite (id, etablissement_id, salle_id, lieu_id, du, au,
                        motif_code, motif, lot_id, evenement_ref, par)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                 RETURNING id, salle_id, lieu_id, du, au, motif_code, motif, lot_id, evenement_ref, annulee_le",
            )
            .bind(Uuid::new_v4())
            .bind(a.etablissement_id)
            .bind(s)
            .bind(l)
            .bind(du)
            .bind(au)
            .bind(motif_code)
            .bind(&motif)
            .bind(lot_id)
            .bind(evenement_ref)
            .bind(a.user_id)
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
            journaliser(
                &mut t,
                a,
                "indisponibilite",
                &i.id.to_string(),
                "declaree",
                None,
                Some(json!({ "cible": c, "du": du, "au": au, "motif_code": motif_code, "lot_id": lot_id })),
            )
            .await?;
            crees.push(i);
        }
        let creneaux_touches = creneaux_touches(&mut t, a.etablissement_id, &distinctes, du, au).await?;
        // Aucun motif libre dans l'événement : il peut décrire une situation sensible.
        let ev = publier(
            &mut t,
            a,
            "referentiel.salle_indisponible",
            lot_id.unwrap_or_else(|| crees[0].id),
            json!({ "lot_id": lot_id, "nombre": crees.len(), "motif_code": motif_code, "du": du, "au": au }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(RapportIndisponibilite { lot_id, indisponibilites: crees, creneaux_touches })
    }

    /// Annule une indisponibilité (jamais supprimée) ; si elle appartient à un lot, `tout_le_lot`
    /// annule les autres salles du lot avec elle.
    pub async fn structure_indisponibilite_annuler(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        tout_le_lot: bool,
    ) -> Result<u64, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let lot: Option<Option<Uuid>> = sqlx::query_scalar(
            "SELECT lot_id FROM ent_salle_indisponibilite WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_optional(&mut *t)
        .await
        .map_err(db)?;
        let Some(lot) = lot else { return Err(introuvable()) };
        let lot = if tout_le_lot { lot } else { None };
        let n = sqlx::query(
            "UPDATE ent_salle_indisponibilite SET annulee_le = COALESCE(annulee_le, now()),
                    annulee_par = COALESCE(annulee_par, $4)
              WHERE etablissement_id = $1 AND annulee_le IS NULL AND (id = $2 OR ($3::uuid IS NOT NULL AND lot_id = $3))",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(lot)
        .bind(a.user_id)
        .execute(&mut *t)
        .await
        .map_err(db)?
        .rows_affected();
        journaliser(&mut t, a, "indisponibilite", &id.to_string(), "annulee", None, Some(json!({ "nombre": n }))).await?;
        t.commit().await.map_err(db)?;
        Ok(n)
    }
}

/// Créneaux hebdomadaires qui occupent l'une des salles un jour de la période (fuseau de
/// l'établissement). Au-delà d'un an, tous les jours de la semaine sont concernés.
async fn creneaux_touches(
    conn: &mut PgConnection,
    etab: Uuid,
    salles: &HashSet<Uuid>,
    du: DateTime<Utc>,
    au: DateTime<Utc>,
) -> Result<i64, EntDbError> {
    if salles.is_empty() {
        return Ok(0);
    }
    let ids: Vec<Uuid> = salles.iter().copied().collect();
    let n: i64 = sqlx::query_scalar(
        "WITH fz AS (SELECT COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1),
                                     'Europe/Paris') AS z),
              bornes AS (SELECT (($3::timestamptz AT TIME ZONE fz.z))::date AS d0,
                                (($4::timestamptz AT TIME ZONE fz.z - interval '1 microsecond'))::date AS d1 FROM fz),
              jours AS (SELECT DISTINCT extract(isodow FROM g)::int AS j
                          FROM bornes, generate_series(d0, LEAST(d1, d0 + 366), interval '1 day') g)
         SELECT count(*) FROM ent_edt_creneau c
           JOIN ent_salle s ON s.etablissement_id = c.etablissement_id AND lower(c.salle) = lower(s.code)
          WHERE c.etablissement_id = $1 AND s.id = ANY($2) AND c.jour_semaine IN (SELECT j FROM jours)",
    )
    .bind(etab)
    .bind(&ids)
    .bind(du)
    .bind(au)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(n)
}

// ───────────────────────── Port de disponibilité (R-S14) ─────────────────────────

/// Port lu par le socle 10 (emploi du temps) et le socle 42 (sorties).
pub mod structure_disponibilite {
    use super::*;

    /// Une salle ou un lieu est-il utilisable de `debut` à `fin` (instants UTC, même jour dans le
    /// fuseau de l'établissement) ? Faux si : fermé à cette date ; indisponibilité non annulée qui
    /// recoupe ; **des fenêtres existent** et aucune ne couvre l'intervalle (jour de semaine,
    /// heures, dates). Une salle d'un lieu extérieur doit aussi satisfaire les règles du lieu.
    pub async fn disponible(
        conn: &mut PgConnection,
        etab: Uuid,
        cible: Cible,
        debut: DateTime<Utc>,
        fin: DateTime<Utc>,
    ) -> Result<bool, EntDbError> {
        if fin <= debut {
            return Err(invalide());
        }
        let d0 = structure::heure_locale(conn, etab, debut).await?;
        let d1 = structure::heure_locale(conn, etab, fin).await?;
        if d0.date() != d1.date() {
            // Un créneau ne traverse pas minuit : intervalle refusé plutôt que deviné.
            return Err(invalide());
        }
        let (jour, h0, h1) = (d0.date(), d0.time(), d1.time());
        let mut cibles = vec![cible];
        if let Cible::Salle(s) = cible {
            let lieu: Option<Uuid> =
                sqlx::query_scalar("SELECT lieu_id FROM ent_salle WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(s)
                    .fetch_optional(&mut *conn)
                    .await
                    .map_err(db)?
                    .ok_or_else(introuvable)?;
            if let Some(l) = lieu {
                cibles.push(Cible::Lieu(l));
            }
        }
        for c in cibles {
            if !cible_disponible(conn, etab, c, debut, fin, jour, h0, h1).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    async fn cible_disponible(
        conn: &mut PgConnection,
        etab: Uuid,
        c: Cible,
        debut: DateTime<Utc>,
        fin: DateTime<Utc>,
        jour: NaiveDate,
        h0: NaiveTime,
        h1: NaiveTime,
    ) -> Result<bool, EntDbError> {
        let (salle, lieu) = c.colonnes();
        let table = if salle.is_some() { "ent_salle" } else { "ent_lieu" };
        let id = salle.or(lieu).ok_or_else(invalide)?;
        let ferme: Option<Option<NaiveDate>> = sqlx::query_scalar(&format!(
            "SELECT ferme_le FROM {table} WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        match ferme {
            None => return Err(introuvable()),
            Some(Some(f)) if f <= jour => return Ok(false),
            _ => {}
        }
        let indispo: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_salle_indisponibilite
                             WHERE etablissement_id = $1 AND annulee_le IS NULL
                               AND salle_id IS NOT DISTINCT FROM $2 AND lieu_id IS NOT DISTINCT FROM $3
                               AND du < $5 AND au > $4)",
        )
        .bind(etab)
        .bind(salle)
        .bind(lieu)
        .bind(debut)
        .bind(fin)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if indispo {
            return Ok(false);
        }
        let (total, couvrant): (i64, i64) = sqlx::query_as(
            "SELECT count(*),
                    count(*) FILTER (WHERE jour_semaine = extract(isodow FROM $4::date)::int
                                       AND heure_debut <= $5 AND heure_fin >= $6
                                       AND (du IS NULL OR du <= $4) AND (au IS NULL OR au >= $4))
               FROM ent_disponibilite
              WHERE etablissement_id = $1 AND annulee_le IS NULL
                AND salle_id IS NOT DISTINCT FROM $2 AND lieu_id IS NOT DISTINCT FROM $3",
        )
        .bind(etab)
        .bind(salle)
        .bind(lieu)
        .bind(jour)
        .bind(h0)
        .bind(h1)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        Ok(total == 0 || couvrant > 0)
    }
}

// ───────────────────────── Import de salles en lot (STR-08) ─────────────────────────

/// Colonnes reconnues (en-tête obligatoire). Toute autre colonne refuse le fichier : on ne perd
/// jamais une information en silence.
const COLONNES_IMPORT: [&str; 11] = [
    "code",
    "libelle",
    "site",
    "batiment",
    "lieu",
    "etage",
    "type",
    "capacite",
    "equipements",
    "accessible_pmr",
    "reservable",
];

#[derive(Debug, Clone, Serialize)]
pub struct RejetImport {
    /// Numéro de ligne dans le fichier (l'en-tête est la ligne 1).
    pub ligne: usize,
    pub code: Option<String>,
    pub raison: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RapportImportSalles {
    /// `true` : rien n'a été écrit, c'est ce qui se passerait.
    pub apercu: bool,
    pub lignes: usize,
    pub creees: usize,
    pub modifiees: usize,
    pub inchangees: usize,
    pub rejets: Vec<RejetImport>,
}

/// Découpe un CSV simple (séparateur `;` ou `,` détecté sur l'en-tête, guillemets doublés).
fn parser_csv(texte: &str) -> Result<Vec<(usize, Vec<String>)>, EntDbError> {
    let texte = texte.strip_prefix('\u{feff}').unwrap_or(texte);
    let premiere = texte.lines().next().unwrap_or("");
    let sep = if premiere.matches(';').count() >= premiere.matches(',').count() && premiere.contains(';') {
        ';'
    } else {
        ','
    };
    let mut lignes = Vec::new();
    let mut champ = String::new();
    let mut ligne: Vec<String> = Vec::new();
    let mut guillemets = false;
    let mut n = 1usize;
    let mut debut_ligne = 1usize;
    let mut chars = texte.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if guillemets && chars.peek() == Some(&'"') => {
                champ.push('"');
                chars.next();
            }
            '"' => guillemets = !guillemets,
            c if c == sep && !guillemets => ligne.push(std::mem::take(&mut champ)),
            '\r' if !guillemets => {}
            '\n' if !guillemets => {
                ligne.push(std::mem::take(&mut champ));
                if !(ligne.len() == 1 && ligne[0].trim().is_empty()) {
                    lignes.push((debut_ligne, std::mem::take(&mut ligne)));
                } else {
                    ligne.clear();
                }
                n += 1;
                debut_ligne = n;
            }
            c => {
                if c == '\n' {
                    n += 1;
                }
                champ.push(c);
            }
        }
    }
    if guillemets {
        return Err(invalide());
    }
    ligne.push(champ);
    if !(ligne.len() == 1 && ligne[0].trim().is_empty()) {
        lignes.push((debut_ligne, ligne));
    }
    Ok(lignes)
}

fn oui_non(v: &str) -> Result<Option<bool>, ()> {
    match v.trim().to_lowercase().as_str() {
        "" => Ok(None),
        "oui" | "o" | "true" | "1" | "x" => Ok(Some(true)),
        "non" | "n" | "false" | "0" => Ok(Some(false)),
        _ => Err(()),
    }
}

fn caracteres_ok(s: &str) -> bool {
    !s.chars().any(|c| c.is_control() && c != '\t')
}

impl PgEnt {
    /// STR-08 : importe ou met à jour des salles depuis un CSV collé (en-tête obligatoire :
    /// `code;libelle;site` + colonnes facultatives `batiment;lieu;etage;type;capacite;equipements;
    /// accessible_pmr;reservable`, équipements séparés par `|`).
    ///
    /// * clé = code de salle (insensible à la casse) : idempotent, un second passage donne
    ///   « inchangées » ; une cellule vide laisse la valeur existante en mise à jour ;
    /// * site, bâtiment et lieu doivent **exister** (par code) : rien n'est créé ;
    /// * `apercu = true` joue exactement la même importation puis l'annule (aucun effet) ;
    /// * les lignes valides sont appliquées, les autres rejetées une à une avec leur raison ;
    /// * un fichier trop gros, sans en-tête ou avec une colonne inconnue est refusé en entier.
    pub async fn structure_salles_importer(
        &self,
        a: &Acteur<'_>,
        csv: &str,
        apercu: bool,
    ) -> Result<RapportImportSalles, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if csv.len() > IMPORT_OCTETS_MAX {
            return Err(invalide());
        }
        let mut lignes = parser_csv(csv)?.into_iter();
        let (_, entete) = lignes.next().ok_or_else(invalide)?;
        let entete: Vec<String> = entete.iter().map(|c| c.trim().to_lowercase()).collect();
        if entete.iter().any(|c| !COLONNES_IMPORT.contains(&c.as_str()))
            || entete.iter().collect::<HashSet<_>>().len() != entete.len()
            || ["code", "libelle", "site"].iter().any(|c| !entete.iter().any(|e| e == c))
        {
            return Err(invalide());
        }
        let corps: Vec<(usize, Vec<String>)> = lignes.collect();
        if corps.len() > IMPORT_LIGNES_MAX {
            return Err(invalide());
        }

        let mut t = tx(self, a).await?;
        let etab = a.etablissement_id;
        let sites: Vec<(Uuid, String, String, Option<NaiveDate>)> =
            sqlx::query_as("SELECT id, code, libelle, ferme_le FROM ent_site WHERE etablissement_id = $1")
                .bind(etab)
                .fetch_all(&mut *t)
                .await
                .map_err(db)?;
        let batiments: Vec<(Uuid, Uuid, String, Option<NaiveDate>)> =
            sqlx::query_as("SELECT id, site_id, code, ferme_le FROM ent_batiment WHERE etablissement_id = $1")
                .bind(etab)
                .fetch_all(&mut *t)
                .await
                .map_err(db)?;
        let lieux: Vec<(Uuid, String, Option<NaiveDate>)> =
            sqlx::query_as("SELECT id, code, ferme_le FROM ent_lieu WHERE etablissement_id = $1")
                .bind(etab)
                .fetch_all(&mut *t)
                .await
                .map_err(db)?;
        let existantes: HashMap<String, SalleDetail> =
            sqlx::query_as::<_, SalleDetail>(&format!("{SQL_SALLE} WHERE r.etablissement_id = $1"))
                .bind(etab)
                .fetch_all(&mut *t)
                .await
                .map_err(db)?
                .into_iter()
                .map(|s| (s.code.to_uppercase(), s))
                .collect();
        let reservable_defaut = structure::parametre(&mut t, etab, "structure.salles_reservables_par_defaut")
            .await?
            .as_bool()
            .unwrap_or(false);

        let mut rapport =
            RapportImportSalles { apercu, lignes: corps.len(), creees: 0, modifiees: 0, inchangees: 0, rejets: vec![] };
        let mut vus: HashSet<String> = HashSet::new();
        for (num, cellules) in corps {
            let cell = |nom: &str| -> String {
                entete.iter().position(|e| e == nom).and_then(|i| cellules.get(i)).map_or(String::new(), |c| c.trim().to_string())
            };
            let code = cell("code").to_uppercase();
            let rejet = |raison: &str| RejetImport { ligne: num, code: (!code.is_empty()).then(|| code.clone()), raison: raison.into() };
            if cellules.len() > entete.len() {
                rapport.rejets.push(rejet("trop de colonnes"));
                continue;
            }
            if cellules.iter().any(|c| !caracteres_ok(c)) {
                rapport.rejets.push(rejet("caractère de commande interdit"));
                continue;
            }
            if code.is_empty() || code.chars().count() > 32 {
                rapport.rejets.push(rejet("code absent ou trop long (32 au plus)"));
                continue;
            }
            if !vus.insert(code.clone()) {
                rapport.rejets.push(rejet("code déjà présent plus haut dans le fichier"));
                continue;
            }
            let existante = existantes.get(&code);
            let libelle = cell("libelle");
            if libelle.chars().count() > 120 || (libelle.is_empty() && existante.is_none()) {
                rapport.rejets.push(rejet("libellé absent ou trop long (120 au plus)"));
                continue;
            }
            let site_txt = cell("site").to_lowercase();
            let site = sites.iter().find(|s| s.1.to_lowercase() == site_txt).or_else(|| sites.iter().find(|s| s.2.to_lowercase() == site_txt));
            let Some(site) = site else {
                rapport.rejets.push(rejet("site inconnu (les sites ne sont jamais créés par un import)"));
                continue;
            };
            if site.3.is_some() {
                rapport.rejets.push(rejet("site fermé"));
                continue;
            }
            let bat_txt = cell("batiment").to_uppercase();
            let batiment_id = if bat_txt.is_empty() {
                None
            } else {
                match batiments.iter().find(|b| b.1 == site.0 && b.2.to_uppercase() == bat_txt) {
                    Some(b) if b.3.is_none() => Some(b.0),
                    Some(_) => {
                        rapport.rejets.push(rejet("bâtiment fermé"));
                        continue;
                    }
                    None => {
                        rapport.rejets.push(rejet("bâtiment inconnu dans ce site"));
                        continue;
                    }
                }
            };
            let lieu_txt = cell("lieu").to_uppercase();
            let lieu_id = if lieu_txt.is_empty() {
                None
            } else {
                match lieux.iter().find(|l| l.1.to_uppercase() == lieu_txt) {
                    Some(l) if l.2.is_none() => Some(l.0),
                    Some(_) => {
                        rapport.rejets.push(rejet("lieu fermé"));
                        continue;
                    }
                    None => {
                        rapport.rejets.push(rejet("lieu inconnu"));
                        continue;
                    }
                }
            };
            let type_txt = cell("type").to_lowercase();
            if !type_txt.is_empty() && !TYPES_SALLE.contains(&type_txt.as_str()) {
                rapport.rejets.push(rejet("type de salle inconnu"));
                continue;
            }
            let capacite = match cell("capacite").as_str() {
                "" => None,
                c => match c.parse::<i32>() {
                    Ok(n) if (0..=500).contains(&n) => Some(n),
                    _ => {
                        rapport.rejets.push(rejet("capacité invalide (0 à 500)"));
                        continue;
                    }
                },
            };
            let (Ok(pmr), Ok(reservable)) = (oui_non(&cell("accessible_pmr")), oui_non(&cell("reservable"))) else {
                rapport.rejets.push(rejet("valeur oui/non attendue"));
                continue;
            };
            let equip_txt = cell("equipements");
            let equipements: Option<Vec<String>> = if equip_txt.is_empty() {
                None
            } else {
                let mut v: Vec<String> = Vec::new();
                for e in equip_txt.split('|').map(str::trim).filter(|e| !e.is_empty()) {
                    if e.chars().count() > 60 {
                        v.clear();
                        break;
                    }
                    if !v.iter().any(|x| x == e) {
                        v.push(e.to_string());
                    }
                }
                if v.is_empty() || v.len() > 40 {
                    rapport.rejets.push(rejet("équipements invalides (60 caractères et 40 éléments au plus)"));
                    continue;
                }
                Some(v)
            };
            let etage = (!cell("etage").is_empty()).then(|| cell("etage"));
            if etage.as_ref().is_some_and(|e| e.chars().count() > 20) {
                rapport.rejets.push(rejet("étage trop long"));
                continue;
            }
            let type_final = if type_txt.is_empty() {
                existante.map_or("banalisee".to_string(), |s| s.type_salle.clone())
            } else {
                type_txt
            };
            if lieu_id.is_some() && type_final != "exterieur" {
                rapport.rejets.push(rejet("une salle de lieu extérieur est de type « exterieur »"));
                continue;
            }
            match existante {
                None => {
                    sqlx::query(
                        "INSERT INTO ent_salle (id, etablissement_id, code, libelle, capacite, actif, created_by_user_id,
                                site_id, batiment_id, lieu_id, etage, type, equipements, accessible_pmr, reservable)
                         VALUES ($1,$2,$3,$4,$5,true,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(etab)
                    .bind(&code)
                    .bind(&libelle)
                    .bind(capacite)
                    .bind(a.user_id)
                    .bind(site.0)
                    .bind(batiment_id)
                    .bind(lieu_id)
                    .bind(&etage)
                    .bind(&type_final)
                    .bind(equipements.clone().unwrap_or_default())
                    .bind(pmr)
                    .bind(reservable.unwrap_or(reservable_defaut))
                    .execute(&mut *t)
                    .await
                    .map_err(db)?;
                    rapport.creees += 1;
                }
                Some(ex) => {
                    if ex.ferme_le.is_some() {
                        rapport.rejets.push(rejet("salle fermée : rouvrez-la avant de la modifier"));
                        continue;
                    }
                    let nouveau_site = site.0;
                    let nouveau_bat = batiment_id.or(if ex.site_id == Some(nouveau_site) { ex.batiment_id } else { None });
                    let nouveau_lieu = lieu_id.or(ex.lieu_id);
                    let nouveau_libelle = if libelle.is_empty() { ex.libelle.clone() } else { libelle };
                    let nouvelle_capacite = capacite.or(ex.capacite);
                    let nouvel_etage = etage.or(ex.etage.clone());
                    let nouveaux_equip = equipements.unwrap_or_else(|| ex.equipements.clone());
                    let nouveau_pmr = pmr.or(ex.accessible_pmr);
                    let nouveau_reservable = reservable.unwrap_or(ex.reservable);
                    let identique = ex.site_id == Some(nouveau_site)
                        && ex.batiment_id == nouveau_bat
                        && ex.lieu_id == nouveau_lieu
                        && ex.libelle == nouveau_libelle
                        && ex.capacite == nouvelle_capacite
                        && ex.etage == nouvel_etage
                        && ex.type_salle == type_final
                        && ex.equipements == nouveaux_equip
                        && ex.accessible_pmr == nouveau_pmr
                        && ex.reservable == nouveau_reservable;
                    if identique {
                        rapport.inchangees += 1;
                        continue;
                    }
                    sqlx::query(
                        "UPDATE ent_salle SET libelle = $3, capacite = $4, site_id = $5, batiment_id = $6, lieu_id = $7,
                                etage = $8, type = $9, equipements = $10, accessible_pmr = $11, reservable = $12
                          WHERE etablissement_id = $1 AND id = $2",
                    )
                    .bind(etab)
                    .bind(ex.id)
                    .bind(&nouveau_libelle)
                    .bind(nouvelle_capacite)
                    .bind(nouveau_site)
                    .bind(nouveau_bat)
                    .bind(nouveau_lieu)
                    .bind(&nouvel_etage)
                    .bind(&type_final)
                    .bind(&nouveaux_equip)
                    .bind(nouveau_pmr)
                    .bind(nouveau_reservable)
                    .execute(&mut *t)
                    .await
                    .map_err(db)?;
                    rapport.modifiees += 1;
                }
            }
        }
        if apercu {
            // Même chemin que l'import réel, puis annulation : rien n'est écrit.
            drop(t);
            return Ok(rapport);
        }
        let ev = if rapport.creees + rapport.modifiees > 0 {
            journaliser(
                &mut t,
                a,
                "import_salles",
                "lot",
                "importe",
                None,
                Some(json!({ "creees": rapport.creees, "modifiees": rapport.modifiees,
                             "inchangees": rapport.inchangees, "rejets": rapport.rejets.len() })),
            )
            .await?;
            Some(
                publier(
                    &mut t,
                    a,
                    "referentiel.salles_importees",
                    etab,
                    json!({ "creees": rapport.creees, "modifiees": rapport.modifiees }),
                )
                .await?,
            )
        } else {
            None
        };
        t.commit().await.map_err(db)?;
        if let Some(ev) = ev {
            self.diffuser_apres_commit(&[ev]);
        }
        Ok(rapport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_point_virgule_guillemets_et_lignes_vides() {
        let l = parser_csv("\u{feff}code;libelle;site\r\nA1;\"Salle; 1\";P\n\nB2;\"Dit \"\"x\"\"\";P\n").unwrap();
        assert_eq!(l.len(), 3);
        assert_eq!(l[1].1, vec!["A1", "Salle; 1", "P"]);
        assert_eq!(l[2].0, 4, "numéro de ligne du fichier");
        assert_eq!(l[2].1[1], "Dit \"x\"");
    }

    #[test]
    fn csv_virgule_et_guillemet_non_ferme() {
        assert_eq!(parser_csv("code,libelle,site\nA,B,C").unwrap()[1].1, vec!["A", "B", "C"]);
        assert!(parser_csv("code;libelle\n\"A;B").is_err());
    }

    #[test]
    fn oui_non_valeurs() {
        assert_eq!(oui_non("Oui"), Ok(Some(true)));
        assert_eq!(oui_non(""), Ok(None));
        assert!(oui_non("peut-être").is_err());
    }

    #[test]
    fn couleur_hexa() {
        assert!(couleur_valide("#1A2b3C"));
        assert!(!couleur_valide("1A2b3C"));
        assert!(!couleur_valide("#12345"));
    }

    #[test]
    fn salle_exterieure_exige_le_type() {
        let mut s = SalleSaisie {
            code: "g1".into(),
            libelle: "Gymnase".into(),
            site_id: Uuid::nil(),
            batiment_id: None,
            lieu_id: Some(Uuid::new_v4()),
            etage: None,
            type_salle: "gymnase".into(),
            capacite: Some(30),
            equipements: vec![],
            accessible_pmr: None,
            reservable: None,
        };
        assert!(valider_salle(&s).is_err());
        s.type_salle = "exterieur".into();
        assert_eq!(valider_salle(&s).unwrap().code, "G1");
        s.capacite = Some(501);
        assert!(valider_salle(&s).is_err());
    }
}
