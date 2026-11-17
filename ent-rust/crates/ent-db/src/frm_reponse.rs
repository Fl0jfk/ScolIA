//! Socle 18, lot FO3 — répondre à un formulaire, **questions « accord » comprises** : les décisions d'accord et la soumission
//! s'écrivent dans la même transaction (`docs/socles/18_FORMULAIRES.md` §6.2, §11.3, §11.4 ; règles R-FC2, R-FC16).
//!
//! * Le répondant est **calculé** (titulaire communicant ayant l'autorité parentale, élève selon l'unité et la capacité), jamais
//!   choisi par l'appelant ; un parent sous interdiction de communication ne voit ni ne répond (lien communicant).
//! * Une question « accord » passe par le **même calcul** que `accords_decider` : décideur, qualité, version, usages, règle
//!   `un` / `tous`, avis de l'enfant, désaccord. Un accord refusé ou retiré refuse, jamais ne valide ; une erreur annule tout.
//! * Seule **sa** décision est rendue : jamais celle de l'autre titulaire (FD-03).
//! * Unités : `eleve_un_titulaire` (la première réponse vaut, l'autre titulaire est prévenu sans contenu), `eleve_chaque_titulaire`
//!   (une par titulaire), `personne` et `foyer` (une par répondant, tous enfants confondus), `eleve_lui_meme`.

use crate::accords::{ChoixUsage, DecisionSaisie, MaDecision};
use crate::accords_service::Preuve;
use crate::accords_verifier::objet_par_code;
use crate::frm_moteur::*;
use crate::frm_service::{
    charger, charger_structure, derniere_publiee, evenements, frm_publier_evt, introuvable,
};
use crate::notifications::db;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct AccordSaisie {
    pub question_id: Uuid,
    pub usages: Vec<ChoixUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReponseSaisie {
    /// Élève concerné (une cible du formulaire).
    pub eleve_id: Uuid,
    #[serde(default)]
    pub valeurs: Vec<ValeurSaisie>,
    #[serde(default)]
    pub accords: Vec<AccordSaisie>,
}

/// Ce que le répondant reçoit : l'accusé de **sa** réponse et **ses** décisions.
#[derive(Debug, Clone, Serialize)]
pub struct RecuReponse {
    pub soumission_id: Uuid,
    pub version_numero: i32,
    pub remplace_une_reponse: bool,
    pub decisions: Vec<MaDecision>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ARepondre {
    pub formulaire_id: Uuid,
    pub titre: String,
    pub eleve_id: Uuid,
    pub echeance: Option<DateTime<Utc>>,
    pub deja_repondu: bool,
}

fn conflit(m: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(m)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

/// Qualité sous laquelle `user` répond pour `eleve` à un formulaire d'une unité donnée, ou `None` (indiscernable d'un inconnu).
async fn qualite_repondant(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    user: Uuid,
    eleve: Uuid,
    unite: &str,
) -> Result<Option<&'static str>, EntDbError> {
    let (decide_seul,): (bool,) = sqlx::query_as("SELECT ent_eleve_decide_seul($1, $2)")
        .bind(etab)
        .bind(eleve)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let compte: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3")
        .bind(etab)
        .bind(eleve)
        .bind(user)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
    if compte.is_some() {
        return Ok(if unite == "eleve_lui_meme" {
            Some("eleve")
        } else if decide_seul {
            Some("eleve_majeur")
        } else {
            None
        });
    }
    if unite == "eleve_lui_meme" || decide_seul {
        // Un élève qui décide seul : ses parents ne répondent plus (R-FC12) ; un sondage « lui-même » n'est pas pour les parents.
        return Ok(None);
    }
    let tit: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM ent_eleve_lien_communicant
          WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3 AND autorite_parentale LIMIT 1",
    )
    .bind(etab)
    .bind(eleve)
    .bind(user)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(tit.map(|_| "titulaire"))
}

impl PgEnt {
    /// Formulaires à remplir par moi : ceux dont l'un de mes enfants (ou moi-même, élève) est cible, ouverts, pour lesquels je suis
    /// le répondant calculé. Aucun contenu de la réponse d'un autre.
    pub async fn frm_a_repondre(
        &self,
        etab: Uuid,
        user: Uuid,
    ) -> Result<Vec<ARepondre>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let l: Vec<(Uuid, String, Uuid, Option<DateTime<Utc>>, bool)> = sqlx::query_as(
            "SELECT f.id, f.titre, c.eleve_id, f.echeance,
                    EXISTS (SELECT 1 FROM ent_frm_soumission s
                             WHERE s.etablissement_id = f.etablissement_id AND s.formulaire_id = f.id AND s.eleve_id = c.eleve_id
                               AND s.repondant_user_id = $2 AND s.statut = 'envoyee')
               FROM ent_frm_formulaire f
               JOIN ent_frm_cible c ON c.etablissement_id = f.etablissement_id AND c.formulaire_id = f.id AND c.retire_le IS NULL
              WHERE f.etablissement_id = $1 AND f.statut = 'publie' AND (f.ouverture IS NULL OR f.ouverture <= now())
                AND (f.echeance IS NULL OR f.echeance > now())
                AND (
                    (f.unite <> 'eleve_lui_meme' AND NOT ent_eleve_decide_seul(f.etablissement_id, c.eleve_id)
                     AND EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                                  WHERE l.etablissement_id = f.etablissement_id AND l.eleve_id = c.eleve_id
                                    AND l.user_id = $2 AND l.autorite_parentale))
                    OR (EXISTS (SELECT 1 FROM ent_eleve_compte k
                                 WHERE k.etablissement_id = f.etablissement_id AND k.eleve_id = c.eleve_id AND k.user_id = $2)
                        AND (f.unite = 'eleve_lui_meme' OR ent_eleve_decide_seul(f.etablissement_id, c.eleve_id))))
              ORDER BY f.echeance NULLS LAST, f.titre LIMIT 200",
        )
        .bind(etab)
        .bind(user)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(l.into_iter()
            .map(
                |(formulaire_id, titre, eleve_id, echeance, deja_repondu)| ARepondre {
                    formulaire_id,
                    titre,
                    eleve_id,
                    echeance,
                    deja_repondu,
                },
            )
            .collect())
    }

    /// Le formulaire tel que le voit le répondant (dernière version publiée) : questions, options, conditions. Introuvable si
    /// l'élève n'est pas cible ou si le répondant n'est pas celui du calcul.
    pub async fn frm_formulaire_pour_repondre(
        &self,
        etab: Uuid,
        user: Uuid,
        formulaire_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<StructureVue, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = charger(&mut tx, etab, formulaire_id).await?;
        let cible: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_frm_cible WHERE etablissement_id = $1 AND formulaire_id = $2 AND eleve_id = $3 AND retire_le IS NULL",
        )
        .bind(etab)
        .bind(formulaire_id)
        .bind(eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if cible.is_none() || !matches!(f.statut.as_str(), "publie" | "clos") {
            return Err(introuvable());
        }
        qualite_repondant(&mut tx, etab, user, eleve_id, &f.unite)
            .await?
            .ok_or_else(introuvable)?;
        let (v, n) = derniere_publiee(&mut tx, etab, formulaire_id)
            .await?
            .ok_or_else(introuvable)?;
        charger_structure(&mut tx, etab, formulaire_id, v, n, true).await
    }

    /// Répondre (ou corriger sa réponse tant que le formulaire est ouvert). Tout ou rien : valeurs, soumission et décisions
    /// d'accord s'écrivent ensemble.
    pub async fn frm_repondre(
        &self,
        etab: Uuid,
        user: Uuid,
        formulaire_id: Uuid,
        r: ReponseSaisie,
    ) -> Result<RecuReponse, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let f = charger(&mut tx, etab, formulaire_id).await?;
        let cible: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_frm_cible WHERE etablissement_id = $1 AND formulaire_id = $2 AND eleve_id = $3 AND retire_le IS NULL",
        )
        .bind(etab)
        .bind(formulaire_id)
        .bind(r.eleve_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if cible.is_none() || !matches!(f.statut.as_str(), "publie" | "clos") {
            return Err(introuvable());
        }
        let qualite = qualite_repondant(&mut tx, etab, user, r.eleve_id, &f.unite)
            .await?
            .ok_or_else(introuvable)?;
        // Ouvert ? (le formulaire clos est lisible, pas modifiable)
        let maintenant = Utc::now();
        if f.statut != "publie" || f.ouverture.is_some_and(|o| o > maintenant) {
            return Err(conflit("ce formulaire n'est pas ouvert aux réponses"));
        }
        if f.echeance.is_some_and(|e| e <= maintenant) {
            return Err(conflit(
                "l'échéance de ce formulaire est passée : adressez-vous à l'établissement",
            ));
        }
        let (version_id, numero) = derniere_publiee(&mut tx, etab, formulaire_id)
            .await?
            .ok_or_else(introuvable)?;
        // Les réponses d'un même répondant se sérialisent (double envoi, deux onglets).
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 19))")
            .bind(format!("{etab}:{formulaire_id}:{user}"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        // Unités : qui a déjà répondu ?
        let ancienne: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_frm_soumission
              WHERE etablissement_id = $1 AND formulaire_id = $2 AND eleve_id = $3 AND repondant_user_id = $4 AND statut = 'envoyee'",
        )
        .bind(etab)
        .bind(formulaire_id)
        .bind(r.eleve_id)
        .bind(user)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        match f.unite.as_str() {
            "eleve_un_titulaire" => {
                let autre: Option<(i32,)> = sqlx::query_as(
                    "SELECT 1 FROM ent_frm_soumission
                      WHERE etablissement_id = $1 AND formulaire_id = $2 AND eleve_id = $3 AND statut = 'envoyee' AND repondant_user_id <> $4",
                )
                .bind(etab)
                .bind(formulaire_id)
                .bind(r.eleve_id)
                .bind(user)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                if autre.is_some() {
                    return Err(conflit("un responsable a déjà répondu pour cet élève : demandez à l'établissement si une correction est nécessaire"));
                }
            }
            "personne" | "foyer" => {
                let ailleurs: Option<(i32,)> = sqlx::query_as(
                    "SELECT 1 FROM ent_frm_soumission
                      WHERE etablissement_id = $1 AND formulaire_id = $2 AND repondant_user_id = $3 AND statut = 'envoyee' AND eleve_id <> $4",
                )
                .bind(etab)
                .bind(formulaire_id)
                .bind(user)
                .bind(r.eleve_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                if ailleurs.is_some() {
                    return Err(conflit("vous avez déjà répondu à ce formulaire"));
                }
            }
            _ => {}
        }
        // Structure et réponses.
        let s = charger_structure(&mut tx, etab, formulaire_id, version_id, numero, true).await?;
        let questions: HashMap<Uuid, &QuestionVue> = s
            .sections
            .iter()
            .flat_map(|sec| sec.questions.iter())
            .map(|q| (q.id, q))
            .collect();
        let mut par_question: HashMap<Uuid, &ValeurSaisie> = HashMap::new();
        for v in &r.valeurs {
            if par_question.insert(v.question_id, v).is_some() {
                return Err(invalide("une seule réponse par question"));
            }
            let q = questions.get(&v.question_id).ok_or_else(introuvable)?;
            if q.type_question == "accord" {
                return Err(invalide(
                    "une question « accord » se renseigne dans les accords",
                ));
            }
        }
        let mut accords: HashMap<Uuid, &AccordSaisie> = HashMap::new();
        for a in &r.accords {
            let q = questions.get(&a.question_id).ok_or_else(introuvable)?;
            if q.type_question != "accord" || accords.insert(a.question_id, a).is_some() {
                return Err(invalide("accord inconnu ou en double"));
            }
        }
        let mut choix: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
        for v in &r.valeurs {
            choix.insert(v.question_id, v.option_ids.iter().copied().collect());
        }
        let visibles = questions_visibles(&s, &choix);
        let mut lignes: Vec<(Uuid, LigneValeur)> = Vec::new();
        for sec in &s.sections {
            for q in &sec.questions {
                if !type_repondable(&q.type_question) {
                    continue;
                }
                let visible = visibles.contains(&q.id);
                let donnee = par_question.get(&q.id).copied();
                let accord = accords.get(&q.id).copied();
                if !visible {
                    if donnee.is_some() || accord.is_some() {
                        return Err(invalide(
                            "une réponse porte sur une question qui n'est pas affichée",
                        ));
                    }
                    continue;
                }
                if q.type_question == "accord" {
                    if accord.is_none() && q.obligatoire {
                        return Err(invalide("une question obligatoire n'a pas de réponse"));
                    }
                    continue;
                }
                match donnee {
                    None if q.obligatoire => {
                        return Err(invalide("une question obligatoire n'a pas de réponse"))
                    }
                    None => {}
                    Some(v) => {
                        for l in controler_valeur(q, v).map_err(invalide)? {
                            lignes.push((q.id, l));
                        }
                    }
                }
            }
        }
        // Soumission : l'ancienne réponse du même répondant est « remplacée » (elle garde sa date).
        if let Some((ancien,)) = ancienne {
            sqlx::query("UPDATE ent_frm_soumission SET statut = 'remplacee' WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(ancien)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let soumission = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_frm_soumission (id, etablissement_id, formulaire_id, version_id, eleve_id, repondant_user_id, qualite, remplace_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(soumission)
        .bind(etab)
        .bind(formulaire_id)
        .bind(version_id)
        .bind(r.eleve_id)
        .bind(user)
        .bind(qualite)
        .bind(ancienne.map(|(a,)| a))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (question, l) in &lignes {
            sqlx::query(
                "INSERT INTO ent_frm_valeur
                    (id, etablissement_id, soumission_id, version_id, question_id, option_id, ligne_id, rang, valeur_texte,
                     valeur_nombre, valeur_date, valeur_heure)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10::float8::numeric,$11,$12)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(soumission)
            .bind(version_id)
            .bind(question)
            .bind(l.option_id)
            .bind(l.ligne_id)
            .bind(l.rang)
            .bind(&l.texte)
            .bind(l.nombre)
            .bind(l.date)
            .bind(l.heure)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        // Décisions d'accord : dans la même transaction, par le même calcul que `accords_decider`.
        let mut decisions = Vec::new();
        for sec in &s.sections {
            for q in sec.questions.iter().filter(|q| q.type_question == "accord") {
                let Some(a) = accords.get(&q.id) else {
                    continue;
                };
                let code = q.objet_accord_code.as_deref().ok_or_else(introuvable)?;
                let o = objet_par_code(&mut tx, etab, code)
                    .await?
                    .ok_or_else(introuvable)?;
                let d = DecisionSaisie {
                    objet_code: code.to_string(),
                    eleve_id: if o.sujet_type == "personne" {
                        None
                    } else {
                        Some(r.eleve_id)
                    },
                    usages: a.usages.clone(),
                };
                let (t, m) = self
                    .decider_dans_tx(
                        tx,
                        etab,
                        user,
                        d,
                        Preuve::Port(&crate::accords::SignatureAbsente),
                        false,
                        Some(soumission),
                    )
                    .await?;
                tx = t;
                decisions.push(m);
            }
        }
        sqlx::query(
            "INSERT INTO ent_frm_journal (id, etablissement_id, acte, formulaire_id, version_id, acteur_user_id, nb)
             VALUES ($1,$2,'reponse',$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(formulaire_id)
        .bind(version_id)
        .bind(user)
        .bind((lignes.len() + decisions.len()) as i32)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        frm_publier_evt(
            &mut tx,
            etab,
            evenements::REPONSE,
            formulaire_id,
            Some(r.eleve_id),
            user,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(RecuReponse {
            soumission_id: soumission,
            version_numero: numero,
            remplace_une_reponse: ancienne.is_some(),
            decisions,
        })
    }
}
