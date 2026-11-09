//! Socle 5, lot S4 : l'objet « cours » (`05_STRUCTURE.md` §9.7, §17 M7, STR-57, Q-S3, Q-S20, R-S5).
//!
//! Un **cours** dit *ce qui est enseigné à qui* : une matière, à une classe entière OU à un groupe,
//! pour une année. Le **service** (`ent_service_enseignement`, socle 6) dit *qui* l'enseigne ; un
//! cours peut avoir zéro service (**poste vacant**), un ou plusieurs (co-intervention).
//!
//! * droits : lecture `referentiel.lire` (tout le personnel ; la liste ne nomme aucun professeur),
//!   écriture et détail des services `referentiel.gerer` ; l'établissement vient toujours de la
//!   session ;
//! * invariants tenus par la base (migration 341) : public, matière et année d'un cours ne changent
//!   jamais (changer de public = fermer et ouvrir), un cours se ferme et ne se supprime pas, un seul
//!   cours ouvert par (année, public, matière), année clôturée = gel ;
//! * chaque écriture est journalisée (avant / après) ; les événements ne portent aucune identité ;
//! * le **public** d'un cours à une date n'est calculé que par `ent_public_cours_jour` ; la
//!   visibilité d'un professeur ne passe que par là (D-022).
//!
//! **Contrat de lecture pour les autres socles** : le module [`cours`] (fonctions qui prennent la
//! connexion ou la transaction de l'appelant, sans acteur : l'appelant a déjà autorisé).
//! * socle 6 (personnels) : [`cours::lire`], [`cours::services_du_cours`], [`cours::vacants`],
//!   [`cours::poser_service`] — il pose ses services/interventions SUR un cours, jamais l'inverse ;
//! * socle 10 (emploi du temps) : [`cours::du_service`], [`cours::lire`], [`cours::public`] et la
//!   colonne `ent_edt_creneau.cours_id` (remplie par la base à partir du service ; un créneau peut
//!   viser un cours sans service) ;
//! * socles 12, 20, 21 (appel, cahier, évaluations) : `ent_service_enseignement.cours_id`.

use chrono::NaiveDate;
use ent_auth::Action;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::structure_etab::structure::aujourdhui;
use crate::structure_etab::{journaliser, net, publier};
use crate::tenant_tx::TenantTx;

pub const SEMAINES: [&str; 3] = ["toutes", "A", "B"];

async fn tx(e: &PgEnt, a: &Acteur<'_>) -> Result<TenantTx<'static>, EntDbError> {
    TenantTx::begin(e.pool(), a.etablissement_id)
        .await
        .map_err(db)
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CoursVue {
    pub id: Uuid,
    pub annee_id: Uuid,
    pub libelle: String,
    pub matiere_id: Option<Uuid>,
    pub offre_id: Option<Uuid>,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    /// Code de la classe ou du groupe.
    pub cible_code: String,
    pub heures_hebdo: Option<f64>,
    pub semaines: Option<String>,
    pub ouvert_le: NaiveDate,
    pub ferme_le: Option<NaiveDate>,
    /// `saisie`, `reprise`, `service`, `generation`.
    pub source: String,
    /// Reprise d'un service sans matière ni libellé : à revoir par la gestion.
    pub a_verifier: bool,
    pub nb_services: i64,
    /// Ouvert et sans aucun service.
    pub vacant: bool,
}

const SQL_COURS: &str = "SELECT c.id, c.annee_id, c.libelle, c.matiere_id, c.offre_id, c.classe_id, c.groupe_id,
        COALESCE(cl.code, g.code, '') AS cible_code, c.heures_hebdo::float8 AS heures_hebdo, c.semaines,
        c.ouvert_le, c.ferme_le, c.source, c.a_verifier,
        (SELECT count(*) FROM ent_service_enseignement s
          WHERE s.etablissement_id = c.etablissement_id AND s.cours_id = c.id) AS nb_services,
        (c.ferme_le IS NULL AND NOT EXISTS (SELECT 1 FROM ent_service_enseignement s
          WHERE s.etablissement_id = c.etablissement_id AND s.cours_id = c.id)) AS vacant
   FROM ent_cours c
   LEFT JOIN ent_classe cl ON cl.id = c.classe_id AND cl.etablissement_id = c.etablissement_id
   LEFT JOIN ent_groupe g ON g.id = c.groupe_id AND g.etablissement_id = c.etablissement_id
  WHERE c.etablissement_id = $1";

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ServiceDuCours {
    pub service_id: Uuid,
    pub prof_user_id: Uuid,
    pub professeur_principal: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoursDetail {
    #[serde(flatten)]
    pub cours: CoursVue,
    pub services: Vec<ServiceDuCours>,
}

fn json_cours(c: &CoursVue) -> Value {
    json!({
        "libelle": c.libelle, "matiere_id": c.matiere_id, "offre_id": c.offre_id,
        "classe_id": c.classe_id, "groupe_id": c.groupe_id, "heures_hebdo": c.heures_hebdo,
        "semaines": c.semaines, "ouvert_le": c.ouvert_le, "ferme_le": c.ferme_le,
    })
}

// ───────────────────────── Contrat de lecture (socles 6, 10, …) ─────────────────────────

/// Fonctions sans acteur, qui prennent la connexion ou la transaction de l'appelant
/// (`&mut TenantTx`). L'appelant a déjà autorisé ; l'établissement est toujours un paramètre.
pub mod cours {
    use super::*;

    /// Un cours de l'établissement (`None` s'il est inconnu ou d'un autre établissement).
    pub async fn lire(
        conn: &mut PgConnection,
        etab: Uuid,
        cours_id: Uuid,
    ) -> Result<Option<CoursVue>, EntDbError> {
        sqlx::query_as(&format!("{SQL_COURS} AND c.id = $2"))
            .bind(etab)
            .bind(cours_id)
            .fetch_optional(conn)
            .await
            .map_err(db)
    }

    /// Cours auquel se rattache un service (`None` : service inconnu ou pas encore repris).
    pub async fn du_service(
        conn: &mut PgConnection,
        etab: Uuid,
        service_id: Uuid,
    ) -> Result<Option<Uuid>, EntDbError> {
        let r: Option<Option<Uuid>> = sqlx::query_scalar(
            "SELECT cours_id FROM ent_service_enseignement WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(service_id)
        .fetch_optional(conn)
        .await
        .map_err(db)?;
        Ok(r.flatten())
    }

    /// Services posés sur un cours (aucun = poste vacant).
    pub async fn services_du_cours(
        conn: &mut PgConnection,
        etab: Uuid,
        cours_id: Uuid,
    ) -> Result<Vec<ServiceDuCours>, EntDbError> {
        sqlx::query_as(
            "SELECT id AS service_id, prof_user_id, professeur_principal
               FROM ent_service_enseignement
              WHERE etablissement_id = $1 AND cours_id = $2 ORDER BY created_at, id",
        )
        .bind(etab)
        .bind(cours_id)
        .fetch_all(conn)
        .await
        .map_err(db)
    }

    /// Postes vacants à `jour` : cours ouverts, sans aucun service.
    pub async fn vacants(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
    ) -> Result<Vec<Uuid>, EntDbError> {
        sqlx::query_scalar("SELECT cours_id FROM ent_cours_vacants($1, $2) ORDER BY 1")
            .bind(etab)
            .bind(jour)
            .fetch_all(conn)
            .await
            .map_err(db)
    }

    /// Public d'un cours à `jour` par la règle unique (inclusions et exclusions comprises).
    pub async fn public(
        conn: &mut PgConnection,
        etab: Uuid,
        jour: NaiveDate,
        cours_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        sqlx::query_scalar(
            "SELECT eleve_id FROM ent_public_cours_jour($1, $2, $3, NULL, NULL) ORDER BY 1",
        )
        .bind(etab)
        .bind(jour)
        .bind(cours_id)
        .fetch_all(conn)
        .await
        .map_err(db)
    }

    /// Pose un service SUR un cours (le professeur prend le poste). La cible du service est recopiée
    /// du cours par la base ; le professeur doit être un membre du personnel de l'établissement.
    /// Le socle 6 remplacera cette pose par ses interventions sans changer le cours.
    pub async fn poser_service(
        conn: &mut PgConnection,
        etab: Uuid,
        cours_id: Uuid,
        prof_user_id: Uuid,
        professeur_principal: bool,
    ) -> Result<Uuid, EntDbError> {
        let c = lire(conn, etab, cours_id).await?.ok_or_else(introuvable)?;
        if c.ferme_le.is_some() {
            return Err(invalide());
        }
        if professeur_principal && c.classe_id.is_none() {
            return Err(invalide());
        }
        let membre: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE user_id = $1 AND etablissement_id = $2",
        )
        .bind(prof_user_id)
        .bind(etab)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if !membre.is_some_and(|(r,)| crate::referentiel_repo::role_peut_lire_referentiel(&r)) {
            return Err(introuvable());
        }
        let deja: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_service_enseignement
                             WHERE etablissement_id = $1 AND cours_id = $2 AND prof_user_id = $3)",
        )
        .bind(etab)
        .bind(cours_id)
        .bind(prof_user_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if deja {
            return Err(conflit());
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_service_enseignement
               (id, etablissement_id, annee_id, prof_user_id, cours_id, professeur_principal)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(etab)
        .bind(c.annee_id)
        .bind(prof_user_id)
        .bind(cours_id)
        .bind(professeur_principal)
        .execute(conn)
        .await
        .map_err(db)?;
        Ok(id)
    }

    /// Créneaux d'emploi du temps sans cours (compteur de M11 : doit tomber à zéro).
    pub async fn creneaux_sans_cours(
        conn: &mut PgConnection,
        etab: Uuid,
    ) -> Result<i64, EntDbError> {
        sqlx::query_scalar(
            "SELECT count(*) FROM ent_edt_creneau WHERE etablissement_id = $1 AND cours_id IS NULL",
        )
        .bind(etab)
        .fetch_one(conn)
        .await
        .map_err(db)
    }
}

// ───────────────────────── Commandes ─────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CoursSaisie {
    /// Public : exactement un des deux (l'année en découle).
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub matiere_id: Option<Uuid>,
    /// Obligatoire sans matière ; sinon, par défaut le libellé de la matière.
    pub libelle: Option<String>,
    pub offre_id: Option<Uuid>,
    pub heures_hebdo: Option<f64>,
    pub semaines: Option<String>,
    /// Défaut : début de l'année.
    pub ouvert_le: Option<NaiveDate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoursModif {
    pub libelle: Option<String>,
    pub offre_id: Option<Uuid>,
    pub heures_hebdo: Option<f64>,
    pub semaines: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoursFerme {
    pub cours: CoursVue,
    /// Créneaux hebdomadaires qui visent encore ce cours (à reprendre au socle 10).
    pub creneaux_touches: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EtatReprise {
    pub services: i64,
    pub services_avec_cours: i64,
    pub services_a_traiter: i64,
    pub cours: i64,
    pub cours_vacants: i64,
    pub cours_a_verifier: i64,
    pub creneaux: i64,
    pub creneaux_avec_service: i64,
    pub creneaux_sans_cours: i64,
    pub membres: i64,
    pub membres_sur_un_cours: i64,
}

fn heures_valides(h: Option<f64>) -> Result<Option<f64>, EntDbError> {
    match h {
        None => Ok(None),
        Some(v) if v.is_finite() && (0.0..=40.0).contains(&v) => {
            Ok(Some((v * 100.0).round() / 100.0))
        }
        Some(_) => Err(invalide()),
    }
}

fn semaines_valides(s: Option<&str>) -> Result<Option<String>, EntDbError> {
    match net(s, 10)? {
        None => Ok(None),
        Some(v) if SEMAINES.contains(&v.as_str()) => Ok(Some(v)),
        Some(_) => Err(invalide()),
    }
}

impl PgEnt {
    /// Cours de l'établissement. Sans `annee_id`, l'année qui contient aujourd'hui.
    pub async fn structure_cours_liste(
        &self,
        a: &Acteur<'_>,
        annee_id: Option<Uuid>,
        classe_id: Option<Uuid>,
        groupe_id: Option<Uuid>,
        vacants_seulement: bool,
        avec_fermes: bool,
    ) -> Result<Vec<CoursVue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let jour = aujourdhui(&mut t, a.etablissement_id).await?;
        let annee: Option<Uuid> = match annee_id {
            Some(y) => Some(y),
            None => sqlx::query_scalar(
                "SELECT id FROM ent_annee_scolaire WHERE etablissement_id = $1
                    AND date_debut <= $2 AND date_fin >= $2 ORDER BY date_debut DESC LIMIT 1",
            )
            .bind(a.etablissement_id)
            .bind(jour)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?,
        };
        let Some(annee) = annee else {
            t.commit().await.map_err(db)?;
            return Ok(Vec::new());
        };
        let rows = sqlx::query_as::<_, CoursVue>(&format!(
            "{SQL_COURS} AND c.annee_id = $2
                AND ($3::uuid IS NULL OR c.classe_id = $3) AND ($4::uuid IS NULL OR c.groupe_id = $4)
                AND ($5 OR c.ferme_le IS NULL)
              ORDER BY cible_code, c.libelle, c.id"
        ))
        .bind(a.etablissement_id)
        .bind(annee)
        .bind(classe_id)
        .bind(groupe_id)
        .bind(avec_fermes)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(if vacants_seulement {
            rows.into_iter().filter(|c| c.vacant).collect()
        } else {
            rows
        })
    }

    /// Postes vacants à la date du jour de l'établissement.
    pub async fn structure_cours_vacants(
        &self,
        a: &Acteur<'_>,
    ) -> Result<Vec<CoursVue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut t = tx(self, a).await?;
        let jour = aujourdhui(&mut t, a.etablissement_id).await?;
        let ids = cours::vacants(&mut t, a.etablissement_id, jour).await?;
        let rows = sqlx::query_as::<_, CoursVue>(&format!(
            "{SQL_COURS} AND c.id = ANY($2) ORDER BY cible_code, c.libelle, c.id"
        ))
        .bind(a.etablissement_id)
        .bind(&ids)
        .fetch_all(&mut *t)
        .await
        .map_err(db)?;
        t.commit().await.map_err(db)?;
        Ok(rows)
    }

    /// Un cours avec ses services (identité des professeurs : gestion seulement).
    pub async fn structure_cours_detail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<CoursDetail, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let c = cours::lire(&mut t, a.etablissement_id, id)
            .await?
            .ok_or_else(introuvable)?;
        let services = cours::services_du_cours(&mut t, a.etablissement_id, id).await?;
        t.commit().await.map_err(db)?;
        Ok(CoursDetail { cours: c, services })
    }

    /// STR-57 : crée un cours (poste vacant tant qu'aucun service n'est posé).
    pub async fn structure_cours_creer(
        &self,
        a: &Acteur<'_>,
        s: CoursSaisie,
    ) -> Result<CoursVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        if s.classe_id.is_some() == s.groupe_id.is_some() {
            return Err(invalide());
        }
        let libelle_saisi = net(s.libelle.as_deref(), 160)?;
        let heures = heures_valides(s.heures_hebdo)?;
        let semaines = semaines_valides(s.semaines.as_deref())?;
        let mut t = tx(self, a).await?;
        // L'année est celle du public (jamais celle du corps de la requête).
        let annee: Option<Uuid> = match (s.classe_id, s.groupe_id) {
            (Some(c), _) => sqlx::query_scalar(
                "SELECT annee_id FROM ent_classe WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(c)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?,
            (_, Some(g)) => sqlx::query_scalar(
                "SELECT annee_id FROM ent_groupe WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(a.etablissement_id)
            .bind(g)
            .fetch_optional(&mut *t)
            .await
            .map_err(db)?,
            _ => None,
        };
        let annee = annee.ok_or_else(introuvable)?;
        let libelle = match s.matiere_id {
            Some(m) => {
                let r: Option<(String, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
                    "SELECT libelle, archivee_le FROM ent_matiere WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(m)
                .fetch_optional(&mut *t)
                .await
                .map_err(db)?;
                let (lib, archivee) = r.ok_or_else(introuvable)?;
                if archivee.is_some() {
                    return Err(invalide());
                }
                libelle_saisi.unwrap_or(lib)
            }
            None => libelle_saisi.ok_or_else(invalide)?,
        };
        let debut: NaiveDate = sqlx::query_scalar(
            "SELECT date_debut FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(annee)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_cours (id, etablissement_id, annee_id, libelle, matiere_id, offre_id,
                    classe_id, groupe_id, heures_hebdo, semaines, ouvert_le, source, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9::float8::numeric,$10,$11,'saisie',$12)",
        )
        .bind(id)
        .bind(a.etablissement_id)
        .bind(annee)
        .bind(&libelle)
        .bind(s.matiere_id)
        .bind(s.offre_id)
        .bind(s.classe_id)
        .bind(s.groupe_id)
        .bind(heures)
        .bind(&semaines)
        .bind(s.ouvert_le.unwrap_or(debut))
        .bind(a.user_id)
        .execute(&mut *t)
        .await;
        if let Err(e) = r {
            // Un cours ouvert existe déjà pour ce public et cette matière : conflit explicite.
            if e.as_database_error()
                .is_some_and(|d| d.code().as_deref() == Some("23505"))
            {
                return Err(conflit());
            }
            return Err(db(e));
        }
        let c = cours::lire(&mut t, a.etablissement_id, id)
            .await?
            .ok_or_else(introuvable)?;
        journaliser(
            &mut t,
            a,
            "cours",
            &id.to_string(),
            "cree",
            None,
            Some(json_cours(&c)),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.cours_cree",
            id,
            json!({ "annee_id": annee, "vacant": true }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(c)
    }

    /// STR-57 : modifie le libellé, l'offre, le volume ou les semaines. Le public, la matière et
    /// l'année ne changent jamais (fermer puis ouvrir).
    pub async fn structure_cours_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        m: CoursModif,
    ) -> Result<CoursVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let libelle = net(m.libelle.as_deref(), 160)?;
        if m.libelle.is_some() && libelle.is_none() {
            return Err(invalide());
        }
        let heures = heures_valides(m.heures_hebdo)?;
        let semaines = semaines_valides(m.semaines.as_deref())?;
        let mut t = tx(self, a).await?;
        let avant = cours::lire(&mut t, a.etablissement_id, id)
            .await?
            .ok_or_else(introuvable)?;
        if avant.ferme_le.is_some() {
            return Err(conflit());
        }
        sqlx::query(
            "UPDATE ent_cours SET libelle = COALESCE($3, libelle), offre_id = COALESCE($4, offre_id),
                    heures_hebdo = COALESCE($5::float8::numeric, heures_hebdo),
                    semaines = COALESCE($6, semaines), a_verifier = false
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&libelle)
        .bind(m.offre_id)
        .bind(heures)
        .bind(&semaines)
        .execute(&mut *t)
        .await
        .map_err(db)?;
        let apres = cours::lire(&mut t, a.etablissement_id, id)
            .await?
            .ok_or_else(introuvable)?;
        journaliser(
            &mut t,
            a,
            "cours",
            &id.to_string(),
            "modifie",
            Some(json_cours(&avant)),
            Some(json_cours(&apres)),
        )
        .await?;
        let ev = publier(&mut t, a, "referentiel.cours_modifie", id, json!({})).await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// STR-57 : ferme un cours à une date (dernier jour inclus). Définitif ; les services et les
    /// créneaux restent lisibles (historique), le public du cours est vide après la date.
    pub async fn structure_cours_fermer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        a_la_date: NaiveDate,
    ) -> Result<CoursFerme, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let avant = cours::lire(&mut t, a.etablissement_id, id)
            .await?
            .ok_or_else(introuvable)?;
        if avant.ferme_le.is_some() {
            return Err(conflit());
        }
        if a_la_date < avant.ouvert_le {
            return Err(invalide());
        }
        sqlx::query("UPDATE ent_cours SET ferme_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(id)
            .bind(a_la_date)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        let creneaux_touches: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM ent_edt_creneau WHERE etablissement_id = $1 AND cours_id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .fetch_one(&mut *t)
        .await
        .map_err(db)?;
        let apres = cours::lire(&mut t, a.etablissement_id, id)
            .await?
            .ok_or_else(introuvable)?;
        journaliser(
            &mut t,
            a,
            "cours",
            &id.to_string(),
            "ferme",
            Some(json_cours(&avant)),
            Some(json_cours(&apres)),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.cours_ferme",
            id,
            json!({ "ferme_le": a_la_date, "creneaux_touches": creneaux_touches }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(CoursFerme {
            cours: apres,
            creneaux_touches,
        })
    }

    /// Pourvoit un poste : pose un service du professeur sur le cours (vacant ou non : un second
    /// professeur est une co-intervention). Le socle 6 reprendra cette pose avec ses interventions.
    pub async fn structure_cours_pourvoir(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        prof_user_id: Uuid,
        professeur_principal: bool,
    ) -> Result<ServiceDuCours, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        if professeur_principal {
            sqlx::query(
                "UPDATE ent_service_enseignement SET professeur_principal = false
                  WHERE etablissement_id = $1 AND professeur_principal
                    AND classe_id = (SELECT classe_id FROM ent_cours WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .execute(&mut *t)
            .await
            .map_err(db)?;
        }
        let sid = cours::poser_service(
            &mut t,
            a.etablissement_id,
            id,
            prof_user_id,
            professeur_principal,
        )
        .await?;
        journaliser(
            &mut t,
            a,
            "cours_service",
            &sid.to_string(),
            "pose",
            None,
            Some(json!({ "cours_id": id, "professeur_principal": professeur_principal })),
        )
        .await?;
        let ev = publier(
            &mut t,
            a,
            "referentiel.cours_pourvu",
            id,
            json!({ "service_id": sid }),
        )
        .await?;
        t.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ServiceDuCours {
            service_id: sid,
            prof_user_id,
            professeur_principal,
        })
    }

    /// Comparaison avant / après de la reprise (critère de fin de M7) : aucun service ni créneau
    /// ni membre perdu, ce qui reste à traiter, le compteur de créneaux sans cours.
    pub async fn structure_cours_reprise(&self, a: &Acteur<'_>) -> Result<EtatReprise, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let mut t = tx(self, a).await?;
        let e = a.etablissement_id;
        let compte = |sql: &'static str| sqlx::query_scalar::<_, i64>(sql).bind(e);
        let services =
            compte("SELECT count(*) FROM ent_service_enseignement WHERE etablissement_id = $1")
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
        let services_avec_cours = compte("SELECT count(*) FROM ent_service_enseignement WHERE etablissement_id = $1 AND cours_id IS NOT NULL")
            .fetch_one(&mut *t).await.map_err(db)?;
        let services_a_traiter =
            compte("SELECT count(*) FROM ent_cours_a_traiter WHERE etablissement_id = $1")
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
        let cours_n = compte("SELECT count(*) FROM ent_cours WHERE etablissement_id = $1")
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        let cours_a_verifier =
            compte("SELECT count(*) FROM ent_cours WHERE etablissement_id = $1 AND a_verifier")
                .fetch_one(&mut *t)
                .await
                .map_err(db)?;
        let creneaux = compte("SELECT count(*) FROM ent_edt_creneau WHERE etablissement_id = $1")
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        let creneaux_avec_service = compte("SELECT count(*) FROM ent_edt_creneau WHERE etablissement_id = $1 AND service_id IS NOT NULL")
            .fetch_one(&mut *t).await.map_err(db)?;
        let membres = compte("SELECT count(*) FROM ent_cours_membre WHERE etablissement_id = $1")
            .fetch_one(&mut *t)
            .await
            .map_err(db)?;
        let membres_sur_un_cours = compte("SELECT count(*) FROM ent_cours_membre m WHERE m.etablissement_id = $1 AND EXISTS (SELECT 1 FROM ent_cours c WHERE c.id = m.cours_id AND c.etablissement_id = m.etablissement_id)")
            .fetch_one(&mut *t).await.map_err(db)?;
        let jour = aujourdhui(&mut t, e).await?;
        let cours_vacants = cours::vacants(&mut t, e, jour).await?.len() as i64;
        let creneaux_sans_cours = cours::creneaux_sans_cours(&mut t, e).await?;
        t.commit().await.map_err(db)?;
        Ok(EtatReprise {
            services,
            services_avec_cours,
            services_a_traiter,
            cours: cours_n,
            cours_vacants,
            cours_a_verifier,
            creneaux,
            creneaux_avec_service,
            creneaux_sans_cours,
            membres,
            membres_sur_un_cours,
        })
    }
}
