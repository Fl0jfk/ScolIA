//! Référentiel scolaire : années, classes, groupes, affectations, services d'enseignement.
//!
//! Domaine propriétaire des tables `ent_annee_scolaire`, `ent_classe`, `ent_groupe`,
//! `ent_affectation` et `ent_service_enseignement` (migration 092).
//!
//! **Compatibilité** : les colonnes TEXT `classe` de `ent_eleve`, `ent_edt_creneau`,
//! `ent_devoir` restent en place et font encore foi pour les droits existants. Ce module
//! les *rejoint* (`amorcer_referentiel`) et reste synchronisé quand la classe d'un élève
//! change (`sync_affectation_classe`, appelé par `PgEnt::change_classe`).
//!
//! **Cloisonnement** : toutes les requêtes filtrent `etablissement_id` ; une référence
//! (classe, élève, professeur, matière) d'un autre établissement est « introuvable ».
//!
//! **Droits** : écriture réservée à Admin / Direction / Administratif ; lecture à tout le
//! personnel. `enseigne_a` est exposée mais **volontairement non branchée** sur les droits
//! d'accès aux notes, absences ou fils (décision séparée).

use std::collections::{HashMap, HashSet};

use chrono::{Datelike, Duration, NaiveDate};
use domain_eleve::{annee_scolaire_de, today_utc, DomainError, Eleve};
use ent_auth::Role;
use sqlx::FromRow;
use uuid::Uuid;

use crate::pg_err::map_sqlx;
use crate::service::{EntDbError, PgEnt};

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct AnneeScolaire {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub libelle: String,
    pub date_debut: NaiveDate,
    pub date_fin: NaiveDate,
    pub courante: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct Classe {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub niveau: Option<String>,
}

/// Classe + effectif à une date + professeur principal éventuel.
#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ClasseResume {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub niveau: Option<String>,
    pub effectif: i64,
    pub professeur_principal_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct Groupe {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub annee_id: Uuid,
    pub code: String,
    pub libelle: String,
    pub classe_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct Affectation {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub annee_id: Uuid,
    pub eleve_id: Uuid,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub date_debut: NaiveDate,
    pub date_fin: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct ServiceEnseignement {
    pub id: Uuid,
    pub etablissement_id: Uuid,
    pub annee_id: Uuid,
    pub prof_user_id: Uuid,
    pub matiere_id: Option<Uuid>,
    /// Libellé de la matière du référentiel si elle est liée, sinon libellé saisi.
    pub matiere_libelle: String,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub professeur_principal: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RapportAmorcage {
    pub annee_id: Uuid,
    pub annee_libelle: String,
    pub classes_creees: i64,
    pub affectations_creees: i64,
    pub services_crees: i64,
    pub divergences: i64,
}

/// Commande de création d'un service d'enseignement (classe OU groupe).
#[derive(Debug, Clone)]
pub struct CreerService {
    pub prof_user_id: Uuid,
    pub matiere_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub professeur_principal: bool,
}

/// Marqueurs transmis par l'API **après** une autorisation positive du moteur de droits
/// (D-017, `referentiel.gerer` / `referentiel.lire`) : ils font passer le garde historique
/// par rôle de ces fonctions (passerelle R2). Ne jamais les construire depuis une entrée
/// utilisateur ni une session.
pub const ROLE_AUTORISE_MOTEUR_GESTION: &str = "@moteur:referentiel.gerer";
pub const ROLE_AUTORISE_MOTEUR_LECTURE: &str = "@moteur:referentiel.lire";

/// Écriture autorisée : Admin, Direction, Administratif.
pub fn role_peut_gerer_referentiel(role: &str) -> bool {
    role == ROLE_AUTORISE_MOTEUR_GESTION
        || matches!(
        Role::parse(role),
        Some(Role::Admin | Role::Direction | Role::Administratif)
    )
}

/// Lecture autorisée : tout le personnel.
pub fn role_peut_lire_referentiel(role: &str) -> bool {
    role == ROLE_AUTORISE_MOTEUR_GESTION
        || role == ROLE_AUTORISE_MOTEUR_LECTURE
        || Role::parse(role).is_some_and(Role::is_staff)
}

/// Niveau déduit du code d'une classe (« 4B » → « 4e », « 2nde3 » → « 2nde »), `None` si
/// le code ne suit pas la convention usuelle. Information d'affichage uniquement.
pub fn niveau_de_code(code: &str) -> Option<String> {
    let c = code.trim().to_lowercase();
    let mut chars = c.chars();
    match chars.next()? {
        n @ ('3' | '4' | '5' | '6') => Some(format!("{n}e")),
        '2' => Some("2nde".into()),
        '1' => Some("1re".into()),
        't' => Some("Tle".into()),
        _ => None,
    }
}

/// Bornes (1er septembre → 31 août) de l'année scolaire qui contient `jour`.
fn bornes_annee(jour: NaiveDate) -> (NaiveDate, NaiveDate) {
    let y = if jour.month() >= 9 {
        jour.year()
    } else {
        jour.year() - 1
    };
    (
        NaiveDate::from_ymd_opt(y, 9, 1).expect("1er septembre"),
        NaiveDate::from_ymd_opt(y + 1, 8, 31).expect("31 août"),
    )
}

fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

fn invalide() -> EntDbError {
    EntDbError::Domain(DomainError::ValeurInvalide)
}

fn db(err: sqlx::Error) -> EntDbError {
    EntDbError::Domain(map_sqlx("referentiel_repo", err))
}

fn exiger_gestion(role: &str) -> Result<(), EntDbError> {
    if role_peut_gerer_referentiel(role) {
        Ok(())
    } else {
        Err(EntDbError::Forbidden)
    }
}

fn exiger_lecture(role: &str) -> Result<(), EntDbError> {
    if role_peut_lire_referentiel(role) {
        Ok(())
    } else {
        Err(EntDbError::Forbidden)
    }
}

const COLONNES_CLASSE_RESUME: &str = r#"
    c.id, c.etablissement_id, c.annee_id, c.code, c.libelle, c.niveau,
    (SELECT count(*) FROM ent_affectation a
       JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
      WHERE a.etablissement_id = c.etablissement_id AND a.classe_id = c.id
        AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
        AND (e.date_sortie IS NULL OR e.date_sortie > $3))::bigint AS effectif,
    (SELECT s.prof_user_id FROM ent_service_enseignement s
      WHERE s.etablissement_id = c.etablissement_id AND s.classe_id = c.id
        AND s.professeur_principal LIMIT 1) AS professeur_principal_id
"#;

impl PgEnt {
    // ───────────────────────────── Années ─────────────────────────────

    pub async fn referentiel_annees(
        &self,
        role: &str,
        etablissement_id: Uuid,
    ) -> Result<Vec<AnneeScolaire>, EntDbError> {
        exiger_lecture(role)?;
        sqlx::query_as::<_, AnneeScolaire>(
            "SELECT id, etablissement_id, libelle, date_debut, date_fin, courante
             FROM ent_annee_scolaire WHERE etablissement_id = $1 ORDER BY date_debut DESC",
        )
        .bind(etablissement_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    pub async fn referentiel_creer_annee(
        &self,
        role: &str,
        etablissement_id: Uuid,
        libelle: &str,
        date_debut: NaiveDate,
        date_fin: NaiveDate,
        courante: bool,
    ) -> Result<AnneeScolaire, EntDbError> {
        exiger_gestion(role)?;
        let libelle = libelle.trim();
        if libelle.is_empty() || libelle.chars().count() > 40 || date_fin <= date_debut {
            return Err(invalide());
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(etablissement_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if courante {
            // Socle 5 S0 (défaut n° 5) : cette ancienne route ne retire plus la courante sans
            // la clôturer ; elle ne peut poser « courante » que s'il n'y en a pas encore. Pour
            // changer d'année : préparer, puis ouvrir (REF-02).
            let existe: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante)",
            )
            .bind(etablissement_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if existe {
                return Err(EntDbError::Domain(DomainError::Conflit));
            }
        }
        let a = sqlx::query_as::<_, AnneeScolaire>(
            "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
             VALUES ($1,$2,$3,$4,$5,$6)
             RETURNING id, etablissement_id, libelle, date_debut, date_fin, courante",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(libelle)
        .bind(date_debut)
        .bind(date_fin)
        .bind(courante)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(a)
    }

    /// Année marquée « courante » (sans rien créer).
    pub async fn referentiel_annee_courante(
        &self,
        etablissement_id: Uuid,
    ) -> Result<Option<AnneeScolaire>, EntDbError> {
        sqlx::query_as::<_, AnneeScolaire>(
            "SELECT id, etablissement_id, libelle, date_debut, date_fin, courante
             FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante",
        )
        .bind(etablissement_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)
    }

    /// Année scolaire qui contient `jour` ; créée (bascule au 1er septembre) si absente.
    /// Elle devient « courante » si c'est l'année du jour et qu'aucune ne l'est encore.
    pub async fn referentiel_assurer_annee(
        &self,
        etablissement_id: Uuid,
        jour: NaiveDate,
    ) -> Result<AnneeScolaire, EntDbError> {
        let trouvee = sqlx::query_as::<_, AnneeScolaire>(
            "SELECT id, etablissement_id, libelle, date_debut, date_fin, courante
             FROM ent_annee_scolaire
             WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2
             ORDER BY courante DESC, date_debut DESC LIMIT 1",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let est_annee_du_jour = annee_scolaire_de(jour) == annee_scolaire_de(today_utc());
        if let Some(a) = trouvee {
            if est_annee_du_jour && !a.courante && self.referentiel_annee_courante(etablissement_id).await?.is_none() {
                return self.marquer_courante(etablissement_id, a.id).await;
            }
            return Ok(a);
        }
        let (debut, fin) = bornes_annee(jour);
        let libelle = annee_scolaire_de(jour);
        let a_courante = self.referentiel_annee_courante(etablissement_id).await?.is_some();
        sqlx::query(
            "INSERT INTO ent_annee_scolaire (id, etablissement_id, libelle, date_debut, date_fin, courante)
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(&libelle)
        .bind(debut)
        .bind(fin)
        .bind(est_annee_du_jour && !a_courante)
        .execute(self.pool())
        .await
        .map_err(db)?;
        sqlx::query_as::<_, AnneeScolaire>(
            "SELECT id, etablissement_id, libelle, date_debut, date_fin, courante
             FROM ent_annee_scolaire WHERE etablissement_id = $1 AND libelle = $2",
        )
        .bind(etablissement_id)
        .bind(&libelle)
        .fetch_one(self.pool())
        .await
        .map_err(db)
    }

    async fn marquer_courante(
        &self,
        etablissement_id: Uuid,
        annee_id: Uuid,
    ) -> Result<AnneeScolaire, EntDbError> {
        sqlx::query_as::<_, AnneeScolaire>(
            "UPDATE ent_annee_scolaire SET courante = true
             WHERE etablissement_id = $1 AND id = $2
             RETURNING id, etablissement_id, libelle, date_debut, date_fin, courante",
        )
        .bind(etablissement_id)
        .bind(annee_id)
        .fetch_one(self.pool())
        .await
        .map_err(db)
    }

    /// Année courante, créée au besoin à partir de la date du jour.
    pub async fn referentiel_annee_courante_assuree(
        &self,
        etablissement_id: Uuid,
    ) -> Result<AnneeScolaire, EntDbError> {
        if let Some(a) = self.referentiel_annee_courante(etablissement_id).await? {
            return Ok(a);
        }
        self.referentiel_assurer_annee(etablissement_id, today_utc())
            .await
    }

    async fn annee_de(
        &self,
        etablissement_id: Uuid,
        annee_id: Uuid,
    ) -> Result<AnneeScolaire, EntDbError> {
        sqlx::query_as::<_, AnneeScolaire>(
            "SELECT id, etablissement_id, libelle, date_debut, date_fin, courante
             FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(annee_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    // ───────────────────────────── Classes ─────────────────────────────

    pub async fn referentiel_creer_classe(
        &self,
        role: &str,
        etablissement_id: Uuid,
        annee_id: Option<Uuid>,
        code: &str,
        libelle: Option<&str>,
        niveau: Option<&str>,
    ) -> Result<Classe, EntDbError> {
        exiger_gestion(role)?;
        let code = code.trim();
        if code.is_empty() || code.chars().count() > 40 {
            return Err(invalide());
        }
        let annee = match annee_id {
            Some(id) => self.annee_de(etablissement_id, id).await?,
            None => self.referentiel_annee_courante_assuree(etablissement_id).await?,
        };
        self.exiger_annee_modifiable(etablissement_id, annee.id).await?;
        let libelle = libelle.map(str::trim).filter(|l| !l.is_empty()).unwrap_or(code);
        let niveau = niveau
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .or_else(|| niveau_de_code(code));
        sqlx::query_as::<_, Classe>(
            "INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle, niveau)
             VALUES ($1,$2,$3,$4,$5,$6)
             RETURNING id, etablissement_id, annee_id, code, libelle, niveau",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(annee.id)
        .bind(code)
        .bind(libelle)
        .bind(niveau)
        .fetch_one(self.pool())
        .await
        .map_err(db)
    }

    pub async fn referentiel_classe(
        &self,
        etablissement_id: Uuid,
        classe_id: Uuid,
    ) -> Result<Option<Classe>, EntDbError> {
        sqlx::query_as::<_, Classe>(
            "SELECT id, etablissement_id, annee_id, code, libelle, niveau
             FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(classe_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)
    }

    /// Classe d'une année par son code (celui du champ texte historique).
    pub async fn referentiel_classe_par_code(
        &self,
        etablissement_id: Uuid,
        annee_id: Uuid,
        code: &str,
    ) -> Result<Option<Classe>, EntDbError> {
        sqlx::query_as::<_, Classe>(
            "SELECT id, etablissement_id, annee_id, code, libelle, niveau
             FROM ent_classe WHERE etablissement_id = $1 AND annee_id = $2 AND code = $3",
        )
        .bind(etablissement_id)
        .bind(annee_id)
        .bind(code.trim())
        .fetch_optional(self.pool())
        .await
        .map_err(db)
    }

    /// Classes de l'année (la courante par défaut) avec effectif au `jour`.
    pub async fn referentiel_classes(
        &self,
        role: &str,
        etablissement_id: Uuid,
        annee_id: Option<Uuid>,
        jour: NaiveDate,
    ) -> Result<Vec<ClasseResume>, EntDbError> {
        exiger_lecture(role)?;
        let annee = match annee_id {
            Some(id) => self.annee_de(etablissement_id, id).await?,
            None => self.referentiel_annee_courante_assuree(etablissement_id).await?,
        };
        sqlx::query_as::<_, ClasseResume>(&format!(
            "SELECT {COLONNES_CLASSE_RESUME} FROM ent_classe c
             WHERE c.etablissement_id = $1 AND c.annee_id = $2 ORDER BY c.code"
        ))
        .bind(etablissement_id)
        .bind(annee.id)
        .bind(jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    pub async fn referentiel_classe_resume(
        &self,
        role: &str,
        etablissement_id: Uuid,
        classe_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Option<ClasseResume>, EntDbError> {
        exiger_lecture(role)?;
        sqlx::query_as::<_, ClasseResume>(&format!(
            "SELECT {COLONNES_CLASSE_RESUME} FROM ent_classe c
             WHERE c.etablissement_id = $1 AND c.id = $2"
        ))
        // $2 est ici l'identifiant de classe ; $3 la date (même numérotation que la liste).
        .bind(etablissement_id)
        .bind(classe_id)
        .bind(jour)
        .fetch_optional(self.pool())
        .await
        .map_err(db)
    }

    /// Classes d'un professeur à la date `jour` (via ses services, directs ou par groupe).
    pub async fn classes_du_prof(
        &self,
        etablissement_id: Uuid,
        prof_user_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<ClasseResume>, EntDbError> {
        // Lecture seule (S0 n° 3) : jamais de création d'année ; aucune année à cette date = liste vide.
        let Some(annee_id): Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND date_debut <= $2 AND date_fin >= $2
              ORDER BY courante DESC, date_debut DESC LIMIT 1",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        else {
            return Ok(Vec::new());
        };
        sqlx::query_as::<_, ClasseResume>(&format!(
            "SELECT {COLONNES_CLASSE_RESUME} FROM ent_classe c
             WHERE c.etablissement_id = $1 AND c.annee_id = $2
               AND EXISTS (
                 SELECT 1 FROM ent_service_enseignement s
                  WHERE s.etablissement_id = c.etablissement_id AND s.prof_user_id = $4
                    AND (s.classe_id = c.id
                         OR s.groupe_id IN (SELECT g.id FROM ent_groupe g
                                             WHERE g.etablissement_id = c.etablissement_id
                                               AND g.classe_id = c.id)))
             ORDER BY c.code"
        ))
        .bind(etablissement_id)
        .bind(annee_id)
        .bind(jour)
        .bind(prof_user_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Élèves affectés à la classe à la date `jour` (hors élèves sortis), par ordre alphabétique.
    pub async fn eleves_de_la_classe(
        &self,
        etablissement_id: Uuid,
        classe_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Eleve>, EntDbError> {
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT a.eleve_id FROM ent_affectation a
               JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.classe_id = $2
                AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
                AND (e.date_sortie IS NULL OR e.date_sortie > $3)",
        )
        .bind(etablissement_id)
        .bind(classe_id)
        .bind(jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = ids.into_iter().map(|(i,)| i).collect();
        Ok(self.eleves_par_ids(etablissement_id, &ids).await?)
    }

    // ───────────────────────────── Groupes ─────────────────────────────

    pub async fn referentiel_creer_groupe(
        &self,
        role: &str,
        etablissement_id: Uuid,
        annee_id: Option<Uuid>,
        code: &str,
        libelle: Option<&str>,
        classe_id: Option<Uuid>,
    ) -> Result<Groupe, EntDbError> {
        exiger_gestion(role)?;
        let code = code.trim();
        if code.is_empty() || code.chars().count() > 40 {
            return Err(invalide());
        }
        let annee = match (annee_id, classe_id) {
            (_, Some(cid)) => {
                let classe = self
                    .referentiel_classe(etablissement_id, cid)
                    .await?
                    .ok_or_else(introuvable)?;
                if annee_id.is_some_and(|a| a != classe.annee_id) {
                    return Err(invalide());
                }
                self.annee_de(etablissement_id, classe.annee_id).await?
            }
            (Some(id), None) => self.annee_de(etablissement_id, id).await?,
            (None, None) => self.referentiel_annee_courante_assuree(etablissement_id).await?,
        };
        self.exiger_annee_modifiable(etablissement_id, annee.id).await?;
        let libelle = libelle.map(str::trim).filter(|l| !l.is_empty()).unwrap_or(code);
        sqlx::query_as::<_, Groupe>(
            "INSERT INTO ent_groupe (id, etablissement_id, annee_id, code, libelle, classe_id)
             VALUES ($1,$2,$3,$4,$5,$6)
             RETURNING id, etablissement_id, annee_id, code, libelle, classe_id",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(annee.id)
        .bind(code)
        .bind(libelle)
        .bind(classe_id)
        .fetch_one(self.pool())
        .await
        .map_err(db)
    }

    pub async fn referentiel_groupes_de_classe(
        &self,
        etablissement_id: Uuid,
        classe_id: Uuid,
    ) -> Result<Vec<Groupe>, EntDbError> {
        sqlx::query_as::<_, Groupe>(
            "SELECT id, etablissement_id, annee_id, code, libelle, classe_id
             FROM ent_groupe WHERE etablissement_id = $1 AND classe_id = $2 ORDER BY code",
        )
        .bind(etablissement_id)
        .bind(classe_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    async fn eleves_par_ids(
        &self,
        etablissement_id: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<Eleve>, DomainError> {
        crate::eleve_repo::PgEleveRepo::new(self.pool().clone())
            .list_by_ids(etablissement_id, ids)
            .await
    }

    // ─────────────────────────── Affectations ───────────────────────────

    /// Affecte un élève à une classe (clôt l'affectation de classe en cours de l'année).
    pub async fn referentiel_affecter_eleve_classe(
        &self,
        role: &str,
        etablissement_id: Uuid,
        classe_id: Uuid,
        eleve_id: Uuid,
        date_debut: Option<NaiveDate>,
    ) -> Result<Affectation, EntDbError> {
        exiger_gestion(role)?;
        let classe = self
            .referentiel_classe(etablissement_id, classe_id)
            .await?
            .ok_or_else(introuvable)?;
        self.exiger_eleve(etablissement_id, eleve_id).await?;
        let annee = self.annee_de(etablissement_id, classe.annee_id).await?;
        let debut = date_debut.unwrap_or_else(|| today_utc().clamp(annee.date_debut, annee.date_fin));
        if debut < annee.date_debut || debut > annee.date_fin {
            return Err(invalide());
        }
        self.exiger_annee_modifiable(etablissement_id, annee.id).await?;
        self.poser_affectation_classe(&annee, &classe, eleve_id, debut, "saisie")
            .await
    }

    /// Affecte un élève à un groupe (plusieurs groupes possibles ; sans doublon en cours).
    pub async fn referentiel_affecter_eleve_groupe(
        &self,
        role: &str,
        etablissement_id: Uuid,
        groupe_id: Uuid,
        eleve_id: Uuid,
        date_debut: Option<NaiveDate>,
    ) -> Result<Affectation, EntDbError> {
        exiger_gestion(role)?;
        let groupe = sqlx::query_as::<_, Groupe>(
            "SELECT id, etablissement_id, annee_id, code, libelle, classe_id
             FROM ent_groupe WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(groupe_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        self.exiger_eleve(etablissement_id, eleve_id).await?;
        let annee = self.annee_de(etablissement_id, groupe.annee_id).await?;
        self.exiger_annee_modifiable(etablissement_id, annee.id).await?;
        let debut = date_debut.unwrap_or_else(|| today_utc().clamp(annee.date_debut, annee.date_fin));
        if debut < annee.date_debut || debut > annee.date_fin {
            return Err(invalide());
        }
        let existe: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_affectation
             WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3 AND date_fin IS NULL",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(groupe_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        if existe.is_some() {
            return Err(EntDbError::Domain(DomainError::Conflit));
        }
        sqlx::query_as::<_, Affectation>(
            "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, groupe_id, date_debut, source)
             VALUES ($1,$2,$3,$4,$5,$6,'saisie')
             RETURNING id, etablissement_id, annee_id, eleve_id, classe_id, groupe_id, date_debut, date_fin",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(annee.id)
        .bind(eleve_id)
        .bind(groupe_id)
        .bind(debut)
        .fetch_one(self.pool())
        .await
        .map_err(db)
    }

    /// Élèves membres d'un groupe à la date `jour` (hors élèves sortis), ordre alphabétique.
    pub async fn referentiel_membres_groupe(
        &self,
        etablissement_id: Uuid,
        groupe_id: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Eleve>, EntDbError> {
        self.groupe_de(etablissement_id, groupe_id).await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT a.eleve_id FROM ent_affectation a
               JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
              WHERE a.etablissement_id = $1 AND a.groupe_id = $2
                AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
                AND (e.date_sortie IS NULL OR e.date_sortie > $3)",
        )
        .bind(etablissement_id)
        .bind(groupe_id)
        .bind(jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = ids.into_iter().map(|(i,)| i).collect();
        Ok(self.eleves_par_ids(etablissement_id, &ids).await?)
    }

    async fn groupe_de(&self, etablissement_id: Uuid, groupe_id: Uuid) -> Result<Groupe, EntDbError> {
        sqlx::query_as::<_, Groupe>(
            "SELECT id, etablissement_id, annee_id, code, libelle, classe_id
             FROM ent_groupe WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etablissement_id)
        .bind(groupe_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    /// Ajoute un élève à un groupe (RP1-02) : droit de gestion du référentiel, **journalisé**.
    pub async fn referentiel_ajouter_membre_groupe(
        &self,
        role: &str,
        actor_user_id: Uuid,
        etablissement_id: Uuid,
        groupe_id: Uuid,
        eleve_id: Uuid,
        date_debut: Option<NaiveDate>,
    ) -> Result<Affectation, EntDbError> {
        let a = self
            .referentiel_affecter_eleve_groupe(role, etablissement_id, groupe_id, eleve_id, date_debut)
            .await?;
        let g = self.groupe_de(etablissement_id, groupe_id).await?;
        self.journaliser_groupe(
            etablissement_id,
            actor_user_id,
            eleve_id,
            "groupe_membre_ajoute",
            format!("Élève ajouté au groupe {}", g.libelle),
            &g,
            a.date_debut,
        )
        .await;
        Ok(a)
    }

    /// Retire un élève d'un groupe (RP1-02) : clôt l'affectation à `date_fin` (aujourd'hui par
    /// défaut) ; une affectation qui n'a pas encore commencé est supprimée (correction).
    /// Droit de gestion du référentiel, **journalisé**.
    pub async fn referentiel_retirer_membre_groupe(
        &self,
        role: &str,
        actor_user_id: Uuid,
        etablissement_id: Uuid,
        groupe_id: Uuid,
        eleve_id: Uuid,
        date_fin: Option<NaiveDate>,
    ) -> Result<(), EntDbError> {
        exiger_gestion(role)?;
        let g = self.groupe_de(etablissement_id, groupe_id).await?;
        self.exiger_annee_modifiable(etablissement_id, g.annee_id).await?;
        let fin = date_fin.unwrap_or_else(today_utc);
        let ouverte: Option<(Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT id, date_debut FROM ent_affectation
              WHERE etablissement_id = $1 AND eleve_id = $2 AND groupe_id = $3 AND date_fin IS NULL",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(groupe_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let (id, debut) = ouverte.ok_or_else(introuvable)?;
        if fin < debut {
            sqlx::query("DELETE FROM ent_affectation WHERE etablissement_id = $1 AND id = $2")
                .bind(etablissement_id)
                .bind(id)
                .execute(self.pool())
                .await
                .map_err(db)?;
        } else {
            sqlx::query(
                "UPDATE ent_affectation SET date_fin = $3 WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etablissement_id)
            .bind(id)
            .bind(fin)
            .execute(self.pool())
            .await
            .map_err(db)?;
        }
        self.journaliser_groupe(
            etablissement_id,
            actor_user_id,
            eleve_id,
            "groupe_membre_retire",
            format!("Élève retiré du groupe {}", g.libelle),
            &g,
            fin,
        )
        .await;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn journaliser_groupe(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        eleve_id: Uuid,
        action: &str,
        libelle: String,
        g: &Groupe,
        date_effet: NaiveDate,
    ) {
        let r = self
            .enregistrer_journal_interne(domain_eleve::EnregistrerJournal {
                etablissement_id,
                eleve_id: Some(eleve_id),
                domaine: "referentiel".into(),
                action: action.into(),
                libelle,
                payload: serde_json::json!({
                    "groupe_id": g.id, "groupe_code": g.code, "date_effet": date_effet,
                }),
                visible_famille: false,
                actor_user_id: Some(actor_user_id),
            })
            .await;
        if let Err(e) = r {
            tracing::error!(error = %e, "échec d'écriture du journal d'appartenance à un groupe");
        }
    }

    async fn exiger_eleve(&self, etablissement_id: Uuid, eleve_id: Uuid) -> Result<(), EntDbError> {
        let ok: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                .bind(etablissement_id)
                .bind(eleve_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?;
        ok.map(|_| ()).ok_or_else(introuvable)
    }

    /// Cœur partagé (saisie, synchronisation) : une seule classe « en cours » par année.
    async fn poser_affectation_classe(
        &self,
        annee: &AnneeScolaire,
        classe: &Classe,
        eleve_id: Uuid,
        date_effet: NaiveDate,
        source: &str,
    ) -> Result<Affectation, EntDbError> {
        let etab = annee.etablissement_id;
        let debut = date_effet.max(annee.date_debut);
        let mut tx = self.pool().begin().await.map_err(db)?;
        let ouverte: Option<Affectation> = sqlx::query_as(
            "SELECT id, etablissement_id, annee_id, eleve_id, classe_id, groupe_id, date_debut, date_fin
             FROM ent_affectation
             WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3
               AND classe_id IS NOT NULL AND date_fin IS NULL FOR UPDATE",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(annee.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(o) = ouverte {
            if o.classe_id == Some(classe.id) {
                tx.commit().await.map_err(db)?;
                return Ok(o);
            }
            if debut <= o.date_debut {
                // Correction d'une affectation posée à tort (même jour ou antérieure).
                sqlx::query("DELETE FROM ent_affectation WHERE id = $1 AND etablissement_id = $2")
                    .bind(o.id)
                    .bind(etab)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
            } else {
                sqlx::query(
                    "UPDATE ent_affectation SET date_fin = $3 WHERE id = $1 AND etablissement_id = $2",
                )
                .bind(o.id)
                .bind(etab)
                .bind(debut - Duration::days(1))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }
        let n = sqlx::query_as::<_, Affectation>(
            "INSERT INTO ent_affectation (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut, source)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             RETURNING id, etablissement_id, annee_id, eleve_id, classe_id, groupe_id, date_debut, date_fin",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(annee.id)
        .bind(eleve_id)
        .bind(classe.id)
        .bind(debut)
        .bind(source)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(n)
    }

    /// Reflète dans le référentiel un changement de classe (texte) d'un élève :
    /// appelée par `change_classe`. Une classe vide clôt simplement l'affectation en cours ;
    /// une classe inconnue est créée dans l'année concernée.
    pub async fn sync_affectation_classe(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
        nouvelle_classe: &str,
        date_effet: NaiveDate,
    ) -> Result<(), EntDbError> {
        let annee = self
            .referentiel_assurer_annee(etablissement_id, date_effet)
            .await?;
        let code = nouvelle_classe.trim();
        if code.is_empty() {
            sqlx::query(
                "UPDATE ent_affectation SET date_fin = GREATEST(date_debut, $4::date - 1)
                 WHERE etablissement_id = $1 AND eleve_id = $2 AND annee_id = $3
                   AND classe_id IS NOT NULL AND date_fin IS NULL",
            )
            .bind(etablissement_id)
            .bind(eleve_id)
            .bind(annee.id)
            .bind(date_effet)
            .execute(self.pool())
            .await
            .map_err(db)?;
            return Ok(());
        }
        let classe = match self
            .referentiel_classe_par_code(etablissement_id, annee.id, code)
            .await?
        {
            Some(c) => c,
            None => {
                sqlx::query(
                    "INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle, niveau)
                     VALUES ($1,$2,$3,$4,$4,$5) ON CONFLICT DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(etablissement_id)
                .bind(annee.id)
                .bind(code)
                .bind(niveau_de_code(code))
                .execute(self.pool())
                .await
                .map_err(db)?;
                self.referentiel_classe_par_code(etablissement_id, annee.id, code)
                    .await?
                    .ok_or_else(introuvable)?
            }
        };
        self.poser_affectation_classe(&annee, &classe, eleve_id, date_effet, "sync_classe")
            .await?;
        Ok(())
    }

    // ───────────────────────────── Services ─────────────────────────────

    pub async fn referentiel_creer_service(
        &self,
        role: &str,
        etablissement_id: Uuid,
        cmd: CreerService,
    ) -> Result<ServiceEnseignement, EntDbError> {
        exiger_gestion(role)?;
        // Classe XOR groupe, et la cible doit appartenir à l'établissement.
        let (annee_id, classe_id, groupe_id) = match (cmd.classe_id, cmd.groupe_id) {
            (Some(c), None) => {
                let classe = self
                    .referentiel_classe(etablissement_id, c)
                    .await?
                    .ok_or_else(introuvable)?;
                (classe.annee_id, Some(c), None)
            }
            (None, Some(g)) => {
                let a: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT annee_id FROM ent_groupe WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etablissement_id)
                .bind(g)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?;
                (a.ok_or_else(introuvable)?.0, None, Some(g))
            }
            _ => return Err(invalide()),
        };
        if cmd.professeur_principal && classe_id.is_none() {
            return Err(invalide());
        }
        self.exiger_annee_modifiable(etablissement_id, annee_id).await?;
        // Le professeur doit être un membre du personnel de cet établissement.
        let membre: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE user_id = $1 AND etablissement_id = $2",
        )
        .bind(cmd.prof_user_id)
        .bind(etablissement_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        if !membre.is_some_and(|(r,)| role_peut_lire_referentiel(&r)) {
            return Err(introuvable());
        }
        let libelle_matiere: String = match cmd.matiere_id {
            Some(m) => {
                let r: Option<(String,)> = sqlx::query_as(
                    "SELECT libelle FROM ent_matiere WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etablissement_id)
                .bind(m)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?;
                r.ok_or_else(introuvable)?.0
            }
            None => String::new(),
        };
        let mut tx = self.pool().begin().await.map_err(db)?;
        if cmd.professeur_principal {
            sqlx::query(
                "UPDATE ent_service_enseignement SET professeur_principal = false
                 WHERE etablissement_id = $1 AND classe_id = $2 AND professeur_principal",
            )
            .bind(etablissement_id)
            .bind(classe_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let s = sqlx::query_as::<_, ServiceEnseignement>(
            "INSERT INTO ent_service_enseignement
               (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle,
                classe_id, groupe_id, professeur_principal)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             RETURNING id, etablissement_id, annee_id, prof_user_id, matiere_id,
                       matiere_libelle, classe_id, groupe_id, professeur_principal",
        )
        .bind(Uuid::new_v4())
        .bind(etablissement_id)
        .bind(annee_id)
        .bind(cmd.prof_user_id)
        .bind(cmd.matiere_id)
        .bind(&libelle_matiere)
        .bind(classe_id)
        .bind(groupe_id)
        .bind(cmd.professeur_principal)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(s)
    }

    /// Services d'une classe, y compris ceux de ses groupes.
    pub async fn referentiel_services_de_classe(
        &self,
        etablissement_id: Uuid,
        classe_id: Uuid,
    ) -> Result<Vec<ServiceEnseignement>, EntDbError> {
        sqlx::query_as::<_, ServiceEnseignement>(
            "SELECT s.id, s.etablissement_id, s.annee_id, s.prof_user_id, s.matiere_id,
                    coalesce(m.libelle, s.matiere_libelle) AS matiere_libelle,
                    s.classe_id, s.groupe_id, s.professeur_principal
               FROM ent_service_enseignement s
               LEFT JOIN ent_matiere m ON m.id = s.matiere_id AND m.etablissement_id = s.etablissement_id
              WHERE s.etablissement_id = $1
                AND s.prof_user_id IS NOT NULL  -- P3 : un service vacant n'a pas (encore) de professeur
                AND (s.classe_id = $2
                     OR s.groupe_id IN (SELECT g.id FROM ent_groupe g
                                         WHERE g.etablissement_id = $1 AND g.classe_id = $2))
              ORDER BY s.professeur_principal DESC, matiere_libelle, s.created_at",
        )
        .bind(etablissement_id)
        .bind(classe_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    /// Le professeur enseigne-t-il à cet élève à cette date ?
    /// (service du professeur ∩ affectation de l'élève, dans l'année qui contient la date.)
    ///
    /// **Non branchée** sur les droits existants (notes, absences, fils) : décision séparée.
    pub async fn enseigne_a(
        &self,
        etablissement_id: Uuid,
        prof_user_id: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_public_cours_jour($1, $4, NULL, $2, $3))",
        )
        .bind(etablissement_id)
        .bind(prof_user_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        Ok(ok)
    }

    // ───────────────────────────── Amorçage ─────────────────────────────

    /// Rejoint l'existant, **idempotent** : crée (sans jamais modifier ni supprimer) les
    /// classes des valeurs distinctes de `ent_eleve.classe` et `ent_edt_creneau.classe`,
    /// les affectations des élèves présents, et les services (prof × classe × matière) des
    /// créneaux EDT, pour l'année courante.
    pub async fn amorcer_referentiel(
        &self,
        etablissement_id: Uuid,
    ) -> Result<RapportAmorcage, EntDbError> {
        let annee = self.referentiel_annee_courante_assuree(etablissement_id).await?;
        let jour = today_utc().clamp(annee.date_debut, annee.date_fin);

        // 1. Classes.
        let codes: Vec<(String,)> = sqlx::query_as(
            "SELECT DISTINCT btrim(classe) FROM ent_eleve
              WHERE etablissement_id = $1 AND classe IS NOT NULL AND btrim(classe) <> ''
                AND (date_sortie IS NULL OR date_sortie > $2)
             UNION
             SELECT DISTINCT btrim(classe) FROM ent_edt_creneau
              WHERE etablissement_id = $1 AND btrim(classe) <> ''",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut rapport = RapportAmorcage {
            annee_id: annee.id,
            annee_libelle: annee.libelle.clone(),
            ..Default::default()
        };
        for (code,) in &codes {
            let r = sqlx::query(
                "INSERT INTO ent_classe (id, etablissement_id, annee_id, code, libelle, niveau)
                 VALUES ($1,$2,$3,$4,$4,$5) ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etablissement_id)
            .bind(annee.id)
            .bind(code)
            .bind(niveau_de_code(code))
            .execute(self.pool())
            .await
            .map_err(db)?;
            rapport.classes_creees += r.rows_affected() as i64;
        }

        // 2. Affectations des élèves présents (aucune affectation existante n'est modifiée).
        let classes: HashMap<String, Uuid> = sqlx::query_as::<_, (String, Uuid)>(
            "SELECT code, id FROM ent_classe WHERE etablissement_id = $1 AND annee_id = $2",
        )
        .bind(etablissement_id)
        .bind(annee.id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let ouvertes: HashMap<Uuid, Uuid> = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT eleve_id, classe_id FROM ent_affectation
              WHERE etablissement_id = $1 AND annee_id = $2
                AND classe_id IS NOT NULL AND date_fin IS NULL",
        )
        .bind(etablissement_id)
        .bind(annee.id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let eleves: Vec<(Uuid, String, Option<NaiveDate>)> = sqlx::query_as(
            "SELECT id, btrim(classe), date_entree FROM ent_eleve
              WHERE etablissement_id = $1 AND classe IS NOT NULL AND btrim(classe) <> ''
                AND (date_sortie IS NULL OR date_sortie > $2)",
        )
        .bind(etablissement_id)
        .bind(jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        for (eleve_id, code, entree) in eleves {
            let Some(&classe_id) = classes.get(&code) else {
                continue;
            };
            match ouvertes.get(&eleve_id) {
                Some(&deja) if deja == classe_id => {}
                Some(_) => rapport.divergences += 1,
                None => {
                    let debut = entree.unwrap_or(annee.date_debut).max(annee.date_debut);
                    if debut > annee.date_fin {
                        continue;
                    }
                    let r = sqlx::query(
                        "INSERT INTO ent_affectation
                           (id, etablissement_id, annee_id, eleve_id, classe_id, date_debut, source)
                         VALUES ($1,$2,$3,$4,$5,$6,'amorcage') ON CONFLICT DO NOTHING",
                    )
                    .bind(Uuid::new_v4())
                    .bind(etablissement_id)
                    .bind(annee.id)
                    .bind(eleve_id)
                    .bind(classe_id)
                    .bind(debut)
                    .execute(self.pool())
                    .await
                    .map_err(db)?;
                    rapport.affectations_creees += r.rows_affected() as i64;
                }
            }
        }

        // 3. Services d'enseignement depuis l'emploi du temps.
        let r = sqlx::query(
            "INSERT INTO ent_service_enseignement
               (id, etablissement_id, annee_id, prof_user_id, matiere_id, matiere_libelle, classe_id)
             SELECT gen_random_uuid(), $1, $2, t.prof_user_id, t.matiere_id, t.matiere_libelle, t.classe_id
               FROM (SELECT DISTINCT c.prof_user_id, m.id AS matiere_id,
                            coalesce(m.libelle, c.matiere_libelle) AS matiere_libelle,
                            cl.id AS classe_id
                       FROM ent_edt_creneau c
                       JOIN ent_classe cl
                         ON cl.etablissement_id = c.etablissement_id AND cl.annee_id = $2
                        AND cl.code = btrim(c.classe)
                       LEFT JOIN ent_matiere m
                         ON m.id = c.matiere_id AND m.etablissement_id = c.etablissement_id
                      WHERE c.etablissement_id = $1) t
             ON CONFLICT DO NOTHING",
        )
        .bind(etablissement_id)
        .bind(annee.id)
        .execute(self.pool())
        .await
        .map_err(db)?;
        rapport.services_crees = r.rows_affected() as i64;
        Ok(rapport)
    }

    /// Codes de classe distincts d'un ensemble d'identifiants d'élèves (utilitaire de tests).
    pub async fn referentiel_codes_classes(
        &self,
        etablissement_id: Uuid,
        annee_id: Uuid,
    ) -> Result<HashSet<String>, EntDbError> {
        let v: Vec<(String,)> = sqlx::query_as(
            "SELECT code FROM ent_classe WHERE etablissement_id = $1 AND annee_id = $2",
        )
        .bind(etablissement_id)
        .bind(annee_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(v.into_iter().map(|(c,)| c).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn niveau_deduit_du_code() {
        assert_eq!(niveau_de_code("4B").as_deref(), Some("4e"));
        assert_eq!(niveau_de_code(" 6a ").as_deref(), Some("6e"));
        assert_eq!(niveau_de_code("3C").as_deref(), Some("3e"));
        assert_eq!(niveau_de_code("2nde3").as_deref(), Some("2nde"));
        assert_eq!(niveau_de_code("1ES").as_deref(), Some("1re"));
        assert_eq!(niveau_de_code("TS2").as_deref(), Some("Tle"));
        assert_eq!(niveau_de_code("ULIS"), None);
        assert_eq!(niveau_de_code(""), None);
    }

    #[test]
    fn bornes_de_l_annee() {
        let d = |a, m, j| NaiveDate::from_ymd_opt(a, m, j).unwrap();
        assert_eq!(bornes_annee(d(2026, 11, 4)), (d(2026, 9, 1), d(2027, 8, 31)));
        assert_eq!(bornes_annee(d(2027, 8, 31)), (d(2026, 9, 1), d(2027, 8, 31)));
        assert_eq!(bornes_annee(d(2027, 9, 1)), (d(2027, 9, 1), d(2028, 8, 31)));
    }

    #[test]
    fn droits_du_referentiel() {
        for r in ["admin", "direction", "administratif"] {
            assert!(role_peut_gerer_referentiel(r) && role_peut_lire_referentiel(r));
        }
        for r in ["prof", "cpe", "compta", "infirmerie", "psy"] {
            assert!(!role_peut_gerer_referentiel(r) && role_peut_lire_referentiel(r), "{r}");
        }
        for r in ["parent", "eleve", "inconnu"] {
            assert!(!role_peut_gerer_referentiel(r) && !role_peut_lire_referentiel(r), "{r}");
        }
    }
}
