//! Socle 18, lot FO1 — gestion des objets d'accord, versions, décisions, retrait, « mes accords » et reprise
//! (`docs/socles/18_FORMULAIRES.md` §6–§10, §21.1 : FRM-100 à FRM-119 ; règles R-FC1, R-FC6, R-FC15, R-FC16, R-FC19,
//! R-FC21, R-FC43, R-FC46, R-FC51, R-FC52).
//!
//! * Tout accès passe par [`TenantTx`] (RLS d'établissement, FK composites).
//! * Une décision est un **ajout seul** ; le retrait est une nouvelle décision (`retire`), sans passkey, effective
//!   immédiatement (le verdict est calculé à chaque appel, §9).
//! * Le décideur est **calculé** (titulaire communicant ayant le droit du lien, élève selon l'âge et la capacité), jamais
//!   choisi par l'appelant (R-FC2) ; l'avis de l'enfant n'est recueilli que par **son propre compte** (R-FC51).
//! * Le texte vu par le décideur est celui de la version publiée en cours, figé avec son empreinte (R-FC52).
//! * Notifications : par l'outbox du socle 8, en enveloppe neutre (`formulaire`), seulement aux décideurs concernés par
//!   une version de fond.

#![allow(clippy::type_complexity)]

use crate::accords::*;
use crate::accords_verifier::{
    age_au, condition_droit_lien, objet_par_code, version_en_cours, ObjetLigne, COLONNES_OBJET,
};
use crate::formulaires::{objet_sensible, SANTE_ACTIVEE};
use crate::notifications::{db, notifier_personne, EnvoiAPlanifier};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use std::collections::{BTreeSet, HashMap, HashSet};
use uuid::Uuid;

pub(crate) fn introuvable() -> EntDbError {
    EntDbError::Domain(domain_eleve::DomainError::NotFound)
}

fn code_valide(c: &str) -> bool {
    crate::formulaires::objet_valide(c)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn journaliser(
    conn: &mut PgConnection,
    etab: Uuid,
    acte: &str,
    objet_id: Option<Uuid>,
    version_id: Option<Uuid>,
    eleve: Option<Uuid>,
    personne: Option<Uuid>,
    acteur: Uuid,
    nb: Option<i32>,
    detail: Option<&str>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_acc_journal
            (id, etablissement_id, acte, objet_id, version_id, sujet_eleve_id, sujet_personne_id, acteur_user_id, nb, detail)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(acte)
    .bind(objet_id)
    .bind(version_id)
    .bind(eleve)
    .bind(personne)
    .bind(acteur)
    .bind(nb)
    .bind(detail)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

pub(crate) async fn publier_evt(
    tx: &mut crate::tenant_tx::TenantTx<'_>,
    etab: Uuid,
    type_name: &str,
    aggregate_id: Uuid,
    eleve: Option<Uuid>,
    acteur: Uuid,
    payload: serde_json::Value,
) -> Result<Uuid, EntDbError> {
    Ok(tx
        .publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: type_name.to_string(),
            aggregate: AggregateKind::Annonce,
            aggregate_id,
            eleve_id: eleve,
            payload,
            actor_user_id: Some(acteur.to_string()),
        })
        .await?
        .id)
}

/// Preuve demandée à une décision : un port générique (adapteur de test, brique absente) ou une **signature D6** déjà faite
/// (socle 7) que la décision retient et dont elle garde le lien.
pub(crate) enum Preuve<'a> {
    Port(&'a dyn PortSignature),
    D6(Uuid),
}

/// Ce que la vérification d'une signature D6 a établi.
pub(crate) struct PreuveRetenue {
    pub signature_id: Uuid,
    pub circuit_id: Uuid,
    pub niveau: String,
    pub methode: String,
    /// Canal de la décision (`passkey`, `code`, `guichet`, `papier`).
    pub canal: &'static str,
}

/// Empreinte d'une version : objet, numéro, texte et usages couverts (triés), longueurs encodées.
pub(crate) fn empreinte_version(objet_code: &str, numero: i32, texte: &str, usages: &BTreeSet<String>) -> String {
    let n = numero.to_string();
    let u = usages.iter().cloned().collect::<Vec<_>>().join(",");
    empreinte_encodee(&[objet_code, &n, texte, &u])
}

impl PgEnt {
    // -----------------------------------------------------------------------
    // Droits
    // -----------------------------------------------------------------------

    async fn exiger_accords_parametrer(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AutorisationsParametrer).await
    }

    /// Lecture de gestion : `autorisations.gerer` ou `autorisations.parametrer`.
    async fn exiger_gestion(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, acteur, role, Action::AutorisationsGerer).await?
            || self.a_action_etablissement(etab, acteur, role, Action::AutorisationsParametrer).await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    // -----------------------------------------------------------------------
    // Catalogue et objets (FRM-100, FRM-101, FRM-104, FRM-107, FRM-109)
    // -----------------------------------------------------------------------

    /// Catalogue de référence et état d'activation dans l'établissement.
    pub async fn accords_catalogue(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<EntreeCatalogue>, EntDbError> {
        self.exiger_gestion(etab, acteur, role).await?;
        sqlx::query_as(
            "SELECT c.code, c.libelle, c.nature, c.regle_min, c.sujet_type,
                    EXISTS (SELECT 1 FROM ent_acc_objet o
                             WHERE o.etablissement_id = $1 AND o.code = c.code AND o.actif) AS active
               FROM ent_acc_catalogue c ORDER BY c.nature, c.code",
        )
        .bind(etab)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }

    /// Activer un objet du catalogue (FRM-100) : l'objet, ses usages et une version 1 en **brouillon** avec le texte de
    /// départ (provisoire : à relire puis à publier, R-FC72). Réactive un objet désactivé.
    pub async fn accords_activer_objet(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        catalogue_code: &str,
    ) -> Result<ObjetAccord, EntDbError> {
        self.exiger_accords_parametrer(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let existant = objet_par_code(&mut tx, etab, catalogue_code).await?;
        if let Some(o) = existant {
            if o.actif {
                return Err(EntDbError::ConflitDossier("cet objet est déjà activé"));
            }
            sqlx::query("UPDATE ent_acc_objet SET actif = true, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(o.id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            journaliser(&mut tx, etab, "objet_active", Some(o.id), None, None, None, acteur, None, Some("reactive")).await?;
            publier_evt(&mut tx, etab, evenements::OBJET_ACTIVE, o.id, None, acteur, json!({ "objet_id": o.id, "reactive": true }))
                .await?;
            tx.commit().await.map_err(db)?;
            return self.accords_objet(etab, catalogue_code).await;
        }
        let cat: Option<(String, String, Option<i16>, Option<String>, String, String, String, Option<String>, String, String)> =
            sqlx::query_as(
                "SELECT nature, regle_min, avis_eleve_des_ans, droit_lien_code, preuve_min, validite, sujet_type,
                        desaccord_objet, libelle, texte_depart
                   FROM ent_acc_catalogue WHERE code = $1",
            )
            .bind(catalogue_code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        let Some((nature, regle, avis, droit, preuve, validite, sujet, desaccord, libelle, texte)) = cat else {
            return Err(introuvable());
        };
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_acc_objet
                (id, etablissement_id, code, libelle, nature, regle, avis_eleve_des_ans, droit_lien_code, preuve_min, validite,
                 sujet_type, catalogue_code, desaccord_objet, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$3,$12,$13)",
        )
        .bind(id)
        .bind(etab)
        .bind(catalogue_code)
        .bind(&libelle)
        .bind(&nature)
        .bind(&regle)
        .bind(avis)
        .bind(&droit)
        .bind(&preuve)
        .bind(&validite)
        .bind(&sujet)
        .bind(&desaccord)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_acc_usage (id, etablissement_id, objet_id, code, libelle, mission, ordre)
             SELECT gen_random_uuid(), $1, $2, u.code, u.libelle, u.mission, u.ordre
               FROM ent_acc_catalogue_usage u WHERE u.catalogue_code = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(catalogue_code)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let usages_tous: BTreeSet<String> = sqlx::query_as::<_, (String,)>(
            "SELECT code FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(c,)| c)
        .collect();
        let version_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_acc_version (id, etablissement_id, objet_id, numero, texte, empreinte, cree_par)
             VALUES ($1,$2,$3,1,$4,$5,$6)",
        )
        .bind(version_id)
        .bind(etab)
        .bind(id)
        .bind(&texte)
        .bind(empreinte_version(catalogue_code, 1, &texte, &usages_tous))
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_acc_version_usage (etablissement_id, version_id, usage_id)
             SELECT $1, $2, id FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $3",
        )
        .bind(etab)
        .bind(version_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, "objet_active", Some(id), Some(version_id), None, None, acteur, None, Some(catalogue_code))
            .await?;
        publier_evt(&mut tx, etab, evenements::OBJET_ACTIVE, id, None, acteur, json!({ "objet_id": id, "catalogue": catalogue_code }))
            .await?;
        tx.commit().await.map_err(db)?;
        self.accords_objet(etab, catalogue_code).await
    }

    /// Créer un objet propre (FRM-101) : la règle ne peut jamais baisser sous celle du catalogue (R-FC6), un consentement
    /// ne porte pas sur la mission (R-FC1), aucune donnée sensible tant que le module santé est désactivé (R-FC23).
    pub async fn accords_creer_objet(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        n: NouvelObjet,
    ) -> Result<ObjetAccord, EntDbError> {
        self.exiger_accords_parametrer(etab, acteur, role).await?;
        let invalide = |m: &'static str| Err(EntDbError::SaisieInvalide(m));
        if !code_valide(&n.code) {
            return invalide("code invalide (minuscules, chiffres et _)");
        }
        if !SANTE_ACTIVEE && objet_sensible(&n.code) {
            return invalide("donnée de santé ou sensible : interdite tant que l'hébergeur HDS n'est pas en place");
        }
        let libelle = n.libelle.trim().to_string();
        let texte = n.texte.trim().to_string();
        if libelle.is_empty() || libelle.chars().count() > 120 {
            return invalide("libellé invalide");
        }
        if texte.is_empty() || texte.chars().count() > 10_000 {
            return invalide("texte invalide (1 à 10 000 caractères)");
        }
        if !NATURES.contains(&n.nature.as_str())
            || !REGLES.contains(&n.regle.as_str())
            || !PREUVES.contains(&n.preuve_min.as_str())
            || !VALIDITES.contains(&n.validite.as_str())
            || !["eleve", "personne"].contains(&n.sujet_type.as_str())
        {
            return invalide("nature, règle, preuve, validité ou sujet inconnu");
        }
        if n.droit_lien_code.as_deref().is_some_and(|d| !DROITS_LIEN.contains(&d)) {
            return invalide("droit du lien inconnu");
        }
        if (n.regle == "soi") != (n.sujet_type == "personne") {
            return invalide("la règle `soi` va avec un sujet personne, et inversement");
        }
        if n.validite == "jusqu_a" && n.valide_jusqu_au.is_none() {
            return invalide("une validité « jusqu'à » demande une date");
        }
        if n.nature == "information" && n.regle == "tous" {
            return invalide("une information ne se refuse pas : règle `tous` interdite");
        }
        if n.desaccord_objet.as_deref().is_some_and(|d| !["image", "sortie", "general"].contains(&d)) {
            return invalide("famille de désaccord inconnue (image, sortie ou general)");
        }
        // FD-17 : l'image d'un élève exige l'accord de tous les titulaires, jamais « un seul accord suffit ».
        if n.sujet_type == "eleve" && objet_exige_tous(&n.code, n.desaccord_objet.as_deref()) && n.regle != "tous" {
            return Err(EntDbError::ConflitDossier(
                "le droit à l'image exige la règle `tous` : l'accord de chaque titulaire de l'autorité parentale",
            ));
        }
        if n.usages.is_empty() || n.usages.len() > 30 {
            return invalide("au moins un usage (30 au plus)");
        }
        let mut codes = HashSet::new();
        for u in &n.usages {
            if !code_valide(&u.code) || u.libelle.trim().is_empty() || !codes.insert(u.code.clone()) {
                return invalide("usage invalide ou en double");
            }
        }
        if let Err(m) = controle_nature_usages(&n.nature, n.usages.iter().filter(|u| u.mission).count(), n.usages.len()) {
            return invalide(m);
        }
        if let Some(a) = n.avis_eleve_des_ans {
            if !(6..=17).contains(&a) || n.nature != "consentement" {
                return invalide("l'avis de l'élève ne concerne qu'un consentement (6 à 17 ans)");
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if objet_par_code(&mut tx, etab, &n.code).await?.is_some() {
            return Err(EntDbError::ConflitDossier("un objet porte déjà ce code"));
        }
        // R-FC6 : monter, jamais baisser sous la règle du catalogue pour la même famille (même code).
        let cat: Option<(String, String)> = sqlx::query_as("SELECT regle_min, nature FROM ent_acc_catalogue WHERE code = $1")
            .bind(&n.code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
        if let Some((min, nature_cat)) = &cat {
            if *nature_cat != n.nature || !regle_ne_baisse_pas(min, &n.regle) {
                return Err(EntDbError::ConflitDossier(
                    "la règle d'accord ne peut pas baisser sous celle du catalogue (monter seulement)",
                ));
            }
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_acc_objet
                (id, etablissement_id, code, libelle, nature, regle, avis_eleve_des_ans, droit_lien_code, preuve_min, validite,
                 valide_jusqu_au, evenement_fin, sujet_type, catalogue_code, cree_par, desaccord_objet)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
        )
        .bind(id)
        .bind(etab)
        .bind(&n.code)
        .bind(&libelle)
        .bind(&n.nature)
        .bind(&n.regle)
        .bind(n.avis_eleve_des_ans)
        .bind(&n.droit_lien_code)
        .bind(&n.preuve_min)
        .bind(&n.validite)
        .bind(n.valide_jusqu_au)
        .bind(n.evenement_fin)
        .bind(&n.sujet_type)
        .bind(cat.as_ref().map(|_| n.code.clone()))
        .bind(acteur)
        .bind(&n.desaccord_objet)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let mut tous = BTreeSet::new();
        for (i, u) in n.usages.iter().enumerate() {
            sqlx::query(
                "INSERT INTO ent_acc_usage (id, etablissement_id, objet_id, code, libelle, mission, ordre)
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(&u.code)
            .bind(u.libelle.trim())
            .bind(u.mission)
            .bind(i as i16)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            tous.insert(u.code.clone());
        }
        let version_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_acc_version (id, etablissement_id, objet_id, numero, texte, empreinte, cree_par)
             VALUES ($1,$2,$3,1,$4,$5,$6)",
        )
        .bind(version_id)
        .bind(etab)
        .bind(id)
        .bind(&texte)
        .bind(empreinte_version(&n.code, 1, &texte, &tous))
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_acc_version_usage (etablissement_id, version_id, usage_id)
             SELECT $1, $2, id FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $3",
        )
        .bind(etab)
        .bind(version_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, "objet_cree", Some(id), Some(version_id), None, None, acteur, None, Some(&n.code)).await?;
        publier_evt(&mut tx, etab, evenements::OBJET_ACTIVE, id, None, acteur, json!({ "objet_id": id })).await?;
        tx.commit().await.map_err(db)?;
        self.accords_objet(etab, &n.code).await
    }

    pub(crate) async fn accords_objet(&self, etab: Uuid, code: &str) -> Result<ObjetAccord, EntDbError> {
        let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        sqlx::query_as(&OBJET_VUE_SQL.replace("{FILTRE}", "o.code = $2"))
            .bind(etab)
            .bind(code)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?
            .ok_or_else(introuvable)
    }

    /// Objets d'accord de l'établissement (actifs et désactivés).
    pub async fn accords_objets(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<Vec<ObjetAccord>, EntDbError> {
        self.exiger_gestion(etab, acteur, role).await?;
        sqlx::query_as(&OBJET_VUE_SQL.replace("{FILTRE}", "true"))
            .bind(etab)
            .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await
            .map_err(db)
    }

    /// Usages d'un objet.
    pub async fn accords_usages(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
    ) -> Result<Vec<UsageAccord>, EntDbError> {
        self.exiger_gestion(etab, acteur, role).await?;
        sqlx::query_as(
            "SELECT u.code, u.libelle, u.mission FROM ent_acc_usage u
               JOIN ent_acc_objet o ON o.etablissement_id = u.etablissement_id AND o.id = u.objet_id
              WHERE u.etablissement_id = $1 AND o.code = $2 ORDER BY u.ordre, u.code",
        )
        .bind(etab)
        .bind(objet_code)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }

    /// Désactiver un objet (FRM-107, fin d'usage) : les décisions restent ; plus aucun usage n'est permis (`inconnu`).
    pub async fn accords_desactiver_objet(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
    ) -> Result<(), EntDbError> {
        self.exiger_accords_parametrer(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let o = objet_par_code(&mut tx, etab, objet_code).await?.ok_or_else(introuvable)?;
        if !o.actif {
            return Err(EntDbError::ConflitDossier("cet objet est déjà désactivé"));
        }
        sqlx::query("UPDATE ent_acc_objet SET actif = false, modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(o.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journaliser(&mut tx, etab, "objet_desactive", Some(o.id), None, None, None, acteur, None, Some(objet_code)).await?;
        publier_evt(&mut tx, etab, evenements::OBJET_DESACTIVE, o.id, None, acteur, json!({ "objet_id": o.id })).await?;
        tx.commit().await.map_err(db)
    }

    // -----------------------------------------------------------------------
    // Versions (FRM-102, FRM-103, FRM-104, FRM-109)
    // -----------------------------------------------------------------------

    /// Rédiger (ou reprendre) le brouillon de la prochaine version : texte et usages couverts (FRM-102). Des usages peuvent
    /// être ajoutés à l'objet ; publier une version qui en ajoute exige une qualification de **fond** (CK-32).
    pub async fn accords_rediger_version(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
        r: RedactionVersion,
    ) -> Result<VersionAccord, EntDbError> {
        self.exiger_accords_parametrer(etab, acteur, role).await?;
        let texte = r.texte.trim().to_string();
        if texte.is_empty() || texte.chars().count() > 10_000 {
            return Err(EntDbError::SaisieInvalide("texte invalide (1 à 10 000 caractères)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let o = objet_par_code(&mut tx, etab, objet_code).await?.ok_or_else(introuvable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 14))")
            .bind(format!("{etab}:{}", o.id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        // Usages nouveaux de l'objet.
        for u in &r.nouveaux_usages {
            if !code_valide(&u.code) || u.libelle.trim().is_empty() {
                return Err(EntDbError::SaisieInvalide("usage invalide"));
            }
            sqlx::query(
                "INSERT INTO ent_acc_usage (id, etablissement_id, objet_id, code, libelle, mission, ordre)
                 VALUES ($1,$2,$3,$4,$5,$6, (SELECT COALESCE(max(ordre), 0) + 1 FROM ent_acc_usage WHERE etablissement_id = $2 AND objet_id = $3))
                 ON CONFLICT (etablissement_id, objet_id, code) DO NOTHING",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(o.id)
            .bind(&u.code)
            .bind(u.libelle.trim())
            .bind(u.mission)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let voulus: BTreeSet<String> = r.usages.iter().cloned().chain(r.nouveaux_usages.iter().map(|u| u.code.clone())).collect();
        if voulus.is_empty() || voulus.len() > 40 {
            return Err(EntDbError::SaisieInvalide("au moins un usage couvert (40 au plus)"));
        }
        let connus: Vec<(Uuid, String, bool)> = sqlx::query_as(
            "SELECT id, code, mission FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $2",
        )
        .bind(etab)
        .bind(o.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let par_code: HashMap<&str, (Uuid, bool)> = connus.iter().map(|(i, c, m)| (c.as_str(), (*i, *m))).collect();
        if voulus.iter().any(|c| !par_code.contains_key(c.as_str())) {
            return Err(EntDbError::SaisieInvalide("usage inconnu pour cet objet"));
        }
        let missions = voulus.iter().filter(|c| par_code[c.as_str()].1).count();
        if let Err(m) = controle_nature_usages(&o.nature, missions, voulus.len()) {
            return Err(EntDbError::SaisieInvalide(m));
        }
        // Brouillon existant, sinon prochain numéro.
        let brouillon: Option<(Uuid, i32)> = sqlx::query_as(
            "SELECT id, numero FROM ent_acc_version WHERE etablissement_id = $1 AND objet_id = $2 AND publiee_le IS NULL",
        )
        .bind(etab)
        .bind(o.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (version_id, numero) = match brouillon {
            Some((id, n)) => {
                sqlx::query("DELETE FROM ent_acc_version_usage WHERE etablissement_id = $1 AND version_id = $2")
                    .bind(etab)
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                (id, n)
            }
            None => {
                let (n,): (i32,) = sqlx::query_as(
                    "SELECT COALESCE(max(numero), 0) + 1 FROM ent_acc_version WHERE etablissement_id = $1 AND objet_id = $2",
                )
                .bind(etab)
                .bind(o.id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_acc_version (id, etablissement_id, objet_id, numero, texte, empreinte, cree_par)
                     VALUES ($1,$2,$3,$4,$5,$6,$7)",
                )
                .bind(id)
                .bind(etab)
                .bind(o.id)
                .bind(n)
                .bind(&texte)
                .bind(empreinte_version(objet_code, n, &texte, &voulus))
                .bind(acteur)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                (id, n)
            }
        };
        sqlx::query("UPDATE ent_acc_version SET texte = $3, empreinte = $4 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(version_id)
            .bind(&texte)
            .bind(empreinte_version(objet_code, numero, &texte, &voulus))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        for c in &voulus {
            sqlx::query("INSERT INTO ent_acc_version_usage (etablissement_id, version_id, usage_id) VALUES ($1,$2,$3)")
                .bind(etab)
                .bind(version_id)
                .bind(par_code[c.as_str()].0)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        journaliser(&mut tx, etab, "version_redigee", Some(o.id), Some(version_id), None, None, acteur, None, None).await?;
        tx.commit().await.map_err(db)?;
        self.accords_version(etab, version_id).await
    }

    async fn accords_version(&self, etab: Uuid, version_id: Uuid) -> Result<VersionAccord, EntDbError> {
        let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let v: Option<(Uuid, i32, String, Option<String>, Option<DateTime<Utc>>, Option<Uuid>, Option<DateTime<Utc>>)> =
            sqlx::query_as(
                "SELECT id, numero, texte, type_changement, publiee_le, publiee_par, texte_relu_le
                   FROM ent_acc_version WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(version_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
        let (id, numero, texte, type_changement, publiee_le, publiee_par, texte_relu_le) = v.ok_or_else(introuvable)?;
        let usages: Vec<(String,)> = sqlx::query_as(
            "SELECT u.code FROM ent_acc_version_usage vu
               JOIN ent_acc_usage u ON u.etablissement_id = vu.etablissement_id AND u.id = vu.usage_id
              WHERE vu.etablissement_id = $1 AND vu.version_id = $2 ORDER BY u.ordre, u.code",
        )
        .bind(etab)
        .bind(version_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(VersionAccord {
            id,
            numero,
            texte,
            type_changement,
            publiee_le,
            publiee_par,
            texte_relu_le,
            usages: usages.into_iter().map(|(c,)| c).collect(),
        })
    }

    /// Historique des versions et de ce que chacune couvrait (FRM-104).
    pub async fn accords_historique(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
    ) -> Result<Vec<VersionAccord>, EntDbError> {
        self.exiger_gestion(etab, acteur, role).await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT v.id FROM ent_acc_version v JOIN ent_acc_objet o ON o.etablissement_id = v.etablissement_id AND o.id = v.objet_id
              WHERE v.etablissement_id = $1 AND o.code = $2 ORDER BY v.numero",
        )
        .bind(etab)
        .bind(objet_code)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        for (id,) in ids {
            sortie.push(self.accords_version(etab, id).await?);
        }
        Ok(sortie)
    }

    /// Publier le brouillon (FRM-103). Version 1 : sans qualification. Ensuite : `forme` (accords valables, personne n'est
    /// sollicité), `fond` (nouvelle demande aux décideurs), `restriction` (usage retiré). **Un usage ajouté force le fond**
    /// (CK-32) ; absent, la qualification est `fond` par prudence (R-FC19).
    pub async fn accords_publier_version(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
        type_changement: Option<&str>,
    ) -> Result<VersionAccord, EntDbError> {
        self.exiger_accords_parametrer(etab, acteur, role).await?;
        if type_changement.is_some_and(|c| !["forme", "fond", "restriction"].contains(&c)) {
            return Err(EntDbError::SaisieInvalide("changement : forme, fond ou restriction"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let o = objet_par_code(&mut tx, etab, objet_code).await?.ok_or_else(introuvable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 14))")
            .bind(format!("{etab}:{}", o.id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let brouillon: (Uuid, i32, String) = sqlx::query_as(
            "SELECT id, numero, texte FROM ent_acc_version
              WHERE etablissement_id = $1 AND objet_id = $2 AND publiee_le IS NULL",
        )
        .bind(etab)
        .bind(o.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or(EntDbError::ConflitDossier("aucun brouillon à publier"))?;
        let (version_id, numero, texte) = brouillon;
        let usages_v = |v: Uuid| {
            sqlx::query_as::<_, (String,)>(
                "SELECT u.code FROM ent_acc_version_usage vu
                   JOIN ent_acc_usage u ON u.etablissement_id = vu.etablissement_id AND u.id = vu.usage_id
                  WHERE vu.etablissement_id = $1 AND vu.version_id = $2",
            )
            .bind(etab)
            .bind(v)
        };
        let nouveaux: BTreeSet<String> = usages_v(version_id).fetch_all(&mut *tx).await.map_err(db)?.into_iter().map(|(c,)| c).collect();
        if nouveaux.is_empty() {
            return Err(EntDbError::ConflitDossier("la version ne couvre aucun usage"));
        }
        let precedente = version_en_cours(&mut tx, etab, o.id).await?;
        let retenu: Option<&str> = match &precedente {
            None => {
                if type_changement.is_some() {
                    return Err(EntDbError::SaisieInvalide("une première version n'a pas de qualification de changement"));
                }
                None
            }
            Some((prec_id, _, _, _, _)) => {
                let anciens: BTreeSet<String> =
                    usages_v(*prec_id).fetch_all(&mut *tx).await.map_err(db)?.into_iter().map(|(c,)| c).collect();
                let ajoutes = nouveaux.difference(&anciens).count();
                let retires = anciens.difference(&nouveaux).count();
                let choisi = type_changement.unwrap_or("fond");
                if ajoutes > 0 && choisi != "fond" {
                    return Err(EntDbError::ConflitDossier(
                        "un usage ajouté exige une version de fond : les familles doivent être sollicitées de nouveau",
                    ));
                }
                if choisi == "restriction" && retires == 0 {
                    return Err(EntDbError::ConflitDossier("une restriction retire au moins un usage"));
                }
                Some(match choisi {
                    "forme" => "forme",
                    "restriction" => "restriction",
                    _ => "fond",
                })
            }
        };
        sqlx::query(
            "UPDATE ent_acc_version SET publiee_le = now(), publiee_par = $3, type_changement = $4, empreinte = $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(version_id)
        .bind(acteur)
        .bind(retenu)
        .bind(empreinte_version(objet_code, numero, &texte, &nouveaux))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, "version_publiee", Some(o.id), Some(version_id), None, None, acteur, None, retenu).await?;
        let evt = publier_evt(
            &mut tx,
            etab,
            evenements::VERSION_PUBLIEE,
            o.id,
            None,
            acteur,
            json!({ "objet_id": o.id, "version": numero, "type": retenu }),
        )
        .await?;
        // Version de fond : nouvelle demande aux décideurs qui avaient décidé (enveloppe neutre, sans titre).
        if retenu == Some("fond") {
            let decideurs: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
                "SELECT DISTINCT d.decideur_user_id, d.sujet_eleve_id FROM ent_acc_decision d
                  WHERE d.etablissement_id = $1 AND d.objet_id = $2 AND d.decision <> 'annulee_par_etablissement'
                    AND d.canal NOT IN ('reprise_sans_preuve')",
            )
            .bind(etab)
            .bind(o.id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let maintenant = Utc::now();
            let mut deja = HashSet::new();
            for (user_id, eleve) in decideurs {
                if !deja.insert(user_id) {
                    continue;
                }
                notifier_personne(
                    &mut tx,
                    EnvoiAPlanifier {
                        etablissement_id: etab,
                        user_id,
                        kind: "formulaire",
                        titre: "Un formulaire vous attend",
                        corps: "Un accord à renouveler vous attend dans l'ENT (rubrique Mes accords).",
                        eleve_id: eleve,
                        notification_id: None,
                        source_type: "systeme",
                        source_id: o.id,
                        cle_dedup: format!("acc:{}:{numero}", o.id),
                        droit_requis: if eleve.is_some() { Some("lien") } else { None },
                        regroupable: true,
                        canaux_voulus: &["email", "push"],
                    },
                    evt,
                    maintenant,
                )
                .await?;
            }
        }
        tx.commit().await.map_err(db)?;
        self.accords_version(etab, version_id).await
    }

    /// Marquer le texte d'une version comme relu (juriste ou direction selon la nature, R-FC72) : fin du « texte
    /// provisoire ».
    pub async fn accords_relire_version(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
        numero: i32,
    ) -> Result<VersionAccord, EntDbError> {
        self.exiger_accords_parametrer(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let o = objet_par_code(&mut tx, etab, objet_code).await?.ok_or_else(introuvable)?;
        let v: Option<(Uuid, bool)> = sqlx::query_as(
            "SELECT id, publiee_le IS NOT NULL FROM ent_acc_version WHERE etablissement_id = $1 AND objet_id = $2 AND numero = $3",
        )
        .bind(etab)
        .bind(o.id)
        .bind(numero)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (version_id, publiee) = v.ok_or_else(introuvable)?;
        if !publiee {
            return Err(EntDbError::ConflitDossier("relire après publication seulement"));
        }
        sqlx::query(
            "UPDATE ent_acc_version SET texte_relu_le = now(), texte_relu_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND texte_relu_le IS NULL",
        )
        .bind(etab)
        .bind(version_id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, "version_relue", Some(o.id), Some(version_id), None, None, acteur, None, None).await?;
        tx.commit().await.map_err(db)?;
        self.accords_version(etab, version_id).await
    }

    /// Aperçu de ce que voit un décideur (FRM-109) : le texte de la version en cours et les usages à choisir, par profil
    /// (`titulaire`, `eleve`, `majeur`).
    pub async fn accords_apercu(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
        profil: &str,
    ) -> Result<(String, Vec<UsageAccord>, bool), EntDbError> {
        self.exiger_gestion(etab, acteur, role).await?;
        if !["titulaire", "eleve", "majeur"].contains(&profil) {
            return Err(EntDbError::SaisieInvalide("profil : titulaire, eleve ou majeur"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let o = objet_par_code(&mut tx, etab, objet_code).await?.ok_or_else(introuvable)?;
        let (version_id, _, texte, _, relue) = version_en_cours(&mut tx, etab, o.id).await?.ok_or_else(introuvable)?;
        let usages: Vec<UsageAccord> = sqlx::query_as(
            "SELECT u.code, u.libelle, u.mission FROM ent_acc_version_usage vu
               JOIN ent_acc_usage u ON u.etablissement_id = vu.etablissement_id AND u.id = vu.usage_id
              WHERE vu.etablissement_id = $1 AND vu.version_id = $2 AND NOT u.mission ORDER BY u.ordre, u.code",
        )
        .bind(etab)
        .bind(version_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok((texte, usages, !relue))
    }

    /// Décisions par décideur et preuves, pour un élève et un objet (secrétariat, direction : `autorisations.gerer`).
    /// **Lecture journalisée** (FD-09, R-FC38).
    pub async fn accords_lire_decisions(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet_code: &str,
        eleve_id: Uuid,
    ) -> Result<Vec<DecisionDetail>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::AutorisationsGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let o = objet_par_code(&mut tx, etab, objet_code).await?.ok_or_else(introuvable)?;
        let lignes: Vec<DecisionDetail> = sqlx::query_as(
            "SELECT d.id AS decision_id, d.decideur_user_id, d.qualite, d.decision, d.canal, v.numero AS version,
                    d.decide_le, d.preuve_signature_id
               FROM ent_acc_decision d JOIN ent_acc_version v ON v.etablissement_id = d.etablissement_id AND v.id = d.version_id
              WHERE d.etablissement_id = $1 AND d.objet_id = $2 AND d.sujet_eleve_id = $3
              ORDER BY d.decide_le DESC, d.id DESC",
        )
        .bind(etab)
        .bind(o.id)
        .bind(eleve_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, "lecture_detaillee", Some(o.id), None, Some(eleve_id), None, acteur, Some(lignes.len() as i32), Some("decisions"))
            .await?;
        tx.commit().await.map_err(db)?;
        Ok(lignes)
    }

    // -----------------------------------------------------------------------
    // Décider, retirer (FRM-113 à FRM-116)
    // -----------------------------------------------------------------------

    /// Qualité sous laquelle `user` décide pour le sujet, ou `None` (indiscernable d'un inconnu).
    pub(crate) async fn qualite_decideur(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        eleve: Uuid,
        objet: &ObjetLigne,
        jour: NaiveDate,
    ) -> Result<Option<&'static str>, EntDbError> {
        let naissance: Option<(Option<NaiveDate>, bool)> = sqlx::query_as(
            "SELECT date_naissance, ent_eleve_decide_seul($1, id) FROM ent_eleve WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some((naissance, decide_seul)) = naissance else {
            return Ok(None);
        };
        let compte: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM ent_eleve_compte WHERE etablissement_id = $1 AND eleve_id = $2 AND user_id = $3",
        )
        .bind(etab)
        .bind(eleve)
        .bind(user)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if decide_seul {
            return Ok(compte.map(|_| "eleve_majeur"));
        }
        if compte.is_some() {
            // Avis de l'enfant (R-FC51) : par son propre compte ; co-signature ou décision propre selon la règle.
            return Ok(match objet.regle.as_str() {
                "eleve_seul" | "eleve_et_titulaires" => Some("eleve_cosignataire"),
                _ => {
                    let age = naissance.map(|n| age_au(n, jour));
                    match (objet.nature.as_str(), objet.avis_eleve_des_ans, age) {
                        ("consentement", Some(des), Some(age)) if age >= i32::from(des) => Some("eleve_avis"),
                        _ => None,
                    }
                }
            });
        }
        if objet.regle == "eleve_seul" {
            return Ok(None);
        }
        let cond = condition_droit_lien(objet.droit_lien_code.as_deref());
        let titulaire: Option<(i32,)> = sqlx::query_as(&format!(
            "SELECT 1 FROM ent_eleve_lien_communicant l
              WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND l.user_id = $3 AND l.autorite_parentale AND {cond}"
        ))
        .bind(etab)
        .bind(eleve)
        .bind(user)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        Ok(titulaire.map(|_| "titulaire"))
    }

    /// Décider (FRM-113 à FRM-115) : usage par usage, sur la version publiée en cours. Le décideur et sa qualité sont
    /// **calculés**. Retourne **sa** décision, jamais celle d'un autre (R-FC16). Voir [`PgEnt::accords_decider_avec`] pour
    /// brancher la brique de signature.
    pub async fn accords_decider(&self, etab: Uuid, user: Uuid, d: DecisionSaisie) -> Result<MaDecision, EntDbError> {
        self.accords_decider_avec(etab, user, d, &SignatureAbsente).await
    }

    /// Décider avec un port de signature (socle 7, lot D6). Une preuve exigée que le port ne peut pas fournir (S2) refuse
    /// la décision plutôt que de la simuler.
    pub async fn accords_decider_avec(
        &self,
        etab: Uuid,
        user: Uuid,
        d: DecisionSaisie,
        signature: &dyn PortSignature,
    ) -> Result<MaDecision, EntDbError> {
        self.decider_interne(etab, user, d, signature, false).await
    }

    /// Retirer un accord (FRM-116), usage par usage ou en entier : **aussi simple que le don**, sans passkey, effet
    /// immédiat (R-FC15). Après le départ d'une sortie, un retrait est une demande à l'établissement (R-FC14).
    pub async fn accords_retirer(
        &self,
        etab: Uuid,
        user: Uuid,
        objet_code: &str,
        eleve_id: Option<Uuid>,
        usages: Option<Vec<String>>,
    ) -> Result<MaDecision, EntDbError> {
        let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let o = objet_par_code(&mut conn, etab, objet_code).await?.ok_or_else(introuvable)?;
        let sujet = self.sujet_de(&mut conn, etab, user, &o, eleve_id).await?;
        let accordes: Vec<(String,)> = sqlx::query_as(
            "SELECT us.code FROM (
                SELECT DISTINCT ON (u.usage_id) u.usage_id, u.accorde
                  FROM ent_acc_decision d
                  JOIN ent_acc_decision_usage u ON u.decision_id = d.id AND u.etablissement_id = d.etablissement_id
                 WHERE d.etablissement_id = $1 AND d.objet_id = $2 AND d.decideur_user_id = $3
                   AND COALESCE(d.sujet_eleve_id, d.sujet_personne_id) = $4
                   AND d.decision <> 'annulee_par_etablissement'
                 ORDER BY u.usage_id, d.decide_le DESC, d.id DESC) r
              JOIN ent_acc_usage us ON us.id = r.usage_id AND us.etablissement_id = $1
             WHERE r.accorde ORDER BY us.code",
        )
        .bind(etab)
        .bind(o.id)
        .bind(user)
        .bind(sujet)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        drop(conn);
        let choisis: Vec<String> = match usages {
            Some(u) => u,
            None => accordes.iter().map(|(c,)| c.clone()).collect(),
        };
        let a_retirer: Vec<ChoixUsage> = choisis
            .into_iter()
            .filter(|c| accordes.iter().any(|(a,)| a == c))
            .map(|code| ChoixUsage { code, accorde: false })
            .collect();
        if a_retirer.is_empty() {
            return Err(EntDbError::ConflitDossier("aucun accord à retirer"));
        }
        self.decider_interne(etab, user, DecisionSaisie { objet_code: objet_code.to_string(), eleve_id, usages: a_retirer }, &SignatureAbsente, true)
            .await
    }

    /// Sujet de la décision : l'élève (objet d'élève) ou la personne du compte (objet `soi`).
    async fn sujet_de(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        user: Uuid,
        objet: &ObjetLigne,
        eleve_id: Option<Uuid>,
    ) -> Result<Uuid, EntDbError> {
        if objet.sujet_type == "personne" {
            if eleve_id.is_some() {
                return Err(EntDbError::SaisieInvalide("cet accord est personnel : pas d'élève"));
            }
            let p: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
            )
            .bind(etab)
            .bind(user)
            .fetch_optional(&mut *conn)
            .await
            .map_err(db)?;
            return p.map(|(i,)| i).ok_or_else(introuvable);
        }
        eleve_id.ok_or(EntDbError::SaisieInvalide("élève requis pour cet accord"))
    }

    async fn decider_interne(
        &self,
        etab: Uuid,
        user: Uuid,
        d: DecisionSaisie,
        signature: &dyn PortSignature,
        retrait: bool,
    ) -> Result<MaDecision, EntDbError> {
        let tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (tx, m) = self.decider_dans_tx(tx, etab, user, d, Preuve::Port(signature), retrait, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(m)
    }

    /// Cœur d'une décision, **dans la transaction de l'appelant** (réponse à un formulaire : les décisions et la soumission
    /// s'écrivent ensemble ou pas du tout). Rend la transaction, que l'appelant valide.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn decider_dans_tx(
        &self,
        mut tx: TenantTx<'static>,
        etab: Uuid,
        user: Uuid,
        d: DecisionSaisie,
        preuve_demandee: Preuve<'_>,
        retrait: bool,
        soumission_id: Option<Uuid>,
    ) -> Result<(TenantTx<'static>, MaDecision), EntDbError> {
        if d.usages.is_empty() || d.usages.len() > 40 {
            return Err(EntDbError::SaisieInvalide("au moins un usage (40 au plus)"));
        }
        let mut vus = HashSet::new();
        if d.usages.iter().any(|u| !code_valide(&u.code) || !vus.insert(u.code.clone())) {
            return Err(EntDbError::SaisieInvalide("usage invalide ou en double"));
        }
        let o = objet_par_code(&mut tx, etab, &d.objet_code).await?.ok_or_else(introuvable)?;
        if !o.actif {
            return Err(EntDbError::ConflitDossier("cet accord n'est plus demandé"));
        }
        let jour = self.aujourdhui_etablissement(etab).await?;
        let sujet = self.sujet_de(&mut tx, etab, user, &o, d.eleve_id).await?;
        let qualite: &'static str = if o.sujet_type == "personne" {
            "personne_soi"
        } else {
            self.qualite_decideur(&mut tx, etab, user, sujet, &o, jour)
                .await?
                .ok_or_else(introuvable)?
        };
        // Après l'événement, un retrait d'autorisation est une demande à l'établissement (R-FC14, CK-35).
        if retrait && o.nature == "autorisation" && o.evenement_fin.is_some_and(|f| f < Utc::now()) {
            return Err(EntDbError::ConflitDossier(
                "l'événement a commencé : adressez votre demande à l'établissement",
            ));
        }
        let (version_id, numero, _, empreinte_v, _) =
            version_en_cours(&mut tx, etab, o.id).await?.ok_or(EntDbError::ConflitDossier("aucune version publiée"))?;
        // Usages couverts par la version en cours, hors mission (« non requis », rien à décider).
        let couverts: Vec<(Uuid, String, bool)> = sqlx::query_as(
            "SELECT u.id, u.code, u.mission FROM ent_acc_version_usage vu
               JOIN ent_acc_usage u ON u.etablissement_id = vu.etablissement_id AND u.id = vu.usage_id
              WHERE vu.etablissement_id = $1 AND vu.version_id = $2",
        )
        .bind(etab)
        .bind(version_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut choix: Vec<(Uuid, &str, bool)> = Vec::new();
        for u in &d.usages {
            let Some((id, _, mission)) = couverts.iter().find(|(_, c, _)| *c == u.code) else {
                return Err(EntDbError::SaisieInvalide("usage non couvert par la version en cours"));
            };
            if *mission {
                return Err(EntDbError::SaisieInvalide("usage de mission : aucun accord à donner"));
            }
            choix.push((*id, u.code.as_str(), u.accorde));
        }
        // Les décisions d'un même décideur pour un même sujet et objet se sérialisent.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 13))")
            .bind(format!("{etab}:{user}:{sujet}:{}", o.id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        // Dernière décision de ce décideur par usage : on n'écrit que ce qui change.
        let dernieres: HashMap<Uuid, bool> = sqlx::query_as::<_, (Uuid, bool)>(
            "SELECT DISTINCT ON (u.usage_id) u.usage_id, u.accorde
               FROM ent_acc_decision x
               JOIN ent_acc_decision_usage u ON u.decision_id = x.id AND u.etablissement_id = x.etablissement_id
              WHERE x.etablissement_id = $1 AND x.objet_id = $2 AND x.decideur_user_id = $3
                AND COALESCE(x.sujet_eleve_id, x.sujet_personne_id) = $4 AND x.decision <> 'annulee_par_etablissement'
              ORDER BY u.usage_id, x.decide_le DESC, x.id DESC",
        )
        .bind(etab)
        .bind(o.id)
        .bind(user)
        .bind(sujet)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .collect();
        let changes: Vec<&(Uuid, &str, bool)> = choix.iter().filter(|(id, _, acc)| dernieres.get(id) != Some(acc)).collect();
        let usages_sortie: Vec<(String, bool)> = choix.iter().map(|(_, c, a)| (c.to_string(), *a)).collect();
        if changes.is_empty() {
            return Ok((
                tx,
                MaDecision {
                    decision_id: Uuid::nil(),
                    decision: "inchangee".to_string(),
                    qualite: qualite.to_string(),
                    version: numero,
                    usages: usages_sortie,
                },
            ));
        }
        // Valeur de la décision : `retire` (tout ce qui change était accordé), `refuse` (rien d'accordé), sinon `accorde`.
        let tout_faux = changes.iter().all(|(_, _, a)| !*a);
        let etait_accorde = changes.iter().all(|(id, _, _)| dernieres.get(id) == Some(&true));
        let decision = if tout_faux && etait_accorde {
            "retire"
        } else if tout_faux {
            "refuse"
        } else {
            "accorde"
        };
        // Preuve (socle 7) : demandée pour un don, jamais pour un retrait (R-FC15). S2 sans brique : refus (pas de simulation).
        let (preuve, preuve_d6): (Option<Uuid>, Option<PreuveRetenue>) = if decision == "retire" || o.preuve_min == "S0" {
            (None, None)
        } else {
            match preuve_demandee {
                Preuve::Port(signature) => (
                    signature
                        .signer(DemandeSignature {
                            etablissement_id: etab,
                            objet_code: &o.code,
                            version_numero: numero,
                            empreinte_version: &empreinte_v,
                            decideur_user_id: user,
                            qualite,
                            niveau: &o.preuve_min,
                        })
                        .await?,
                    None,
                ),
                // Un refus ne demande aucune signature (il protège) : seule une décision qui accorde doit être prouvée.
                Preuve::D6(_) if decision == "refuse" => (None, None),
                Preuve::D6(signature_id) => {
                    let r = self
                        .verifier_signature_d6(&mut tx, etab, user, sujet, &o.code, o.id, version_id, &o.preuve_min, signature_id)
                        .await?;
                    (Some(signature_id), Some(r))
                }
            }
        };
        let maintenant = Utc::now();
        let id = Uuid::new_v4();
        let mut elements = vec![empreinte_v.clone(), sujet.to_string(), user.to_string(), qualite.to_string(), decision.to_string()];
        let mut lignes: Vec<String> = changes.iter().map(|(_, c, a)| format!("{c}={}", u8::from(*a))).collect();
        lignes.sort();
        elements.push(lignes.join(","));
        elements.push(maintenant.to_rfc3339());
        let refs: Vec<&str> = elements.iter().map(String::as_str).collect();
        let (eleve_sujet, personne_sujet) = if o.sujet_type == "personne" { (None, Some(sujet)) } else { (Some(sujet), None) };
        sqlx::query(
            "INSERT INTO ent_acc_decision
                (id, etablissement_id, objet_id, version_id, sujet_eleve_id, sujet_personne_id, decideur_user_id, qualite,
                 decision, canal, preuve_signature_id, decide_le, empreinte, formulaire_soumission_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(id)
        .bind(etab)
        .bind(o.id)
        .bind(version_id)
        .bind(eleve_sujet)
        .bind(personne_sujet)
        .bind(user)
        .bind(qualite)
        .bind(decision)
        .bind(preuve_d6.as_ref().map_or("en_ligne", |p| p.canal))
        .bind(preuve)
        .bind(maintenant)
        .bind(empreinte_encodee(&refs))
        .bind(soumission_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(p) = &preuve_d6 {
            sqlx::query(
                "INSERT INTO ent_acc_preuve (decision_id, etablissement_id, signature_id, circuit_id, niveau, methode)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(id)
            .bind(etab)
            .bind(p.signature_id)
            .bind(p.circuit_id)
            .bind(&p.niveau)
            .bind(&p.methode)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        for (usage_id, _, accorde) in changes.iter().map(|c| **c) {
            sqlx::query(
                "INSERT INTO ent_acc_decision_usage (etablissement_id, decision_id, usage_id, accorde) VALUES ($1,$2,$3,$4)",
            )
            .bind(etab)
            .bind(id)
            .bind(usage_id)
            .bind(accorde)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        journaliser(
            &mut tx,
            etab,
            if decision == "retire" { "retrait" } else { "decision" },
            Some(o.id),
            Some(version_id),
            eleve_sujet,
            personne_sujet,
            user,
            Some(changes.len() as i32),
            None,
        )
        .await?;
        let usages_changes: Vec<&str> = changes.iter().map(|(_, c, _)| *c).collect();
        publier_evt(
            &mut tx,
            etab,
            if decision == "retire" { evenements::RETIRE } else { evenements::DECISION },
            id,
            eleve_sujet,
            user,
            json!({ "objet_id": o.id, "objet": o.code, "version": numero, "decision_id": id, "usages": usages_changes }),
        )
        .await?;
        publier_evt(
            &mut tx,
            etab,
            evenements::ETAT_MODIFIE,
            o.id,
            eleve_sujet,
            user,
            json!({ "objet_id": o.id, "objet": o.code, "usages": usages_changes }),
        )
        .await?;
        // Règle `un` : un refus après un accord d'un autre titulaire est un désaccord connu (R-FC7) : événement à la direction.
        if decision == "refuse" && o.regle == "un" {
            let autres: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM ent_acc_decision x JOIN ent_acc_decision_usage u ON u.decision_id = x.id AND u.etablissement_id = x.etablissement_id
                  WHERE x.etablissement_id = $1 AND x.objet_id = $2 AND COALESCE(x.sujet_eleve_id, x.sujet_personne_id) = $3
                    AND x.decideur_user_id <> $4 AND u.accorde AND x.qualite = 'titulaire' LIMIT 1",
            )
            .bind(etab)
            .bind(o.id)
            .bind(sujet)
            .bind(user)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if autres.is_some() {
                publier_evt(&mut tx, etab, evenements::DESACCORD, o.id, eleve_sujet, user, json!({ "objet_id": o.id }))
                    .await?;
                // FO3 : le désaccord est **remonté** au dossier de l'élève (socle 4) pour l'image et la sortie : il bloque
                // alors tous les objets de la famille jusqu'à sa levée (pièce et motif).
                if let (Some(eleve), Some(famille)) = (eleve_sujet, o.desaccord_objet.as_deref()) {
                    if famille == "image" || famille == "sortie" {
                        self.ouvrir_desaccord(&mut tx, etab, eleve, famille, Some(o.id), "accord", user).await?;
                    }
                }
            }
        }
        Ok((tx, MaDecision { decision_id: id, decision: decision.to_string(), qualite: qualite.to_string(), version: numero, usages: usages_sortie }))
    }

    // -----------------------------------------------------------------------
    // Mes accords (FRM-117)
    // -----------------------------------------------------------------------

    /// « Mes accords » : par enfant (et pour moi-même) et par objet, les usages, **ma** décision, la version et la date.
    /// Jamais la décision d'un autre titulaire (R-FC16, R-FC58).
    pub async fn accords_mes_accords(&self, etab: Uuid, user: Uuid) -> Result<Vec<MesAccordsSujet>, EntDbError> {
        let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        let objets: Vec<ObjetLigne> = sqlx::query_as(&format!(
            "SELECT {COLONNES_OBJET} FROM ent_acc_objet WHERE etablissement_id = $1 AND actif ORDER BY libelle"
        ))
        .bind(etab)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        // Enfants dont je suis titulaire communicant, ou moi-même (compte élève).
        let enfants: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT DISTINCT e.id, e.prenom FROM ent_eleve_lien_communicant l
               JOIN ent_eleve e ON e.etablissement_id = l.etablissement_id AND e.id = l.eleve_id
              WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.autorite_parentale
             UNION
             SELECT e.id, e.prenom FROM ent_eleve_compte c
               JOIN ent_eleve e ON e.etablissement_id = c.etablissement_id AND e.id = c.eleve_id
              WHERE c.etablissement_id = $1 AND c.user_id = $2",
        )
        .bind(etab)
        .bind(user)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let personne: Option<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT id, prenom FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
        )
        .bind(etab)
        .bind(user)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        let mut sujets: Vec<(Option<Uuid>, Uuid, String)> =
            enfants.into_iter().map(|(id, p)| (Some(id), id, p)).collect();
        if let Some((pid, prenom)) = personne {
            sujets.push((None, pid, prenom.unwrap_or_default()));
        }
        for (eleve, sujet_id, prenom) in sujets {
            let mut accords = Vec::new();
            for o in &objets {
                if (o.sujet_type == "eleve") != eleve.is_some() {
                    continue;
                }
                let qualite: &'static str = match eleve {
                    None => "personne_soi",
                    Some(e) => match self.qualite_decideur(&mut conn, etab, user, e, o, jour).await? {
                        Some(q) => q,
                        None => continue,
                    },
                };
                let Some((version_id, numero, texte, _, relue)) = version_en_cours(&mut conn, etab, o.id).await? else {
                    continue;
                };
                // Mes dernières décisions par usage (jamais celles d'un autre décideur).
                let mes: Vec<(Uuid, bool, DateTime<Utc>, Option<Uuid>)> = sqlx::query_as(
                    "SELECT DISTINCT ON (u.usage_id) u.usage_id, u.accorde, x.decide_le, x.preuve_signature_id
                       FROM ent_acc_decision x
                       JOIN ent_acc_decision_usage u ON u.decision_id = x.id AND u.etablissement_id = x.etablissement_id
                      WHERE x.etablissement_id = $1 AND x.objet_id = $2 AND x.decideur_user_id = $3
                        AND COALESCE(x.sujet_eleve_id, x.sujet_personne_id) = $4 AND x.decision <> 'annulee_par_etablissement'
                      ORDER BY u.usage_id, x.decide_le DESC, x.id DESC",
                )
                .bind(etab)
                .bind(o.id)
                .bind(user)
                .bind(sujet_id)
                .fetch_all(&mut *conn)
                .await
                .map_err(db)?;
                let usages: Vec<(Uuid, String, String)> = sqlx::query_as(
                    "SELECT u.id, u.code, u.libelle FROM ent_acc_version_usage vu
                       JOIN ent_acc_usage u ON u.etablissement_id = vu.etablissement_id AND u.id = vu.usage_id
                      WHERE vu.etablissement_id = $1 AND vu.version_id = $2 AND NOT u.mission ORDER BY u.ordre, u.code",
                )
                .bind(etab)
                .bind(version_id)
                .fetch_all(&mut *conn)
                .await
                .map_err(db)?;
                if usages.is_empty() {
                    continue;
                }
                // Ma plus ancienne décision sur cet objet, et première version qui couvre chaque usage (CK-30).
                let (ma_premiere,): (Option<i32>,) = sqlx::query_as(
                    "SELECT min(v.numero) FROM ent_acc_decision x
                       JOIN ent_acc_version v ON v.etablissement_id = x.etablissement_id AND v.id = x.version_id
                      WHERE x.etablissement_id = $1 AND x.objet_id = $2 AND x.decideur_user_id = $3
                        AND COALESCE(x.sujet_eleve_id, x.sujet_personne_id) = $4 AND x.decision <> 'annulee_par_etablissement'",
                )
                .bind(etab)
                .bind(o.id)
                .bind(user)
                .bind(sujet_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                let couvertures: HashMap<Uuid, i32> = sqlx::query_as::<_, (Uuid, i32)>(
                    "SELECT vu.usage_id, min(v.numero) FROM ent_acc_version_usage vu
                       JOIN ent_acc_version v ON v.etablissement_id = vu.etablissement_id AND v.id = vu.version_id
                      WHERE vu.etablissement_id = $1 AND v.objet_id = $2 AND v.publiee_le IS NOT NULL GROUP BY vu.usage_id",
                )
                .bind(etab)
                .bind(o.id)
                .fetch_all(&mut *conn)
                .await
                .map_err(db)?
                .into_iter()
                .collect();
                accords.push(MonAccord {
                    objet_code: o.code.clone(),
                    libelle: o.libelle.clone(),
                    nature: o.nature.clone(),
                    version_courante: numero,
                    texte,
                    texte_provisoire: !relue,
                    qualite: qualite.to_string(),
                    usages: Vec::new(),
                });
                let dernier = accords.last_mut().expect("poussé ci-dessus");
                for (uid, code, libelle) in usages {
                    let m = mes.iter().find(|m| m.0 == uid);
                    dernier.usages.push(MonUsage {
                        code,
                        libelle,
                        ma_decision: m.map(|m| m.1),
                        decide_le: m.map(|m| m.2),
                        a_renouveler: m.is_none()
                            && ma_premiere.is_some_and(|mv| couvertures.get(&uid).is_some_and(|c| mv < *c)),
                        preuve: m.map(|m| if m.3.is_some() { "signature".to_string() } else { "en_ligne".to_string() }),
                    });
                }
            }
            if !accords.is_empty() {
                sortie.push(MesAccordsSujet { eleve_id: eleve, prenom, accords });
            }
        }
        Ok(sortie)
    }
}

// ---------------------------------------------------------------------------
// Reprise de l'existant (migrations 193 et 269/275) — R-FC43 : jamais « accordé » sans preuve
// ---------------------------------------------------------------------------

/// Objets de consentement administratif du socle 4 (`ent_personne_consentement`, 275) → (objet d'accord, usage).
const REPRISE_275: [(&str, &str, &str); 4] = [
    ("diffusion_associations_parents", "coordonnees_associations", "liste_parents_association"),
    ("diffusion_transport", "coordonnees_transport", "transporteur"),
    ("photo_remise", "photo_personne_autorisee", "remise_accueil"),
    ("annuaire_parents_classe", "liste_representants", "liste_classe"),
];

impl PgEnt {
    /// Reprend les formulaires, autorisations et consentements de l'ancien module (`ent_formulaire`,
    /// `ent_formulaire_reponse`) dans les objets, versions et décisions du socle 18. **Rejouable** : une ligne d'origine
    /// n'est reprise qu'une fois (clé `source_reprise`), l'ancien module reste en écriture jusqu'au retrait (FO13).
    ///
    /// * L'ancien texte n'avait pas d'usages : chaque objet repris porte un seul usage technique
    ///   (`reprise_ancien_module`). **Un accord ancien ne vaut donc jamais pour un usage précis** : il rend les usages
    ///   détaillés `a_renouveler` ; un **refus ancien protège tous les usages** (refus global hérité).
    /// * Les décisions restent des décisions de l'ancien module (canal `reprise_dossier`), avec leur date.
    pub async fn accords_reprendre_formulaires(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<BilanReprise, EntDbError> {
        self.exiger_accords_parametrer(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let mut bilan = BilanReprise::default();
        // Un objet, sa nature (celle de sa première version : FD-15 interdit désormais le mélange) et son dernier titre.
        let objets: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT DISTINCT ON (objet) objet, type, titre FROM (
                SELECT objet, type, titre, version FROM ent_formulaire
                 WHERE etablissement_id = $1 AND objet IS NOT NULL AND statut IN ('publie', 'archive')
                   AND type IN ('consentement', 'autorisation')) f
              ORDER BY objet, version DESC",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (code, nature, titre) in objets {
            if !code_valide(&code) {
                bilan.ignorees += 1;
                continue;
            }
            let existant = objet_par_code(&mut tx, etab, &code).await?;
            let objet_id = match existant {
                Some(o) => {
                    let porte_usage: Option<(i32,)> = sqlx::query_as(
                        "SELECT 1 FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $2 AND code = $3",
                    )
                    .bind(etab)
                    .bind(o.id)
                    .bind(USAGE_REPRISE)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
                    if porte_usage.is_none() {
                        // Objet déjà géré nativement sous ce code : jamais mélangé avec des décisions d'un autre modèle.
                        bilan.ignorees += 1;
                        continue;
                    }
                    o.id
                }
                None => {
                    let cat: Option<(String, String, Option<i16>, Option<String>, String, String, Option<String>)> = sqlx::query_as(
                        "SELECT nature, regle_min, avis_eleve_des_ans, droit_lien_code, preuve_min, validite, desaccord_objet
                           FROM ent_acc_catalogue WHERE code = $1 AND sujet_type = 'eleve'",
                    )
                    .bind(&code)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
                    if cat.as_ref().is_some_and(|c| c.0 != nature) {
                        bilan.ignorees += 1;
                        continue;
                    }
                    let (regle, avis, droit, preuve, validite, desaccord, du_catalogue) = match cat {
                        Some((_, r, a, d, p, v, z)) => (r, a, d, p, v, z, true),
                        None => (
                            if crate::formulaires::regle_par_defaut(Some(&code)) == crate::formulaires::Regle::Tous { "tous" } else { "un" }
                                .to_string(),
                            None,
                            Some(if nature == "autorisation" { "autoriser_sorties" } else { "signer_documents" }.to_string()),
                            "S1".to_string(),
                            if nature == "autorisation" { "evenement" } else { "annee_scolaire" }.to_string(),
                            None,
                            false,
                        ),
                    };
                    let id = Uuid::new_v4();
                    let mut libelle: String = titre.chars().take(120).collect();
                    if libelle.trim().is_empty() {
                        libelle = code.clone();
                    }
                    sqlx::query(
                        "INSERT INTO ent_acc_objet
                            (id, etablissement_id, code, libelle, nature, regle, avis_eleve_des_ans, droit_lien_code, preuve_min,
                             validite, sujet_type, catalogue_code, desaccord_objet, cree_par)
                         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'eleve',$11,$12,$13)",
                    )
                    .bind(id)
                    .bind(etab)
                    .bind(&code)
                    .bind(libelle.trim())
                    .bind(&nature)
                    .bind(&regle)
                    .bind(avis)
                    .bind(&droit)
                    .bind(&preuve)
                    .bind(&validite)
                    .bind(if du_catalogue { Some(code.clone()) } else { None })
                    .bind(&desaccord)
                    .bind(acteur)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    sqlx::query(
                        "INSERT INTO ent_acc_usage (id, etablissement_id, objet_id, code, libelle, mission, ordre)
                         VALUES ($1,$2,$3,$4,'Usage non précisé (repris de l''ancien module)', false, 0)",
                    )
                    .bind(Uuid::new_v4())
                    .bind(etab)
                    .bind(id)
                    .bind(USAGE_REPRISE)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    bilan.objets += 1;
                    id
                }
            };
            let usage_reprise: (Uuid,) = sqlx::query_as(
                "SELECT id FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $2 AND code = $3",
            )
            .bind(etab)
            .bind(objet_id)
            .bind(USAGE_REPRISE)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            // Versions (dans l'ordre : un numéro publié dépasse toujours les précédents).
            let versions: Vec<(i32, String, Option<String>, Option<DateTime<Utc>>, Option<Uuid>)> = sqlx::query_as(
                "SELECT version, texte, changement, publie_le, publie_par FROM ent_formulaire
                  WHERE etablissement_id = $1 AND objet = $2 AND statut IN ('publie', 'archive') AND publie_le IS NOT NULL
                  ORDER BY version",
            )
            .bind(etab)
            .bind(&code)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            for (numero, texte, changement, publie_le, publie_par) in versions {
                let deja: Option<(i32,)> = sqlx::query_as(
                    "SELECT 1 FROM ent_acc_version WHERE etablissement_id = $1 AND objet_id = $2 AND numero = $3",
                )
                .bind(etab)
                .bind(objet_id)
                .bind(numero)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
                if deja.is_some() {
                    continue;
                }
                let usages: BTreeSet<String> = [USAGE_REPRISE.to_string()].into_iter().collect();
                let vid = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO ent_acc_version (id, etablissement_id, objet_id, numero, texte, empreinte, cree_par)
                     VALUES ($1,$2,$3,$4,$5,$6,$7)",
                )
                .bind(vid)
                .bind(etab)
                .bind(objet_id)
                .bind(numero)
                .bind(&texte)
                .bind(empreinte_version(&code, numero, &texte, &usages))
                .bind(acteur)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                sqlx::query("INSERT INTO ent_acc_version_usage (etablissement_id, version_id, usage_id) VALUES ($1,$2,$3)")
                    .bind(etab)
                    .bind(vid)
                    .bind(usage_reprise.0)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                sqlx::query(
                    "UPDATE ent_acc_version SET publiee_le = COALESCE($3, now()), publiee_par = $4, type_changement = $5
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(vid)
                .bind(publie_le)
                .bind(publie_par)
                .bind(if numero > 1 { changement.or(Some("fond".to_string())) } else { None })
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                bilan.versions += 1;
            }
            // Décisions : une ligne d'origine n'est reprise qu'une fois.
            let inserees = sqlx::query(
                "INSERT INTO ent_acc_decision
                    (id, etablissement_id, objet_id, version_id, sujet_eleve_id, decideur_user_id, qualite, decision, canal,
                     decide_le, empreinte, source_reprise, source_reprise_id)
                 SELECT gen_random_uuid(), r.etablissement_id, $2, v.id, r.eleve_id, r.signataire_user_id,
                        CASE r.qualite WHEN 'eleve_majeur' THEN 'eleve_majeur' ELSE 'titulaire' END,
                        CASE r.decision WHEN 'accepte' THEN 'accorde' WHEN 'refuse' THEN 'refuse' ELSE 'retire' END,
                        'reprise_dossier', r.decide_le,
                        encode(sha256(convert_to('193:' || r.id::text || ':' || r.empreinte, 'UTF8')), 'hex'),
                        'formulaire_193', r.id
                   FROM ent_formulaire_reponse r
                   JOIN ent_acc_version v ON v.etablissement_id = r.etablissement_id AND v.objet_id = $2 AND v.numero = r.version
                  WHERE r.etablissement_id = $1 AND r.objet = $3
                 ON CONFLICT (source_reprise, source_reprise_id) WHERE source_reprise IS NOT NULL DO NOTHING",
            )
            .bind(etab)
            .bind(objet_id)
            .bind(&code)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
            sqlx::query(
                "INSERT INTO ent_acc_decision_usage (etablissement_id, decision_id, usage_id, accorde)
                 SELECT d.etablissement_id, d.id, $3, d.decision = 'accorde' FROM ent_acc_decision d
                  WHERE d.etablissement_id = $1 AND d.objet_id = $2 AND d.source_reprise = 'formulaire_193'
                    AND NOT EXISTS (SELECT 1 FROM ent_acc_decision_usage x WHERE x.decision_id = d.id)",
            )
            .bind(etab)
            .bind(objet_id)
            .bind(usage_reprise.0)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            bilan.decisions += inserees as usize;
            let (total,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_formulaire_reponse WHERE etablissement_id = $1 AND objet = $2",
            )
            .bind(etab)
            .bind(&code)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            bilan.deja_reprises += (total as usize).saturating_sub(inserees as usize);
        }
        journaliser(&mut tx, etab, "reprise", None, None, None, None, acteur, Some(bilan.decisions as i32), Some("formulaire_193")).await?;
        tx.commit().await.map_err(db)?;
        Ok(bilan)
    }

    /// Reprend les consentements administratifs du socle 4 (`ent_personne_consentement`, 275) : coordonnées aux
    /// associations de parents, au transporteur, photo de remise, annuaire de la classe. Chaque consentement devient une
    /// décision `personne_soi` sur l'objet du catalogue correspondant (texte de départ, **provisoire**), avec son canal
    /// d'origine ; un accord retiré sans remplacement donne un retrait daté.
    pub async fn accords_reprendre_consentements_personnes(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<BilanReprise, EntDbError> {
        self.exiger_accords_parametrer(etab, acteur, role).await?;
        let mut bilan = BilanReprise::default();
        for (ancien, objet_code, usage_code) in REPRISE_275 {
            // L'objet du catalogue doit exister et avoir une version publiée (texte de départ, à relire).
            let existe = {
                let mut c = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                objet_par_code(&mut c, etab, objet_code).await?
            };
            if existe.is_none() {
                self.accords_activer_objet(etab, acteur, role, objet_code).await?;
                bilan.objets += 1;
            }
            let publiee = {
                let mut c = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                let o = objet_par_code(&mut c, etab, objet_code).await?.ok_or_else(introuvable)?;
                version_en_cours(&mut c, etab, o.id).await?.is_some()
            };
            if !publiee {
                self.accords_publier_version(etab, acteur, role, objet_code, None).await?;
                bilan.versions += 1;
            }
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let o = objet_par_code(&mut tx, etab, objet_code).await?.ok_or_else(introuvable)?;
            let (version_id, _, _, _, _) = version_en_cours(&mut tx, etab, o.id).await?.ok_or_else(introuvable)?;
            let (usage_id,): (Uuid,) = sqlx::query_as(
                "SELECT id FROM ent_acc_usage WHERE etablissement_id = $1 AND objet_id = $2 AND code = $3",
            )
            .bind(etab)
            .bind(o.id)
            .bind(usage_code)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            let inserees = sqlx::query(
                "INSERT INTO ent_acc_decision
                    (id, etablissement_id, objet_id, version_id, sujet_personne_id, decideur_user_id, qualite, decision, canal,
                     date_signature_papier, saisie_par_user_id, decide_le, empreinte, source_reprise, source_reprise_id)
                 SELECT gen_random_uuid(), c.etablissement_id, $2, $3, c.personne_id, COALESCE(p.user_id, c.donne_par),
                        'personne_soi', CASE c.reponse WHEN 'accorde' THEN 'accorde' ELSE 'refuse' END,
                        CASE c.canal WHEN 'papier' THEN 'papier' WHEN 'guichet' THEN 'guichet' ELSE 'en_ligne' END,
                        CASE c.canal WHEN 'papier' THEN (c.du AT TIME ZONE 'Europe/Paris')::date END,
                        c.donne_par, c.du,
                        encode(sha256(convert_to('275:' || c.id::text || ':' || c.texte_version, 'UTF8')), 'hex'),
                        'personne_consentement_275', c.id
                   FROM ent_personne_consentement c
                   JOIN ent_personne p ON p.etablissement_id = c.etablissement_id AND p.id = c.personne_id
                  WHERE c.etablissement_id = $1 AND c.objet = $4 AND COALESCE(p.user_id, c.donne_par) IS NOT NULL
                 ON CONFLICT (source_reprise, source_reprise_id) WHERE source_reprise IS NOT NULL DO NOTHING",
            )
            .bind(etab)
            .bind(o.id)
            .bind(version_id)
            .bind(ancien)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
            // Accord fermé sans remplacement : retrait daté (la personne ou l'établissement a retiré son accord).
            let retraits = sqlx::query(
                "INSERT INTO ent_acc_decision
                    (id, etablissement_id, objet_id, version_id, sujet_personne_id, decideur_user_id, qualite, decision, canal,
                     saisie_par_user_id, decide_le, empreinte, source_reprise, source_reprise_id)
                 SELECT gen_random_uuid(), c.etablissement_id, $2, $3, c.personne_id, COALESCE(p.user_id, c.retire_par, c.donne_par),
                        'personne_soi', 'retire', 'en_ligne', c.retire_par, c.retire_le,
                        encode(sha256(convert_to('275r:' || c.id::text, 'UTF8')), 'hex'),
                        'personne_consentement_275', md5(c.id::text || 'retrait')::uuid
                   FROM ent_personne_consentement c
                   JOIN ent_personne p ON p.etablissement_id = c.etablissement_id AND p.id = c.personne_id
                  WHERE c.etablissement_id = $1 AND c.objet = $4 AND c.reponse = 'accorde' AND c.retire_le IS NOT NULL
                    AND COALESCE(p.user_id, c.retire_par, c.donne_par) IS NOT NULL
                    AND NOT EXISTS (SELECT 1 FROM ent_personne_consentement n
                                     WHERE n.etablissement_id = c.etablissement_id AND n.personne_id = c.personne_id
                                       AND n.objet = c.objet AND n.du >= c.retire_le)
                 ON CONFLICT (source_reprise, source_reprise_id) WHERE source_reprise IS NOT NULL DO NOTHING",
            )
            .bind(etab)
            .bind(o.id)
            .bind(version_id)
            .bind(ancien)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
            sqlx::query(
                "INSERT INTO ent_acc_decision_usage (etablissement_id, decision_id, usage_id, accorde)
                 SELECT d.etablissement_id, d.id, $3, d.decision = 'accorde' FROM ent_acc_decision d
                  WHERE d.etablissement_id = $1 AND d.objet_id = $2 AND d.source_reprise = 'personne_consentement_275'
                    AND NOT EXISTS (SELECT 1 FROM ent_acc_decision_usage x WHERE x.decision_id = d.id)",
            )
            .bind(etab)
            .bind(o.id)
            .bind(usage_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            bilan.decisions += (inserees + retraits) as usize;
            journaliser(&mut tx, etab, "reprise", Some(o.id), Some(version_id), None, None, acteur, Some((inserees + retraits) as i32), Some("personne_consentement_275"))
                .await?;
            tx.commit().await.map_err(db)?;
        }
        Ok(bilan)
    }
}

/// Vue d'un objet avec la version publiée en cours et l'état de relecture du texte.
const OBJET_VUE_SQL: &str = "SELECT o.id, o.code, o.libelle, o.nature, o.regle, o.avis_eleve_des_ans, o.droit_lien_code,
        o.preuve_min, o.validite, o.sujet_type, o.catalogue_code, o.actif,
        (SELECT v.numero FROM ent_acc_version v WHERE v.etablissement_id = o.etablissement_id AND v.objet_id = o.id
            AND v.publiee_le IS NOT NULL ORDER BY v.numero DESC LIMIT 1) AS version_publiee,
        COALESCE((SELECT v.texte_relu_le IS NULL FROM ent_acc_version v WHERE v.etablissement_id = o.etablissement_id
            AND v.objet_id = o.id AND v.publiee_le IS NOT NULL ORDER BY v.numero DESC LIMIT 1), true) AS texte_provisoire
   FROM ent_acc_objet o WHERE o.etablissement_id = $1 AND ({FILTRE}) ORDER BY o.libelle";
