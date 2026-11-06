//! Accompagnement et besoins particuliers (socle 2, lot S2-11, ACC-01 → ACC-05) :
//! dispositifs PAP / PPS / PPRE / ULIS, aménagements pédagogiques et d'examen,
//! affectation d'un AESH, équipe éducative, révision et clôture.
//!
//! **Minimisation.** Les professeurs ne reçoivent que des **aménagements** (codes du
//! catalogue fermé [`CATALOGUE_AMENAGEMENTS`], libellés neutres : « tiers-temps »,
//! « police adaptée »…) : ni le type de dispositif, ni son libellé libre, ni les pièces,
//! ni un diagnostic. Les événements ne portent que des identifiants et des états.
//! Les comptes rendus d'équipe éducative sont chiffrés (socle 1, D3).

use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::vie_scolaire::{db, introuvable, invalide};
use chrono::{Datelike, NaiveDate};
use domain_eleve::today_utc;
use ent_auth::{Action, Role};
use ent_events::{AggregateKind, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use sqlx::PgConnection;
use std::collections::BTreeMap;
use uuid::Uuid;

pub mod types_acc {
    pub const DISPOSITIF_AJOUTE: &str = "eleve.dispositif_ajoute";
    pub const AMENAGEMENTS_DEFINIS: &str = "accompagnement.amenagements_definis";
    pub const AESH_AFFECTE: &str = "accompagnement.aesh_affecte";
    pub const EQUIPE_REUNIE: &str = "accompagnement.equipe_reunie";
    pub const DISPOSITIF_REVISE: &str = "accompagnement.dispositif_revise";
}

/// Aménagement du catalogue : le libellé est le seul texte communiqué aux professeurs.
pub struct EntreeAmenagement {
    pub code: &'static str,
    pub libelle: &'static str,
}

/// Aménagements normalisés (ACC-02). Aucun libellé ne nomme un trouble ou un diagnostic.
pub const CATALOGUE_AMENAGEMENTS: &[EntreeAmenagement] = &[
    EntreeAmenagement { code: "tiers_temps", libelle: "Tiers-temps" },
    EntreeAmenagement { code: "police_adaptee", libelle: "Police de caractères adaptée" },
    EntreeAmenagement { code: "agrandissement_sujets", libelle: "Sujets agrandis" },
    EntreeAmenagement { code: "place_premier_rang", libelle: "Place au premier rang" },
    EntreeAmenagement { code: "ordinateur", libelle: "Ordinateur pour écrire" },
    EntreeAmenagement { code: "secretaire", libelle: "Secrétaire d'examen" },
    EntreeAmenagement { code: "lecteur", libelle: "Consignes lues à voix haute" },
    EntreeAmenagement { code: "salle_separee", libelle: "Salle séparée" },
    EntreeAmenagement { code: "pauses_surveillees", libelle: "Pauses surveillées" },
    EntreeAmenagement { code: "consignes_reformulees", libelle: "Consignes reformulées" },
    EntreeAmenagement { code: "calculatrice", libelle: "Calculatrice autorisée" },
    EntreeAmenagement { code: "polycopies_fournis", libelle: "Cours polycopiés fournis" },
    EntreeAmenagement { code: "evaluation_orale", libelle: "Évaluation à l'oral possible" },
    EntreeAmenagement { code: "mise_en_page_aeree", libelle: "Évaluations en mise en page aérée" },
];

pub fn amenagement_du_catalogue(code: &str) -> Option<&'static EntreeAmenagement> {
    CATALOGUE_AMENAGEMENTS.iter().find(|e| e.code == code)
}

fn libelle_amenagement(code: &str) -> String {
    amenagement_du_catalogue(code)
        .map(|e| e.libelle.to_string())
        .unwrap_or_else(|| code.to_string())
}

pub const KINDS_DISPOSITIF: [&str; 6] = ["pap", "pps", "ppre", "ulis", "gevasco", "autre"];
pub const PORTEES_AMENAGEMENT: [&str; 3] = ["cours", "examen", "les_deux"];

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Dispositif {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub kind: String,
    pub libelle: Option<String>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
    pub date_revision: Option<NaiveDate>,
    pub statut: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DispositifVue {
    #[serde(flatten)]
    pub dispositif: Dispositif,
    /// Révision échue ou à moins de 30 jours (dispositif actif seulement).
    pub revision_due: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Amenagement {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub portee: String,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, sqlx::FromRow)]
struct AmenagementRow {
    id: Uuid,
    eleve_id: Uuid,
    code: String,
    portee: String,
    date_debut: NaiveDate,
    date_fin: Option<NaiveDate>,
}

impl From<AmenagementRow> for Amenagement {
    fn from(r: AmenagementRow) -> Self {
        Self {
            libelle: libelle_amenagement(&r.code),
            id: r.id,
            eleve_id: r.eleve_id,
            code: r.code,
            portee: r.portee,
            date_debut: r.date_debut,
            date_fin: r.date_fin,
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AeshAffectation {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub aesh_nom: String,
    pub heures_hebdo: Option<i16>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReunionEquipe {
    pub id: Uuid,
    pub eleve_id: Uuid,
    pub kind: String,
    pub date_reunion: NaiveDate,
    pub statut: String,
    pub compte_rendu: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ReunionRow {
    id: Uuid,
    eleve_id: Uuid,
    kind: String,
    date_reunion: NaiveDate,
    statut: String,
    compte_rendu: String,
}

/// Suivi d'accompagnement d'un élève (personnel habilité seulement).
#[derive(Debug, Clone, Serialize)]
pub struct DossierAccompagnement {
    pub eleve_id: Uuid,
    pub dispositifs: Vec<DispositifVue>,
    pub amenagements: Vec<Amenagement>,
    pub aesh: Vec<AeshAffectation>,
    pub reunions: Vec<ReunionEquipe>,
}

/// Vue « professeur » : élève + aménagements, rien d'autre.
#[derive(Debug, Clone, Serialize)]
pub struct AmenagementsEleve {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub amenagements: Vec<Amenagement>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DispositifARevoir {
    pub dispositif_id: Uuid,
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub classe: Option<String>,
    pub kind: String,
    pub date_revision: NaiveDate,
    pub jours_restants: i32,
}

#[derive(Debug, Clone)]
pub struct OuvrirDispositif {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub kind: String,
    pub libelle: Option<String>,
    pub document_id: Option<Uuid>,
    pub date_debut: Option<NaiveDate>,
    pub date_revision: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct DefinirAmenagement {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub dispositif_id: Option<Uuid>,
    pub code: String,
    pub portee: String,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct AffecterAesh {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub dispositif_id: Option<Uuid>,
    pub aesh_nom: String,
    pub heures_hebdo: Option<i16>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct PlanifierReunion {
    pub etablissement_id: Uuid,
    pub eleve_id: Uuid,
    pub dispositif_id: Option<Uuid>,
    pub kind: String,
    pub date_reunion: NaiveDate,
}

const T_REUNION: &str = "ent_equipe_reunion";

fn un_an_apres(d: NaiveDate) -> NaiveDate {
    d.with_year(d.year() + 1)
        .unwrap_or_else(|| d + chrono::Duration::days(365))
}

fn court(v: &str, max: usize) -> Result<String, EntDbError> {
    let t = v.trim();
    if t.chars().count() > max || t.chars().any(|c| c.is_control() && c != '\n') {
        return Err(invalide());
    }
    Ok(t.to_string())
}

async fn evt(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregate_id: Uuid,
    eleve_id: Uuid,
    payload: serde_json::Value,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Eleve,
        aggregate_id,
        eleve_id: Some(eleve_id),
        payload,
        actor_user_id: Some(acteur.to_string()),
    })
    .await?;
    Ok(())
}

impl PgEnt {
    /// Gestion de l'accompagnement sur cet élève (`accompagnement.gerer`).
    async fn peut_gerer_accompagnement(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<bool, EntDbError> {
        self.staff_eleve(etab, acteur, role, Action::AccompagnementGerer, eleve_id)
            .await
    }

    async fn exiger_accompagnement(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_eleve_existe(etab, eleve_id).await?;
        if self.peut_gerer_accompagnement(etab, acteur, role, eleve_id).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// ACC-01 : ouvrir un dispositif. PAP aussi ouvert au personnel de santé
    /// (`sante.saisir`) ; les autres types relèvent de `accompagnement.gerer`.
    pub async fn ouvrir_dispositif_for_actor(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: OuvrirDispositif,
    ) -> Result<DispositifVue, EntDbError> {
        let etab = cmd.etablissement_id;
        if !KINDS_DISPOSITIF.contains(&cmd.kind.as_str()) {
            return Err(invalide());
        }
        self.exiger_eleve_existe(etab, cmd.eleve_id).await?;
        let autorise = self
            .peut_gerer_accompagnement(etab, acteur, role, cmd.eleve_id)
            .await?
            || (cmd.kind == "pap"
                && self
                    .staff_eleve(etab, acteur, role, Action::SanteSaisir, cmd.eleve_id)
                    .await?);
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        let libelle = {
            let l = court(cmd.libelle.as_deref().unwrap_or(""), 120)?;
            (!l.is_empty()).then_some(l)
        };
        let debut = cmd.date_debut.unwrap_or_else(today_utc);
        let revision = cmd.date_revision.unwrap_or_else(|| un_an_apres(debut));
        if revision < debut {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if let Some(doc) = cmd.document_id {
            let ok: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_eleve_document
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3",
            )
            .bind(etab)
            .bind(cmd.eleve_id)
            .bind(doc)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if ok.is_none() {
                return Err(invalide());
            }
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve_dispositif
                (id, etablissement_id, eleve_id, kind, libelle, document_id, date_debut,
                 date_revision, ouvert_par_user_id, statut)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'actif')",
        )
        .bind(id)
        .bind(etab)
        .bind(cmd.eleve_id)
        .bind(&cmd.kind)
        .bind(&libelle)
        .bind(cmd.document_id)
        .bind(debut)
        .bind(revision)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        evt(
            &mut tx,
            etab,
            types_acc::DISPOSITIF_AJOUTE,
            id,
            cmd.eleve_id,
            json!({ "dispositif_id": id }),
            acteur,
        )
        .await?;
        let d = lire_dispositif(&mut tx, etab, id).await?;
        tx.commit().await.map_err(db)?;
        Ok(vue_dispositif(d))
    }

    /// ACC-05 : réviser (nouvelle échéance) ou clore (aménagements liés retirés).
    pub async fn reviser_dispositif_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        dispositif_id: Uuid,
        nouvelle_revision: Option<NaiveDate>,
        clore: bool,
    ) -> Result<DispositifVue, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = lire_dispositif(&mut tx, etab, dispositif_id).await?;
        if !self.peut_gerer_accompagnement(etab, acteur, role, d.eleve_id).await? {
            return Err(EntDbError::Forbidden);
        }
        if d.statut == "clos" {
            return Err(EntDbError::ConflitDossier("dispositif déjà clos"));
        }
        let aujourdhui = today_utc();
        if clore {
            sqlx::query(
                "UPDATE ent_eleve_dispositif SET statut = 'clos', clos_at = now(),
                        date_fin = COALESCE(date_fin, $3)
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(dispositif_id)
            .bind(aujourdhui)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_amenagement SET actif = false, retire_at = now()
                  WHERE etablissement_id = $1 AND dispositif_id = $2 AND actif",
            )
            .bind(etab)
            .bind(dispositif_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        } else {
            let prochaine = nouvelle_revision.unwrap_or_else(|| un_an_apres(aujourdhui));
            if prochaine < aujourdhui {
                return Err(invalide());
            }
            sqlx::query(
                "UPDATE ent_eleve_dispositif SET date_revision = $3, revise_at = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(dispositif_id)
            .bind(prochaine)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        evt(
            &mut tx,
            etab,
            types_acc::DISPOSITIF_REVISE,
            dispositif_id,
            d.eleve_id,
            json!({ "dispositif_id": dispositif_id, "action": if clore { "clos" } else { "revise" } }),
            acteur,
        )
        .await?;
        let d = lire_dispositif(&mut tx, etab, dispositif_id).await?;
        tx.commit().await.map_err(db)?;
        Ok(vue_dispositif(d))
    }

    /// ACC-02 : définir un aménagement du catalogue (remplace l'aménagement actif de même code).
    pub async fn definir_amenagement_for_actor(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: DefinirAmenagement,
    ) -> Result<Amenagement, EntDbError> {
        let etab = cmd.etablissement_id;
        self.exiger_accompagnement(etab, acteur, role, cmd.eleve_id).await?;
        if amenagement_du_catalogue(&cmd.code).is_none()
            || !PORTEES_AMENAGEMENT.contains(&cmd.portee.as_str())
        {
            return Err(EntDbError::SaisieInvalide("aménagement hors catalogue"));
        }
        let debut = today_utc();
        if cmd.date_fin.is_some_and(|f| f < debut) {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if let Some(did) = cmd.dispositif_id {
            let ok: Option<(String,)> = sqlx::query_as(
                "SELECT statut FROM ent_eleve_dispositif
                  WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3",
            )
            .bind(etab)
            .bind(cmd.eleve_id)
            .bind(did)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match ok {
                Some((s,)) if s == "actif" => {}
                _ => return Err(invalide()),
            }
        }
        let r: AmenagementRow = sqlx::query_as(
            "INSERT INTO ent_amenagement
                (id, etablissement_id, eleve_id, dispositif_id, code, portee, date_debut, date_fin,
                 defini_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             ON CONFLICT (etablissement_id, eleve_id, code) WHERE actif DO UPDATE SET
                dispositif_id = EXCLUDED.dispositif_id, portee = EXCLUDED.portee,
                date_debut = EXCLUDED.date_debut, date_fin = EXCLUDED.date_fin,
                defini_par_user_id = EXCLUDED.defini_par_user_id
             RETURNING id, eleve_id, code, portee, date_debut, date_fin",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(cmd.eleve_id)
        .bind(cmd.dispositif_id)
        .bind(&cmd.code)
        .bind(&cmd.portee)
        .bind(debut)
        .bind(cmd.date_fin)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        evt(
            &mut tx,
            etab,
            types_acc::AMENAGEMENTS_DEFINIS,
            r.id,
            cmd.eleve_id,
            json!({ "amenagement_id": r.id, "action": "defini" }),
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(r.into())
    }

    pub async fn retirer_amenagement_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        amenagement_id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l: Option<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_amenagement WHERE etablissement_id = $1 AND id = $2 AND actif",
        )
        .bind(etab)
        .bind(amenagement_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (eleve_id,) = l.ok_or_else(introuvable)?;
        if !self.peut_gerer_accompagnement(etab, acteur, role, eleve_id).await? {
            return Err(EntDbError::Forbidden);
        }
        sqlx::query(
            "UPDATE ent_amenagement SET actif = false, retire_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(amenagement_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        evt(
            &mut tx,
            etab,
            types_acc::AMENAGEMENTS_DEFINIS,
            amenagement_id,
            eleve_id,
            json!({ "amenagement_id": amenagement_id, "action": "retire" }),
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// ACC-03 : affecter un AESH (nom et quotité ; le compte n'est pas requis).
    pub async fn affecter_aesh_for_actor(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: AffecterAesh,
    ) -> Result<AeshAffectation, EntDbError> {
        let etab = cmd.etablissement_id;
        self.exiger_accompagnement(etab, acteur, role, cmd.eleve_id).await?;
        let nom = court(&cmd.aesh_nom, 120)?;
        if nom.is_empty()
            || cmd.date_fin.is_some_and(|f| f < cmd.date_debut)
            || cmd.heures_hebdo.is_some_and(|h| !(1..=60).contains(&h))
        {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        verifier_dispositif(&mut tx, etab, cmd.eleve_id, cmd.dispositif_id).await?;
        let a: AeshAffectation = sqlx::query_as(
            "INSERT INTO ent_aesh_affectation
                (id, etablissement_id, eleve_id, dispositif_id, aesh_nom, heures_hebdo, date_debut,
                 date_fin, affecte_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             RETURNING id, eleve_id, aesh_nom, heures_hebdo, date_debut, date_fin",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(cmd.eleve_id)
        .bind(cmd.dispositif_id)
        .bind(&nom)
        .bind(cmd.heures_hebdo)
        .bind(cmd.date_debut)
        .bind(cmd.date_fin)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        evt(
            &mut tx,
            etab,
            types_acc::AESH_AFFECTE,
            a.id,
            cmd.eleve_id,
            json!({ "affectation_id": a.id }),
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(a)
    }

    /// ACC-04 : planifier une réunion d'équipe éducative / de suivi.
    pub async fn planifier_reunion_for_actor(
        &self,
        acteur: Uuid,
        role: &str,
        cmd: PlanifierReunion,
    ) -> Result<ReunionEquipe, EntDbError> {
        let etab = cmd.etablissement_id;
        self.exiger_accompagnement(etab, acteur, role, cmd.eleve_id).await?;
        if !["equipe_educative", "equipe_suivi"].contains(&cmd.kind.as_str()) {
            return Err(invalide());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        verifier_dispositif(&mut tx, etab, cmd.eleve_id, cmd.dispositif_id).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_equipe_reunion
                (id, etablissement_id, eleve_id, dispositif_id, kind, date_reunion, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(cmd.eleve_id)
        .bind(cmd.dispositif_id)
        .bind(&cmd.kind)
        .bind(cmd.date_reunion)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        evt(
            &mut tx,
            etab,
            types_acc::EQUIPE_REUNIE,
            id,
            cmd.eleve_id,
            json!({ "reunion_id": id, "statut": "planifiee" }),
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ReunionEquipe {
            id,
            eleve_id: cmd.eleve_id,
            kind: cmd.kind,
            date_reunion: cmd.date_reunion,
            statut: "planifiee".into(),
            compte_rendu: String::new(),
        })
    }

    /// Consigne le compte rendu d'une réunion (chiffré) et la marque tenue.
    pub async fn consigner_reunion_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        reunion_id: Uuid,
        compte_rendu: &str,
    ) -> Result<(), EntDbError> {
        let cr = court(compte_rendu, 8000)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l: Option<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_equipe_reunion
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'planifiee'",
        )
        .bind(etab)
        .bind(reunion_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (eleve_id,) = l.ok_or_else(introuvable)?;
        if !self.peut_gerer_accompagnement(etab, acteur, role, eleve_id).await? {
            return Err(EntDbError::Forbidden);
        }
        let scelle = self
            .infirmerie_repo()
            .champs()
            .sceller(T_REUNION, "compte_rendu", reunion_id, &cr)?;
        sqlx::query(
            "UPDATE ent_equipe_reunion SET compte_rendu = $3, statut = 'tenue', updated_at = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(reunion_id)
        .bind(scelle)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        evt(
            &mut tx,
            etab,
            types_acc::EQUIPE_REUNIE,
            reunion_id,
            eleve_id,
            json!({ "reunion_id": reunion_id, "statut": "tenue" }),
            acteur,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Suivi d'accompagnement complet d'un élève : `accompagnement.gerer` ou personnel
    /// de santé. Consultation journalisée.
    pub async fn dossier_accompagnement_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<DossierAccompagnement, EntDbError> {
        self.exiger_eleve_existe(etab, eleve_id).await?;
        let gestion = self.peut_gerer_accompagnement(etab, acteur, role, eleve_id).await?;
        if !gestion
            && !self
                .staff_eleve(etab, acteur, role, Action::SanteLire, eleve_id)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let dispositifs: Vec<Dispositif> = sqlx::query_as(
            "SELECT id, eleve_id, kind, libelle, date_debut, date_fin, date_revision, statut
               FROM ent_eleve_dispositif
              WHERE etablissement_id = $1 AND eleve_id = $2 AND kind <> 'pai'
              ORDER BY statut, date_debut DESC",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let amenagements: Vec<AmenagementRow> = sqlx::query_as(
            "SELECT id, eleve_id, code, portee, date_debut, date_fin FROM ent_amenagement
              WHERE etablissement_id = $1 AND eleve_id = $2 AND actif ORDER BY code",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let aesh: Vec<AeshAffectation> = sqlx::query_as(
            "SELECT id, eleve_id, aesh_nom, heures_hebdo, date_debut, date_fin
               FROM ent_aesh_affectation WHERE etablissement_id = $1 AND eleve_id = $2
              ORDER BY date_debut DESC",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let reunions: Vec<ReunionRow> = sqlx::query_as(
            "SELECT id, eleve_id, kind, date_reunion, statut, compte_rendu FROM ent_equipe_reunion
              WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY date_reunion DESC",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let champs = self.infirmerie_repo().champs();
        let mut reunions_vue = Vec::new();
        for r in reunions {
            reunions_vue.push(ReunionEquipe {
                // Le compte rendu n'est lisible que de la gestion de l'accompagnement.
                compte_rendu: if gestion {
                    champs.ouvrir(T_REUNION, "compte_rendu", r.id, &r.compte_rendu)?
                } else {
                    String::new()
                },
                id: r.id,
                eleve_id: r.eleve_id,
                kind: r.kind,
                date_reunion: r.date_reunion,
                statut: r.statut,
            });
        }
        self.tracer_acces_sensible(
            etab,
            acteur,
            role,
            Some(eleve_id),
            "consultation",
            "dossier_accompagnement",
            "Consultation du suivi d'accompagnement",
        )
        .await;
        Ok(DossierAccompagnement {
            eleve_id,
            dispositifs: dispositifs.into_iter().map(vue_dispositif).collect(),
            amenagements: amenagements.into_iter().map(Into::into).collect(),
            aesh,
            reunions: reunions_vue,
        })
    }

    /// Aménagements d'un élève : professeurs de l'élève (`amenagements.lire`, portée
    /// « mes classes »), gestion, famille liée, élève lui-même. Aucun autre champ.
    pub async fn amenagements_eleve_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<Amenagement>, EntDbError> {
        self.exiger_eleve_existe(etab, eleve_id).await?;
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        let autorise = match parsed {
            Role::Parent => self.lien_parent_eleve(etab, acteur, eleve_id).await?.is_some(),
            Role::Eleve => self.eleve_id_for_compte(etab, acteur).await? == Some(eleve_id),
            _ => {
                self.staff_eleve(etab, acteur, role, Action::AmenagementsLire, eleve_id)
                    .await?
                    || self.peut_gerer_accompagnement(etab, acteur, role, eleve_id).await?
            }
        };
        if !autorise {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let rows: Vec<AmenagementRow> = sqlx::query_as(
            "SELECT id, eleve_id, code, portee, date_debut, date_fin FROM ent_amenagement
              WHERE etablissement_id = $1 AND eleve_id = $2 AND actif
                AND date_debut <= CURRENT_DATE AND (date_fin IS NULL OR date_fin >= CURRENT_DATE)
              ORDER BY code",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Les élèves de l'acteur (selon la portée de `amenagements.lire`) qui ont des
    /// aménagements — l'écran « mes élèves aménagés » du professeur.
    pub async fn mes_amenagements_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<AmenagementsEleve>, EntDbError> {
        if !self
            .a_action_quelconque(etab, acteur, role, Action::AmenagementsLire)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let limite = self
            .eleves_autorises(etab, acteur, role, Action::AmenagementsLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        #[derive(sqlx::FromRow)]
        struct L {
            id: Uuid,
            eleve_id: Uuid,
            code: String,
            portee: String,
            date_debut: NaiveDate,
            date_fin: Option<NaiveDate>,
            nom: String,
            prenom: String,
            classe: Option<String>,
        }
        let rows: Vec<L> = sqlx::query_as(
            "SELECT a.id, a.eleve_id, a.code, a.portee, a.date_debut, a.date_fin,
                    e.nom, e.prenom, e.classe
               FROM ent_amenagement a
               JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.actif
                AND a.date_debut <= CURRENT_DATE AND (a.date_fin IS NULL OR a.date_fin >= CURRENT_DATE)
              ORDER BY e.nom, e.prenom, a.code",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut par_eleve: BTreeMap<(String, String, Uuid), AmenagementsEleve> = BTreeMap::new();
        for l in rows {
            if limite.as_ref().is_some_and(|ens| !ens.contains(&l.eleve_id)) {
                continue;
            }
            let eleve_id = l.eleve_id;
            let e = par_eleve
                .entry((l.nom.clone(), l.prenom.clone(), eleve_id))
                .or_insert_with(|| AmenagementsEleve {
                    eleve_id,
                    nom: l.nom,
                    prenom: l.prenom,
                    classe: l.classe,
                    amenagements: Vec::new(),
                });
            e.amenagements.push(
                AmenagementRow {
                    id: l.id,
                    eleve_id: l.eleve_id,
                    code: l.code,
                    portee: l.portee,
                    date_debut: l.date_debut,
                    date_fin: l.date_fin,
                }
                .into(),
            );
        }
        Ok(par_eleve.into_values().collect())
    }

    /// Dispositifs actifs dont la révision est échue ou proche.
    pub async fn dispositifs_a_reviser_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        horizon_jours: i32,
    ) -> Result<Vec<DispositifARevoir>, EntDbError> {
        if !self
            .a_action_quelconque(etab, acteur, role, Action::AccompagnementGerer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let limite = self
            .eleves_autorises(etab, acteur, role, Action::AccompagnementGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut l: Vec<DispositifARevoir> = sqlx::query_as(
            "SELECT d.id AS dispositif_id, d.eleve_id, e.nom, e.prenom, e.classe, d.kind,
                    d.date_revision, (d.date_revision - CURRENT_DATE)::int AS jours_restants
               FROM ent_eleve_dispositif d
               JOIN ent_eleve e ON e.id = d.eleve_id AND e.etablissement_id = d.etablissement_id
              WHERE d.etablissement_id = $1 AND d.statut = 'actif' AND d.kind <> 'pai'
                AND d.date_revision IS NOT NULL
                AND d.date_revision <= CURRENT_DATE + $2::int
              ORDER BY d.date_revision, e.nom",
        )
        .bind(etab)
        .bind(horizon_jours.clamp(0, 365))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        if let Some(ens) = limite {
            l.retain(|d| ens.contains(&d.eleve_id));
        }
        Ok(l)
    }
}

fn vue_dispositif(d: Dispositif) -> DispositifVue {
    let bientot = today_utc() + chrono::Duration::days(30);
    DispositifVue {
        revision_due: d.statut == "actif" && d.date_revision.is_some_and(|r| r <= bientot),
        dispositif: d,
    }
}

async fn lire_dispositif(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<Dispositif, EntDbError> {
    let d: Option<Dispositif> = sqlx::query_as(
        "SELECT id, eleve_id, kind, libelle, date_debut, date_fin, date_revision, statut
           FROM ent_eleve_dispositif WHERE etablissement_id = $1 AND id = $2 AND kind <> 'pai'",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    d.ok_or_else(introuvable)
}

/// Le dispositif lié (facultatif) existe, est de CET élève et de cet établissement.
async fn verifier_dispositif(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve_id: Uuid,
    dispositif_id: Option<Uuid>,
) -> Result<(), EntDbError> {
    let Some(did) = dispositif_id else {
        return Ok(());
    };
    let ok: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM ent_eleve_dispositif
          WHERE etablissement_id = $1 AND eleve_id = $2 AND id = $3 AND kind <> 'pai'",
    )
    .bind(etab)
    .bind(eleve_id)
    .bind(did)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    ok.map(|_| ()).ok_or_else(invalide)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_amenagements_sans_diagnostic() {
        const INTERDITS: &[&str] = &[
            "dys", "tdah", "autis", "trouble", "handicap", "épilep", "diagnostic", "maladie",
            "syndrome", "déficien", "deficien", "tsa", "hpi", "haut potentiel",
        ];
        let mut vus = std::collections::HashSet::new();
        for e in CATALOGUE_AMENAGEMENTS {
            assert!(vus.insert(e.code), "code en double : {}", e.code);
            let l = e.libelle.to_lowercase();
            let c = e.code.to_lowercase();
            for m in INTERDITS {
                assert!(!l.contains(m), "libellé « {} » contient « {m} »", e.libelle);
                assert!(!c.contains(m), "code « {} » contient « {m} »", e.code);
            }
        }
    }

    #[test]
    fn kinds_et_portees() {
        assert!(KINDS_DISPOSITIF.contains(&"pap"));
        assert!(!KINDS_DISPOSITIF.contains(&"pai"));
        assert_eq!(un_an_apres(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()).year(), 2027);
    }
}
