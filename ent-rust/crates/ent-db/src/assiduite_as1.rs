//! Socle 12, lot AS1 : référentiels de l'assiduité et droits (`docs/socles/12_ASSIDUITE.md`
//! §15.1, §22.1, §24). Natures de motif (L131-8), profils académiques, motifs d'établissement
//! **versionnés**, **règle d'assiduité datée** (R-AS15, DA-11), activités appelées (RA-02).
//!
//! Droits : moteur D-017 (`assiduite.regle_gerer` pour le paramétrage, `absences.lire_motif` ou
//! `assiduite.signaler` pour la lecture) ; aucun rôle codé en dur (DA-18). Les écritures ouvrent une
//! transaction d'établissement (`TenantTx`, RLS) et publient leur événement dans la même transaction
//! (outbox) : le payload ne porte que des codes et des identifiants (R-AS25).

use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use crate::vie_scolaire::{db, introuvable};
use chrono::{NaiveDate, NaiveTime};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Types d'événements du socle 12 (constantes locales : le catalogue central est partagé entre lots).
pub mod types_asd {
    pub const REGLE_DEFINIE: &str = "assiduite.regle_definie";
    pub const MOTIF_CREE: &str = "assiduite.motif_cree";
    pub const MOTIF_MODIFIE: &str = "assiduite.motif_modifie";
    pub const ACTIVITE_APPELEE_MODIFIEE: &str = "assiduite.activite_appelee_modifiee";
}

/// Code du motif technique créé par l'appel (jamais proposé aux familles : DA-20).
pub const MOTIF_TECHNIQUE_APPEL: &str = "appel";

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NatureMotif {
    pub code: String,
    pub libelle: String,
    pub legitime_l131_8: bool,
    pub medical: bool,
    pub ordre: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProfilNiveau {
    pub niveau: i16,
    pub libelle: String,
    pub seuil_demi_journees: i16,
    pub delai_jours: Option<i16>,
    pub destinataire_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilAcademique {
    pub id: Uuid,
    pub code_academie: String,
    pub version: i32,
    pub libelle: String,
    pub mois_type: String,
    pub outil: String,
    pub age_limite_penal: i16,
    pub niveaux: Vec<ProfilNiveau>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MotifAsd {
    pub id: Uuid,
    pub code: String,
    pub nature_code: String,
    pub libelle: String,
    pub libelle_famille: Option<String>,
    pub visible_famille: bool,
    pub actif: bool,
}

/// Création ou modification d'un motif d'établissement.
#[derive(Debug, Clone, Deserialize)]
pub struct SaisieMotif {
    pub code: String,
    pub nature_code: String,
    pub libelle: String,
    pub libelle_famille: Option<String>,
    pub visible_famille: bool,
    pub actif: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VersionMotif {
    pub libelle: String,
    pub libelle_famille: Option<String>,
    pub visible_famille: bool,
    pub actif: bool,
    pub auteur_user_id: Option<Uuid>,
    pub cree_le: chrono::DateTime<chrono::Utc>,
}

/// Une version de la règle d'assiduité d'un établissement (R-AS15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegleAsd {
    pub id: Uuid,
    pub valide_du: NaiveDate,
    pub definition_demi_journee: String,
    pub seuil_minutes: i32,
    pub heure_coupure: NaiveTime,
    pub type_mois: String,
    pub profil_id: Option<Uuid>,
    pub seuil_signalement_demi_journees: i32,
    pub delai_consolidation_min: i32,
    pub delai_verification_min: i32,
    pub rythme_relance: String,
    pub seuil_retard_absence_pct: i32,
    pub seuil_absences_repetees: i32,
    pub seuil_retards: i32,
    /// Posée à l'arrêt d'un mois qui l'a utilisée : la règle ne change plus jamais.
    pub figee: bool,
}

impl RegleAsd {
    /// Valeurs par défaut du code, pour un établissement sans version (jamais en base).
    pub fn defaut(valide_du: NaiveDate) -> Self {
        Self {
            id: Uuid::nil(),
            valide_du,
            definition_demi_journee: "minutes".into(),
            seuil_minutes: 30,
            heure_coupure: NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default(),
            type_mois: "civil".into(),
            profil_id: None,
            seuil_signalement_demi_journees: 4,
            delai_consolidation_min: 10,
            delai_verification_min: 15,
            rythme_relance: "standard".into(),
            seuil_retard_absence_pct: 50,
            seuil_absences_repetees: 3,
            seuil_retards: 5,
            figee: false,
        }
    }
}

/// Saisie d'une nouvelle version de règle (`valide_du` ≥ aujourd'hui de l'établissement).
#[derive(Debug, Clone, Deserialize)]
pub struct SaisieRegle {
    pub valide_du: NaiveDate,
    pub definition_demi_journee: String,
    pub seuil_minutes: i32,
    pub heure_coupure: NaiveTime,
    pub type_mois: String,
    pub profil_id: Option<Uuid>,
    pub seuil_signalement_demi_journees: i32,
    pub delai_consolidation_min: i32,
    pub delai_verification_min: i32,
    pub rythme_relance: String,
    pub seuil_retard_absence_pct: i32,
    pub seuil_absences_repetees: i32,
    pub seuil_retards: i32,
}

impl SaisieRegle {
    fn valider(&self) -> Result<(), EntDbError> {
        let ok = matches!(self.definition_demi_journee.as_str(), "une_seance" | "minutes" | "entiere")
            && matches!(self.type_mois.as_str(), "civil" | "glissant")
            && matches!(self.rythme_relance.as_str(), "court" | "standard" | "long")
            && (1..=600).contains(&self.seuil_minutes)
            && (1..=62).contains(&self.seuil_signalement_demi_journees)
            && (0..=240).contains(&self.delai_consolidation_min)
            && (0..=240).contains(&self.delai_verification_min)
            && (1..=100).contains(&self.seuil_retard_absence_pct)
            && (1..=100).contains(&self.seuil_absences_repetees)
            && (1..=100).contains(&self.seuil_retards);
        if ok {
            Ok(())
        } else {
            Err(EntDbError::SaisieInvalide("règle d'assiduité hors bornes"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActiviteAppelee {
    pub type_activite: String,
    pub active: bool,
}

pub const TYPES_ACTIVITE_APPELEE: [&str; 5] =
    ["permanence", "etude", "cdi", "demi_pension", "accompagnement"];

const COLONNES_REGLE: &str = "id, valide_du, definition_demi_journee, seuil_minutes, heure_coupure, type_mois,
    profil_id, seuil_signalement_demi_journees, delai_consolidation_min, delai_verification_min,
    rythme_relance, seuil_retard_absence_pct, seuil_absences_repetees, seuil_retards,
    (figee_le IS NOT NULL) AS figee";

fn code_valide(code: &str) -> bool {
    let n = code.len();
    (2..=40).contains(&n)
        && code
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

impl PgEnt {
    // ── Référentiels globaux ────────────────────────────────────────────────────────────────

    /// Natures de motif (les cinq de L131-8 en tête). Référentiel commun, lisible du personnel.
    pub async fn natures_motif_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<NatureMotif>, EntDbError> {
        self.exiger_lecture_assiduite(etablissement_id, acteur, role).await?;
        sqlx::query_as::<_, NatureMotif>(
            "SELECT code, libelle, legitime_l131_8, medical, ordre FROM ref_asd_nature_motif ORDER BY ordre, code",
        )
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Profils académiques de signalement (paramètres d'académie, commun à tous les établissements).
    pub async fn profils_academiques_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<ProfilAcademique>, EntDbError> {
        self.exiger_lecture_assiduite(etablissement_id, acteur, role).await?;
        let rows: Vec<(Uuid, String, i32, String, String, String, i16)> = sqlx::query_as(
            "SELECT id, code_academie, version, libelle, mois_type, outil, age_limite_penal
               FROM ref_asd_profil_academique ORDER BY code_academie, version",
        )
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(rows.len());
        for (id, code_academie, version, libelle, mois_type, outil, age) in rows {
            let niveaux = sqlx::query_as::<_, ProfilNiveau>(
                "SELECT niveau, libelle, seuil_demi_journees, delai_jours, destinataire_code
                   FROM ref_asd_profil_niveau WHERE profil_id = $1 ORDER BY niveau",
            )
            .bind(id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
            out.push(ProfilAcademique {
                id,
                code_academie,
                version,
                libelle,
                mois_type,
                outil,
                age_limite_penal: age,
                niveaux,
            });
        }
        Ok(out)
    }

    /// Lecture du paramétrage de l'assiduité : qui lit les absences, signale ou paramètre.
    async fn exiger_lecture_assiduite(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        for a in [Action::AssiduiteRegleGerer, Action::AssiduiteSignaler, Action::AbsencesLireMotif] {
            if self.a_action_etablissement(etablissement_id, acteur, role, a).await? {
                return Ok(());
            }
        }
        Err(EntDbError::Forbidden)
    }

    // ── Motifs d'établissement ──────────────────────────────────────────────────────────────

    /// Amorce, une seule fois et sans écraser, les motifs d'un établissement créé après la reprise
    /// (motifs courants reliés à une nature) : le parcours des familles n'est jamais vide.
    async fn amorcer_motifs(&self, etablissement_id: Uuid) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM ent_asd_motif WHERE etablissement_id = $1")
            .bind(etablissement_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        if n > 0 {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO ent_asd_motif (id, etablissement_id, code, nature_code, libelle)
             SELECT md5('asd_motif:' || $1::text || ':' || m.code)::uuid, $1, m.code,
                    CASE m.code WHEN 'maladie' THEN 'maladie_enfant' WHEN 'familial' THEN 'raison_familiale' ELSE 'autre' END,
                    m.libelle
               FROM ent_absence_motif m WHERE m.code <> $2
             ON CONFLICT DO NOTHING",
        )
        .bind(etablissement_id)
        .bind(MOTIF_TECHNIQUE_APPEL)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_asd_motif_version (id, etablissement_id, motif_id, libelle, libelle_famille, visible_famille, actif)
             SELECT md5('asd_motif_v1:' || m.id::text)::uuid, m.etablissement_id, m.id, m.libelle, m.libelle_famille,
                    m.visible_famille, m.actif
               FROM ent_asd_motif m WHERE m.etablissement_id = $1
             ON CONFLICT DO NOTHING",
        )
        .bind(etablissement_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Motifs proposés aux familles et aux élèves majeurs : actifs, visibles, jamais le motif technique.
    pub async fn motifs_famille(&self, etablissement_id: Uuid) -> Result<Vec<MotifAsd>, EntDbError> {
        self.amorcer_motifs(etablissement_id).await?;
        sqlx::query_as::<_, MotifAsd>(
            "SELECT id, code, nature_code, libelle, libelle_famille, visible_famille, actif
               FROM ent_asd_motif
              WHERE etablissement_id = $1 AND actif AND visible_famille AND code <> $2
              ORDER BY libelle, code",
        )
        .bind(etablissement_id)
        .bind(MOTIF_TECHNIQUE_APPEL)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)
    }

    /// Tous les motifs de l'établissement, inactifs compris (vie scolaire, direction).
    pub async fn motifs_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<MotifAsd>, EntDbError> {
        self.exiger_lecture_assiduite(etablissement_id, acteur, role).await?;
        self.amorcer_motifs(etablissement_id).await?;
        sqlx::query_as::<_, MotifAsd>(
            "SELECT id, code, nature_code, libelle, libelle_famille, visible_famille, actif
               FROM ent_asd_motif WHERE etablissement_id = $1 ORDER BY libelle, code",
        )
        .bind(etablissement_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)
    }

    /// Crée un motif d'établissement rattaché à une nature (action `assiduite.regle_gerer`).
    pub async fn creer_motif_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        s: SaisieMotif,
    ) -> Result<MotifAsd, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AssiduiteRegleGerer)
            .await?;
        self.valider_motif(&s, true)?;
        self.amorcer_motifs(etablissement_id).await?;
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        let nature_ok: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ref_asd_nature_motif WHERE code = $1")
            .bind(&s.nature_code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        if nature_ok.is_none() {
            return Err(EntDbError::SaisieInvalide("nature de motif inconnue"));
        }
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_asd_motif (id, etablissement_id, code, nature_code, libelle, libelle_famille, visible_famille, actif)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (etablissement_id, code) DO NOTHING",
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(&s.code)
        .bind(&s.nature_code)
        .bind(s.libelle.trim())
        .bind(s.libelle_famille.as_deref().map(str::trim))
        .bind(s.visible_famille)
        .bind(s.actif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("ce code de motif existe déjà"));
        }
        Self::version_motif_tx(&mut tx, etablissement_id, id, &s, acteur).await?;
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_asd::MOTIF_CREE.to_string(),
                aggregate: AggregateKind::ParamEtab,
                aggregate_id: id,
                eleve_id: None,
                payload: json!({ "code": s.code, "nature": s.nature_code }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(MotifAsd {
            id,
            code: s.code,
            nature_code: s.nature_code,
            libelle: s.libelle.trim().to_string(),
            libelle_famille: s.libelle_famille.map(|l| l.trim().to_string()),
            visible_famille: s.visible_famille,
            actif: s.actif,
        })
    }

    /// Modifie le libellé, la visibilité ou l'état d'un motif ; chaque changement est historisé
    /// (DA-10, DA-20). La nature et le code ne changent pas : un motif déjà utilisé garde son sens.
    pub async fn modifier_motif_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        motif_id: Uuid,
        s: SaisieMotif,
    ) -> Result<MotifAsd, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AssiduiteRegleGerer)
            .await?;
        self.valider_motif(&s, false)?;
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        let courant: Option<(String, String)> = sqlx::query_as(
            "SELECT code, nature_code FROM ent_asd_motif WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(motif_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((code, nature)) = courant else {
            return Err(introuvable());
        };
        sqlx::query(
            "UPDATE ent_asd_motif SET libelle = $3, libelle_famille = $4, visible_famille = $5, actif = $6,
                    modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(motif_id)
        .bind(s.libelle.trim())
        .bind(s.libelle_famille.as_deref().map(str::trim))
        .bind(s.visible_famille)
        .bind(s.actif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Self::version_motif_tx(&mut tx, etablissement_id, motif_id, &s, acteur).await?;
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_asd::MOTIF_MODIFIE.to_string(),
                aggregate: AggregateKind::ParamEtab,
                aggregate_id: motif_id,
                eleve_id: None,
                payload: json!({ "code": code, "actif": s.actif }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(MotifAsd {
            id: motif_id,
            code,
            nature_code: nature,
            libelle: s.libelle.trim().to_string(),
            libelle_famille: s.libelle_famille.map(|l| l.trim().to_string()),
            visible_famille: s.visible_famille,
            actif: s.actif,
        })
    }

    /// Historique des libellés d'un motif (ajout seul).
    pub async fn versions_motif_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        motif_id: Uuid,
    ) -> Result<Vec<VersionMotif>, EntDbError> {
        self.exiger_lecture_assiduite(etablissement_id, acteur, role).await?;
        sqlx::query_as::<_, VersionMotif>(
            "SELECT libelle, libelle_famille, visible_famille, actif, auteur_user_id, cree_le
               FROM ent_asd_motif_version WHERE etablissement_id = $1 AND motif_id = $2 ORDER BY cree_le, id",
        )
        .bind(etablissement_id)
        .bind(motif_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)
    }

    fn valider_motif(&self, s: &SaisieMotif, avec_code: bool) -> Result<(), EntDbError> {
        let libelle = s.libelle.trim();
        let ok = (1..=200).contains(&libelle.chars().count())
            && s.libelle_famille
                .as_deref()
                .is_none_or(|l| (1..=200).contains(&l.trim().chars().count()))
            && (!avec_code || (code_valide(&s.code) && s.code != MOTIF_TECHNIQUE_APPEL));
        if ok {
            Ok(())
        } else {
            Err(EntDbError::SaisieInvalide("motif d'absence invalide"))
        }
    }

    async fn version_motif_tx(
        tx: &mut sqlx::PgConnection,
        etablissement_id: Uuid,
        motif_id: Uuid,
        s: &SaisieMotif,
        acteur: Uuid,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "INSERT INTO ent_asd_motif_version
                (id, etablissement_id, motif_id, libelle, libelle_famille, visible_famille, actif, auteur_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(motif_id)
        .bind(s.libelle.trim())
        .bind(s.libelle_famille.as_deref().map(str::trim))
        .bind(s.visible_famille)
        .bind(s.actif)
        .bind(acteur)
        .execute(tx)
        .await
        .map_err(db)?;
        Ok(())
    }

    // ── Règle d'assiduité datée ─────────────────────────────────────────────────────────────

    /// Règle en vigueur à `jour` : la version la plus récente dont `valide_du ≤ jour`
    /// (valeurs par défaut du code si l'établissement n'en a aucune).
    pub async fn regle_asd_en_vigueur(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<RegleAsd, EntDbError> {
        let sql = format!(
            "SELECT {COLONNES_REGLE} FROM ent_asd_regle
              WHERE etablissement_id = $1 AND valide_du <= $2 ORDER BY valide_du DESC LIMIT 1"
        );
        let r = sqlx::query_as::<_, RegleAsd>(&sql)
            .bind(etablissement_id)
            .bind(jour)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
            .await
            .map_err(db)?;
        Ok(r.unwrap_or_else(|| RegleAsd::defaut(jour)))
    }

    /// Règle en vigueur à `jour` pour qui lit le paramétrage de l'assiduité.
    pub async fn regle_asd_en_vigueur_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        jour: NaiveDate,
    ) -> Result<RegleAsd, EntDbError> {
        self.exiger_lecture_assiduite(etablissement_id, acteur, role).await?;
        self.regle_asd_en_vigueur(etablissement_id, jour).await
    }

    /// Toutes les versions de la règle, de la plus récente à la plus ancienne.
    pub async fn regles_asd_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<RegleAsd>, EntDbError> {
        self.exiger_lecture_assiduite(etablissement_id, acteur, role).await?;
        let sql = format!(
            "SELECT {COLONNES_REGLE} FROM ent_asd_regle WHERE etablissement_id = $1 ORDER BY valide_du DESC"
        );
        sqlx::query_as::<_, RegleAsd>(&sql)
            .bind(etablissement_id)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
            .await
            .map_err(db)
    }

    /// Pose une nouvelle version de la règle, **jamais dans le passé** (action de direction,
    /// `assiduite.regle_gerer`). Une version du même jour non figée est remplacée ; une version
    /// figée par un mois arrêté ne change plus (le déclencheur de la base le garantit aussi).
    pub async fn definir_regle_asd_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        s: SaisieRegle,
    ) -> Result<RegleAsd, EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AssiduiteRegleGerer)
            .await?;
        s.valider()?;
        let aujourdhui = self.aujourdhui_etablissement(etablissement_id).await?;
        if s.valide_du < aujourdhui {
            return Err(EntDbError::SaisieInvalide(
                "une règle d'assiduité ne s'applique jamais dans le passé",
            ));
        }
        self.ecrire_regle(etablissement_id, acteur, s).await
    }

    /// Écriture commune (API ci-dessus et ancien `PUT /vs/regle-assiduite`).
    pub(crate) async fn ecrire_regle(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        s: SaisieRegle,
    ) -> Result<RegleAsd, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        if let Some(pid) = s.profil_id {
            let ok: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ref_asd_profil_academique WHERE id = $1")
                .bind(pid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
            if ok.is_none() {
                return Err(EntDbError::SaisieInvalide("profil académique inconnu"));
            }
        }
        let existante: Option<(Uuid, bool)> = sqlx::query_as(
            "SELECT id, figee_le IS NOT NULL FROM ent_asd_regle
              WHERE etablissement_id = $1 AND valide_du = $2 FOR UPDATE",
        )
        .bind(etablissement_id)
        .bind(s.valide_du)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some((id, figee)) = existante {
            if figee {
                return Err(EntDbError::ConflitDossier("règle figée par un mois arrêté"));
            }
            sqlx::query("DELETE FROM ent_asd_regle WHERE etablissement_id = $1 AND id = $2")
                .bind(etablissement_id)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_asd_regle
                (id, etablissement_id, valide_du, definition_demi_journee, seuil_minutes, heure_coupure,
                 type_mois, profil_id, seuil_signalement_demi_journees, delai_consolidation_min,
                 delai_verification_min, rythme_relance, seuil_retard_absence_pct, seuil_absences_repetees,
                 seuil_retards, decide_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
        )
        .bind(id)
        .bind(etablissement_id)
        .bind(s.valide_du)
        .bind(&s.definition_demi_journee)
        .bind(s.seuil_minutes)
        .bind(s.heure_coupure)
        .bind(&s.type_mois)
        .bind(s.profil_id)
        .bind(s.seuil_signalement_demi_journees)
        .bind(s.delai_consolidation_min)
        .bind(s.delai_verification_min)
        .bind(&s.rythme_relance)
        .bind(s.seuil_retard_absence_pct)
        .bind(s.seuil_absences_repetees)
        .bind(s.seuil_retards)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_asd::REGLE_DEFINIE.to_string(),
                aggregate: AggregateKind::ParamEtab,
                aggregate_id: id,
                eleve_id: None,
                payload: json!({ "valide_du": s.valide_du.to_string(), "definition": s.definition_demi_journee }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(RegleAsd {
            id,
            valide_du: s.valide_du,
            definition_demi_journee: s.definition_demi_journee,
            seuil_minutes: s.seuil_minutes,
            heure_coupure: s.heure_coupure,
            type_mois: s.type_mois,
            profil_id: s.profil_id,
            seuil_signalement_demi_journees: s.seuil_signalement_demi_journees,
            delai_consolidation_min: s.delai_consolidation_min,
            delai_verification_min: s.delai_verification_min,
            rythme_relance: s.rythme_relance,
            seuil_retard_absence_pct: s.seuil_retard_absence_pct,
            seuil_absences_repetees: s.seuil_absences_repetees,
            seuil_retards: s.seuil_retards,
            figee: false,
        })
    }

    /// Fige les règles utilisées par un mois arrêté (appelé par AS4 à l'arrêt d'un mois ; exposé ici
    /// pour que le critère « une règle utilisée par un mois arrêté est immuable » soit vérifié dès AS1).
    pub(crate) async fn figer_regle_en_vigueur(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        sqlx::query(
            "UPDATE ent_asd_regle SET figee_le = now()
              WHERE etablissement_id = $1 AND figee_le IS NULL
                AND id = (SELECT id FROM ent_asd_regle WHERE etablissement_id = $1 AND valide_du <= $2
                           ORDER BY valide_du DESC LIMIT 1)",
        )
        .bind(etablissement_id)
        .bind(jour)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ── Activités appelées ──────────────────────────────────────────────────────────────────

    /// Activités soumises à l'appel (permanence, étude, CDI…) : R131-5 l'impose à toute activité
    /// organisée pendant le temps scolaire ; l'établissement choisit celles qu'il appelle.
    pub async fn activites_appelees_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<ActiviteAppelee>, EntDbError> {
        self.exiger_lecture_assiduite(etablissement_id, acteur, role).await?;
        let mut v = self.activites_appelees(etablissement_id).await?;
        for t in TYPES_ACTIVITE_APPELEE {
            if !v.iter().any(|a| a.type_activite == t) {
                v.push(ActiviteAppelee { type_activite: t.to_string(), active: false });
            }
        }
        v.sort_by(|a, b| a.type_activite.cmp(&b.type_activite));
        Ok(v)
    }

    pub(crate) async fn activites_appelees(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Vec<ActiviteAppelee>, EntDbError> {
        sqlx::query_as::<_, ActiviteAppelee>(
            "SELECT type_activite, active FROM ent_asd_activite_appelee WHERE etablissement_id = $1",
        )
        .bind(etablissement_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?)
        .await
        .map_err(db)
    }

    /// Active ou désactive l'appel d'une activité (action `assiduite.regle_gerer`).
    pub async fn definir_activite_appelee_for_actor(
        &self,
        etablissement_id: Uuid,
        acteur: Uuid,
        role: &str,
        type_activite: &str,
        active: bool,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etablissement_id, acteur, role, Action::AssiduiteRegleGerer)
            .await?;
        if !TYPES_ACTIVITE_APPELEE.contains(&type_activite) {
            return Err(EntDbError::SaisieInvalide("type d'activité inconnu"));
        }
        let mut tx = TenantTx::begin(self.pool(), etablissement_id).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_asd_activite_appelee (etablissement_id, type_activite, active)
             VALUES ($1,$2,$3)
             ON CONFLICT (etablissement_id, type_activite) DO UPDATE SET active = EXCLUDED.active, modifie_le = now()",
        )
        .bind(etablissement_id)
        .bind(type_activite)
        .bind(active)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = tx
            .publier(MetierEventDraft {
                etablissement_id,
                type_name: types_asd::ACTIVITE_APPELEE_MODIFIEE.to_string(),
                aggregate: AggregateKind::ParamEtab,
                aggregate_id: Uuid::new_v4(),
                eleve_id: None,
                payload: json!({ "type": type_activite, "active": active }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    /// Une activité de ce type est-elle appelée dans l'établissement ?
    pub(crate) async fn activite_est_appelee(
        &self,
        etablissement_id: Uuid,
        type_activite: &str,
    ) -> Result<bool, EntDbError> {
        Ok(self
            .activites_appelees(etablissement_id)
            .await?
            .iter()
            .any(|a| a.type_activite == type_activite && a.active))
    }
}
