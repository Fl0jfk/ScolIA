//! Cahier de textes, lot C2 : travaux (nature, durée, échéance-séance, public restreint, plusieurs cours,
//! modification avec ou sans avis, retrait). Voir `cahier_c2.rs` pour la projection transitoire et la plage calme.
//!
//! Jour et heure de l'établissement : l'échéance est saisie en jour et heure **locaux** et convertie en instant
//! par `ent_cdt_instant_local` ; l'heure UTC n'est jamais saisie à la main. Une échéance **avancée** d'un travail
//! visible demande une confirmation explicite et prévient toujours (§9.1, R-C24).

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use ent_auth::Action;
use ent_events::types;
use ent_types::cahier_c2::*;
use serde_json::json;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::cahier::publier_cdt;
use crate::cahier_c2::{
    conflit, instant_programme, invalide, texte_borne, SeanceInfo, SELECT_SEANCE_INFO,
};
use crate::ref_commun::{db, introuvable, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Heure par défaut d'une échéance saisie par un jour seul : avant la plage calme.
const HEURE_ECHEANCE_DEFAUT: NaiveTime = match NaiveTime::from_hms_opt(19, 0, 0) {
    Some(h) => h,
    None => panic!("heure valide"),
};

#[derive(Debug, FromRow)]
struct TravailRow {
    id: Uuid,
    cours_id: Uuid,
    relie_id: Option<Uuid>,
    seance_donnee_id: Option<Uuid>,
    donne_le: NaiveDate,
    echeance_mode: String,
    echeance_seance_id: Option<Uuid>,
    echeance_le: DateTime<Utc>,
    nature_code: String,
    nature_id: Uuid,
    titre: String,
    consigne: String,
    duree_estimee_min: Option<i16>,
    rendu: String,
    retard_accepte: bool,
    public: String,
    ia_autorisee: String,
    etat: String,
    publie_a: Option<DateTime<Utc>>,
    visible_a: Option<DateTime<Utc>>,
    retire_motif: Option<String>,
    auteur_user_id: Option<Uuid>,
    version: i32,
    ancien_id: Option<Uuid>,
    modifie_le: DateTime<Utc>,
    auteur: String,
    echeance_jour: NaiveDate,
    echeance_heure: String,
    nb_cibles: i64,
}

const SELECT_TRAVAIL: &str = "SELECT t.id, t.cours_id, t.relie_id, t.seance_donnee_id, t.donne_le, t.echeance_mode,
       t.echeance_seance_id, t.echeance_le, n.code AS nature_code, t.nature_id, t.titre, t.consigne,
       t.duree_estimee_min, t.rendu, t.retard_accepte, t.public, t.ia_autorisee, t.etat, t.publie_a, t.visible_a,
       t.retire_motif, t.auteur_user_id, t.version, t.ancien_id, t.modifie_le,
       ent_cdt_nom_affichage(t.etablissement_id, t.auteur_user_id) AS auteur,
       ent_aujourdhui_etab(t.etablissement_id, t.echeance_le) AS echeance_jour,
       to_char(ent_cdt_heure_locale(t.etablissement_id, t.echeance_le), 'HH24:MI') AS echeance_heure,
       (SELECT count(*) FROM ent_cdt_travail_cible c
         WHERE c.etablissement_id = t.etablissement_id AND c.travail_id = t.id) AS nb_cibles
  FROM ent_cdt_travail t
  JOIN ent_cdt_nature n ON n.etablissement_id = t.etablissement_id AND n.id = t.nature_id
 WHERE t.etablissement_id = $1";

async fn formats_de(
    tx: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
) -> Result<Vec<String>, EntDbError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT format_code FROM ent_cdt_travail_format WHERE etablissement_id = $1 AND travail_id = $2 ORDER BY format_code",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|(f,)| f).collect())
}

async fn travail_dto(
    tx: &mut PgConnection,
    etab: Uuid,
    t: TravailRow,
) -> Result<TravailC2Dto, EntDbError> {
    let formats = formats_de(tx, etab, t.id).await?;
    let differee = matches!(t.etat.as_str(), "publie" | "programme")
        && matches!((t.visible_a, t.publie_a), (Some(v), Some(p)) if v > p);
    Ok(TravailC2Dto {
        id: t.id,
        cours_id: t.cours_id,
        relie_id: t.relie_id,
        seance_donnee_id: t.seance_donnee_id,
        donne_le: t.donne_le,
        echeance_mode: t.echeance_mode,
        echeance_seance_id: t.echeance_seance_id,
        echeance_le: t.echeance_le,
        echeance_jour: t.echeance_jour,
        echeance_heure: t.echeance_heure,
        nature: t.nature_code,
        titre: t.titre,
        consigne: t.consigne,
        duree_estimee_min: t.duree_estimee_min,
        rendu: t.rendu,
        retard_accepte: t.retard_accepte,
        formats,
        public: t.public,
        nb_eleves_cibles: t.nb_cibles,
        ia_autorisee: t.ia_autorisee,
        etat: t.etat,
        publie_a: t.publie_a,
        visible_a: t.visible_a,
        differee_par_plage_calme: differee,
        auteur: t.auteur,
        version: t.version,
        modifie_le: t.modifie_le,
        retire_motif: t.retire_motif,
    })
}

/// Projette le travail dans `ent_travail` (anciens lecteurs : agenda, remises, charge). Un travail à public restreint
/// n'est **jamais** projeté (les anciens lecteurs l'afficheraient à toute la classe) : il reste lisible du client C2.
async fn miroir_travail(tx: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<bool, EntDbError> {
    let t: TravailRow = sqlx::query_as(&format!("{SELECT_TRAVAIL} AND t.id = $2"))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
    if t.public != "cours" {
        return Ok(false);
    }
    let (classe_id, groupe_id, libelle): (Option<Uuid>, Option<Uuid>, String) =
        sqlx::query_as("SELECT classe_id, groupe_id, libelle FROM ent_cours WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(t.cours_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
    let formats = formats_de(tx, etab, id).await?;
    let mid = t.ancien_id.unwrap_or(t.id);
    let en_ligne = t.rendu == "en_ligne";
    let date_limite = if en_ligne { Some(t.echeance_le) } else { None };
    let type_ancien = if en_ligne { "a_rendre" } else { "a_faire" };
    let donne_le = t.donne_le.min(t.echeance_jour);
    let retire = t.etat == "retire";
    let auteur = t.auteur_user_id.unwrap_or_default();
    let maj = sqlx::query(
        "UPDATE ent_travail SET titre = $3, consigne = $4, type = $5, pour_le = $6, donne_le = $7,
                duree_estimee_min = $8, rendu_en_ligne = $9, date_limite = $10, acceptation_retard = $11,
                formats_acceptes = $12, version = $13, updated_at = now(),
                supprime_le = CASE WHEN $14 THEN COALESCE(supprime_le, now()) ELSE NULL END,
                retire_par_user_id = CASE WHEN $14 THEN retire_par_user_id ELSE NULL END,
                retire_motif = CASE WHEN $14 THEN retire_motif ELSE NULL END
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(mid)
    .bind(&t.titre)
    .bind(&t.consigne)
    .bind(type_ancien)
    .bind(t.echeance_jour)
    .bind(donne_le)
    .bind(t.duree_estimee_min.map(i32::from))
    .bind(en_ligne)
    .bind(date_limite)
    .bind(t.retard_accepte)
    .bind(&formats)
    .bind(t.version)
    .bind(retire)
    .execute(&mut *tx)
    .await
    .map_err(db)?
    .rows_affected();
    if maj == 0 {
        sqlx::query(
            "INSERT INTO ent_travail (id, etablissement_id, seance_id, classe_id, groupe_id, matiere_libelle, auteur_user_id,
                    type, titre, consigne, donne_le, pour_le, duree_estimee_min, rendu_en_ligne, date_limite,
                    acceptation_retard, formats_acceptes, version)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
        )
        .bind(mid)
        .bind(etab)
        .bind(t.seance_donnee_id)
        .bind(classe_id)
        .bind(groupe_id)
        .bind(&libelle)
        .bind(auteur)
        .bind(type_ancien)
        .bind(&t.titre)
        .bind(&t.consigne)
        .bind(donne_le)
        .bind(t.echeance_jour)
        .bind(t.duree_estimee_min.map(i32::from))
        .bind(en_ligne)
        .bind(date_limite)
        .bind(t.retard_accepte)
        .bind(&formats)
        .bind(t.version)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    }
    if retire {
        sqlx::query(
            "UPDATE ent_travail o SET retire_par_user_id = n.retire_par_user_id, retire_motif = n.retire_motif
               FROM ent_cdt_travail n WHERE o.etablissement_id = $1 AND o.id = $2 AND n.etablissement_id = $1 AND n.id = $3",
        )
        .bind(etab)
        .bind(mid)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    }
    if t.ancien_id.is_none() {
        sqlx::query("UPDATE ent_cdt_travail SET ancien_id = id WHERE etablissement_id = $1 AND id = $2 AND ancien_id IS NULL")
            .bind(etab)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
    }
    Ok(true)
}

/// Relit l'ancienne ligne si une ancienne route l'a modifiée depuis la dernière écriture de C2.
async fn rafraichir_travail(
    tx: &mut PgConnection,
    etab: Uuid,
    t: &TravailRow,
) -> Result<(), EntDbError> {
    let Some(mid) = t.ancien_id else {
        return Ok(());
    };
    let ancienne: Option<(String, String, Option<i32>, bool, NaiveDate, bool, DateTime<Utc>, bool)> = sqlx::query_as(
        "SELECT titre, consigne, duree_estimee_min, rendu_en_ligne, pour_le, acceptation_retard, updated_at,
                supprime_le IS NOT NULL
           FROM ent_travail WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(mid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?;
    let Some((titre, consigne, duree, _en_ligne, pour_le, retard, maj, supprime)) = ancienne else {
        return Ok(());
    };
    let change = titre != t.titre
        || consigne != t.consigne
        || duree.map(|d| d as i16) != t.duree_estimee_min
        || pour_le != t.echeance_jour
        || retard != t.retard_accepte;
    if maj <= t.modifie_le || (!change && !supprime) || (supprime && t.etat == "retire") {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO ent_cdt_travail_version (id, etablissement_id, travail_id, version, titre, consigne,
                duree_estimee_min, echeance_le, etat, auteur_user_id, auteur_agent_id, motif)
         SELECT gen_random_uuid(), t.etablissement_id, t.id, t.version, t.titre, t.consigne, t.duree_estimee_min,
                t.echeance_le, t.etat, t.auteur_user_id, t.auteur_agent_id,
                'reprise d''une modification faite par l''ancienne route'
           FROM ent_cdt_travail t WHERE t.etablissement_id = $1 AND t.id = $2",
    )
    .bind(etab)
    .bind(t.id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_cdt_travail SET titre = $3, consigne = $4, duree_estimee_min = $5, retard_accepte = $6,
                echeance_mode = CASE WHEN $7 THEN 'date_heure' ELSE echeance_mode END,
                echeance_seance_id = CASE WHEN $7 THEN NULL ELSE echeance_seance_id END,
                echeance_le = CASE WHEN $7 THEN ent_cdt_instant_local($1, $8, ent_cdt_heure_locale($1, echeance_le)) ELSE echeance_le END,
                etat = CASE WHEN $9 THEN 'retire' ELSE etat END,
                version = version + 1, modifie_le = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(t.id)
    .bind(titre)
    .bind(consigne)
    .bind(duree.map(|d| d as i16))
    .bind(retard)
    .bind(pour_le != t.echeance_jour)
    .bind(pour_le)
    .bind(supprime)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    Ok(())
}

fn rendu_valide(r: &str) -> bool {
    matches!(r, "aucun" | "en_ligne" | "en_classe")
}

fn acteur_auteur(etab: Uuid, user: Option<Uuid>) -> Acteur<'static> {
    Acteur {
        etablissement_id: etab,
        user_id: user.unwrap_or_default(),
        role: "prof",
    }
}

/// Échéance résolue, prête à être écrite.
struct Echeance {
    mode: &'static str,
    seance_id: Option<Uuid>,
    le: DateTime<Utc>,
}

impl PgEnt {
    /// Projette les travaux échus (publiés et visibles, public du cours) dans `ent_travail` et émet l'avis de
    /// nouveau travail, une seule fois, quand la plage calme est passée.
    pub(crate) async fn cdt_projeter_travaux_echus(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
    ) -> Result<(), EntDbError> {
        let rows: Vec<(Uuid, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT id, auteur_user_id, rendu FROM ent_cdt_travail
              WHERE etablissement_id = $1 AND ancien_id IS NULL AND public = 'cours'
                AND etat IN ('programme', 'publie') AND visible_a <= now()
              ORDER BY visible_a LIMIT 200 FOR UPDATE SKIP LOCKED",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (id, auteur, rendu) in rows {
            sqlx::query("UPDATE ent_cdt_travail SET etat = 'publie' WHERE etablissement_id = $1 AND id = $2 AND etat = 'programme'")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            if miroir_travail(tx, etab, id).await? {
                let type_evt = if rendu == "en_ligne" {
                    types::CAHIER_TRAVAIL_A_RENDRE
                } else {
                    types::CAHIER_TRAVAIL_DONNE
                };
                publier_cdt(
                    tx,
                    &acteur_auteur(etab, auteur),
                    type_evt,
                    id,
                    None,
                    json!({ "travail_id": id, "publication": "echue" }),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn cdt_charger_travail(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<TravailRow, EntDbError> {
        sqlx::query_as::<_, TravailRow>(&format!("{SELECT_TRAVAIL} AND t.id = $2"))
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_optional(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)
    }

    /// Résout et contrôle l'échéance pour un cours (§9.1, §8.3) : après le don, dans l'année, jour ouvré, hors plage
    /// calme (sauf réglage lycée), jamais dans le passé.
    #[allow(clippy::too_many_arguments)]
    async fn cdt_echeance(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        cours_id: Uuid,
        annee_id: Uuid,
        donnee: Option<&SeanceInfo>,
        maintenant: DateTime<Utc>,
        explicite: Option<Uuid>,
        prochaine: bool,
        jour: Option<NaiveDate>,
        heure: Option<NaiveTime>,
        plage_calme_permise: bool,
    ) -> Result<Echeance, EntDbError> {
        let ech = if let Some(sid) = explicite {
            let s: SeanceInfo = sqlx::query_as(&format!("{SELECT_SEANCE_INFO} AND s.id = $2"))
                .bind(etab)
                .bind(sid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?;
            if s.cours_id != Some(cours_id) {
                return Err(invalide("la séance d'échéance n'appartient pas au cours"));
            }
            if s.statut == "annulee" {
                return Err(conflit("la séance d'échéance est annulée"));
            }
            Echeance {
                mode: "seance",
                seance_id: Some(s.id),
                le: s.debut_le,
            }
        } else if prochaine {
            let reference = donnee
                .map(|d| d.fin_le)
                .filter(|f| *f > maintenant)
                .unwrap_or(maintenant);
            let s: Option<SeanceInfo> = sqlx::query_as(&format!(
                "{SELECT_SEANCE_INFO} AND ent_cdt_cours_de_seance(s.etablissement_id, s.id) = $2
                    AND s.statut <> 'annulee' AND ent_cdt_instant_local(s.etablissement_id, s.jour, s.heure_debut) > $3
                  ORDER BY s.jour, s.heure_debut LIMIT 1"
            ))
            .bind(etab)
            .bind(cours_id)
            .bind(reference)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some(s) = s else {
                return Err(invalide(
                    "aucune séance à venir pour ce cours : choisissez une date",
                ));
            };
            Echeance {
                mode: "seance",
                seance_id: Some(s.id),
                le: s.debut_le,
            }
        } else if let Some(j) = jour {
            let h = heure.unwrap_or(HEURE_ECHEANCE_DEFAUT);
            let (le,): (DateTime<Utc>,) =
                sqlx::query_as("SELECT ent_cdt_instant_local($1, $2, $3)")
                    .bind(etab)
                    .bind(j)
                    .bind(h)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
            let (ferme,): (bool,) = sqlx::query_as("SELECT ent_cdt_jour_ferme($1, $2)")
                .bind(etab)
                .bind(j)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            if ferme {
                return Err(invalide("ce jour est fermé (week-end, vacances ou fermeture) : choisissez une séance ou un jour ouvré"));
            }
            let calme = h < NaiveTime::from_hms_opt(7, 0, 0).unwrap_or_default()
                || h >= NaiveTime::from_hms_opt(20, 0, 0).unwrap_or_default();
            if calme && !plage_calme_permise {
                return Err(invalide("pas d'échéance pendant la plage calme (20 h – 7 h) : choisissez une heure en journée"));
            }
            Echeance {
                mode: "date_heure",
                seance_id: None,
                le,
            }
        } else {
            return Err(invalide(
                "indiquez l'échéance : séance, prochaine séance ou date",
            ));
        };
        if ech.le <= maintenant {
            return Err(invalide("l'échéance est déjà passée"));
        }
        if let Some(d) = donnee {
            if ech.le <= d.debut_le {
                return Err(invalide(
                    "l'échéance doit suivre la séance où le travail est donné",
                ));
            }
        }
        let (fin_annee, jour_ech): (NaiveDate, NaiveDate) = sqlx::query_as(
            "SELECT a.date_fin, ent_aujourdhui_etab($1, $3) FROM ent_annee_scolaire a WHERE a.etablissement_id = $1 AND a.id = $2",
        )
        .bind(etab)
        .bind(annee_id)
        .bind(ech.le)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if jour_ech > fin_annee {
            return Err(invalide("l'échéance dépasse la fin de l'année scolaire"));
        }
        Ok(ech)
    }

    /// CDT-131 à 134 : donne un travail à un ou plusieurs cours (un travail par cours, reliés), avec nature, durée,
    /// échéance (séance, prochaine séance ou date), public restreint, brouillon, publication ou programmation.
    pub async fn cdt_travaux_creer(
        &self,
        a: &Acteur<'_>,
        body: TravailC2Body,
    ) -> Result<TravauxCreesDto, EntDbError> {
        let titre = texte_borne(
            &body.titre,
            200,
            true,
            "titre vide ou trop long (200 caractères au plus)",
        )?;
        let consigne = texte_borne(
            &body.consigne,
            20_000,
            true,
            "consigne vide ou trop longue (20 000 caractères au plus)",
        )?;
        if body.cours_ids.is_empty() || body.cours_ids.len() > 10 {
            return Err(invalide("indiquez de 1 à 10 cours"));
        }
        let mut cours_uniques = body.cours_ids.clone();
        cours_uniques.sort();
        cours_uniques.dedup();
        if cours_uniques.len() != body.cours_ids.len() {
            return Err(invalide("un cours est indiqué deux fois"));
        }
        if !rendu_valide(&body.rendu) {
            return Err(invalide("rendu : aucun, en_ligne ou en_classe"));
        }
        if !matches!(
            body.ia_autorisee.as_str(),
            "non" | "recherche" | "relecture" | "libre"
        ) {
            return Err(invalide(
                "usage de l'IA : non, recherche, relecture ou libre",
            ));
        }
        if !matches!(
            body.publication.as_str(),
            "brouillon" | "maintenant" | "programmer"
        ) {
            return Err(invalide(
                "publication : brouillon, maintenant ou programmer",
            ));
        }
        if body.formats.len() > 12
            || body.formats.iter().any(|f| {
                f.is_empty()
                    || f.len() > 10
                    || !f
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
        {
            return Err(invalide(
                "formats acceptés invalides (extensions en minuscules, douze au plus)",
            ));
        }
        if !body.formats.is_empty() && body.rendu != "en_ligne" {
            return Err(invalide(
                "les formats acceptés ne concernent qu'un rendu en ligne",
            ));
        }
        let choix = u8::from(body.echeance_seance_id.is_some())
            + u8::from(body.echeance_prochaine_seance)
            + u8::from(body.echeance_jour.is_some());
        if choix != 1 {
            return Err(invalide(
                "indiquez une seule échéance : séance, prochaine séance ou date",
            ));
        }
        if body.echeance_seance_id.is_some() && body.cours_ids.len() > 1 {
            return Err(invalide(
                "avec plusieurs cours, choisissez « prochaine séance » ou une date",
            ));
        }
        if !body.eleve_ids.is_empty() && body.cours_ids.len() > 1 {
            return Err(invalide(
                "un public restreint ne s'applique qu'à un seul cours",
            ));
        }
        if body.eleve_ids.len() > 60 {
            return Err(invalide("soixante élèves au plus pour un public restreint"));
        }
        // Droits : chaque cours, à la date de l'action (R-C2) ; le remplaçant n'écrit que sur sa séance.
        let donnee: Option<SeanceInfo> = match body.seance_donnee_id {
            Some(id) => Some(self.cdt_seance(a.etablissement_id, id).await?),
            None => None,
        };
        let mut cours_rows = Vec::new();
        for cid in &body.cours_ids {
            let c = self.cdt_cours(a.etablissement_id, *cid).await?;
            self.cdt_exiger_saisie(a, *cid, donnee.as_ref()).await?;
            self.cdt_exiger_annee_ouverte(a.etablissement_id, c.annee_id)
                .await?;
            cours_rows.push(c);
        }
        if let Some(d) = &donnee {
            if !cours_rows.iter().any(|c| d.cours_id == Some(c.id)) {
                return Err(invalide(
                    "la séance où le travail est donné n'appartient à aucun des cours indiqués",
                ));
            }
            if d.statut == "annulee" {
                return Err(conflit("la séance où le travail est donné est annulée"));
            }
        }
        let jour_action = self.jour_etab(a.etablissement_id).await?;
        let reglage_publication = self
            .cdt_publication_defaut(a.etablissement_id, cours_rows[0].annee_id)
            .await?;
        self.cdt_projeter_echus(a.etablissement_id).await?;

        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        sqlx::query("SELECT ent_cdt_assurer_natures($1)")
            .bind(a.etablissement_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let nature: Option<(Uuid, Option<i16>)> = sqlx::query_as(
            "SELECT id, duree_defaut_min FROM ent_cdt_nature WHERE etablissement_id = $1 AND code = $2 AND archivee_le IS NULL",
        )
        .bind(a.etablissement_id)
        .bind(body.nature.trim())
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((nature_id, duree_defaut)) = nature else {
            return Err(invalide("nature de travail inconnue"));
        };
        let duree = body.duree_estimee_min.or(duree_defaut);
        if duree.is_some_and(|d| !(1..=600).contains(&d)) {
            return Err(invalide("durée estimée : de 1 à 600 minutes"));
        }
        let (maintenant,): (DateTime<Utc>,) = sqlx::query_as("SELECT now()")
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let voulue_explicite = instant_programme(
            &mut tx,
            a.etablissement_id,
            body.publie_a,
            body.publie_jour,
            body.publie_heure,
        )
        .await?;
        let plage_calme_permise: bool = sqlx::query_scalar(
            "SELECT COALESCE((SELECT echeance_plage_calme_lycee FROM ent_cdt_reglage WHERE etablissement_id = $1 AND annee_id = $2), false)",
        )
        .bind(a.etablissement_id)
        .bind(cours_rows[0].annee_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        // Public restreint : seulement des élèves du cours **à la date de l'action** (R-C25).
        if !body.eleve_ids.is_empty() {
            let mut uniques = body.eleve_ids.clone();
            uniques.sort();
            uniques.dedup();
            let c = &cours_rows[0];
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(DISTINCT a.eleve_id)
                   FROM ent_affectation a
                   JOIN ent_eleve e ON e.etablissement_id = a.etablissement_id AND e.id = a.eleve_id
                  WHERE a.etablissement_id = $1 AND a.eleve_id = ANY($2)
                    AND ((a.classe_id IS NOT NULL AND a.classe_id = $3) OR (a.groupe_id IS NOT NULL AND a.groupe_id = $4))
                    AND a.date_debut <= $5 AND (a.date_fin IS NULL OR a.date_fin >= $5)
                    AND (e.date_sortie IS NULL OR e.date_sortie > $5)",
            )
            .bind(a.etablissement_id)
            .bind(&uniques)
            .bind(c.classe_id)
            .bind(c.groupe_id)
            .bind(jour_action)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if n as usize != uniques.len() {
                return Err(invalide(
                    "un élève indiqué ne fait pas partie du cours à cette date",
                ));
            }
        }
        let agent: Option<Uuid> = sqlx::query_scalar("SELECT ent_cdt_agent_du_compte($1, $2)")
            .bind(a.etablissement_id)
            .bind(a.user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let restreint = !body.eleve_ids.is_empty();
        let mut crees: Vec<Uuid> = Vec::new();
        let mut avertissements: Vec<String> = Vec::new();
        for c in &cours_rows {
            let donnee_ici = donnee.as_ref().filter(|d| d.cours_id == Some(c.id));
            let ech = self
                .cdt_echeance(
                    &mut tx,
                    a.etablissement_id,
                    c.id,
                    c.annee_id,
                    donnee_ici,
                    maintenant,
                    body.echeance_seance_id,
                    body.echeance_prochaine_seance,
                    body.echeance_jour,
                    body.echeance_heure,
                    plage_calme_permise,
                )
                .await?;
            // Publication : défaut du réglage (S20-01) = fin de la séance où le travail est donné.
            let (etat, publie_a): (&str, Option<DateTime<Utc>>) = match body.publication.as_str() {
                "brouillon" => ("brouillon", None),
                "maintenant" => ("publie", Some(maintenant)),
                _ => {
                    let voulue = voulue_explicite.unwrap_or(
                        match (reglage_publication.as_str(), donnee_ici) {
                            ("immediate", _) | (_, None) => maintenant,
                            (_, Some(d)) => d.fin_le,
                        },
                    );
                    if voulue <= maintenant {
                        ("publie", Some(maintenant))
                    } else {
                        ("programme", Some(voulue))
                    }
                }
            };
            let visible_a: Option<DateTime<Utc>> = match publie_a {
                Some(p) => {
                    let (v,): (DateTime<Utc>,) = sqlx::query_as("SELECT ent_cdt_visible_a($1, $2)")
                        .bind(a.etablissement_id)
                        .bind(p)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(db)?;
                    Some(v)
                }
                None => None,
            };
            if let Some(v) = visible_a {
                if publie_a.is_some_and(|p| v > p)
                    && !avertissements
                        .iter()
                        .any(|w| w == "differe_par_plage_calme")
                {
                    avertissements.push("differe_par_plage_calme".into());
                }
                if ech.le - v < chrono::Duration::hours(24)
                    && !avertissements.iter().any(|w| w == "donne_moins_de_24h")
                {
                    avertissements.push("donne_moins_de_24h".into());
                }
            }
            let (donne_le,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, $2)")
                .bind(a.etablissement_id)
                .bind(donnee_ici.map(|d| d.debut_le).unwrap_or(maintenant))
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            let id = Uuid::new_v4();
            let relie = crees.first().copied();
            sqlx::query(
                "INSERT INTO ent_cdt_travail (id, etablissement_id, cours_id, annee_id, seance_donnee_id, donne_le, echeance_mode,
                        echeance_seance_id, echeance_le, nature_id, titre, consigne, duree_estimee_min, rendu, retard_accepte,
                        public, ia_autorisee, relie_id, etat, publie_a, visible_a, auteur_user_id, auteur_agent_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)",
            )
            .bind(id)
            .bind(a.etablissement_id)
            .bind(c.id)
            .bind(c.annee_id)
            .bind(donnee_ici.map(|d| d.id))
            .bind(donne_le)
            .bind(ech.mode)
            .bind(ech.seance_id)
            .bind(ech.le)
            .bind(nature_id)
            .bind(&titre)
            .bind(&consigne)
            .bind(duree)
            .bind(&body.rendu)
            .bind(body.retard_accepte)
            .bind(if restreint { "restreint" } else { "cours" })
            .bind(&body.ia_autorisee)
            .bind(relie)
            .bind(etat)
            .bind(publie_a)
            .bind(visible_a)
            .bind(a.user_id)
            .bind(agent)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            for f in &body.formats {
                sqlx::query("INSERT INTO ent_cdt_travail_format (etablissement_id, travail_id, format_code) VALUES ($1, $2, $3)")
                    .bind(a.etablissement_id)
                    .bind(id)
                    .bind(f)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
            }
            if restreint {
                let mut uniques = body.eleve_ids.clone();
                uniques.sort();
                uniques.dedup();
                for e in uniques {
                    sqlx::query("INSERT INTO ent_cdt_travail_cible (etablissement_id, travail_id, eleve_id) VALUES ($1, $2, $3)")
                        .bind(a.etablissement_id)
                        .bind(id)
                        .bind(e)
                        .execute(&mut *tx)
                        .await
                        .map_err(db)?;
                }
            }
            let visible_maintenant = etat == "publie" && visible_a.is_some_and(|v| v <= maintenant);
            if visible_maintenant && miroir_travail(&mut tx, a.etablissement_id, id).await? {
                let type_evt = if body.rendu == "en_ligne" {
                    types::CAHIER_TRAVAIL_A_RENDRE
                } else {
                    types::CAHIER_TRAVAIL_DONNE
                };
                publier_cdt(
                    &mut tx,
                    a,
                    type_evt,
                    id,
                    None,
                    json!({ "travail_id": id, "cours_id": c.id }),
                )
                .await?;
            } else if etat == "programme" {
                publier_cdt(
                    &mut tx,
                    a,
                    types::CAHIER_PROGRAMME,
                    id,
                    None,
                    json!({ "travail_id": id, "publie_a": publie_a, "visible_a": visible_a }),
                )
                .await?;
            }
            crees.push(id);
        }
        let mut travaux = Vec::new();
        for id in crees {
            let row: TravailRow = sqlx::query_as(&format!("{SELECT_TRAVAIL} AND t.id = $2"))
                .bind(a.etablissement_id)
                .bind(id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            travaux.push(travail_dto(&mut tx, a.etablissement_id, row).await?);
        }
        tx.commit().await.map_err(db)?;
        Ok(TravauxCreesDto {
            travaux,
            avertissements,
        })
    }

    /// Lit un travail : intervenants en saisie du cours ; sinon lecture du cahier (publié et visible seulement).
    pub async fn cdt_travail_lire(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<TravailC2Dto, EntDbError> {
        let t = self.cdt_charger_travail(a, id).await?;
        if self.cdt_exiger_saisie(a, t.cours_id, None).await.is_err() {
            let cours = self.cdt_cours(a.etablissement_id, t.cours_id).await?;
            let cible = crate::cahier::Cible {
                classe_id: cours.classe_id,
                groupe_id: cours.groupe_id,
            };
            let (visible,): (bool,) =
                sqlx::query_as("SELECT $1::timestamptz IS NOT NULL AND $1::timestamptz <= now()")
                    .bind(t.visible_a)
                    .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
                    .await
                    .map_err(db)?;
            let lisible = matches!(t.etat.as_str(), "publie" | "retire") && visible;
            if !lisible || !self.cahier_peut(a, Action::CahierLire, cible).await? {
                return Err(introuvable());
            }
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        travail_dto(&mut tx, a.etablissement_id, t).await
    }

    /// R-C29 : versions précédentes d'un travail.
    pub async fn cdt_travail_versions(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
    ) -> Result<Vec<VersionTravailDto>, EntDbError> {
        self.cdt_travail_lire(a, id).await?;
        let rows: Vec<(i32, String, String, Option<i16>, Option<DateTime<Utc>>, String, String, Option<String>, bool, DateTime<Utc>)> =
            sqlx::query_as(
                "SELECT v.version, v.titre, v.consigne, v.duree_estimee_min, v.echeance_le, v.etat,
                        ent_cdt_nom_affichage(v.etablissement_id, v.auteur_user_id), v.motif, v.sans_avis, v.cree_le
                   FROM ent_cdt_travail_version v
                  WHERE v.etablissement_id = $1 AND v.travail_id = $2 ORDER BY v.version DESC",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_all(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
            .await
            .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    version,
                    titre,
                    consigne,
                    duree_estimee_min,
                    echeance_le,
                    etat,
                    auteur,
                    motif,
                    sans_avis,
                    cree_le,
                )| VersionTravailDto {
                    version,
                    titre,
                    consigne,
                    duree_estimee_min,
                    echeance_le,
                    etat,
                    auteur,
                    motif,
                    sans_avis,
                    cree_le,
                },
            )
            .collect())
    }

    /// CDT-142 : modifie un travail (libre avant publication ; versionné ensuite). Avis aux élèves seulement si
    /// l'auteur ne choisit pas « modification mineure, sans avis » ; une échéance **avancée** demande une
    /// confirmation explicite et prévient toujours.
    pub async fn cdt_travail_modifier(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        body: ModifierTravailC2Body,
    ) -> Result<TravailModifieDto, EntDbError> {
        let t0 = self.cdt_charger_travail(a, id).await?;
        self.cdt_exiger_saisie(a, t0.cours_id, None).await?;
        let cours = self.cdt_cours(a.etablissement_id, t0.cours_id).await?;
        self.cdt_exiger_annee_ouverte(a.etablissement_id, cours.annee_id)
            .await?;
        let titre = match body.titre.as_deref() {
            Some(x) => Some(texte_borne(
                x,
                200,
                true,
                "titre vide ou trop long (200 caractères au plus)",
            )?),
            None => None,
        };
        let consigne = match body.consigne.as_deref() {
            Some(x) => Some(texte_borne(
                x,
                20_000,
                true,
                "consigne vide ou trop longue (20 000 caractères au plus)",
            )?),
            None => None,
        };
        let motif = match body.motif.as_deref() {
            Some(x) => {
                Some(texte_borne(x, 500, false, "motif trop long")?).filter(|m| !m.is_empty())
            }
            None => None,
        };
        if let Some(r) = &body.rendu {
            if !rendu_valide(r) {
                return Err(invalide("rendu : aucun, en_ligne ou en_classe"));
            }
        }
        if body
            .duree_estimee_min
            .is_some_and(|d| !(1..=600).contains(&d))
        {
            return Err(invalide("durée estimée : de 1 à 600 minutes"));
        }
        if let Some(p) = &body.publication {
            if !matches!(p.as_str(), "maintenant" | "programmer") {
                return Err(invalide("publication : maintenant ou programmer"));
            }
        }
        let change_echeance = body.echeance_seance_id.is_some() || body.echeance_jour.is_some();
        if body.echeance_seance_id.is_some() && body.echeance_jour.is_some() {
            return Err(invalide("indiquez une seule échéance : séance ou date"));
        }
        let donnee: Option<SeanceInfo> = match t0.seance_donnee_id {
            Some(sid) => Some(self.cdt_seance(a.etablissement_id, sid).await?),
            None => None,
        };
        let reglage_publication = self
            .cdt_publication_defaut(a.etablissement_id, cours.annee_id)
            .await?;
        self.cdt_projeter_echus(a.etablissement_id).await?;

        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let t: TravailRow =
            sqlx::query_as(&format!("{SELECT_TRAVAIL} AND t.id = $2 FOR UPDATE OF t"))
                .bind(a.etablissement_id)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?;
        if t.etat == "retire" {
            return Err(conflit("le travail est retiré de la diffusion"));
        }
        rafraichir_travail(&mut tx, a.etablissement_id, &t).await?;
        let t: TravailRow = sqlx::query_as(&format!("{SELECT_TRAVAIL} AND t.id = $2"))
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let (maintenant,): (DateTime<Utc>,) = sqlx::query_as("SELECT now()")
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let voulue_explicite = instant_programme(
            &mut tx,
            a.etablissement_id,
            body.publie_a,
            body.publie_jour,
            body.publie_heure,
        )
        .await?;
        let etait_visible = t.etat == "publie" && t.visible_a.is_some_and(|v| v <= maintenant);
        let rendu = body.rendu.clone().unwrap_or_else(|| t.rendu.clone());
        if t.rendu == "en_ligne" && rendu != "en_ligne" {
            let (remises,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_travail_remise WHERE etablissement_id = $1 AND travail_id = $2",
            )
            .bind(a.etablissement_id)
            .bind(t.ancien_id.unwrap_or(t.id))
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if remises > 0 {
                return Err(conflit("des copies sont déjà rendues : le mode de rendu ne peut plus devenir hors ligne"));
            }
        }
        let nature_id: Uuid = match &body.nature {
            Some(code) => sqlx::query_scalar(
                "SELECT id FROM ent_cdt_nature WHERE etablissement_id = $1 AND code = $2 AND archivee_le IS NULL",
            )
            .bind(a.etablissement_id)
            .bind(code.trim())
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(|| invalide("nature de travail inconnue"))?,
            None => t.nature_id,
        };
        // Échéance éventuelle.
        let (mode, seance_ech, echeance_le) = if change_echeance {
            let plage: bool = sqlx::query_scalar(
                "SELECT COALESCE((SELECT echeance_plage_calme_lycee FROM ent_cdt_reglage WHERE etablissement_id = $1 AND annee_id = $2), false)",
            )
            .bind(a.etablissement_id)
            .bind(cours.annee_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let e = self
                .cdt_echeance(
                    &mut tx,
                    a.etablissement_id,
                    t.cours_id,
                    cours.annee_id,
                    donnee.as_ref(),
                    maintenant,
                    body.echeance_seance_id,
                    false,
                    body.echeance_jour,
                    body.echeance_heure,
                    plage,
                )
                .await?;
            (e.mode.to_string(), e.seance_id, e.le)
        } else {
            (t.echeance_mode.clone(), t.echeance_seance_id, t.echeance_le)
        };
        let echeance_avancee = etait_visible && echeance_le < t.echeance_le;
        if echeance_avancee && !body.confirme_echeance_avancee {
            return Err(conflit("échéance avancée : confirmez explicitement, les élèves et les familles seront prévenus"));
        }
        // Publication d'un brouillon, reprogrammation.
        let (etat, publie_a, visible_a): (String, Option<DateTime<Utc>>, Option<DateTime<Utc>>) =
            match body.publication.as_deref() {
                None => (t.etat.clone(), t.publie_a, t.visible_a),
                Some(_) if t.etat == "publie" => return Err(conflit("le travail est déjà publié")),
                Some(p) => {
                    let (e, pa): (&str, DateTime<Utc>) = if p == "maintenant" {
                        ("publie", maintenant)
                    } else {
                        let voulue = voulue_explicite.unwrap_or(
                            match (reglage_publication.as_str(), donnee.as_ref()) {
                                ("immediate", _) | (_, None) => maintenant,
                                (_, Some(d)) => d.fin_le,
                            },
                        );
                        if voulue <= maintenant {
                            ("publie", maintenant)
                        } else {
                            ("programme", voulue)
                        }
                    };
                    let (v,): (DateTime<Utc>,) = sqlx::query_as("SELECT ent_cdt_visible_a($1, $2)")
                        .bind(a.etablissement_id)
                        .bind(pa)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(db)?;
                    (e.to_string(), Some(pa), Some(v))
                }
            };
        let avis = (!body.sans_avis || echeance_avancee) && etait_visible;
        if t.etat != "brouillon" {
            sqlx::query(
                "INSERT INTO ent_cdt_travail_version (id, etablissement_id, travail_id, version, titre, consigne,
                        duree_estimee_min, echeance_le, etat, auteur_user_id, auteur_agent_id, motif, sans_avis)
                 SELECT gen_random_uuid(), t.etablissement_id, t.id, t.version, t.titre, t.consigne, t.duree_estimee_min,
                        t.echeance_le, t.etat, t.auteur_user_id, t.auteur_agent_id, $3, $4
                   FROM ent_cdt_travail t WHERE t.etablissement_id = $1 AND t.id = $2",
            )
            .bind(a.etablissement_id)
            .bind(id)
            .bind(&motif)
            .bind(!avis && etait_visible)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        sqlx::query(
            "UPDATE ent_cdt_travail SET titre = $3, consigne = $4, duree_estimee_min = $5, nature_id = $6, rendu = $7,
                    echeance_mode = $8, echeance_seance_id = $9, echeance_le = $10, etat = $11, publie_a = $12,
                    visible_a = $13, auteur_user_id = $14, version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(titre.clone().unwrap_or_else(|| t.titre.clone()))
        .bind(consigne.clone().unwrap_or_else(|| t.consigne.clone()))
        .bind(body.duree_estimee_min.or(t.duree_estimee_min))
        .bind(nature_id)
        .bind(&rendu)
        .bind(&mode)
        .bind(seance_ech)
        .bind(echeance_le)
        .bind(&etat)
        .bind(publie_a)
        .bind(visible_a)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let devient_visible =
            etat == "publie" && !etait_visible && visible_a.is_some_and(|v| v <= maintenant);
        if t.ancien_id.is_some() || devient_visible {
            if miroir_travail(&mut tx, a.etablissement_id, id).await? && devient_visible {
                let type_evt = if rendu == "en_ligne" {
                    types::CAHIER_TRAVAIL_A_RENDRE
                } else {
                    types::CAHIER_TRAVAIL_DONNE
                };
                publier_cdt(
                    &mut tx,
                    a,
                    type_evt,
                    id,
                    None,
                    json!({ "travail_id": id, "cours_id": t.cours_id }),
                )
                .await?;
            }
        }
        if etait_visible && t.public == "cours" {
            let mut champs: Vec<&str> = Vec::new();
            if titre.as_deref().is_some_and(|x| x != t.titre) {
                champs.push("titre");
            }
            if consigne.as_deref().is_some_and(|x| x != t.consigne) {
                champs.push("consigne");
            }
            if echeance_le != t.echeance_le {
                champs.push("echeance");
            }
            if rendu != t.rendu {
                champs.push("rendu");
            }
            publier_cdt(
                &mut tx,
                a,
                types::CAHIER_TRAVAIL_MODIFIE,
                id,
                None,
                json!({
                    "travail_id": id,
                    "version": t.version + 1,
                    "deja_vu": avis,
                    "avis": avis,
                    "echeance_avancee": echeance_avancee,
                    "champs": champs,
                }),
            )
            .await?;
        }
        let row: TravailRow = sqlx::query_as(&format!("{SELECT_TRAVAIL} AND t.id = $2"))
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let dto = travail_dto(&mut tx, a.etablissement_id, row).await?;
        tx.commit().await.map_err(db)?;
        Ok(TravailModifieDto {
            travail: dto,
            avis_emis: avis && t.public == "cours",
            echeance_avancee,
        })
    }

    /// CDT-143 : retire un travail de la diffusion (motif obligatoire ; les copies déjà rendues restent à l'élève).
    pub async fn cdt_travail_retirer(
        &self,
        a: &Acteur<'_>,
        id: Uuid,
        motif: &str,
    ) -> Result<TravailC2Dto, EntDbError> {
        let motif = texte_borne(
            motif,
            500,
            true,
            "indiquez le motif du retrait (3 à 500 caractères)",
        )?;
        if motif.chars().count() < 3 {
            return Err(invalide(
                "indiquez le motif du retrait (3 à 500 caractères)",
            ));
        }
        let droits = self
            .droits_effectifs(a.etablissement_id, a.user_id, a.role)
            .await?;
        if !droits.iter().any(|d| d.action == Action::CahierRetirer) {
            return Err(EntDbError::Forbidden);
        }
        let direction = droits.iter().any(|d| {
            d.action == Action::CahierRetirer && d.portee == ent_auth::Portee::Etablissement
        });
        let t0 = self.cdt_charger_travail(a, id).await?;
        let auteur = t0.auteur_user_id == Some(a.user_id);
        if !direction && !auteur && self.cdt_exiger_saisie(a, t0.cours_id, None).await.is_err() {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let t: TravailRow =
            sqlx::query_as(&format!("{SELECT_TRAVAIL} AND t.id = $2 FOR UPDATE OF t"))
                .bind(a.etablissement_id)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?
                .ok_or_else(introuvable)?;
        if t.etat == "retire" {
            return Err(conflit("le travail est déjà retiré"));
        }
        rafraichir_travail(&mut tx, a.etablissement_id, &t).await?;
        sqlx::query(
            "INSERT INTO ent_cdt_travail_version (id, etablissement_id, travail_id, version, titre, consigne,
                    duree_estimee_min, echeance_le, etat, auteur_user_id, auteur_agent_id, motif, sans_avis)
             SELECT gen_random_uuid(), t.etablissement_id, t.id, t.version, t.titre, t.consigne, t.duree_estimee_min,
                    t.echeance_le, t.etat, t.auteur_user_id, t.auteur_agent_id, $3, false
               FROM ent_cdt_travail t WHERE t.etablissement_id = $1 AND t.id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_cdt_travail SET etat = 'retire', retire_motif = $3, retire_par_user_id = $4,
                    version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(id)
        .bind(&motif)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if t.ancien_id.is_some() {
            miroir_travail(&mut tx, a.etablissement_id, id).await?;
        }
        publier_cdt(
            &mut tx,
            a,
            types::CAHIER_TRAVAIL_SUPPRIME,
            id,
            None,
            json!({ "travail_id": id, "retire_par_direction": direction && !auteur }),
        )
        .await?;
        let row: TravailRow = sqlx::query_as(&format!("{SELECT_TRAVAIL} AND t.id = $2"))
            .bind(a.etablissement_id)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let dto = travail_dto(&mut tx, a.etablissement_id, row).await?;
        tx.commit().await.map_err(db)?;
        Ok(dto)
    }
}
