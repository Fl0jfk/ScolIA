//! Socle 18, lot FO1 — le port « a-t-on le droit ? » (`autorisations::verifier`, §5.2, §9).
//!
//! * **Lot** : une classe, une sortie, une image avec douze personnes ; une ligne par sujet.
//! * **Jamais `autorise` par défaut** : toute erreur, tout objet ou usage inconnu, tout sujet inconnu donne `inconnu` ;
//!   l'appelant refuse l'usage (R-FC3).
//! * **Raison codée sans détail** : le détail (qui a refusé, quand) n'est rendu qu'à `autorisations.gerer`.
//! * **Calculé, jamais stocké comme vérité** : le verdict se calcule à chaque appel depuis les décisions (R-FC4). Un retrait
//!   est donc effectif dès la validation de la transaction, sans délai de propagation ni projection à reconstruire.
//! * **Journal agrégé** : une ligne par appel (objet, usage, nombre de sujets), jamais une ligne par élève (R-FC38).

use crate::accords::*;
use crate::notifications::db;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{Datelike, NaiveDate, Utc};
use ent_auth::Action;
use sqlx::PgConnection;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Âge en années révolues au `jour`.
pub(crate) fn age_au(naissance: NaiveDate, jour: NaiveDate) -> i32 {
    let mut age = jour.year() - naissance.year();
    if (jour.month(), jour.day()) < (naissance.month(), naissance.day()) {
        age -= 1;
    }
    age
}

/// Droit du lien exigé par l'objet, exprimé sur `l` (`ent_eleve_lien_communicant`). Seul `autoriser_sorties` se distingue
/// aujourd'hui : les droits `signer_quotidien`, `signer_documents` et `repondre_formulaires` du socle 4 (lot A4) ne sont pas
/// encore des colonnes ; l'autorité parentale, toujours exigée, les couvre en attendant (limite signalée au lot).
pub(crate) fn condition_droit_lien(droit: Option<&str>) -> &'static str {
    match droit {
        Some("autoriser_sorties") => "l.peut_autoriser_sorties",
        _ => "true",
    }
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct ObjetLigne {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub nature: String,
    pub regle: String,
    pub avis_eleve_des_ans: Option<i16>,
    pub droit_lien_code: Option<String>,
    pub preuve_min: String,
    pub validite: String,
    pub valide_jusqu_au: Option<NaiveDate>,
    pub evenement_fin: Option<chrono::DateTime<Utc>>,
    pub sujet_type: String,
    pub actif: bool,
}

pub(crate) const COLONNES_OBJET: &str = "id, code, libelle, nature, regle, avis_eleve_des_ans, droit_lien_code, preuve_min, \
    validite, valide_jusqu_au, evenement_fin, sujet_type, actif";

pub(crate) async fn objet_par_code(
    conn: &mut PgConnection,
    etab: Uuid,
    code: &str,
) -> Result<Option<ObjetLigne>, EntDbError> {
    sqlx::query_as(&format!(
        "SELECT {COLONNES_OBJET} FROM ent_acc_objet WHERE etablissement_id = $1 AND code = $2"
    ))
    .bind(etab)
    .bind(code)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)
}

/// Version publiée en cours d'un objet : (id, numéro, texte, empreinte, relue).
pub(crate) async fn version_en_cours(
    conn: &mut PgConnection,
    etab: Uuid,
    objet_id: Uuid,
) -> Result<Option<(Uuid, i32, String, String, bool)>, EntDbError> {
    sqlx::query_as(
        "SELECT id, numero, texte, empreinte, texte_relu_le IS NOT NULL
           FROM ent_acc_version
          WHERE etablissement_id = $1 AND objet_id = $2 AND publiee_le IS NOT NULL
          ORDER BY numero DESC LIMIT 1",
    )
    .bind(etab)
    .bind(objet_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)
}

fn inconnu(sujets: &[Sujet], raison: &str) -> Vec<VerdictSujet> {
    sujets
        .iter()
        .map(|s| VerdictSujet { sujet: *s, verdict: Verdict::Inconnu, raison: Some(raison.to_string()) })
        .collect()
}

impl PgEnt {
    /// Port `verifier` : un verdict par sujet. **N'échoue jamais** : une erreur interne donne `inconnu` pour tous
    /// les sujets (l'appelant refuse l'usage). Appelé par les socles consommateurs (17, 42, 43, 14, 2, 33, 32, 4) après
    /// leur propre contrôle de droits ; pour un acteur humain, voir [`PgEnt::accords_verifier_pour_acteur`].
    pub async fn accords_verifier(&self, etab: Uuid, demande: &DemandeVerification) -> Vec<VerdictSujet> {
        let r = async {
            let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            self.verifier_interne(&mut conn, etab, demande).await
        }
        .await;
        let verdicts = match r {
            Ok(v) => v,
            Err(_) => return inconnu(&demande.sujets, "erreur_interne"),
        };
        // Journal agrégé (meilleur effort : une panne du journal ne change pas le verdict).
        let _ = self.journal_verification(etab, demande).await;
        verdicts
    }

    async fn journal_verification(&self, etab: Uuid, demande: &DemandeVerification) -> Result<(), EntDbError> {
        let mut detail = format!("{}.{}", demande.objet_code, demande.usage_code);
        detail.truncate(80);
        if !detail.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_.:-".contains(c)) {
            detail = "invalide".to_string();
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_acc_journal (id, etablissement_id, acte, nb, detail)
             VALUES ($1, $2, 'verification', $3, $4)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(demande.sujets.len() as i32)
        .bind(detail)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)
    }

    /// Verdicts pour un acteur humain : `autorisations.lire` dans sa portée (les élèves hors portée sont `inconnu`),
    /// **sans motif ni décideur** (R-FC39) ; la raison codée n'est rendue qu'avec `autorisations.gerer`.
    pub async fn accords_verifier_pour_acteur(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: &DemandeVerification,
    ) -> Result<Vec<VerdictSujet>, EntDbError> {
        let portee = self.eleves_autorises(etab, acteur, role, Action::AutorisationsLire).await?;
        let gerer = self.a_action_etablissement(etab, acteur, role, Action::AutorisationsGerer).await?;
        if portee.as_ref().is_some_and(|p| p.is_empty()) && !gerer {
            // Aucun droit de lecture dans une portée vide : refus net, indiscernable d'une autre absence de droit.
            return Err(EntDbError::Forbidden);
        }
        let mut sujets_autorises = Vec::new();
        let mut hors_portee = Vec::new();
        for s in &demande.sujets {
            match (s, &portee) {
                (Sujet::Eleve(e), Some(p)) if !p.contains(e) => hors_portee.push(*s),
                (Sujet::Personne(_), Some(_)) => hors_portee.push(*s),
                _ => sujets_autorises.push(*s),
            }
        }
        let interne = DemandeVerification { sujets: sujets_autorises, ..demande.clone() };
        let mut v = self.accords_verifier(etab, &interne).await;
        v.extend(inconnu(&hors_portee, "hors_portee"));
        if !gerer {
            for x in &mut v {
                x.raison = None;
            }
        }
        Ok(v)
    }

    /// Grille d'une classe pour un objet et un usage : peut / ne peut pas / en attente, élève par élève
    /// (`/web/classes/{id}/autorisations`). Verdicts seulement, jamais un motif ni un décideur.
    pub async fn accords_verdicts_classe(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
        usage_code: &str,
        classe_id: Uuid,
    ) -> Result<Vec<VerdictEleve>, EntDbError> {
        let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleves: Vec<Uuid> = crate::campagnes::eleves_de_audience(
            &mut conn,
            etab,
            "classe",
            &[classe_id],
            domain_eleve::horloge::jour_paris(),
        )
        .await?
        .into_iter()
        .collect();
        let noms: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, nom, prenom FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2) ORDER BY nom, prenom",
        )
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        drop(conn);
        let portee = self.eleves_autorises(etab, acteur, role, Action::AutorisationsLire).await?;
        let visibles: Vec<&(Uuid, String, String)> = noms
            .iter()
            .filter(|(id, _, _)| portee.as_ref().is_none_or(|p| p.contains(id)))
            .collect();
        if visibles.is_empty() && portee.as_ref().is_some_and(|p| p.is_empty()) {
            return Err(EntDbError::Forbidden);
        }
        let demande = DemandeVerification {
            objet_code: objet_code.to_string(),
            usage_code: usage_code.to_string(),
            sujets: visibles.iter().map(|(id, _, _)| Sujet::Eleve(*id)).collect(),
            jour: None,
        };
        let verdicts: HashMap<Sujet, Verdict> =
            self.accords_verifier(etab, &demande).await.into_iter().map(|v| (v.sujet, v.verdict)).collect();
        Ok(visibles
            .into_iter()
            .map(|(id, nom, prenom)| VerdictEleve {
                eleve_id: *id,
                nom: nom.clone(),
                prenom: prenom.clone(),
                verdict: verdicts.get(&Sujet::Eleve(*id)).copied().unwrap_or(Verdict::Inconnu),
            })
            .collect())
    }

    pub(crate) async fn verifier_interne(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        demande: &DemandeVerification,
    ) -> Result<Vec<VerdictSujet>, EntDbError> {
        let sujets = &demande.sujets;
        if sujets.is_empty() {
            return Ok(Vec::new());
        }
        let Some(objet) = objet_par_code(&mut *conn, etab, &demande.objet_code).await? else {
            return Ok(inconnu(sujets, "objet_inconnu"));
        };
        if !objet.actif {
            return Ok(inconnu(sujets, "objet_desactive"));
        }
        let usage: Option<(Uuid, bool)> = sqlx::query_as(
            "SELECT id, mission FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $2 AND code = $3",
        )
        .bind(etab)
        .bind(objet.id)
        .bind(&demande.usage_code)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some((usage_id, usage_mission)) = usage else {
            return Ok(inconnu(sujets, "usage_inconnu"));
        };
        let Some((version_id, _, _, _, _)) = version_en_cours(&mut *conn, etab, objet.id).await? else {
            return Ok(inconnu(sujets, "version_absente"));
        };
        let couvert: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_acc_version_usage WHERE etablissement_id = $1 AND version_id = $2 AND usage_id = $3",
        )
        .bind(etab)
        .bind(version_id)
        .bind(usage_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if couvert.is_none() {
            return Ok(inconnu(sujets, "usage_non_couvert"));
        }
        let jour = match demande.jour {
            Some(j) => j,
            None => self.aujourdhui_etablissement(etab).await?,
        };
        let maintenant = Utc::now();
        let usage_image = usage_d_image(&objet.code, &demande.usage_code);

        let eleves: Vec<Uuid> = sujets.iter().filter_map(|s| if let Sujet::Eleve(e) = s { Some(*e) } else { None }).collect();
        let personnes: Vec<Uuid> = sujets.iter().filter_map(|s| if let Sujet::Personne(p) = s { Some(*p) } else { None }).collect();
        let tous_ids: Vec<Uuid> = eleves.iter().chain(personnes.iter()).copied().collect();

        // Sujets connus de l'établissement (un identifiant d'un autre établissement est inconnu : cloisonnement).
        let naissances: HashMap<Uuid, Option<NaiveDate>> = sqlx::query_as::<_, (Uuid, Option<NaiveDate>)>(
            "SELECT id, date_naissance FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let personnes_connues: HashMap<Uuid, Option<Uuid>> = sqlx::query_as::<_, (Uuid, Option<Uuid>)>(
            "SELECT id, user_id FROM ent_personne WHERE etablissement_id = $1 AND id = ANY($2)",
        )
        .bind(etab)
        .bind(&personnes)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .into_iter()
        .collect();

        let seuls: HashSet<Uuid> = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2) AND ent_eleve_decide_seul($1, id)",
        )
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(i,)| i)
        .collect();
        let proteges: HashSet<Uuid> = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2) AND ent_eleve_confidentiel($1, id, $3)",
        )
        .bind(etab)
        .bind(&eleves)
        .bind(jour)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(i,)| i)
        .collect();

        // Titulaires qui décident (lien communicant, autorité parentale, droit du lien).
        let cond = condition_droit_lien(objet.droit_lien_code.as_deref());
        let liens: Vec<(Uuid, Uuid)> = sqlx::query_as(&format!(
            "SELECT DISTINCT l.eleve_id, l.user_id FROM ent_eleve_lien_communicant l
              WHERE l.etablissement_id = $1 AND l.eleve_id = ANY($2) AND l.autorite_parentale AND {cond}"
        ))
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut attendus: HashMap<Uuid, usize> = HashMap::new();
        let attendus_rows: Vec<(Uuid, i64)> = sqlx::query_as(&format!(
            "SELECT l.eleve_id, count(DISTINCT l.personne_id) FROM ent_eleve_lien_communicant l
              WHERE l.etablissement_id = $1 AND l.eleve_id = ANY($2) AND l.autorite_parentale AND {cond}
              GROUP BY l.eleve_id"
        ))
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        for (e, n) in attendus_rows {
            attendus.insert(e, n as usize);
        }
        let titulaires: HashSet<(Uuid, Uuid)> = liens.into_iter().filter(|(_, u)| !u.is_nil()).collect();
        let comptes: HashSet<(Uuid, Uuid)> = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT eleve_id, user_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = ANY($2)",
        )
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .into_iter()
        .collect();

        // Dernière décision de chaque décideur pour l'usage (ou refus global hérité de l'ancien module, qui n'avait
        // pas d'usages et protège tous les usages), hors décisions annulées par l'établissement.
        #[derive(sqlx::FromRow)]
        struct Ligne {
            sujet: Uuid,
            decideur_user_id: Uuid,
            qualite: String,
            accorde: bool,
            canal: String,
            decide_le: chrono::DateTime<Utc>,
        }
        let lignes: Vec<Ligne> = sqlx::query_as(
            "SELECT DISTINCT ON (COALESCE(x.sujet_eleve_id, x.sujet_personne_id), x.decideur_user_id)
                    COALESCE(x.sujet_eleve_id, x.sujet_personne_id) AS sujet, x.decideur_user_id, x.qualite,
                    u.accorde, x.canal, x.decide_le
               FROM ent_acc_decision x
               JOIN ent_acc_decision_usage u ON u.decision_id = x.id AND u.etablissement_id = x.etablissement_id
               JOIN ent_acc_usage us ON us.id = u.usage_id AND us.etablissement_id = u.etablissement_id
              WHERE x.etablissement_id = $1 AND x.objet_id = $2
                AND COALESCE(x.sujet_eleve_id, x.sujet_personne_id) = ANY($3)
                AND x.decision <> 'annulee_par_etablissement'
                AND NOT EXISTS (SELECT 1 FROM ent_acc_decision a
                                 WHERE a.etablissement_id = x.etablissement_id AND a.annule_decision_id = x.id)
                AND (u.usage_id = $4 OR (us.code = 'reprise_ancien_module' AND u.accorde = false))
              ORDER BY COALESCE(x.sujet_eleve_id, x.sujet_personne_id), x.decideur_user_id, x.decide_le DESC, x.id DESC",
        )
        .bind(etab)
        .bind(objet.id)
        .bind(&tous_ids)
        .bind(usage_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;

        // Usage ajouté depuis : le décideur avait déjà décidé sur l'objet **avant** la première version qui couvre cet usage
        // (CK-30). Un usage proposé dès la première décision mais laissé décoché n'est pas « à renouveler ».
        let (premiere_couverture,): (Option<i32>,) = sqlx::query_as(
            "SELECT min(v.numero) FROM ent_acc_version_usage vu
               JOIN ent_acc_version v ON v.etablissement_id = vu.etablissement_id AND v.id = vu.version_id
              WHERE vu.etablissement_id = $1 AND vu.usage_id = $2 AND v.publiee_le IS NOT NULL",
        )
        .bind(etab)
        .bind(usage_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        let ont_decide: HashSet<Uuid> = sqlx::query_as::<_, (Uuid,)>(
            "SELECT DISTINCT COALESCE(x.sujet_eleve_id, x.sujet_personne_id) FROM ent_acc_decision x
               JOIN ent_acc_version v ON v.etablissement_id = x.etablissement_id AND v.id = x.version_id
              WHERE x.etablissement_id = $1 AND x.objet_id = $2
                AND COALESCE(x.sujet_eleve_id, x.sujet_personne_id) = ANY($3)
                AND x.decision <> 'annulee_par_etablissement' AND v.numero < $4",
        )
        .bind(etab)
        .bind(objet.id)
        .bind(&tous_ids)
        .bind(premiere_couverture.unwrap_or(0))
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(i,)| i)
        .collect();
        let ont_decide_usage: HashSet<Uuid> = sqlx::query_as::<_, (Uuid,)>(
            "SELECT DISTINCT COALESCE(x.sujet_eleve_id, x.sujet_personne_id)
               FROM ent_acc_decision x
               JOIN ent_acc_decision_usage u ON u.decision_id = x.id AND u.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.objet_id = $2 AND u.usage_id = $4
                AND COALESCE(x.sujet_eleve_id, x.sujet_personne_id) = ANY($3)",
        )
        .bind(etab)
        .bind(objet.id)
        .bind(&tous_ids)
        .bind(usage_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(i,)| i)
        .collect();

        let mut par_sujet: HashMap<Uuid, Vec<&Ligne>> = HashMap::new();
        for l in &lignes {
            par_sujet.entry(l.sujet).or_default().push(l);
        }
        let mut sortie = Vec::with_capacity(sujets.len());
        for s in sujets {
            let (id, age, personne_user) = match s {
                Sujet::Eleve(e) => match naissances.get(e) {
                    Some(n) => (*e, n.map(|n| age_au(n, jour)), None),
                    None => {
                        sortie.push(VerdictSujet { sujet: *s, verdict: Verdict::Inconnu, raison: Some("sujet_inconnu".into()) });
                        continue;
                    }
                },
                Sujet::Personne(p) => match personnes_connues.get(p) {
                    Some(u) => (*p, None, Some(*u)),
                    None => {
                        sortie.push(VerdictSujet { sujet: *s, verdict: Verdict::Inconnu, raison: Some("sujet_inconnu".into()) });
                        continue;
                    }
                },
            };
            // Un objet d'élève ne se vérifie pas pour une personne, et réciproquement.
            if (objet.sujet_type == "eleve") != matches!(s, Sujet::Eleve(_)) {
                sortie.push(VerdictSujet { sujet: *s, verdict: Verdict::Inconnu, raison: Some("sujet_incompatible".into()) });
                continue;
            }
            let mut decisions: Vec<DecisionLue<'_>> = Vec::new();
            for l in par_sujet.get(&id).map(|v| v.as_slice()).unwrap_or(&[]) {
                // Le décideur doit encore être qualifié à la date : lien communicant avec autorité et droit, ou compte
                // de l'élève, ou la personne elle-même (interdiction de communication, fin de lien, retrait du droit :
                // sa décision ne compte plus).
                let qualifie = match (l.qualite.as_str(), s) {
                    ("titulaire" | "tuteur" | "service_gardien" | "delegataire_image" | "curateur_assiste", Sujet::Eleve(e)) => {
                        titulaires.contains(&(*e, l.decideur_user_id))
                    }
                    ("eleve_majeur" | "eleve_avis" | "eleve_cosignataire", Sujet::Eleve(e)) => {
                        comptes.contains(&(*e, l.decideur_user_id))
                    }
                    // Saisie au guichet ou sur papier : l'agent a recueilli la décision de la personne (canal tracé).
                    ("personne_soi", Sujet::Personne(_)) => {
                        personne_user == Some(Some(l.decideur_user_id)) || matches!(l.canal.as_str(), "guichet" | "papier")
                    }
                    _ => false,
                };
                if !qualifie {
                    continue;
                }
                let expiree = l.accorde
                    && decision_expiree(
                        &objet.validite,
                        domain_eleve::horloge::date_paris(l.decide_le),
                        jour,
                        objet.valide_jusqu_au,
                        objet.evenement_fin,
                        maintenant,
                    );
                decisions.push(DecisionLue {
                    qualite: l.qualite.as_str(),
                    accorde: l.accorde,
                    expiree,
                    sans_preuve: l.canal == "reprise_sans_preuve",
                });
            }
            let entree = EntreeVerdict {
                nature: &objet.nature,
                regle: &objet.regle,
                avis_des_ans: objet.avis_eleve_des_ans,
                age_eleve: age,
                usage_mission,
                usage_image,
                protege: proteges.contains(&id),
                decide_seul: seuls.contains(&id),
                titulaires_attendus: attendus.get(&id).copied().unwrap_or(0),
                a_renouveler: ont_decide.contains(&id) && !ont_decide_usage.contains(&id),
                decisions,
            };
            sortie.push(VerdictSujet { sujet: *s, verdict: calculer_verdict(&entree), raison: None });
        }
        Ok(sortie)
    }
}
