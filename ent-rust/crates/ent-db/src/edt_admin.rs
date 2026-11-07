//! S2-5 — administration de l'emploi du temps (EDT-01, EDT-02, EDT-07) : détection des
//! conflits professeur / salle / classe (groupes compris), créneaux, salles, et vue de la
//! semaine (personnel, classe, salle, élève) calculée depuis les créneaux, l'alternance A/B,
//! les exceptions et le calendrier (vacances, fermetures).
//!
//! Droits (D-017) : `edt.gerer` (établissement) pour créer, modifier, supprimer ; tout le
//! personnel lit l'emploi du temps ; un élève ou un responsable (lien actif) ne lit que
//! l'emploi du temps de l'élève concerné. L'établissement vient toujours de la session.
//!
//! Conflits (règle « jamais en silence ») : un créneau en conflit n'est **pas** enregistré ;
//! la réponse liste les conflits (nature, créneau en cause). Deux groupes distincts d'une même
//! classe ne sont pas en conflit (demi-groupes, options) ; la classe entière l'est avec chacun
//! de ses groupes. Une salle n'est comparée qu'à l'intérieur d'un même site.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};
use domain_eleve::{EdtSemaineAb, semaine_ab_au_jour};
use ent_auth::{Action, Role};
use ent_events::AggregateKind;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::ref_commun::{Acteur, conflit, db, introuvable, invalide, publier_agg};
use crate::referentiel_seances::{CreneauSource, ExceptionSource, SeancePlan, planifier_seances};
use crate::referentiel_structure::{CreneauGrille, index_neutralises};
use crate::service::{EntDbError, PgEnt};

// ───────────────────────────── Conflits (logique pure) ─────────────────────────────

/// Créneau réduit à ce qui compte pour détecter un conflit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreneauConflit {
    pub id: Option<Uuid>,
    pub classe: String,
    pub groupe_id: Option<Uuid>,
    pub prof_user_id: Uuid,
    pub salle: Option<String>,
    pub site: String,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub semaine_ab: EdtSemaineAb,
}

/// Indisponibilité déclarée (`ent_edt_contrainte`, kinds `prof_indispo` / `salle_indispo`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Indisponibilite {
    pub prof_user_id: Option<Uuid>,
    pub salle_code: Option<String>,
    pub jour_semaine: Option<i16>,
    pub heure_debut: Option<NaiveTime>,
    pub heure_fin: Option<NaiveTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatureConflit {
    Professeur,
    Salle,
    Classe,
    IndisponibiliteProfesseur,
    IndisponibiliteSalle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflitEdt {
    pub nature: NatureConflit,
    /// Créneau examiné (absent pour une simulation sans identifiant).
    pub creneau_id: Option<Uuid>,
    /// Créneau en cause (absent pour une indisponibilité).
    pub autre_creneau_id: Option<Uuid>,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    /// Précision lisible (classe ou salle concernée) : jamais de donnée personnelle.
    pub detail: String,
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

fn se_chevauchent(a: (NaiveTime, NaiveTime), b: (NaiveTime, NaiveTime)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

/// Deux créneaux tombent-ils la même semaine ? (A et B ne se rencontrent jamais.)
pub fn semaines_compatibles(a: EdtSemaineAb, b: EdtSemaineAb) -> bool {
    !matches!(
        (a, b),
        (EdtSemaineAb::A, EdtSemaineAb::B) | (EdtSemaineAb::B, EdtSemaineAb::A)
    )
}

/// Natures de conflit entre deux créneaux distincts (vide si aucun).
pub fn conflits_entre(a: &CreneauConflit, b: &CreneauConflit) -> Vec<(NatureConflit, String)> {
    let mut out = Vec::new();
    if a.jour_semaine != b.jour_semaine
        || !semaines_compatibles(a.semaine_ab, b.semaine_ab)
        || !se_chevauchent((a.heure_debut, a.heure_fin), (b.heure_debut, b.heure_fin))
    {
        return out;
    }
    if a.prof_user_id == b.prof_user_id {
        out.push((NatureConflit::Professeur, "même professeur".to_string()));
    }
    if let (Some(sa), Some(sb)) = (a.salle.as_deref(), b.salle.as_deref()) {
        if !norm(sa).is_empty() && norm(sa) == norm(sb) && norm(&a.site) == norm(&b.site) {
            out.push((NatureConflit::Salle, format!("salle {}", sa.trim())));
        }
    }
    if norm(&a.classe) == norm(&b.classe) {
        let meme_cible = match (a.groupe_id, b.groupe_id) {
            (None, _) | (_, None) => true, // classe entière contre classe ou groupe
            (Some(x), Some(y)) => x == y,
        };
        if meme_cible {
            out.push((NatureConflit::Classe, format!("classe {}", a.classe.trim())));
        }
    }
    out
}

/// Conflits d'un créneau candidat avec les créneaux existants (lui-même exclu par `id`) et
/// les indisponibilités déclarées.
pub fn detecter_conflits(
    candidat: &CreneauConflit,
    existants: &[CreneauConflit],
    indispos: &[Indisponibilite],
) -> Vec<ConflitEdt> {
    let mut out = Vec::new();
    for e in existants {
        if candidat.id.is_some() && e.id == candidat.id {
            continue;
        }
        for (nature, detail) in conflits_entre(candidat, e) {
            out.push(ConflitEdt {
                nature,
                creneau_id: candidat.id,
                autre_creneau_id: e.id,
                jour_semaine: candidat.jour_semaine,
                heure_debut: candidat.heure_debut,
                heure_fin: candidat.heure_fin,
                detail,
            });
        }
    }
    for i in indispos {
        if i.jour_semaine.is_some_and(|j| j != candidat.jour_semaine) {
            continue;
        }
        let debut = i.heure_debut.unwrap_or(NaiveTime::MIN);
        let fin = i
            .heure_fin
            .unwrap_or_else(|| NaiveTime::from_hms_opt(23, 59, 59).expect("heure"));
        if !se_chevauchent((candidat.heure_debut, candidat.heure_fin), (debut, fin)) {
            continue;
        }
        if i.prof_user_id == Some(candidat.prof_user_id) {
            out.push(ConflitEdt {
                nature: NatureConflit::IndisponibiliteProfesseur,
                creneau_id: candidat.id,
                autre_creneau_id: None,
                jour_semaine: candidat.jour_semaine,
                heure_debut: candidat.heure_debut,
                heure_fin: candidat.heure_fin,
                detail: "professeur indisponible".to_string(),
            });
        }
        if let (Some(code), Some(salle)) = (i.salle_code.as_deref(), candidat.salle.as_deref()) {
            if norm(code) == norm(salle) {
                out.push(ConflitEdt {
                    nature: NatureConflit::IndisponibiliteSalle,
                    creneau_id: candidat.id,
                    autre_creneau_id: None,
                    jour_semaine: candidat.jour_semaine,
                    heure_debut: candidat.heure_debut,
                    heure_fin: candidat.heure_fin,
                    detail: format!("salle {} indisponible", salle.trim()),
                });
            }
        }
    }
    out
}

/// Tous les conflits d'un ensemble de créneaux (chaque paire une seule fois).
pub fn tous_les_conflits(
    creneaux: &[CreneauConflit],
    indispos: &[Indisponibilite],
) -> Vec<ConflitEdt> {
    let mut out = Vec::new();
    for (i, a) in creneaux.iter().enumerate() {
        for b in &creneaux[i + 1..] {
            for (nature, detail) in conflits_entre(a, b) {
                out.push(ConflitEdt {
                    nature,
                    creneau_id: a.id,
                    autre_creneau_id: b.id,
                    jour_semaine: a.jour_semaine,
                    heure_debut: a.heure_debut.max(b.heure_debut),
                    heure_fin: a.heure_fin.min(b.heure_fin),
                    detail,
                });
            }
        }
        out.extend(detecter_conflits(a, &[], indispos).into_iter().filter(|c| {
            !matches!(
                c.nature,
                NatureConflit::Professeur | NatureConflit::Salle | NatureConflit::Classe
            )
        }));
    }
    out
}

// ───────────────────────────── Types ─────────────────────────────

/// Créneau hebdomadaire tel que l'écran d'administration le lit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct CreneauAdmin {
    pub id: Uuid,
    pub classe: String,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub groupe: Option<String>,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    pub prof_nom: Option<String>,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    pub salle: Option<String>,
    pub site: String,
    pub semaine_ab: String,
}

/// Saisie d'un créneau (création ou modification complète).
#[derive(Debug, Clone, Deserialize)]
pub struct CreneauSaisie {
    pub classe: String,
    #[serde(default)]
    pub groupe_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    pub jour_semaine: i16,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    #[serde(default)]
    pub salle: Option<String>,
    #[serde(default)]
    pub site: String,
    #[serde(default = "tous")]
    pub semaine_ab: String,
}

fn tous() -> String {
    "toutes".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "resultat", rename_all = "snake_case")]
pub enum ResultatCreneau {
    /// Enregistré (ou, en aperçu, enregistrable : `creneau` absent).
    Enregistre { creneau: Option<Box<CreneauAdmin>> },
    /// Refusé : au moins un conflit, rien n'est écrit.
    Conflits { conflits: Vec<ConflitEdt> },
}

#[derive(Debug, Clone, FromRow)]
struct LigneConflit {
    id: Uuid,
    classe: String,
    groupe_id: Option<Uuid>,
    prof_user_id: Uuid,
    salle: Option<String>,
    site: String,
    jour_semaine: i16,
    heure_debut: NaiveTime,
    heure_fin: NaiveTime,
    semaine_ab: String,
}

impl LigneConflit {
    fn vers(self) -> CreneauConflit {
        CreneauConflit {
            id: Some(self.id),
            classe: self.classe,
            groupe_id: self.groupe_id,
            prof_user_id: self.prof_user_id,
            salle: self.salle,
            site: self.site,
            jour_semaine: self.jour_semaine,
            heure_debut: self.heure_debut,
            heure_fin: self.heure_fin,
            semaine_ab: EdtSemaineAb::parse(&self.semaine_ab).unwrap_or(EdtSemaineAb::Toutes),
        }
    }
}

const SELECT_ADMIN: &str =
    "SELECT c.id, btrim(c.classe) AS classe, c.classe_id, c.groupe_id, g.code AS groupe,
        c.matiere_libelle, c.prof_user_id,
        coalesce(nullif(btrim(u.display_name), ''), u.email) AS prof_nom,
        c.jour_semaine, c.heure_debut, c.heure_fin, c.salle, c.site, c.semaine_ab
   FROM ent_edt_creneau c
   LEFT JOIN ent_groupe g ON g.id = c.groupe_id AND g.etablissement_id = c.etablissement_id
   LEFT JOIN ent_user u ON u.id = c.prof_user_id";

fn est_personnel(role: &str) -> bool {
    Role::parse(role).is_some_and(|r| r.is_staff())
}

impl PgEnt {
    /// Action `edt.gerer` sur l'établissement entier (moteur de droits, délégations comprises).
    pub(crate) async fn exiger_edt(
        &self,
        a: &Acteur<'_>,
        action: Action,
    ) -> Result<(), EntDbError> {
        self.exiger_referentiel(a, action).await
    }

    /// Variante booléenne pour l'API.
    pub async fn edt_peut(&self, a: &Acteur<'_>, action: Action) -> Result<bool, EntDbError> {
        self.ref_peut(a, action).await
    }

    #[allow(clippy::type_complexity)]
    async fn indispos(&self, etab: Uuid) -> Result<Vec<Indisponibilite>, EntDbError> {
        let rows: Vec<(
            Option<Uuid>,
            Option<String>,
            Option<i16>,
            Option<NaiveTime>,
            Option<NaiveTime>,
        )> = sqlx::query_as(
            "SELECT prof_user_id, salle_code, jour_semaine, heure_debut, heure_fin
                   FROM ent_edt_contrainte
                  WHERE etablissement_id = $1 AND kind IN ('prof_indispo', 'salle_indispo')",
        )
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(p, s, j, d, f)| Indisponibilite {
                prof_user_id: p,
                salle_code: s,
                jour_semaine: j,
                heure_debut: d,
                heure_fin: f,
            })
            .collect())
    }

    async fn creneaux_conflit(&self, etab: Uuid) -> Result<Vec<CreneauConflit>, EntDbError> {
        let rows: Vec<LigneConflit> = sqlx::query_as(
            "SELECT id, btrim(classe) AS classe, groupe_id, prof_user_id, salle, site,
                    jour_semaine, heure_debut, heure_fin, semaine_ab
               FROM ent_edt_creneau WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().map(LigneConflit::vers).collect())
    }

    /// Créneaux de l'établissement (lecture : tout le personnel), filtrables.
    pub async fn edt_creneaux_admin(
        &self,
        a: &Acteur<'_>,
        classe: Option<&str>,
        prof_user_id: Option<Uuid>,
        salle: Option<&str>,
    ) -> Result<Vec<CreneauAdmin>, EntDbError> {
        if !est_personnel(a.role) {
            return Err(EntDbError::Forbidden);
        }
        let sql = format!(
            "{SELECT_ADMIN}
              WHERE c.etablissement_id = $1
                AND ($2::text IS NULL OR lower(btrim(c.classe)) = lower(btrim($2)))
                AND ($3::uuid IS NULL OR c.prof_user_id = $3)
                AND ($4::text IS NULL OR lower(btrim(c.salle)) = lower(btrim($4)))
              ORDER BY c.jour_semaine, c.heure_debut, c.classe"
        );
        sqlx::query_as::<_, CreneauAdmin>(&sql)
            .bind(a.etablissement_id)
            .bind(classe)
            .bind(prof_user_id)
            .bind(salle)
            .fetch_all(self.pool())
            .await
            .map_err(db)
    }

    /// Tous les conflits de l'établissement (EDT-02 : détection, pas seulement refus).
    pub async fn edt_conflits(&self, a: &Acteur<'_>) -> Result<Vec<ConflitEdt>, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let creneaux = self.creneaux_conflit(a.etablissement_id).await?;
        let indispos = self.indispos(a.etablissement_id).await?;
        Ok(tous_les_conflits(&creneaux, &indispos))
    }

    /// Valide la saisie contre le référentiel de l'établissement et la convertit.
    async fn valider_saisie_creneau(
        &self,
        etab: Uuid,
        s: &CreneauSaisie,
        id: Option<Uuid>,
    ) -> Result<CreneauConflit, EntDbError> {
        let classe = s.classe.trim();
        if classe.is_empty()
            || s.matiere_libelle.trim().is_empty()
            || s.matiere_libelle.len() > 120
            || !(1..=7).contains(&s.jour_semaine)
            || s.heure_fin <= s.heure_debut
            || s.site.len() > 80
        {
            return Err(invalide());
        }
        let ab = EdtSemaineAb::parse(&s.semaine_ab).ok_or_else(invalide)?;
        // Classe connue du référentiel de CET établissement (pas de classe fantôme).
        let (classe_ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_classe
              WHERE etablissement_id = $1 AND code = btrim($2) AND archivee_le IS NULL)",
        )
        .bind(etab)
        .bind(classe)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        if !classe_ok {
            return Err(invalide());
        }
        if let Some(g) = s.groupe_id {
            // Groupe de l'établissement, rattaché à cette classe s'il l'est à une classe.
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_groupe g
                   LEFT JOIN ent_classe cl ON cl.id = g.classe_id AND cl.etablissement_id = g.etablissement_id
                  WHERE g.etablissement_id = $1 AND g.id = $2
                    AND (g.classe_id IS NULL OR cl.code = btrim($3)))",
            )
            .bind(etab)
            .bind(g)
            .bind(classe)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        // Le professeur est un personnel de l'établissement (jamais un compte d'ailleurs).
        let role = self.auth_repo().role_in(etab, s.prof_user_id).await?;
        if !role
            .as_deref()
            .and_then(Role::parse)
            .is_some_and(|r| r.is_staff())
        {
            return Err(introuvable());
        }
        let salle = s.salle.as_deref().map(str::trim).filter(|x| !x.is_empty());
        if let Some(code) = salle {
            // Salle du référentiel de l'établissement, active (conflits fiables).
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_salle
                  WHERE etablissement_id = $1 AND lower(code) = lower($2) AND actif)",
            )
            .bind(etab)
            .bind(code)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        Ok(CreneauConflit {
            id,
            classe: classe.to_string(),
            groupe_id: s.groupe_id,
            prof_user_id: s.prof_user_id,
            salle: salle.map(str::to_string),
            site: s.site.trim().to_string(),
            jour_semaine: s.jour_semaine,
            heure_debut: s.heure_debut,
            heure_fin: s.heure_fin,
            semaine_ab: ab,
        })
    }

    /// EDT-02 : crée un créneau. Un conflit professeur / salle / classe (ou une indisponibilité)
    /// refuse l'écriture et le détaille ; `apercu` valide sans écrire.
    pub async fn edt_creneau_creer(
        &self,
        a: &Acteur<'_>,
        s: &CreneauSaisie,
        apercu: bool,
    ) -> Result<ResultatCreneau, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let etab = a.etablissement_id;
        let cand = self.valider_saisie_creneau(etab, s, None).await?;
        let existants = self.creneaux_conflit(etab).await?;
        let indispos = self.indispos(etab).await?;
        let conflits = detecter_conflits(&cand, &existants, &indispos);
        if !conflits.is_empty() {
            return Ok(ResultatCreneau::Conflits { conflits });
        }
        if apercu {
            return Ok(ResultatCreneau::Enregistre { creneau: None });
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 5))")
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        // Revérification sous verrou : deux créations simultanées ne passent pas toutes les deux.
        let rows: Vec<LigneConflit> = sqlx::query_as(
            "SELECT id, btrim(classe) AS classe, groupe_id, prof_user_id, salle, site,
                    jour_semaine, heure_debut, heure_fin, semaine_ab
               FROM ent_edt_creneau WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let existants: Vec<CreneauConflit> = rows.into_iter().map(LigneConflit::vers).collect();
        let conflits = detecter_conflits(&cand, &existants, &indispos);
        if !conflits.is_empty() {
            return Ok(ResultatCreneau::Conflits { conflits });
        }
        sqlx::query(
            "INSERT INTO ent_edt_creneau
               (id, etablissement_id, classe, groupe_id, matiere_libelle, prof_user_id, jour_semaine,
                heure_debut, heure_fin, salle, site, semaine_ab)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(id)
        .bind(etab)
        .bind(&cand.classe)
        .bind(cand.groupe_id)
        .bind(s.matiere_libelle.trim())
        .bind(cand.prof_user_id)
        .bind(cand.jour_semaine)
        .bind(cand.heure_debut)
        .bind(cand.heure_fin)
        .bind(&cand.salle)
        .bind(&cand.site)
        .bind(cand.semaine_ab.as_str())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            ent_events::types::EDT_CRENEAU_CREATED,
            id,
            json!({ "creneau_id": id, "jour_semaine": cand.jour_semaine,
                    "semaine_ab": cand.semaine_ab.as_str() }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        let creneau = self.creneau_admin(etab, id).await?.map(Box::new);
        Ok(ResultatCreneau::Enregistre { creneau })
    }

    async fn creneau_admin(
        &self,
        etab: Uuid,
        id: Uuid,
    ) -> Result<Option<CreneauAdmin>, EntDbError> {
        let sql = format!("{SELECT_ADMIN} WHERE c.etablissement_id = $1 AND c.id = $2");
        sqlx::query_as::<_, CreneauAdmin>(&sql)
            .bind(etab)
            .bind(id)
            .fetch_optional(self.pool())
            .await
            .map_err(db)
    }

    /// EDT-02 : modifie un créneau (mêmes contrôles de conflit, lui-même exclu).
    pub async fn edt_creneau_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        s: &CreneauSaisie,
        apercu: bool,
    ) -> Result<ResultatCreneau, EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let etab = a.etablissement_id;
        if self.creneau_admin(etab, id).await?.is_none() {
            return Err(introuvable());
        }
        let cand = self.valider_saisie_creneau(etab, s, Some(id)).await?;
        let indispos = self.indispos(etab).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 5))")
            .bind(etab)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let rows: Vec<LigneConflit> = sqlx::query_as(
            "SELECT id, btrim(classe) AS classe, groupe_id, prof_user_id, salle, site,
                    jour_semaine, heure_debut, heure_fin, semaine_ab
               FROM ent_edt_creneau WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let existants: Vec<CreneauConflit> = rows.into_iter().map(LigneConflit::vers).collect();
        let conflits = detecter_conflits(&cand, &existants, &indispos);
        if !conflits.is_empty() {
            return Ok(ResultatCreneau::Conflits { conflits });
        }
        if apercu {
            return Ok(ResultatCreneau::Enregistre { creneau: None });
        }
        sqlx::query(
            "UPDATE ent_edt_creneau
                SET classe = $3, groupe_id = $4, matiere_libelle = $5, prof_user_id = $6,
                    jour_semaine = $7, heure_debut = $8, heure_fin = $9, salle = $10,
                    site = $11, semaine_ab = $12
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&cand.classe)
        .bind(cand.groupe_id)
        .bind(s.matiere_libelle.trim())
        .bind(cand.prof_user_id)
        .bind(cand.jour_semaine)
        .bind(cand.heure_debut)
        .bind(cand.heure_fin)
        .bind(&cand.salle)
        .bind(&cand.site)
        .bind(cand.semaine_ab.as_str())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            "edt.creneau_modifie",
            id,
            json!({ "creneau_id": id, "jour_semaine": cand.jour_semaine }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        let creneau = self.creneau_admin(etab, id).await?.map(Box::new);
        Ok(ResultatCreneau::Enregistre { creneau })
    }

    /// Supprime un créneau sans séance ni appel : sinon refus (409), on le modifie ou on
    /// annule ses séances.
    pub async fn edt_creneau_supprimer(&self, a: &Acteur<'_>, id: Uuid) -> Result<(), EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        let etab = a.etablissement_id;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let res =
            sqlx::query("DELETE FROM ent_edt_creneau WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await;
        let res = match res {
            Ok(r) => r,
            Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23503") => {
                return Err(conflit()); // séances, appels ou exceptions rattachés
            }
            Err(e) => return Err(db(e)),
        };
        if res.rows_affected() == 0 {
            return Err(introuvable());
        }
        let ev = publier_agg(
            &mut tx,
            a,
            AggregateKind::Edt,
            "edt.creneau_supprime",
            id,
            json!({ "creneau_id": id }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(())
    }

    /// Met à jour une salle du référentiel (libellé, capacité, active).
    pub async fn edt_salle_modifier(
        &self,
        a: &Acteur<'_>,
        salle_id: Uuid,
        libelle: &str,
        capacite: Option<i32>,
        actif: bool,
    ) -> Result<(), EntDbError> {
        self.exiger_edt(a, Action::EdtGerer).await?;
        if libelle.trim().is_empty()
            || libelle.len() > 120
            || capacite.is_some_and(|c| !(0..=5000).contains(&c))
        {
            return Err(invalide());
        }
        let res = sqlx::query(
            "UPDATE ent_salle SET libelle = btrim($3), capacite = $4, actif = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(salle_id)
        .bind(libelle)
        .bind(capacite)
        .bind(actif)
        .execute(self.pool())
        .await
        .map_err(db)?;
        if res.rows_affected() == 0 {
            return Err(introuvable());
        }
        Ok(())
    }
}

// ───────────────────────────── Plans de séances (sans écriture) ─────────────────────────────

/// Séance attendue, avec le groupe visé et le site de son créneau.
#[derive(Debug, Clone)]
pub struct PlanEdt {
    pub plan: SeancePlan,
    pub groupe_id: Option<Uuid>,
}

#[derive(Debug, Clone, FromRow)]
struct CreneauSourceG {
    id: Uuid,
    classe: String,
    classe_id: Uuid,
    site_id: Option<Uuid>,
    prof_user_id: Uuid,
    matiere_id: Option<Uuid>,
    matiere_libelle: String,
    jour_semaine: i16,
    heure_debut: NaiveTime,
    heure_fin: NaiveTime,
    salle: Option<String>,
    semaine_ab: String,
    service_id: Option<Uuid>,
    groupe_id: Option<Uuid>,
}

impl PgEnt {
    /// Séances attendues sur `[du, au]` (≤ 62 jours), par la même logique pure que la
    /// génération des séances datées (A/B, exceptions, vacances, fermetures), sans rien écrire.
    pub(crate) async fn edt_plans(
        &self,
        etab: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<PlanEdt>, EntDbError> {
        if au < du || (au - du).num_days() > 62 {
            return Err(invalide());
        }
        let annees: Vec<(Uuid, NaiveDate, NaiveDate)> = sqlx::query_as(
            "SELECT id, date_debut, date_fin FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND date_debut <= $3 AND date_fin >= $2
              ORDER BY date_debut",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        if annees.is_empty() {
            return Ok(Vec::new());
        }
        let exceptions: Vec<ExceptionSource> = sqlx::query_as(
            "SELECT creneau_id, jour, kind, remplacant_user_id, nouvelle_salle FROM ent_edt_exception
              WHERE etablissement_id = $1 AND jour BETWEEN $2 AND $3",
        )
        .bind(etab)
        .bind(du)
        .bind(au)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let ab_reference: Option<NaiveDate> = sqlx::query_scalar(
            "SELECT reference_lundi FROM ent_edt_calendrier_ab WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let grille: Vec<CreneauGrille> = sqlx::query_as(
            "SELECT id, site_id, code, libelle, demi_journee, ordre, heure_debut, heure_fin
               FROM ent_grille_horaire WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let jours = self.calendrier_plage(etab, du, au).await?;
        let neutralises = index_neutralises(&jours);
        let mut out = Vec::new();
        for (annee_id, debut, fin) in annees {
            let (d1, d2) = (du.max(debut), au.min(fin));
            let rows: Vec<CreneauSourceG> = sqlx::query_as(
                "SELECT c.id, btrim(c.classe) AS classe, cl.id AS classe_id, cl.site_id,
                        c.prof_user_id, c.matiere_id, c.matiere_libelle, c.jour_semaine,
                        c.heure_debut, c.heure_fin, c.salle, c.semaine_ab, s.id AS service_id,
                        c.groupe_id
                   FROM ent_edt_creneau c
                   JOIN ent_classe cl
                     ON cl.etablissement_id = c.etablissement_id AND cl.annee_id = $2
                    AND cl.code = btrim(c.classe) AND cl.archivee_le IS NULL
                   LEFT JOIN ent_service_enseignement s
                     ON s.id = c.service_id AND s.etablissement_id = c.etablissement_id AND s.annee_id = $2
                  WHERE c.etablissement_id = $1",
            )
            .bind(etab)
            .bind(annee_id)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
            let groupes: HashMap<Uuid, Option<Uuid>> =
                rows.iter().map(|r| (r.id, r.groupe_id)).collect();
            let sources: Vec<CreneauSource> = rows
                .into_iter()
                .map(|r| CreneauSource {
                    id: r.id,
                    classe: r.classe,
                    classe_id: r.classe_id,
                    site_id: r.site_id,
                    prof_user_id: r.prof_user_id,
                    matiere_id: r.matiere_id,
                    matiere_libelle: r.matiere_libelle,
                    jour_semaine: r.jour_semaine,
                    heure_debut: r.heure_debut,
                    heure_fin: r.heure_fin,
                    salle: r.salle,
                    semaine_ab: r.semaine_ab,
                    service_id: r.service_id,
                })
                .collect();
            let (plans, _, _) = planifier_seances(
                &sources,
                d1,
                d2,
                &neutralises,
                ab_reference,
                &exceptions,
                &grille,
            );
            out.extend(plans.into_iter().map(|p| PlanEdt {
                groupe_id: groupes.get(&p.creneau_id).copied().flatten(),
                plan: p,
            }));
        }
        Ok(out)
    }
}

// ───────────────────────────── Vue de la semaine (EDT-07) ─────────────────────────────

/// Ce que l'on regarde : un personnel, une classe, une salle ou un élève.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CibleEdt {
    Personnel(Uuid),
    Classe(Uuid),
    Salle(String),
    Eleve(Uuid),
}

/// Séance de la semaine telle que l'écran l'affiche. Aucun motif libre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeanceSemaine {
    pub creneau_id: Uuid,
    pub jour: NaiveDate,
    pub classe: String,
    pub classe_id: Option<Uuid>,
    pub groupe_id: Option<Uuid>,
    pub matiere_libelle: String,
    pub prof_user_id: Uuid,
    pub prof_nom: Option<String>,
    pub remplacant_user_id: Option<Uuid>,
    pub remplacant_nom: Option<String>,
    pub salle: Option<String>,
    pub heure_debut: NaiveTime,
    pub heure_fin: NaiveTime,
    /// `prevue`, `annulee`, `remplacee`, `deplacee` (cours d'origine), `ajoutee` (cours déplacé).
    pub statut: String,
    /// Nouveau jour d'un cours déplacé (sur la séance d'origine) ou jour d'origine (sur la nouvelle).
    pub deplace_vers: Option<NaiveDate>,
    pub deplace_depuis: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JourFerme {
    pub jour: NaiveDate,
    pub libelle: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemaineEdt {
    pub lundi: NaiveDate,
    /// `A`, `B`, ou absent si aucun calendrier d'alternance n'est configuré.
    pub semaine_ab: Option<String>,
    pub jours_fermes: Vec<JourFerme>,
    pub seances: Vec<SeanceSemaine>,
}

#[derive(Debug, Clone, FromRow)]
struct LigneDeplacee {
    creneau_id: Uuid,
    jour_origine: NaiveDate,
    nouveau_jour: NaiveDate,
    nouvelle_heure_debut: NaiveTime,
    nouvelle_heure_fin: NaiveTime,
    nouvelle_salle: Option<String>,
}

pub fn lundi_de(jour: NaiveDate) -> NaiveDate {
    jour - Duration::days(jour.weekday().num_days_from_monday() as i64)
}

impl PgEnt {
    pub(crate) async fn noms(
        &self,
        etab: Uuid,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, String>, EntDbError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT u.id, coalesce(nullif(btrim(u.display_name), ''), u.email)
               FROM ent_user u JOIN ent_membership m ON m.user_id = u.id
              WHERE m.etablissement_id = $1 AND u.id = ANY($2)",
        )
        .bind(etab)
        .bind(ids)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows.into_iter().collect())
    }

    /// Autorise la lecture de l'EDT d'un élève : personnel, l'élève lui-même, un responsable
    /// dont le lien est actif.
    pub(crate) async fn edt_acces_eleve(
        &self,
        a: &Acteur<'_>,
        eleve_id: Uuid,
    ) -> Result<(), EntDbError> {
        let role = Role::parse(a.role).ok_or(EntDbError::Forbidden)?;
        match role {
            r if r.is_staff() => {}
            Role::Parent => {
                if self
                    .lien_parent_eleve(a.etablissement_id, a.user_id, eleve_id)
                    .await?
                    .is_none()
                {
                    return Err(EntDbError::Forbidden);
                }
            }
            Role::Eleve => {
                let lie = self
                    .eleve_id_for_compte(a.etablissement_id, a.user_id)
                    .await?;
                if lie != Some(eleve_id) {
                    return Err(EntDbError::Forbidden);
                }
            }
            _ => return Err(EntDbError::Forbidden),
        }
        // L'élève doit exister DANS cet établissement (sinon 404, jamais de fuite).
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(a.etablissement_id)
        .bind(eleve_id)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        if ok { Ok(()) } else { Err(introuvable()) }
    }

    /// Classe courante et groupes actifs d'un élève (identifiants).
    pub(crate) async fn edt_cibles_eleve(
        &self,
        etab: Uuid,
        eleve_id: Uuid,
        jour: NaiveDate,
    ) -> Result<(HashSet<Uuid>, HashSet<Uuid>), EntDbError> {
        let rows: Vec<(Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT classe_id, groupe_id FROM ent_affectation
              WHERE etablissement_id = $1 AND eleve_id = $2
                AND date_debut <= $3 AND (date_fin IS NULL OR date_fin >= $3)",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(jour)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut classes: HashSet<Uuid> = rows.iter().filter_map(|r| r.0).collect();
        let groupes: HashSet<Uuid> = rows.iter().filter_map(|r| r.1).collect();
        // Repli : classe portée par la fiche élève (double écriture S2-2).
        if let Some(c) = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT classe_id FROM ent_eleve
              WHERE etablissement_id = $1 AND id = $2 AND (date_sortie IS NULL OR date_sortie > $3)",
        )
        .bind(etab)
        .bind(eleve_id)
        .bind(jour)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .flatten()
        {
            classes.insert(c);
        }
        Ok((classes, groupes))
    }

    /// EDT-07 : l'emploi du temps d'une semaine pour une cible.
    pub async fn edt_semaine(
        &self,
        a: &Acteur<'_>,
        cible: CibleEdt,
        jour: NaiveDate,
    ) -> Result<SemaineEdt, EntDbError> {
        let etab = a.etablissement_id;
        match &cible {
            CibleEdt::Eleve(e) => self.edt_acces_eleve(a, *e).await?,
            _ => {
                if !est_personnel(a.role) {
                    return Err(EntDbError::Forbidden);
                }
            }
        }
        if let CibleEdt::Classe(c) = &cible {
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_classe WHERE etablissement_id = $1 AND id = $2)",
            )
            .bind(etab)
            .bind(c)
            .fetch_one(self.pool())
            .await
            .map_err(db)?;
            if !ok {
                return Err(introuvable());
            }
        }
        let lundi = lundi_de(jour);
        let dimanche = lundi + Duration::days(6);
        let plans = self.edt_plans(etab, lundi, dimanche).await?;
        let deplacements: Vec<LigneDeplacee> = sqlx::query_as(
            "SELECT creneau_id, jour_origine, nouveau_jour, nouvelle_heure_debut, nouvelle_heure_fin,
                    nouvelle_salle
               FROM ent_edt_deplacement
              WHERE etablissement_id = $1
                AND ((jour_origine BETWEEN $2 AND $3) OR (nouveau_jour BETWEEN $2 AND $3))",
        )
        .bind(etab)
        .bind(lundi)
        .bind(dimanche)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let (classes_eleve, groupes_eleve) = match &cible {
            CibleEdt::Eleve(e) => self.edt_cibles_eleve(etab, *e, lundi).await?,
            _ => Default::default(),
        };
        let retient =
            |p: &PlanEdt, salle: Option<&str>, prof: Uuid, remplacant: Option<Uuid>| match &cible {
                CibleEdt::Personnel(u) => prof == *u || remplacant == Some(*u),
                CibleEdt::Classe(c) => p.plan.classe_id == *c,
                CibleEdt::Salle(code) => salle.is_some_and(|s| norm(s) == norm(code)),
                CibleEdt::Eleve(_) => {
                    classes_eleve.contains(&p.plan.classe_id)
                        && p.groupe_id.is_none_or(|g| groupes_eleve.contains(&g))
                }
            };
        let par_creneau: HashMap<Uuid, &PlanEdt> =
            plans.iter().map(|p| (p.plan.creneau_id, p)).collect();
        let depl_par_origine: HashMap<(Uuid, NaiveDate), &LigneDeplacee> = deplacements
            .iter()
            .map(|d| (d.creneau_id, d.jour_origine))
            .zip(deplacements.iter())
            .collect();
        let mut seances: Vec<SeanceSemaine> = Vec::new();
        let mut ids: Vec<Uuid> = Vec::new();
        for p in &plans {
            let pl = &p.plan;
            if !retient(
                p,
                pl.salle.as_deref(),
                pl.prof_user_id,
                pl.remplacant_user_id,
            ) {
                continue;
            }
            let depl = depl_par_origine.get(&(pl.creneau_id, pl.jour));
            let statut = if depl.is_some() {
                "deplacee"
            } else {
                pl.statut.as_str()
            };
            seances.push(SeanceSemaine {
                creneau_id: pl.creneau_id,
                jour: pl.jour,
                classe: pl.classe.clone(),
                classe_id: Some(pl.classe_id),
                groupe_id: p.groupe_id,
                matiere_libelle: pl.matiere_libelle.clone(),
                prof_user_id: pl.prof_user_id,
                prof_nom: None,
                remplacant_user_id: pl.remplacant_user_id,
                remplacant_nom: None,
                salle: pl.salle.clone(),
                heure_debut: pl.heure_debut,
                heure_fin: pl.heure_fin,
                statut: statut.to_string(),
                deplace_vers: depl.map(|d| d.nouveau_jour),
                deplace_depuis: None,
            });
            ids.push(pl.prof_user_id);
            ids.extend(pl.remplacant_user_id);
        }
        // Cours déplacés arrivant dans la semaine : séances datées de source « deplacement ».
        for d in deplacements
            .iter()
            .filter(|d| (lundi..=dimanche).contains(&d.nouveau_jour))
        {
            // Le créneau d'origine peut ne pas tomber cette semaine : on le relit.
            let Some(src) = self
                .edt_creneau_plan_du_jour(etab, d.creneau_id, d.jour_origine, &par_creneau)
                .await?
            else {
                continue;
            };
            let salle = d.nouvelle_salle.clone().or(src.plan.salle.clone());
            if !retient(&src, salle.as_deref(), src.plan.prof_user_id, None) {
                continue;
            }
            seances.push(SeanceSemaine {
                creneau_id: d.creneau_id,
                jour: d.nouveau_jour,
                classe: src.plan.classe.clone(),
                classe_id: Some(src.plan.classe_id),
                groupe_id: src.groupe_id,
                matiere_libelle: src.plan.matiere_libelle.clone(),
                prof_user_id: src.plan.prof_user_id,
                prof_nom: None,
                remplacant_user_id: None,
                remplacant_nom: None,
                salle,
                heure_debut: d.nouvelle_heure_debut,
                heure_fin: d.nouvelle_heure_fin,
                statut: "ajoutee".to_string(),
                deplace_vers: None,
                deplace_depuis: Some(d.jour_origine),
            });
            ids.push(src.plan.prof_user_id);
        }
        ids.sort();
        ids.dedup();
        let noms = self.noms(etab, &ids).await?;
        for s in &mut seances {
            s.prof_nom = noms.get(&s.prof_user_id).cloned();
            s.remplacant_nom = s.remplacant_user_id.and_then(|r| noms.get(&r).cloned());
        }
        seances.sort_by(|x, y| {
            (x.jour, x.heure_debut, &x.classe).cmp(&(y.jour, y.heure_debut, &y.classe))
        });
        let semaine_ab = sqlx::query_scalar::<_, NaiveDate>(
            "SELECT reference_lundi FROM ent_edt_calendrier_ab WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?
        .map(|r| match semaine_ab_au_jour(r, lundi) {
            EdtSemaineAb::B => "B".to_string(),
            _ => "A".to_string(),
        });
        let mut fermes: BTreeMap<NaiveDate, String> = BTreeMap::new();
        for j in self.calendrier_plage(etab, lundi, dimanche).await? {
            if j.site_id.is_none() {
                fermes.entry(j.jour).or_insert(j.libelle);
            }
        }
        Ok(SemaineEdt {
            lundi,
            semaine_ab,
            jours_fermes: fermes
                .into_iter()
                .map(|(jour, libelle)| JourFerme { jour, libelle })
                .collect(),
            seances,
        })
    }

    /// Plan du créneau à son jour d'origine (réutilise les plans déjà calculés, sinon relit).
    async fn edt_creneau_plan_du_jour(
        &self,
        etab: Uuid,
        creneau_id: Uuid,
        jour: NaiveDate,
        deja: &HashMap<Uuid, &PlanEdt>,
    ) -> Result<Option<PlanEdt>, EntDbError> {
        if let Some(p) = deja.get(&creneau_id) {
            if p.plan.jour == jour {
                return Ok(Some((*p).clone()));
            }
        }
        Ok(self
            .edt_plans(etab, jour, jour)
            .await?
            .into_iter()
            .find(|p| p.plan.creneau_id == creneau_id))
    }
}
