//! Socle 18, lot FO2 — création de formulaires par les personnels : nature, unité de réponse, audience dans la portée, modèles
//! prêts, sections, questions, conditions d'affichage sans boucle, versions, publication et validation
//! (`docs/socles/18_FORMULAIRES.md` §11, §21.2 : FRM-120 à FRM-126, FRM-130 à FRM-140, FRM-145, FRM-149 ; défauts FD-06, FD-07).
//!
//! * Tout accès passe par [`TenantTx`] (RLS d'établissement, clés étrangères composites).
//! * **Autorisation côté serveur** : `formulaires.creer` (portée : établissement ou mes classes), `formulaires.valider`
//!   (direction), `formulaires.gerer` (gestion de tous les formulaires). Un formulaire d'un autre auteur est **introuvable**
//!   pour qui n'y a aucun droit (indiscernable d'un identifiant inconnu).
//! * Une version publiée est **figée** (la base refuse toute modification, sauf la correction de forme tracée).
//! * L'audience est dans la **portée** de l'auteur : un professeur ne cible que ses classes.
//! * Aucun JSON : questions, options, lignes, conditions et critères sont des lignes.

#![allow(clippy::type_complexity)]

use crate::accords::objet_exige_tous;
use crate::accords_verifier::{objet_par_code, version_en_cours};
use crate::campagnes::{eleves_de_audience, valider_audience};
use crate::frm_moteur::*;
use crate::notifications::db;
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use std::collections::{BTreeSet, HashMap, HashSet};
use uuid::Uuid;

pub mod evenements {
    pub const PUBLIE: &str = "formulaires.publie";
    pub const CLOS: &str = "formulaires.clos";
    pub const REPONSE: &str = "formulaires.reponse";
}

// Tests : abaisse le seuil de validation pour le fil courant (`#[tokio::test]` = un seul fil), sans créer 200 élèves.
#[cfg(test)]
thread_local! {
    static SEUIL_TEST: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

#[cfg(all(test, feature = "labo"))]
pub(crate) fn forcer_seuil_test(seuil: Option<usize>) {
    SEUIL_TEST.with(|c| c.set(seuil));
}

/// Seuil d'audience au-delà duquel la publication attend la validation d'un autre responsable.
pub(crate) fn seuil_validation() -> usize {
    #[cfg(test)]
    if let Some(s) = SEUIL_TEST.with(|c| c.get()) {
        return s;
    }
    SEUIL_VALIDATION
}

pub(crate) fn introuvable() -> EntDbError {
    EntDbError::Domain(domain_eleve::DomainError::NotFound)
}

fn invalide(msg: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(msg)
}

fn conflit(msg: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(msg)
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct FormRow {
    pub id: Uuid,
    pub titre: String,
    pub description: Option<String>,
    pub nature: String,
    pub unite: String,
    pub confidentialite: String,
    pub destinataire_confidentiel_user_id: Option<Uuid>,
    pub statut: String,
    pub modele_code: Option<String>,
    pub ouverture: Option<DateTime<Utc>>,
    pub echeance: Option<DateTime<Utc>>,
    pub relances: String,
    pub audience_nb: Option<i32>,
    pub auteur_user_id: Uuid,
    pub publie_le: Option<DateTime<Utc>>,
    pub cree_le: DateTime<Utc>,
}

pub(crate) const COLS_FORM: &str = "id, titre, description, nature, unite, confidentialite, destinataire_confidentiel_user_id, \
    statut, modele_code, ouverture, echeance, relances, audience_nb, auteur_user_id, publie_le, cree_le";

pub(crate) async fn charger(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<FormRow, EntDbError> {
    sqlx::query_as(&format!(
        "SELECT {COLS_FORM} FROM ent_frm_formulaire WHERE etablissement_id = $1 AND id = $2"
    ))
    .bind(etab)
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

async fn brouillon(
    conn: &mut PgConnection,
    etab: Uuid,
    form: Uuid,
) -> Result<Option<(Uuid, i32)>, EntDbError> {
    sqlx::query_as("SELECT id, numero FROM ent_frm_version WHERE etablissement_id = $1 AND formulaire_id = $2 AND publiee_le IS NULL")
        .bind(etab)
        .bind(form)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)
}

pub(crate) async fn derniere_publiee(
    conn: &mut PgConnection,
    etab: Uuid,
    form: Uuid,
) -> Result<Option<(Uuid, i32)>, EntDbError> {
    sqlx::query_as(
        "SELECT id, numero FROM ent_frm_version
          WHERE etablissement_id = $1 AND formulaire_id = $2 AND publiee_le IS NOT NULL ORDER BY numero DESC LIMIT 1",
    )
    .bind(etab)
    .bind(form)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)
}

#[allow(clippy::too_many_arguments)]
async fn frm_journaliser(
    conn: &mut PgConnection,
    etab: Uuid,
    acte: &str,
    form: Option<Uuid>,
    version: Option<Uuid>,
    acteur: Uuid,
    nb: Option<i32>,
    detail: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_frm_journal (id, etablissement_id, acte, formulaire_id, version_id, acteur_user_id, nb, detail)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(acte)
    .bind(form)
    .bind(version)
    .bind(acteur)
    .bind(nb)
    .bind(detail)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

pub(crate) async fn frm_publier_evt(
    tx: &mut TenantTx<'static>,
    etab: Uuid,
    type_name: &str,
    formulaire: Uuid,
    eleve: Option<Uuid>,
    acteur: Uuid,
) -> Result<(), EntDbError> {
    // Aucun contenu : identifiants seulement (minimisation, enveloppe neutre).
    tx.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Annonce,
        aggregate_id: formulaire,
        eleve_id: eleve,
        payload: json!({ "formulaire_id": formulaire }),
        actor_user_id: Some(acteur.to_string()),
    })
    .await?;
    Ok(())
}

fn echeance_valide(o: Option<DateTime<Utc>>, e: Option<DateTime<Utc>>) -> Result<(), EntDbError> {
    if let (Some(o), Some(e)) = (o, e) {
        if e <= o {
            return Err(invalide("l'échéance doit suivre l'ouverture"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Lecture de la structure
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct LigneQuestion {
    id: Uuid,
    section_id: Uuid,
    section_ordre: i32,
    ordre: i32,
    t: String,
    libelle: String,
    aide: Option<String>,
    obligatoire: bool,
    sensibilite: String,
    min_nombre: Option<f64>,
    max_nombre: Option<f64>,
    longueur_max: Option<i32>,
    objet_code: Option<String>,
}

pub(crate) async fn charger_structure(
    conn: &mut PgConnection,
    etab: Uuid,
    formulaire_id: Uuid,
    version_id: Uuid,
    numero: i32,
    publiee: bool,
) -> Result<StructureVue, EntDbError> {
    let sections: Vec<(Uuid, i32, String, Option<String>)> = sqlx::query_as(
        "SELECT id, ordre, titre, aide FROM ent_frm_section WHERE etablissement_id = $1 AND version_id = $2 ORDER BY ordre",
    )
    .bind(etab)
    .bind(version_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let questions: Vec<LigneQuestion> = sqlx::query_as(
        "SELECT q.id, q.section_id, s.ordre AS section_ordre, q.ordre, q.type AS t, q.libelle, q.aide, q.obligatoire, q.sensibilite,
                q.min_nombre::float8 AS min_nombre, q.max_nombre::float8 AS max_nombre, q.longueur_max, o.code AS objet_code
           FROM ent_frm_question q
           JOIN ent_frm_section s ON s.etablissement_id = q.etablissement_id AND s.id = q.section_id
           LEFT JOIN ent_acc_objet o ON o.etablissement_id = q.etablissement_id AND o.id = q.objet_accord_id
          WHERE q.etablissement_id = $1 AND q.version_id = $2 ORDER BY s.ordre, q.ordre",
    )
    .bind(etab)
    .bind(version_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let options: Vec<(Uuid, Uuid, i32, String)> = sqlx::query_as(
        "SELECT o.id, o.question_id, o.ordre, o.libelle FROM ent_frm_option o
           JOIN ent_frm_question q ON q.etablissement_id = o.etablissement_id AND q.id = o.question_id
          WHERE o.etablissement_id = $1 AND q.version_id = $2 ORDER BY o.question_id, o.ordre",
    )
    .bind(etab)
    .bind(version_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let lignes: Vec<(Uuid, Uuid, i32, String)> = sqlx::query_as(
        "SELECT l.id, l.question_id, l.ordre, l.libelle FROM ent_frm_ligne l
           JOIN ent_frm_question q ON q.etablissement_id = l.etablissement_id AND q.id = l.question_id
          WHERE l.etablissement_id = $1 AND q.version_id = $2 ORDER BY l.question_id, l.ordre",
    )
    .bind(etab)
    .bind(version_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let conditions: Vec<(Uuid, Uuid, Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT c.id, c.question_id, c.depend_de_question_id, c.option_id, c.operateur FROM ent_frm_condition c
           JOIN ent_frm_question q ON q.etablissement_id = c.etablissement_id AND q.id = c.question_id
          WHERE c.etablissement_id = $1 AND q.version_id = $2",
    )
    .bind(etab)
    .bind(version_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut par_section: HashMap<Uuid, Vec<QuestionVue>> = HashMap::new();
    for q in questions {
        let opts = options
            .iter()
            .filter(|o| o.1 == q.id)
            .map(|o| OptionVue {
                id: o.0,
                ordre: o.2,
                libelle: o.3.clone(),
            })
            .collect();
        let ligs = lignes
            .iter()
            .filter(|o| o.1 == q.id)
            .map(|o| OptionVue {
                id: o.0,
                ordre: o.2,
                libelle: o.3.clone(),
            })
            .collect();
        let conds = conditions
            .iter()
            .filter(|c| c.1 == q.id)
            .map(|c| ConditionVue {
                id: c.0,
                depend_de_question_id: c.2,
                option_id: c.3,
                operateur: c.4.clone(),
            })
            .collect();
        let _ = q.section_ordre;
        par_section
            .entry(q.section_id)
            .or_default()
            .push(QuestionVue {
                id: q.id,
                section_id: q.section_id,
                ordre: q.ordre,
                type_question: q.t,
                libelle: q.libelle,
                aide: q.aide,
                obligatoire: q.obligatoire,
                sensibilite: q.sensibilite,
                min_nombre: q.min_nombre,
                max_nombre: q.max_nombre,
                longueur_max: q.longueur_max,
                objet_accord_code: q.objet_code,
                options: opts,
                lignes: ligs,
                conditions: conds,
            });
    }
    Ok(StructureVue {
        formulaire_id,
        version_id,
        numero,
        publiee,
        sections: sections
            .into_iter()
            .map(|(id, ordre, titre, aide)| SectionVue {
                id,
                ordre,
                titre,
                aide,
                questions: par_section.remove(&id).unwrap_or_default(),
            })
            .collect(),
    })
}

/// Ossature : ce qui fait la **question** (types, nombre d'options, objets, conditions), sans les libellés. Une version « de forme »
/// ne change que les libellés : son ossature est celle de la version précédente.
fn ossature(s: &StructureVue) -> Vec<String> {
    let mut o = Vec::new();
    for sec in &s.sections {
        o.push(format!("S{}", sec.ordre));
        for q in &sec.questions {
            o.push(format!(
                "Q{}.{}|{}|{}|{}|{:?}|{:?}|{:?}|{}|{}|{}|{}",
                sec.ordre,
                q.ordre,
                q.type_question,
                u8::from(q.obligatoire),
                q.sensibilite,
                q.min_nombre,
                q.max_nombre,
                q.longueur_max,
                q.objet_accord_code.clone().unwrap_or_default(),
                q.options.len(),
                q.lignes.len(),
                q.conditions.len()
            ));
        }
    }
    o
}

// ---------------------------------------------------------------------------
// Insertion d'éléments (sans contrôle de droit : l'appelant l'a fait)
// ---------------------------------------------------------------------------

async fn inserer_section(
    conn: &mut PgConnection,
    etab: Uuid,
    version_id: Uuid,
    s: &NouvelleSection,
) -> Result<Uuid, EntDbError> {
    let titre = s.titre.trim();
    if titre.is_empty() || titre.chars().count() > 160 {
        return Err(invalide("titre de section invalide (1 à 160 caractères)"));
    }
    if s.aide.as_deref().is_some_and(|a| a.chars().count() > 1000) {
        return Err(invalide("aide trop longue (1000 caractères au plus)"));
    }
    let (n, max): (i64, Option<i32>) =
        sqlx::query_as("SELECT count(*), max(ordre) FROM ent_frm_section WHERE etablissement_id = $1 AND version_id = $2")
            .bind(etab)
            .bind(version_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
    if n as usize >= MAX_SECTIONS {
        return Err(invalide("trop de sections (30 au plus)"));
    }
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_frm_section (id, etablissement_id, version_id, ordre, titre, aide) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(id)
    .bind(etab)
    .bind(version_id)
    .bind(max.unwrap_or(0) + 1)
    .bind(titre)
    .bind(s.aide.as_deref().map(str::trim).filter(|a| !a.is_empty()))
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(id)
}

/// Insère une question après tous ses contrôles : type, libellé, sensibilité (C3 refusée, C2 réservée), options, lignes et,
/// pour une question « accord », l'objet d'accord (actif, publié, règle `tous` pour l'image, sujet cohérent avec l'unité).
#[allow(clippy::too_many_arguments)]
async fn inserer_question(
    conn: &mut PgConnection,
    etab: Uuid,
    version_id: Uuid,
    f: &FormRow,
    q: &NouvelleQuestion,
    auteur_direction: bool,
) -> Result<Uuid, EntDbError> {
    let sensibilite = controler_question(q, auteur_direction).map_err(invalide)?;
    let section: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM ent_frm_section WHERE etablissement_id = $1 AND id = $2 AND version_id = $3",
    )
    .bind(etab)
    .bind(q.section_id)
    .bind(version_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    if section.is_none() {
        return Err(introuvable());
    }
    let (total,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM ent_frm_question WHERE etablissement_id = $1 AND version_id = $2",
    )
    .bind(etab)
    .bind(version_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if total as usize >= MAX_QUESTIONS {
        return Err(invalide("trop de questions (150 au plus)"));
    }
    let (max,): (Option<i32>,) = sqlx::query_as(
        "SELECT max(ordre) FROM ent_frm_question WHERE etablissement_id = $1 AND section_id = $2",
    )
    .bind(etab)
    .bind(q.section_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let ordre = max.unwrap_or(0) + 1;
    if ordre > 200 {
        return Err(invalide("trop de questions dans cette section"));
    }
    let objet_id = match q.objet_accord_code.as_deref() {
        None => None,
        Some(code) => Some(controler_objet_accord(conn, etab, f, code).await?),
    };
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_frm_question
            (id, etablissement_id, version_id, section_id, ordre, type, libelle, aide, obligatoire, sensibilite, min_nombre,
             max_nombre, longueur_max, objet_accord_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11::float8::numeric,$12::float8::numeric,$13,$14)",
    )
    .bind(id)
    .bind(etab)
    .bind(version_id)
    .bind(q.section_id)
    .bind(ordre)
    .bind(&q.type_question)
    .bind(q.libelle.trim())
    .bind(q.aide.as_deref().map(str::trim).filter(|a| !a.is_empty()))
    .bind(q.obligatoire)
    .bind(sensibilite.code())
    .bind(q.min_nombre)
    .bind(q.max_nombre)
    .bind(q.longueur_max)
    .bind(objet_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let options: Vec<String> = if type_oui_non(&q.type_question) {
        vec!["Oui".to_string(), "Non".to_string()]
    } else {
        q.options.iter().map(|o| o.trim().to_string()).collect()
    };
    for (i, o) in options.iter().enumerate() {
        sqlx::query("INSERT INTO ent_frm_option (id, etablissement_id, question_id, ordre, libelle) VALUES ($1,$2,$3,$4,$5)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(i as i32 + 1)
            .bind(o)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
    }
    for (i, l) in q.lignes.iter().enumerate() {
        sqlx::query("INSERT INTO ent_frm_ligne (id, etablissement_id, question_id, ordre, libelle) VALUES ($1,$2,$3,$4,$5)")
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(i as i32 + 1)
            .bind(l.trim())
            .execute(&mut *conn)
            .await
            .map_err(db)?;
    }
    Ok(id)
}

/// Question « accord » (FRM-124) : l'objet existe, est actif, a une version publiée ; l'image exige la règle `tous` (FD-17) ;
/// le formulaire est « mixte » ; le sujet de l'objet (élève ou personne) va avec l'unité de réponse.
async fn controler_objet_accord(
    conn: &mut PgConnection,
    etab: Uuid,
    f: &FormRow,
    code: &str,
) -> Result<Uuid, EntDbError> {
    if f.nature != "mixte" {
        return Err(conflit(
            "une question « accord » demande un formulaire de nature « mixte »",
        ));
    }
    let o = objet_par_code(&mut *conn, etab, code)
        .await?
        .ok_or_else(introuvable)?;
    if !o.actif {
        return Err(conflit("cet accord n'est pas activé dans l'établissement"));
    }
    if version_en_cours(&mut *conn, etab, o.id).await?.is_none() {
        return Err(conflit("cet accord n'a pas de version publiée"));
    }
    if o.sujet_type == "eleve"
        && objet_exige_tous(&o.code, o.desaccord_objet.as_deref())
        && o.regle != "tous"
    {
        return Err(conflit("le droit à l'image exige la règle `tous` : relevez la règle de l'objet avant de l'utiliser (FD-17)"));
    }
    if (o.sujet_type == "personne") != (f.unite == "personne") {
        return Err(conflit(
            "l'unité de réponse ne va pas avec le sujet de cet accord (élève ou personne)",
        ));
    }
    Ok(o.id)
}

// ---------------------------------------------------------------------------
// Audience
// ---------------------------------------------------------------------------

/// Résolution de l'audience : (inclus après exclusions, nombre d'exclus effectifs). Dans la portée de l'auteur quand elle est
/// donnée : un critère qui déborde est refusé, jamais tronqué en silence.
async fn resoudre_audience(
    conn: &mut PgConnection,
    etab: Uuid,
    formulaire: Uuid,
    portee: Option<&HashSet<Uuid>>,
    jour: chrono::NaiveDate,
) -> Result<(BTreeSet<Uuid>, usize), EntDbError> {
    let criteres: Vec<(String, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT mode, type, cible_id FROM ent_frm_audience_critere WHERE etablissement_id = $1 AND formulaire_id = $2",
    )
    .bind(etab)
    .bind(formulaire)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mut inclus: BTreeSet<Uuid> = BTreeSet::new();
    let mut exclus: BTreeSet<Uuid> = BTreeSet::new();
    for (mode, t, cible) in criteres {
        let eleves: BTreeSet<Uuid> = if t == "eleve" {
            let c = cible.ok_or_else(|| invalide("critère sans cible"))?;
            let connu: Option<(Uuid,)> = sqlx::query_as(
                "SELECT e.id FROM ent_eleve e WHERE e.etablissement_id = $1 AND e.id = $2 AND (e.date_sortie IS NULL OR e.date_sortie > $3)",
            )
            .bind(etab)
            .bind(c)
            .bind(jour)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
            connu.map(|(i,)| i).into_iter().collect()
        } else {
            let ids: Vec<Uuid> = cible.into_iter().collect();
            valider_audience(&mut *conn, etab, &t, &ids).await?;
            eleves_de_audience(&mut *conn, etab, &t, &ids, jour).await?
        };
        if mode == "inclure" {
            if let Some(p) = portee {
                if t == "etablissement" || eleves.iter().any(|e| !p.contains(e)) {
                    return Err(invalide(
                        "cette audience dépasse votre portée (vos classes)",
                    ));
                }
            }
            inclus.extend(eleves);
        } else {
            exclus.extend(eleves);
        }
    }
    let retires = inclus.intersection(&exclus).count();
    let restants = inclus.difference(&exclus).copied().collect();
    Ok((restants, retires))
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

impl PgEnt {
    /// Portée de création : `None` = établissement entier ; `Some(élèves)` = portée limitée (mes classes). Sans droit : refus.
    pub(crate) async fn frm_portee(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<Option<HashSet<Uuid>>, EntDbError> {
        if self
            .a_action_etablissement(etab, user, role, Action::FormulairesCreer)
            .await?
        {
            return Ok(None);
        }
        let p = self
            .eleves_autorises(etab, user, role, Action::FormulairesCreer)
            .await?;
        match p {
            Some(s) if s.is_empty() => Err(EntDbError::Forbidden),
            autre => Ok(autre),
        }
    }

    async fn frm_est_direction(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<bool, EntDbError> {
        self.a_action_etablissement(etab, user, role, Action::FormulairesValider)
            .await
    }

    /// Formulaire à modifier : droit de créer **et** auteur ou co-auteur. Tout autre : introuvable.
    async fn frm_pour_edition(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<FormRow, EntDbError> {
        self.frm_portee(etab, user, role).await?;
        let f = charger(&mut *conn, etab, id).await?;
        let co: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ent_frm_auteur WHERE etablissement_id = $1 AND formulaire_id = $2 AND user_id = $3")
            .bind(etab)
            .bind(id)
            .bind(user)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
        if f.auteur_user_id != user && co.is_none() {
            return Err(introuvable());
        }
        Ok(f)
    }

    /// Formulaire à lire ou à gérer : auteur, co-auteur ou `formulaires.gerer`. Tout autre : introuvable.
    async fn frm_pour_lecture(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(FormRow, bool), EntDbError> {
        let f = charger(&mut *conn, etab, id).await?;
        let co: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ent_frm_auteur WHERE etablissement_id = $1 AND formulaire_id = $2 AND user_id = $3")
            .bind(etab)
            .bind(id)
            .bind(user)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
        let gere = self
            .a_action_etablissement(etab, user, role, Action::FormulairesGerer)
            .await?;
        if f.auteur_user_id != user && co.is_none() && !gere {
            return Err(introuvable());
        }
        Ok((f, gere))
    }

    async fn vue(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        f: FormRow,
    ) -> Result<FormulaireVue, EntDbError> {
        let publiee = derniere_publiee(&mut *conn, etab, f.id)
            .await?
            .map(|(_, n)| n);
        let b = brouillon(&mut *conn, etab, f.id).await?.is_some();
        Ok(FormulaireVue {
            id: f.id,
            titre: f.titre,
            description: f.description,
            nature: f.nature,
            unite: f.unite,
            confidentialite: f.confidentialite,
            statut: f.statut,
            modele_code: f.modele_code,
            ouverture: f.ouverture,
            echeance: f.echeance,
            relances: f.relances,
            audience_nb: f.audience_nb,
            auteur_user_id: f.auteur_user_id,
            version_publiee: publiee,
            brouillon: b,
            cree_le: f.cree_le,
        })
    }

    // ----- Création ---------------------------------------------------------------------------------------------

    /// Créer un formulaire (FRM-120, FRM-126, FRM-130, FRM-140) : nature, unité de réponse, confidentialité, dates, relances.
    /// Un brouillon et sa première version (vide) sont créés ; l'audience se définit ensuite (`frm_definir_audience`).
    pub async fn frm_creer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        n: NouveauFormulaire,
    ) -> Result<FormulaireVue, EntDbError> {
        self.frm_portee(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let id = self
            .inserer_formulaire(&mut tx, etab, user, &n, None)
            .await?;
        let f = charger(&mut tx, etab, id).await?;
        let v = self.vue(&mut tx, etab, f).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    async fn inserer_formulaire(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        n: &NouveauFormulaire,
        modele: Option<&str>,
    ) -> Result<Uuid, EntDbError> {
        let titre = n.titre.trim();
        if titre.is_empty() || titre.chars().count() > 160 {
            return Err(invalide("titre invalide (1 à 160 caractères)"));
        }
        if n.description
            .as_deref()
            .is_some_and(|d| d.chars().count() > 2000)
        {
            return Err(invalide(
                "description trop longue (2000 caractères au plus)",
            ));
        }
        if !NATURES_FORMULAIRE.contains(&n.nature.as_str()) {
            return Err(invalide("nature inconnue (information, collecte ou mixte)"));
        }
        if !UNITES.contains(&n.unite.as_str()) {
            return Err(invalide("unité de réponse inconnue"));
        }
        let confid = n.confidentialite.as_deref().unwrap_or("nominatif");
        if confid == "anonyme" {
            return Err(invalide("le sondage anonyme n'est pas encore disponible (lot suivant) : aucune promesse d'anonymat n'est faite"));
        }
        if !CONFIDENTIALITES.contains(&confid) {
            return Err(invalide(
                "confidentialité inconnue (nominatif ou confidentiel)",
            ));
        }
        let relances = n.relances.as_deref().unwrap_or("j7_j3_j1");
        if !RELANCES.contains(&relances) {
            return Err(invalide("réglage de relances inconnu"));
        }
        echeance_valide(n.ouverture, n.echeance)?;
        match (confid, n.destinataire_confidentiel_user_id) {
            ("confidentiel", None) => {
                return Err(invalide(
                    "un formulaire confidentiel désigne le destinataire de ses réponses",
                ))
            }
            ("nominatif", Some(_)) => {
                return Err(invalide(
                    "un destinataire ne se désigne que pour un formulaire confidentiel",
                ))
            }
            ("confidentiel", Some(d)) => {
                let m: Option<(String,)> = sqlx::query_as(
                    "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
                )
                .bind(etab)
                .bind(d)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
                if m.is_none_or(|(r,)| matches!(r.as_str(), "parent" | "eleve")) {
                    return Err(invalide(
                        "le destinataire doit être un membre du personnel de l'établissement",
                    ));
                }
            }
            _ => {}
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_frm_formulaire
                (id, etablissement_id, titre, description, nature, unite, confidentialite, destinataire_confidentiel_user_id,
                 modele_code, ouverture, echeance, relances, auteur_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(id)
        .bind(etab)
        .bind(titre)
        .bind(n.description.as_deref().map(str::trim).filter(|d| !d.is_empty()))
        .bind(&n.nature)
        .bind(&n.unite)
        .bind(confid)
        .bind(n.destinataire_confidentiel_user_id)
        .bind(modele)
        .bind(n.ouverture)
        .bind(n.echeance)
        .bind(relances)
        .bind(user)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        let version_id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_frm_version (id, etablissement_id, formulaire_id, numero, cree_par) VALUES ($1,$2,$3,1,$4)")
            .bind(version_id)
            .bind(etab)
            .bind(id)
            .bind(user)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        frm_journaliser(
            &mut *conn,
            etab,
            if modele.is_some() { "modele" } else { "cree" },
            Some(id),
            Some(version_id),
            user,
            None,
            modele,
        )
        .await?;
        Ok(id)
    }

    /// Modèles prêts (FRM-121) : code, titre, description, nature, unité.
    pub async fn frm_modeles(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<Vec<(String, String, String, String, String)>, EntDbError> {
        self.frm_portee(etab, user, role).await?;
        Ok(MODELES
            .iter()
            .map(|m| {
                (
                    m.code.to_string(),
                    m.titre.to_string(),
                    m.description.to_string(),
                    m.nature.to_string(),
                    m.unite.to_string(),
                )
            })
            .collect())
    }

    /// Partir d'un modèle prêt (FRM-121, D-023) : coupon de réunion, choix d'atelier, enquête de satisfaction, sortie, invitation.
    /// Le modèle « sortie » exige l'objet d'accord « sortie facultative » activé et publié (la question « accord » en découle).
    pub async fn frm_creer_depuis_modele(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        code: &str,
        titre: Option<String>,
    ) -> Result<FormulaireVue, EntDbError> {
        self.frm_portee(etab, user, role).await?;
        let m = modele(code).ok_or_else(introuvable)?;
        let direction = self.frm_est_direction(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = NouveauFormulaire {
            titre: titre.unwrap_or_else(|| m.titre.to_string()),
            description: Some(m.description.to_string()),
            nature: m.nature.to_string(),
            unite: m.unite.to_string(),
            confidentialite: None,
            destinataire_confidentiel_user_id: None,
            ouverture: None,
            echeance: None,
            relances: None,
        };
        let id = self
            .inserer_formulaire(&mut tx, etab, user, &n, Some(m.code))
            .await?;
        let f = charger(&mut tx, etab, id).await?;
        let (version_id, _) = brouillon(&mut tx, etab, id)
            .await?
            .ok_or_else(introuvable)?;
        for (i, s) in m.sections.iter().enumerate() {
            let section = inserer_section(
                &mut tx,
                etab,
                version_id,
                &NouvelleSection {
                    titre: s.titre.to_string(),
                    aide: None,
                },
            )
            .await?;
            if i == 0 {
                if let Some(objet) = m.objet_accord {
                    inserer_question(
                        &mut tx,
                        etab,
                        version_id,
                        &f,
                        &question_modele(
                            section,
                            "accord",
                            "Autorisation de participer à la sortie",
                            true,
                            &[],
                            None,
                            None,
                            Some(objet),
                        ),
                        direction,
                    )
                    .await?;
                }
            }
            for q in s.questions {
                inserer_question(
                    &mut tx,
                    etab,
                    version_id,
                    &f,
                    &question_modele(
                        section,
                        q.type_question,
                        q.libelle,
                        q.obligatoire,
                        q.options,
                        q.min_nombre,
                        q.max_nombre,
                        None,
                    ),
                    direction,
                )
                .await?;
            }
        }
        let f = charger(&mut tx, etab, id).await?;
        let v = self.vue(&mut tx, etab, f).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    // ----- Contenu du brouillon -----------------------------------------------------------------------------------

    async fn brouillon_modifiable(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        f: &FormRow,
    ) -> Result<Uuid, EntDbError> {
        if f.statut == "soumis" {
            return Err(conflit(
                "le formulaire attend sa validation : retirez-le de la validation pour le modifier",
            ));
        }
        if matches!(f.statut.as_str(), "clos" | "archive") {
            return Err(conflit("ce formulaire est clos ou archivé"));
        }
        brouillon(&mut *conn, etab, f.id)
            .await?
            .map(|(v, _)| v)
            .ok_or(conflit("aucune version en brouillon : créez une nouvelle version pour modifier un formulaire publié"))
    }

    /// Ajouter une section (FRM-122).
    pub async fn frm_ajouter_section(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        s: NouvelleSection,
    ) -> Result<Uuid, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        let version = self.brouillon_modifiable(&mut tx, etab, &f).await?;
        let section = inserer_section(&mut tx, etab, version, &s).await?;
        frm_journaliser(
            &mut tx,
            etab,
            "modifie",
            Some(id),
            Some(version),
            user,
            None,
            Some("section"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(section)
    }

    /// Ajouter une question (FRM-122, FRM-124, FRM-125) : types du §11.2, sensibilité classée, question « accord » liée à un objet
    /// d'accord existant (règle de l'objet respectée).
    pub async fn frm_ajouter_question(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        q: NouvelleQuestion,
    ) -> Result<Uuid, EntDbError> {
        let direction = self.frm_est_direction(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        let version = self.brouillon_modifiable(&mut tx, etab, &f).await?;
        let question = inserer_question(&mut tx, etab, version, &f, &q, direction).await?;
        frm_journaliser(
            &mut tx,
            etab,
            "modifie",
            Some(id),
            Some(version),
            user,
            None,
            Some("question"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(question)
    }

    /// Retirer une question du brouillon (avec ses options, lignes et conditions).
    pub async fn frm_retirer_question(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        question_id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        let version = self.brouillon_modifiable(&mut tx, etab, &f).await?;
        let q: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ent_frm_question WHERE etablissement_id = $1 AND id = $2 AND version_id = $3")
            .bind(etab)
            .bind(question_id)
            .bind(version)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        q.ok_or_else(introuvable)?;
        for sql in [
            "DELETE FROM ent_frm_condition WHERE etablissement_id = $1 AND (question_id = $2 OR depend_de_question_id = $2)",
            "DELETE FROM ent_frm_option WHERE etablissement_id = $1 AND question_id = $2",
            "DELETE FROM ent_frm_ligne WHERE etablissement_id = $1 AND question_id = $2",
            "DELETE FROM ent_frm_question WHERE etablissement_id = $1 AND id = $2",
        ] {
            sqlx::query(sql).bind(etab).bind(question_id).execute(&mut *tx).await.map_err(db)?;
        }
        frm_journaliser(
            &mut tx,
            etab,
            "modifie",
            Some(id),
            Some(version),
            user,
            None,
            Some("question_retiree"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Ajouter une condition d'affichage (FRM-123) : la question dépend d'une question **placée avant elle** (donc jamais de boucle),
    /// de type à choix, et d'une de ses options. Le code le vérifie, la base aussi (déclencheur).
    pub async fn frm_ajouter_condition(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        c: NouvelleCondition,
    ) -> Result<Uuid, EntDbError> {
        if !OPERATEURS.contains(&c.operateur.as_str()) {
            return Err(invalide("opérateur inconnu (egal ou different)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        let version = self.brouillon_modifiable(&mut tx, etab, &f).await?;
        let rangs: Vec<(Uuid, i32, i32, String)> = sqlx::query_as(
            "SELECT q.id, s.ordre, q.ordre, q.type FROM ent_frm_question q
               JOIN ent_frm_section s ON s.etablissement_id = q.etablissement_id AND s.id = q.section_id
              WHERE q.etablissement_id = $1 AND q.version_id = $2",
        )
        .bind(etab)
        .bind(version)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let trouver = |q: Uuid| rangs.iter().find(|r| r.0 == q);
        let (Some(cible), Some(dep)) = (trouver(c.question_id), trouver(c.depend_de_question_id))
        else {
            return Err(introuvable());
        };
        if cible.0 == dep.0 {
            return Err(invalide("une question ne peut dépendre d'elle-même"));
        }
        if !type_a_condition(&dep.3) {
            return Err(invalide(
                "une condition se fonde sur une question à choix ou « oui / non »",
            ));
        }
        if !condition_vers_l_avant((cible.1, cible.2), (dep.1, dep.2)) {
            return Err(invalide("la question dont on dépend doit être placée avant : aucune boucle de conditions n'est permise"));
        }
        let existantes: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT c.question_id, c.depend_de_question_id FROM ent_frm_condition c
               JOIN ent_frm_question q ON q.etablissement_id = c.etablissement_id AND q.id = c.question_id
              WHERE c.etablissement_id = $1 AND q.version_id = $2",
        )
        .bind(etab)
        .bind(version)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut aretes = existantes;
        aretes.push((c.question_id, c.depend_de_question_id));
        if a_une_boucle(&aretes) {
            return Err(invalide("cette condition ferait une boucle"));
        }
        let option: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ent_frm_option WHERE etablissement_id = $1 AND id = $2 AND question_id = $3")
            .bind(etab)
            .bind(c.option_id)
            .bind(c.depend_de_question_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        if option.is_none() {
            return Err(invalide(
                "l'option n'appartient pas à la question dont on dépend",
            ));
        }
        let cid = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_frm_condition (id, etablissement_id, question_id, depend_de_question_id, option_id, operateur)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(cid)
        .bind(etab)
        .bind(c.question_id)
        .bind(c.depend_de_question_id)
        .bind(c.option_id)
        .bind(&c.operateur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        frm_journaliser(
            &mut tx,
            etab,
            "modifie",
            Some(id),
            Some(version),
            user,
            None,
            Some("condition"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(cid)
    }

    /// Réglages du formulaire (FRM-130, FRM-137) : titre, description, ouverture, échéance, relances. Avant la publication, tout ;
    /// après, seules l'échéance (prolongée, jamais raccourcie) et les relances.
    #[allow(clippy::too_many_arguments)]
    pub async fn frm_regler(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        titre: Option<String>,
        description: Option<String>,
        ouverture: Option<DateTime<Utc>>,
        echeance: Option<DateTime<Utc>>,
        relances: Option<String>,
    ) -> Result<FormulaireVue, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        if matches!(f.statut.as_str(), "clos" | "archive") {
            return Err(conflit("ce formulaire est clos ou archivé"));
        }
        let publie = f.publie_le.is_some();
        if publie && (titre.is_some() || description.is_some() || ouverture.is_some()) {
            return Err(conflit("après la publication, seules l'échéance et les relances se règlent (nouvelle version pour le reste)"));
        }
        if let Some(t) = &titre {
            if t.trim().is_empty() || t.trim().chars().count() > 160 {
                return Err(invalide("titre invalide (1 à 160 caractères)"));
            }
        }
        if description
            .as_deref()
            .is_some_and(|d| d.chars().count() > 2000)
        {
            return Err(invalide("description trop longue"));
        }
        if let Some(r) = &relances {
            if !RELANCES.contains(&r.as_str()) {
                return Err(invalide("réglage de relances inconnu"));
            }
        }
        let nouvelle_ouverture = ouverture.or(f.ouverture);
        let nouvelle_echeance = echeance.or(f.echeance);
        echeance_valide(nouvelle_ouverture, nouvelle_echeance)?;
        if let (Some(ancienne), Some(nouvelle)) = (f.echeance, echeance) {
            if publie && nouvelle < ancienne {
                return Err(conflit(
                    "une échéance se prolonge, elle ne se raccourcit pas après la publication",
                ));
            }
        }
        sqlx::query(
            "UPDATE ent_frm_formulaire SET titre = COALESCE($3, titre), description = COALESCE($4, description),
                    ouverture = $5, echeance = $6, relances = COALESCE($7, relances)
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(titre.as_deref().map(str::trim))
        .bind(description.as_deref().map(str::trim))
        .bind(nouvelle_ouverture)
        .bind(nouvelle_echeance)
        .bind(relances.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        frm_journaliser(
            &mut tx,
            etab,
            if echeance.is_some() && publie {
                "prolonge"
            } else {
                "modifie"
            },
            Some(id),
            None,
            user,
            None,
            Some("reglages"),
        )
        .await?;
        let f = charger(&mut tx, etab, id).await?;
        let v = self.vue(&mut tx, etab, f).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    // ----- Audience ---------------------------------------------------------------------------------------------

    /// Définir l'audience (FRM-120, FRM-132) : critères en lignes (inclure / exclure), **dans la portée de l'auteur**. Remplace les
    /// critères précédents (avant la première publication). Rend l'aperçu : nombres seulement, exclus fondus.
    pub async fn frm_definir_audience(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        criteres: Vec<CritereAudience>,
    ) -> Result<ApercuAudience, EntDbError> {
        if criteres.is_empty() || criteres.len() > 100 {
            return Err(invalide("au moins un critère d'audience (100 au plus)"));
        }
        for c in &criteres {
            controler_critere(c).map_err(invalide)?;
        }
        if !criteres.iter().any(|c| c.mode == "inclure") {
            return Err(invalide("au moins un critère « inclure »"));
        }
        let portee = self.frm_portee(etab, user, role).await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        if f.statut != "brouillon" || f.publie_le.is_some() {
            return Err(conflit(
                "l'audience se définit avant la première publication",
            ));
        }
        sqlx::query("DELETE FROM ent_frm_audience_critere WHERE etablissement_id = $1 AND formulaire_id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let mut vus = HashSet::new();
        for c in &criteres {
            if !vus.insert(c.clone()) {
                continue;
            }
            sqlx::query(
                "INSERT INTO ent_frm_audience_critere (id, etablissement_id, formulaire_id, mode, type, cible_id) VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(&c.mode)
            .bind(&c.type_critere)
            .bind(c.cible_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let (restants, exclus) =
            resoudre_audience(&mut tx, etab, id, portee.as_ref(), jour).await?;
        frm_journaliser(
            &mut tx,
            etab,
            "modifie",
            Some(id),
            None,
            user,
            Some(restants.len() as i32),
            Some("audience"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ApercuAudience {
            total: restants.len(),
            exclus,
            validation_requise: restants.len() > seuil_validation(),
        })
    }

    /// Aperçu de l'audience enregistrée (FRM-132) : nombres seulement.
    pub async fn frm_apercu_audience(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ApercuAudience, EntDbError> {
        let portee = self.frm_portee(etab, user, role).await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        let (restants, exclus) =
            resoudre_audience(&mut tx, etab, id, portee.as_ref(), jour).await?;
        Ok(ApercuAudience {
            total: restants.len(),
            exclus,
            validation_requise: restants.len() > seuil_validation(),
        })
    }

    // ----- Lecture ----------------------------------------------------------------------------------------------

    /// Mes formulaires (auteur ou co-auteur) ; avec `formulaires.gerer`, tous ceux de l'établissement.
    pub async fn frm_lister(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<Vec<FormulaireVue>, EntDbError> {
        let gere = self
            .a_action_etablissement(etab, user, role, Action::FormulairesGerer)
            .await?;
        if !gere {
            self.frm_portee(etab, user, role).await?;
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<FormRow> = sqlx::query_as(&format!(
            "SELECT {COLS_FORM} FROM ent_frm_formulaire f
              WHERE f.etablissement_id = $1
                AND ($4 OR f.auteur_user_id = $2 OR EXISTS (SELECT 1 FROM ent_frm_auteur a
                                                              WHERE a.etablissement_id = f.etablissement_id AND a.formulaire_id = f.id AND a.user_id = $2))
              ORDER BY f.cree_le DESC LIMIT $3"
        ))
        .bind(etab)
        .bind(user)
        .bind(200_i64)
        .bind(gere)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::with_capacity(lignes.len());
        for f in lignes {
            out.push(self.vue(&mut tx, etab, f).await?);
        }
        Ok(out)
    }

    pub async fn frm_lire(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<FormulaireVue, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (f, _) = self.frm_pour_lecture(&mut tx, etab, user, role, id).await?;
        self.vue(&mut tx, etab, f).await
    }

    /// Structure d'une version (FRM-131) : le brouillon s'il existe, sinon la dernière publiée ; `numero` désigne une version publiée.
    pub async fn frm_structure(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        numero: Option<i32>,
    ) -> Result<StructureVue, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (f, _) = self.frm_pour_lecture(&mut tx, etab, user, role, id).await?;
        let cible: Option<(Uuid, i32, bool)> = match numero {
            Some(n) => sqlx::query_as(
                "SELECT id, numero, publiee_le IS NOT NULL FROM ent_frm_version WHERE etablissement_id = $1 AND formulaire_id = $2 AND numero = $3",
            )
            .bind(etab)
            .bind(f.id)
            .bind(n)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?,
            None => match brouillon(&mut tx, etab, f.id).await? {
                Some((v, n)) => Some((v, n, false)),
                None => derniere_publiee(&mut tx, etab, f.id).await?.map(|(v, n)| (v, n, true)),
            },
        };
        let (v, n, publiee) = cible.ok_or_else(introuvable)?;
        charger_structure(&mut tx, etab, f.id, v, n, publiee).await
    }

    // ----- Publication -------------------------------------------------------------------------------------------

    /// Publier (FRM-133) : questions figées, audience résolue en cibles. Au-delà du seuil d'audience, la publication attend la
    /// validation d'un autre responsable (FRM-145, quatre yeux) : le formulaire passe en « soumis ».
    pub async fn frm_publier(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        changement: Option<&str>,
    ) -> Result<BilanPublication, EntDbError> {
        let portee = self.frm_portee(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        let r = self
            .publier_interne(&mut tx, etab, user, portee.as_ref(), &f, None, changement)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Valider un formulaire à large audience (FRM-145) : `formulaires.valider`, **jamais par son auteur**. Publie la première
    /// version (formulaire « soumis ») ou une nouvelle version d'un formulaire à large audience.
    pub async fn frm_valider(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        changement: Option<&str>,
    ) -> Result<BilanPublication, EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::FormulairesValider)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = charger(&mut tx, etab, id).await?;
        if f.auteur_user_id == user {
            return Err(refus_interdit("un formulaire ne se valide pas soi-même : un autre responsable valide (quatre yeux)"));
        }
        let co: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ent_frm_auteur WHERE etablissement_id = $1 AND formulaire_id = $2 AND user_id = $3")
            .bind(etab)
            .bind(id)
            .bind(user)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        if co.is_some() {
            return Err(refus_interdit(
                "un co-auteur ne valide pas le formulaire qu'il a écrit",
            ));
        }
        let r = self
            .publier_interne(&mut tx, etab, user, None, &f, Some(user), changement)
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(r)
    }

    /// Refuser la validation : le formulaire revient en brouillon, le motif est un code court tracé au journal.
    pub async fn frm_refuser_validation(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        motif: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, user, role, Action::FormulairesValider)
            .await?;
        if motif.is_empty()
            || !motif
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.:-".contains(c))
            || motif.len() > 80
        {
            return Err(invalide(
                "motif : un code court (minuscules, chiffres, _ . : -)",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = charger(&mut tx, etab, id).await?;
        if f.statut != "soumis" {
            return Err(conflit("ce formulaire n'attend pas de validation"));
        }
        if f.auteur_user_id == user {
            return Err(refus_interdit("un formulaire ne se valide pas soi-même"));
        }
        sqlx::query("UPDATE ent_frm_formulaire SET statut = 'brouillon', soumis_le = NULL WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        frm_journaliser(
            &mut tx,
            etab,
            "refuse_validation",
            Some(id),
            None,
            user,
            None,
            Some(motif),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Retirer un formulaire « soumis » de la validation pour le modifier encore (auteur).
    pub async fn frm_retirer_de_la_validation(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        if f.statut != "soumis" {
            return Err(conflit("ce formulaire n'attend pas de validation"));
        }
        sqlx::query("UPDATE ent_frm_formulaire SET statut = 'brouillon', soumis_le = NULL WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        frm_journaliser(
            &mut tx,
            etab,
            "modifie",
            Some(id),
            None,
            user,
            None,
            Some("retire_validation"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn publier_interne(
        &self,
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        user: Uuid,
        portee: Option<&HashSet<Uuid>>,
        f: &FormRow,
        valideur: Option<Uuid>,
        changement: Option<&str>,
    ) -> Result<BilanPublication, EntDbError> {
        if matches!(f.statut.as_str(), "clos" | "archive") {
            return Err(conflit("ce formulaire est clos ou archivé"));
        }
        let (version_id, numero) = brouillon(&mut *tx, etab, f.id)
            .await?
            .ok_or(conflit("aucune version en brouillon à publier"))?;
        let premiere = f.publie_le.is_none();
        if premiere && !matches!(f.statut.as_str(), "brouillon" | "soumis") {
            return Err(conflit(
                "ce formulaire ne peut pas être publié dans cet état",
            ));
        }
        if valideur.is_some() && premiere && f.statut != "soumis" {
            return Err(conflit("ce formulaire n'attend pas de validation"));
        }
        let structure = charger_structure(&mut *tx, etab, f.id, version_id, numero, false).await?;
        controler_publication(&structure, &f.nature).map_err(invalide)?;
        // Les accords liés doivent toujours être utilisables au moment de publier (FD-17 compris).
        for sec in &structure.sections {
            for q in sec.questions.iter().filter(|q| q.type_question == "accord") {
                let code = q.objet_accord_code.as_deref().ok_or_else(introuvable)?;
                controler_objet_accord(&mut *tx, etab, f, code).await?;
            }
        }
        // Un changement « de forme » ne change que les libellés : l'ossature est celle de la version précédente.
        let type_changement: Option<&str> = if numero > 1 {
            let c = changement.unwrap_or("fond");
            if !["forme", "fond"].contains(&c) {
                return Err(invalide("type de changement inconnu (forme ou fond)"));
            }
            if c == "forme" {
                let (prec, pn) = derniere_publiee(&mut *tx, etab, f.id)
                    .await?
                    .ok_or_else(introuvable)?;
                let avant = charger_structure(&mut *tx, etab, f.id, prec, pn, true).await?;
                if ossature(&avant) != ossature(&structure) {
                    return Err(conflit(
                        "la structure a changé : cette version est « de fond », pas « de forme »",
                    ));
                }
            }
            Some(c)
        } else {
            None
        };
        // Audience : résolue à la première publication ; aux suivantes, l'effectif est celui déjà enregistré.
        let jour = self.aujourdhui_etablissement(etab).await?;
        let (cibles, nb) = if premiere {
            let (restants, _) = resoudre_audience(
                &mut *tx,
                etab,
                f.id,
                if valideur.is_none() { portee } else { None },
                jour,
            )
            .await?;
            if restants.is_empty() {
                return Err(invalide(
                    "l'audience est vide : définissez des destinataires avant de publier",
                ));
            }
            let nb = restants.len();
            (Some(restants), nb)
        } else {
            (None, f.audience_nb.unwrap_or(0).max(0) as usize)
        };
        if nb > seuil_validation() && valideur.is_none() {
            if premiere {
                sqlx::query(
                    "UPDATE ent_frm_formulaire SET statut = 'soumis', soumis_le = now(), audience_nb = $3 WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(f.id)
                .bind(nb as i32)
                .execute(&mut **tx)
                .await
                .map_err(db)?;
                frm_journaliser(
                    &mut *tx,
                    etab,
                    "soumis",
                    Some(f.id),
                    Some(version_id),
                    user,
                    Some(nb as i32),
                    None,
                )
                .await?;
                return Ok(BilanPublication {
                    formulaire_id: f.id,
                    version_numero: numero,
                    statut: "soumis".into(),
                    cibles: nb,
                });
            }
            return Err(conflit(
                "audience large : la nouvelle version attend la validation d'un autre responsable",
            ));
        }
        let elements = elements_empreinte(&structure, &f.titre);
        let refs: Vec<&str> = elements.iter().map(String::as_str).collect();
        let empreinte = crate::accords::empreinte_encodee(&refs);
        sqlx::query(
            "UPDATE ent_frm_version SET publiee_le = now(), publiee_par = $3, empreinte = $4, type_changement = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(version_id)
        .bind(user)
        .bind(&empreinte)
        .bind(type_changement)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        if let Some(cibles) = &cibles {
            for e in cibles {
                sqlx::query(
                    "INSERT INTO ent_frm_cible (id, etablissement_id, formulaire_id, eleve_id) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(f.id)
                .bind(e)
                .execute(&mut **tx)
                .await
                .map_err(db)?;
            }
            sqlx::query(
                "UPDATE ent_frm_formulaire SET statut = 'publie', publie_par = $3, publie_le = now(), audience_nb = $4,
                        valide_par = $5, valide_le = CASE WHEN $5::uuid IS NULL THEN NULL ELSE now() END
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(f.id)
            .bind(user)
            .bind(nb as i32)
            .bind(valideur)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        }
        let detail = format!("v{numero}");
        frm_journaliser(
            &mut *tx,
            etab,
            if premiere { "publie" } else { "version" },
            Some(f.id),
            Some(version_id),
            user,
            Some(nb as i32),
            Some(&detail),
        )
        .await?;
        if valideur.is_some() {
            frm_journaliser(
                &mut *tx,
                etab,
                "valide",
                Some(f.id),
                Some(version_id),
                user,
                None,
                None,
            )
            .await?;
        }
        frm_publier_evt(tx, etab, evenements::PUBLIE, f.id, None, user).await?;
        Ok(BilanPublication {
            formulaire_id: f.id,
            version_numero: numero,
            statut: "publie".into(),
            cibles: nb,
        })
    }

    // ----- Versions, corrections, copie -----------------------------------------------------------------------------

    /// Créer une nouvelle version (FRM-134) : un brouillon copié de la dernière version publiée. Les réponses restent liées à la
    /// version à laquelle elles ont répondu.
    pub async fn frm_nouvelle_version(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<StructureVue, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        if f.statut != "publie" {
            return Err(conflit(
                "seul un formulaire publié se modifie par une nouvelle version",
            ));
        }
        if brouillon(&mut tx, etab, id).await?.is_some() {
            return Err(conflit("une version en brouillon existe déjà"));
        }
        let (source, n) = derniere_publiee(&mut tx, etab, id)
            .await?
            .ok_or_else(introuvable)?;
        let nouvelle = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_frm_version (id, etablissement_id, formulaire_id, numero, cree_par) VALUES ($1,$2,$3,$4,$5)")
            .bind(nouvelle)
            .bind(etab)
            .bind(id)
            .bind(n + 1)
            .bind(user)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        copier_structure(&mut tx, etab, id, source, n, nouvelle).await?;
        frm_journaliser(
            &mut tx,
            etab,
            "modifie",
            Some(id),
            Some(nouvelle),
            user,
            None,
            Some("nouvelle_version"),
        )
        .await?;
        let s = charger_structure(&mut tx, etab, id, nouvelle, n + 1, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(s)
    }

    /// Corriger un libellé ou une aide sans changer la question (FRM-135) : tracé avec l'ancienne valeur. Interdit pour une
    /// question « accord » (le texte d'un accord se corrige par une nouvelle version de l'objet) et si la sensibilité détectée change.
    #[allow(clippy::too_many_arguments)]
    pub async fn frm_corriger_libelle(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        question_id: Uuid,
        libelle: &str,
        aide: Option<&str>,
    ) -> Result<(), EntDbError> {
        let libelle = libelle.trim();
        if libelle.is_empty()
            || libelle.chars().count() > 500
            || aide.is_some_and(|a| a.chars().count() > 1000)
        {
            return Err(invalide("libellé ou aide invalide"));
        }
        if lint_sensibilite(&format!("{} {}", libelle, aide.unwrap_or(""))) == Sensibilite::C3 {
            return Err(invalide(
                "donnée de santé : interdite dans un formulaire ordinaire",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        let q: Option<(String, Option<String>, String, bool)> = sqlx::query_as(
            "SELECT q.libelle, q.aide, q.type, v.publiee_le IS NOT NULL
               FROM ent_frm_question q JOIN ent_frm_version v ON v.etablissement_id = q.etablissement_id AND v.id = q.version_id
              WHERE q.etablissement_id = $1 AND q.id = $2 AND v.formulaire_id = $3",
        )
        .bind(etab)
        .bind(question_id)
        .bind(f.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (ancien, ancienne_aide, t, publiee) = q.ok_or_else(introuvable)?;
        if !publiee {
            return Err(conflit(
                "la version est un brouillon : modifiez-la directement",
            ));
        }
        if t == "accord" {
            return Err(conflit(
                "le texte d'un accord ne se corrige pas ici : nouvelle version de l'objet d'accord",
            ));
        }
        sqlx::query("SELECT set_config('ent.frm_correction', 'on', true)")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE ent_frm_question SET libelle = $3, aide = $4 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(question_id)
            .bind(libelle)
            .bind(aide.map(str::trim).filter(|a| !a.is_empty()))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("SELECT set_config('ent.frm_correction', 'off', true)")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_frm_correction
                (id, etablissement_id, formulaire_id, question_id, ancien_libelle, nouveau_libelle, ancienne_aide, nouvelle_aide, par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(f.id)
        .bind(question_id)
        .bind(&ancien)
        .bind(libelle)
        .bind(&ancienne_aide)
        .bind(aide.map(str::trim).filter(|a| !a.is_empty()))
        .bind(user)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        frm_journaliser(
            &mut tx,
            etab,
            "correction",
            Some(f.id),
            None,
            user,
            None,
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Dupliquer un formulaire (FRM-136) : un nouveau brouillon avec la structure de la dernière version ; l'audience se redéfinit.
    pub async fn frm_dupliquer(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<FormulaireVue, EntDbError> {
        self.frm_portee(etab, user, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (f, _) = self.frm_pour_lecture(&mut tx, etab, user, role, id).await?;
        let (source, n, publiee) = match brouillon(&mut tx, etab, id).await? {
            Some((v, n)) => (v, n, false),
            None => {
                let (v, n) = derniere_publiee(&mut tx, etab, id)
                    .await?
                    .ok_or_else(introuvable)?;
                (v, n, true)
            }
        };
        let _ = publiee;
        let copie_titre: String = format!("Copie de {}", f.titre).chars().take(160).collect();
        let nouveau = NouveauFormulaire {
            titre: copie_titre,
            description: f.description.clone(),
            nature: f.nature.clone(),
            unite: f.unite.clone(),
            confidentialite: Some(f.confidentialite.clone()),
            destinataire_confidentiel_user_id: f.destinataire_confidentiel_user_id,
            ouverture: None,
            echeance: None,
            relances: Some(f.relances.clone()),
        };
        let nid = self
            .inserer_formulaire(&mut tx, etab, user, &nouveau, f.modele_code.as_deref())
            .await?;
        let (nversion, _) = brouillon(&mut tx, etab, nid)
            .await?
            .ok_or_else(introuvable)?;
        copier_structure(&mut tx, etab, nid, source, n, nversion).await?;
        frm_journaliser(
            &mut tx,
            etab,
            "duplique",
            Some(nid),
            Some(nversion),
            user,
            None,
            None,
        )
        .await?;
        let nf = charger(&mut tx, etab, nid).await?;
        let v = self.vue(&mut tx, etab, nf).await?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    // ----- Cycle de vie ---------------------------------------------------------------------------------------------

    /// Clore (FRM-137) : plus de réponse. Auteur, co-auteur ou `formulaires.gerer`.
    pub async fn frm_clore(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.transition(etab, user, role, id, "publie", "clos", "clos", true)
            .await
    }

    /// Rouvrir un formulaire clos (FRM-137).
    pub async fn frm_rouvrir(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        self.transition(etab, user, role, id, "clos", "publie", "rouvert", false)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn transition(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        de: &str,
        vers: &str,
        acte: &str,
        clos: bool,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (f, _) = self.frm_pour_lecture(&mut tx, etab, user, role, id).await?;
        if f.statut != de {
            return Err(conflit(
                "ce formulaire n'est pas dans l'état attendu pour cette opération",
            ));
        }
        sqlx::query(
            "UPDATE ent_frm_formulaire SET statut = $3, clos_le = CASE WHEN $4 THEN now() ELSE NULL END
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(vers)
        .bind(clos)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        frm_journaliser(&mut tx, etab, acte, Some(id), None, user, None, None).await?;
        if clos {
            frm_publier_evt(&mut tx, etab, evenements::CLOS, id, None, user).await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Archiver (FRM-138) : un formulaire clos, ou un brouillon abandonné. Auteur, co-auteur ou `formulaires.gerer`. Irréversible.
    pub async fn frm_archiver(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (f, _) = self.frm_pour_lecture(&mut tx, etab, user, role, id).await?;
        if !matches!(f.statut.as_str(), "clos" | "brouillon") {
            return Err(conflit("clore le formulaire avant de l'archiver"));
        }
        sqlx::query("UPDATE ent_frm_formulaire SET statut = 'archive', archive_par = $3, archive_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .bind(user)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        frm_journaliser(&mut tx, etab, "archive", Some(id), None, user, None, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Supprimer un brouillon **jamais publié** (FRM-149) : la seule suppression permise (la base refuse toute autre).
    pub async fn frm_supprimer_brouillon(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        if f.statut != "brouillon" || f.publie_le.is_some() {
            return Err(conflit("seul un brouillon jamais publié se supprime"));
        }
        let versions: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_frm_version WHERE etablissement_id = $1 AND formulaire_id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (v,) in versions {
            for sql in [
                "DELETE FROM ent_frm_condition WHERE etablissement_id = $1 AND question_id IN (SELECT id FROM ent_frm_question WHERE etablissement_id = $1 AND version_id = $2)",
                "DELETE FROM ent_frm_option WHERE etablissement_id = $1 AND question_id IN (SELECT id FROM ent_frm_question WHERE etablissement_id = $1 AND version_id = $2)",
                "DELETE FROM ent_frm_ligne WHERE etablissement_id = $1 AND question_id IN (SELECT id FROM ent_frm_question WHERE etablissement_id = $1 AND version_id = $2)",
                "DELETE FROM ent_frm_question WHERE etablissement_id = $1 AND version_id = $2",
                "DELETE FROM ent_frm_section WHERE etablissement_id = $1 AND version_id = $2",
                "DELETE FROM ent_frm_version WHERE etablissement_id = $1 AND id = $2",
            ] {
                sqlx::query(sql).bind(etab).bind(v).execute(&mut *tx).await.map_err(db)?;
            }
        }
        for sql in [
            "DELETE FROM ent_frm_audience_critere WHERE etablissement_id = $1 AND formulaire_id = $2",
            "DELETE FROM ent_frm_auteur WHERE etablissement_id = $1 AND formulaire_id = $2",
            "DELETE FROM ent_frm_formulaire WHERE etablissement_id = $1 AND id = $2",
        ] {
            sqlx::query(sql).bind(etab).bind(id).execute(&mut *tx).await.map_err(db)?;
        }
        // Le journal garde la trace (sans identifiant de formulaire : il n'existe plus).
        frm_journaliser(&mut tx, etab, "supprime", None, None, user, None, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ----- Co-auteurs ------------------------------------------------------------------------------------------------

    /// Ajouter un co-auteur (FRM-139) : un membre du personnel qui a lui-même le droit de créer des formulaires. Le co-auteur
    /// modifie le brouillon ; il ne valide jamais son propre travail.
    pub async fn frm_ajouter_co_auteur(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        co_user: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        if f.auteur_user_id != user {
            return Err(introuvable());
        }
        let m: Option<(String,)> = sqlx::query_as(
            "SELECT role FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2",
        )
        .bind(etab)
        .bind(co_user)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((role_cible,)) = m else {
            return Err(invalide("ce compte n'est pas un membre de l'établissement"));
        };
        tx.commit().await.map_err(db)?;
        // Le droit du co-auteur se vérifie hors de la transaction (lecture seule).
        if self.frm_portee(etab, co_user, &role_cible).await.is_err() {
            return Err(invalide(
                "ce compte n'a pas le droit de créer des formulaires",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_frm_auteur (etablissement_id, formulaire_id, user_id, ajoute_par) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
        )
        .bind(etab)
        .bind(id)
        .bind(co_user)
        .bind(user)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        frm_journaliser(
            &mut tx,
            etab,
            "co_auteur",
            Some(id),
            None,
            user,
            None,
            Some("ajoute"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    pub async fn frm_retirer_co_auteur(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        id: Uuid,
        co_user: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = self.frm_pour_edition(&mut tx, etab, user, role, id).await?;
        if f.auteur_user_id != user {
            return Err(introuvable());
        }
        sqlx::query("DELETE FROM ent_frm_auteur WHERE etablissement_id = $1 AND formulaire_id = $2 AND user_id = $3")
            .bind(etab)
            .bind(id)
            .bind(co_user)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        frm_journaliser(
            &mut tx,
            etab,
            "co_auteur",
            Some(id),
            None,
            user,
            None,
            Some("retire"),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }
}

fn refus_interdit(msg: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Interdit, msg)
}

#[allow(clippy::too_many_arguments)]
fn question_modele(
    section: Uuid,
    t: &str,
    libelle: &str,
    obligatoire: bool,
    options: &[&str],
    min_nombre: Option<f64>,
    max_nombre: Option<f64>,
    objet: Option<&str>,
) -> NouvelleQuestion {
    NouvelleQuestion {
        section_id: section,
        type_question: t.to_string(),
        libelle: libelle.to_string(),
        aide: None,
        obligatoire,
        sensibilite: None,
        min_nombre,
        max_nombre,
        longueur_max: None,
        options: options.iter().map(|o| (*o).to_string()).collect(),
        lignes: vec![],
        objet_accord_code: objet.map(String::from),
    }
}

/// Une version se publie si elle est cohérente : au moins une section et une question à réponse, chaque section non vide, la
/// nature du formulaire va avec ses questions (« mixte » ⇔ au moins un accord), aucune boucle de conditions.
fn controler_publication(s: &StructureVue, nature: &str) -> Result<(), &'static str> {
    if s.sections.is_empty() {
        return Err("le formulaire est vide : ajoutez une section et des questions");
    }
    if s.sections.iter().any(|sec| sec.questions.is_empty()) {
        return Err("une section est vide : ajoutez-y une question ou retirez-la");
    }
    let questions: Vec<&QuestionVue> = s
        .sections
        .iter()
        .flat_map(|sec| sec.questions.iter())
        .collect();
    if !questions.iter().any(|q| type_repondable(&q.type_question)) {
        return Err("aucune question n'attend de réponse");
    }
    let accords = questions
        .iter()
        .filter(|q| q.type_question == "accord")
        .count();
    if nature == "mixte" && accords == 0 {
        return Err("un formulaire « mixte » contient au moins une question « accord »");
    }
    if nature != "mixte" && accords > 0 {
        return Err("une question « accord » demande un formulaire « mixte »");
    }
    let aretes: Vec<(Uuid, Uuid)> = questions
        .iter()
        .flat_map(|q| {
            q.conditions
                .iter()
                .map(move |c| (q.id, c.depend_de_question_id))
        })
        .collect();
    if a_une_boucle(&aretes) {
        return Err("les conditions d'affichage font une boucle");
    }
    Ok(())
}

/// Copie la structure d'une version vers une autre (identifiants neufs) : sections, questions, options, lignes puis conditions.
async fn copier_structure(
    conn: &mut PgConnection,
    etab: Uuid,
    formulaire: Uuid,
    source: Uuid,
    numero_source: i32,
    cible: Uuid,
) -> Result<(), EntDbError> {
    let s = charger_structure(&mut *conn, etab, formulaire, source, numero_source, true).await?;
    let mut q_map: HashMap<Uuid, Uuid> = HashMap::new();
    let mut o_map: HashMap<Uuid, Uuid> = HashMap::new();
    for sec in &s.sections {
        let nsec = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_frm_section (id, etablissement_id, version_id, ordre, titre, aide) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(nsec)
            .bind(etab)
            .bind(cible)
            .bind(sec.ordre)
            .bind(&sec.titre)
            .bind(&sec.aide)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        for q in &sec.questions {
            let nq = Uuid::new_v4();
            q_map.insert(q.id, nq);
            sqlx::query(
                "INSERT INTO ent_frm_question
                    (id, etablissement_id, version_id, section_id, ordre, type, libelle, aide, obligatoire, sensibilite, min_nombre,
                     max_nombre, longueur_max, objet_accord_id)
                 SELECT $1, $2, $3, $4, ordre, type, libelle, aide, obligatoire, sensibilite, min_nombre, max_nombre, longueur_max, objet_accord_id
                   FROM ent_frm_question WHERE etablissement_id = $2 AND id = $5",
            )
            .bind(nq)
            .bind(etab)
            .bind(cible)
            .bind(nsec)
            .bind(q.id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
            for o in &q.options {
                let no = Uuid::new_v4();
                o_map.insert(o.id, no);
                sqlx::query("INSERT INTO ent_frm_option (id, etablissement_id, question_id, ordre, libelle) VALUES ($1,$2,$3,$4,$5)")
                    .bind(no)
                    .bind(etab)
                    .bind(nq)
                    .bind(o.ordre)
                    .bind(&o.libelle)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
            }
            for l in &q.lignes {
                sqlx::query("INSERT INTO ent_frm_ligne (id, etablissement_id, question_id, ordre, libelle) VALUES ($1,$2,$3,$4,$5)")
                    .bind(Uuid::new_v4())
                    .bind(etab)
                    .bind(nq)
                    .bind(l.ordre)
                    .bind(&l.libelle)
                    .execute(&mut *conn)
                    .await
                    .map_err(db)?;
            }
        }
    }
    for sec in &s.sections {
        for q in &sec.questions {
            for c in &q.conditions {
                let (Some(nq), Some(nd), Some(no)) = (
                    q_map.get(&q.id),
                    q_map.get(&c.depend_de_question_id),
                    o_map.get(&c.option_id),
                ) else {
                    continue;
                };
                sqlx::query(
                    "INSERT INTO ent_frm_condition (id, etablissement_id, question_id, depend_de_question_id, option_id, operateur)
                     VALUES ($1,$2,$3,$4,$5,$6)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(nq)
                .bind(nd)
                .bind(no)
                .bind(&c.operateur)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            }
        }
    }
    Ok(())
}
