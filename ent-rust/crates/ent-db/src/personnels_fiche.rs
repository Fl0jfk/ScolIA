//! Personnels, lot P1 — fiche, fonctions, disciplines, qualifications, choix de l'agent, identité
//! restreinte, fusion d'agents, identifiants externes (`06_PERSONNELS.md` §10.1, PRS-05 à PRS-13).
//!
//! Même règles que `personnels.rs` : autorisation côté serveur (action du catalogue), une
//! commande = une transaction `TenantTx` (RLS) = fait + journal + événement sans donnée personnelle.
//! Les actions de l'agent sur lui-même (contact d'urgence, photo, pause) n'exigent aucune action à
//! accorder : seulement d'être un agent de l'établissement (portée `moi`, §14.2).

use chrono::{Duration, NaiveDate};
use ent_auth::{Action, Role};
use ent_events::types;
use ent_types::personnels::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::dossier_personnes::aujourdhui_tx;
use crate::personnels::{
    agent_du_user_tx, agent_tx, ajouter_fonction_ligne_tx, conflit, db, ecriture,
    engagements_agent_tx, fonction_id_tx, identite_tx, introuvable, invalide, journal_tx,
    publier_tx, CODES_QUALIFICATION, FAMILLES_FONCTION, LIENS_URGENCE, MOTIFS_RESTREINTE,
    PAUSE_MAX_JOURS, SYSTEMES_IDENTIFIANT,
};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// id, code, libellé, famille, profil proposé, établissement (NULL = fournie).
type LigneCatalogue = (Uuid, String, String, String, Option<String>, Option<Uuid>);
/// id, code, libellé, famille, principale, début, fin.
type LigneFonction = (
    Uuid,
    String,
    String,
    String,
    bool,
    NaiveDate,
    Option<NaiveDate>,
);
/// id, rang, nom, prénom, lien, téléphone.
type LigneContact = (Uuid, i16, String, Option<String>, Option<String>, String);

fn telephone_valide(t: &str) -> bool {
    let t = t.trim();
    (6..=40).contains(&t.chars().count())
        && t.chars()
            .all(|c| c.is_ascii_digit() || " +.-()".contains(c))
}

impl PgEnt {
    /// L'agent de l'acteur, ou 403 : les actions « sur soi-même » exigent d'être agent ici.
    async fn mon_agent_tx(
        conn: &mut PgConnection,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Uuid, EntDbError> {
        if !Role::parse(role).is_some_and(Role::is_staff) {
            return Err(EntDbError::Forbidden);
        }
        agent_du_user_tx(conn, etab, actor)
            .await?
            .ok_or(EntDbError::Forbidden)
    }

    // ---- Lecture de la fiche ----------------------------------------------------------------

    /// Fiche d'organisation d'un agent : `personnels.lire`, ou l'agent lui-même. Le contact d'urgence
    /// n'est rendu qu'à l'agent et à `personnels.personnel.lire` (la consultation par un tiers est
    /// journalisée) ; le motif de l'identité restreinte à `personnels.confidentiel.gerer`.
    pub async fn fiche_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
    ) -> Result<FicheAgentDto, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let soi = agent_du_user_tx(&mut tx, etab, actor).await? == Some(agent_id);
        if !soi
            && !self
                .a_action_etablissement(etab, actor, role, Action::PersonnelsLire)
                .await?
        {
            return Err(EntDbError::Forbidden);
        }
        let voit_personnel = soi
            || self
                .a_action_etablissement(etab, actor, role, Action::PersonnelsPersonnelLire)
                .await?;
        let voit_confidentiel = soi
            || self
                .a_action_etablissement(etab, actor, role, Action::PersonnelsConfidentielGerer)
                .await?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let a = agent_tx(&mut tx, etab, agent_id, false).await?;
        let identite = identite_tx(&mut tx, etab, a.personne_id).await?;
        let (etat,): (String,) = sqlx::query_as("SELECT ent_agent_etat($1, $2, $3)")
            .bind(etab)
            .bind(agent_id)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
        let engagements = engagements_agent_tx(&mut tx, etab, agent_id, jour, true).await?;
        let fonctions = fonctions_tx(&mut tx, etab, agent_id).await?;
        let disciplines: Vec<AgentDisciplineDto> = sqlx::query_as::<_, (Uuid, Option<Uuid>, Option<String>, bool, NaiveDate, Option<NaiveDate>)>(
            "SELECT id, matiere_id, code_discipline_poste, principale, du, au FROM ent_agent_discipline
              WHERE etablissement_id = $1 AND agent_id = $2 AND annule_le IS NULL ORDER BY du",
        )
        .bind(etab)
        .bind(agent_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(id, matiere_id, code, principale, du, au)| AgentDisciplineDto {
            id,
            matiere_id,
            code_discipline_poste: code,
            principale,
            du,
            au,
        })
        .collect();
        let qualifications: Vec<QualificationDto> = sqlx::query_as::<_, (Uuid, String, Option<String>, Option<NaiveDate>, Option<NaiveDate>)>(
            "SELECT id, code, precision, obtenue_le, valable_jusqu_au FROM ent_agent_qualification
              WHERE etablissement_id = $1 AND agent_id = $2 AND annule_le IS NULL ORDER BY code, obtenue_le",
        )
        .bind(etab)
        .bind(agent_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?
        .into_iter()
        .map(|(id, code, precision, obtenue_le, valable)| QualificationDto {
            id,
            code,
            precision,
            obtenue_le,
            valable_jusqu_au: valable,
            expiree: valable.is_some_and(|v| v < jour),
        })
        .collect();
        let identifiants: Vec<IdentifiantAgentDto> =
            sqlx::query_as::<_, (Uuid, String, String, NaiveDate, Option<NaiveDate>)>(
                "SELECT id, systeme, valeur, du, au FROM ent_agent_identifiant
              WHERE etablissement_id = $1 AND agent_id = $2 ORDER BY systeme, du",
            )
            .bind(etab)
            .bind(agent_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
            .into_iter()
            .map(|(id, systeme, valeur, du, au)| IdentifiantAgentDto {
                id,
                systeme,
                valeur,
                du,
                au,
            })
            .collect();
        let contacts_urgence = if voit_personnel {
            let c = contacts_tx(&mut tx, etab, agent_id, jour).await?;
            if !soi && !c.is_empty() {
                journal_tx(
                    &mut tx,
                    etab,
                    "contact_urgence_lu",
                    "Contact d'urgence d'un agent consulté",
                    json!({ "agent_id": agent_id }),
                    actor,
                )
                .await?;
            }
            Some(c)
        } else {
            None
        };
        tx.commit().await.map_err(db)?;
        Ok(FicheAgentDto {
            agent_id,
            identite,
            etat,
            source: a.source,
            matricule_interne: a.matricule_interne,
            personnel_mineur: a.personnel_mineur,
            identite_restreinte: a.identite_restreinte,
            identite_restreinte_motif: if voit_confidentiel {
                a.identite_restreinte_motif
            } else {
                None
            },
            identite_a_separer: a.identite_a_separer,
            notifications_en_pause_jusqu_au: a.notifications_en_pause_jusqu_au,
            photo_visible_collegues: a.photo_visible_collegues,
            photo_visible_eleves: a.photo_visible_eleves,
            photo_visible_familles: a.photo_visible_familles,
            engagements,
            fonctions,
            disciplines,
            qualifications,
            identifiants,
            contacts_urgence,
        })
    }

    // ---- Catalogue et fonctions (PRS-05) ---------------------------------------------------------

    pub async fn fonctions_catalogue_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<FonctionCatalogueDto>, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsAnnuaire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<LigneCatalogue> = sqlx::query_as(
            "SELECT id, code, libelle, famille, profil_propose, etablissement_id FROM ent_fonction
              WHERE (etablissement_id = $1 OR etablissement_id IS NULL) AND archivee_le IS NULL
              ORDER BY famille, libelle",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(lignes
            .into_iter()
            .map(
                |(id, code, libelle, famille, profil_propose, e)| FonctionCatalogueDto {
                    id,
                    code,
                    libelle,
                    famille,
                    profil_propose,
                    propre_etablissement: e.is_some(),
                },
            )
            .collect())
    }

    /// Fonction propre à l'établissement (« animateur en pastorale », « coordinatrice ULIS »…).
    pub async fn creer_fonction_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &CreerFonctionBody,
    ) -> Result<FonctionCatalogueDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let code = body.code.trim();
        let libelle = body.libelle.trim();
        let code_ok = code.len() >= 2
            && code.len() <= 60
            && code.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && code
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !code_ok {
            return Err(invalide(
                "Code de fonction : minuscules, chiffres et tirets bas (2 à 60 caractères).",
            ));
        }
        if libelle.is_empty() || libelle.chars().count() > 120 {
            return Err(invalide("Libellé de fonction : 1 à 120 caractères."));
        }
        if !FAMILLES_FONCTION.contains(&body.famille.as_str()) {
            return Err(invalide("Famille de fonctions inconnue."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Un code fourni par ScolIA ne se redéfinit pas.
        let (existe,): (bool,) =
            sqlx::query_as("SELECT EXISTS (SELECT 1 FROM ent_fonction WHERE etablissement_id IS NULL AND code = $1)")
                .bind(code)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if existe {
            return Err(conflit("Ce code est réservé à une fonction fournie."));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO ent_fonction (id, etablissement_id, code, libelle, famille) VALUES ($1,$2,$3,$4,$5)")
            .bind(id)
            .bind(etab)
            .bind(code)
            .bind(libelle)
            .bind(&body.famille)
            .execute(&mut *tx)
            .await
            .map_err(ecriture)?;
        journal_tx(
            &mut tx,
            etab,
            "fonction_creee",
            "Fonction créée",
            json!({ "fonction_id": id, "code": code }),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(FonctionCatalogueDto {
            id,
            code: code.to_string(),
            libelle: libelle.to_string(),
            famille: body.famille.clone(),
            profil_propose: None,
            propre_etablissement: true,
        })
    }

    /// PRS-05 : ajouter une fonction à un agent. Une seule principale à la fois ; la fonction PROPOSE
    /// des modèles de rôles, elle n'accorde rien.
    pub async fn ajouter_fonction_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
        body: &AgentFonctionBody,
    ) -> Result<AgentFonctionDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let du = body.du.unwrap_or(jour);
        if body.au.is_some_and(|a| a < du) {
            return Err(invalide("La fin de la fonction précède son début."));
        }
        agent_tx(&mut tx, etab, agent_id, true).await?;
        let fonction_id = fonction_id_tx(&mut tx, etab, &body.fonction_code).await?;
        let id = ajouter_fonction_ligne_tx(
            &mut tx,
            etab,
            agent_id,
            fonction_id,
            body.principale,
            du,
            body.au,
            actor,
        )
        .await?;
        let payload = json!({ "agent_id": agent_id, "fonction_code": body.fonction_code, "principale": body.principale, "du": du });
        journal_tx(
            &mut tx,
            etab,
            "fonction_ajoutee",
            "Fonction ajoutée à un agent",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_FONCTIONS_MODIFIEES,
            agent_id,
            payload,
            actor,
        )
        .await?;
        let f = fonctions_tx(&mut tx, etab, agent_id)
            .await?
            .into_iter()
            .find(|f| f.id == id)
            .ok_or_else(introuvable)?;
        tx.commit().await.map_err(db)?;
        Ok(f)
    }

    /// Terminer une fonction (fin datée) — la ligne reste, l'historique aussi.
    pub async fn terminer_fonction_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        ligne_id: Uuid,
        body: &TerminerLigneBody,
    ) -> Result<(), EntDbError> {
        self.terminer_ligne_agent(
            etab,
            actor,
            role,
            "ent_agent_fonction",
            ligne_id,
            body.au,
            types::PERSONNEL_FONCTIONS_MODIFIEES,
            "fonction_terminee",
        )
        .await
    }

    /// PRS-06 : discipline de poste (matière du référentiel et/ou code national).
    pub async fn ajouter_discipline_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
        body: &AgentDisciplineBody,
    ) -> Result<AgentDisciplineDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let code = body
            .code_discipline_poste
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty());
        if code.is_some_and(|c| c.chars().count() > 20) {
            return Err(invalide("Code de discipline : 20 caractères au plus."));
        }
        if body.matiere_id.is_none() && code.is_none() {
            return Err(invalide("Indiquer une matière ou un code de discipline."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let du = body.du.unwrap_or(jour);
        if body.au.is_some_and(|a| a < du) {
            return Err(invalide("La fin de la discipline précède son début."));
        }
        agent_tx(&mut tx, etab, agent_id, true).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_agent_discipline (id, etablissement_id, agent_id, matiere_id, code_discipline_poste,
                    principale, du, au, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(agent_id)
        .bind(body.matiere_id)
        .bind(code)
        .bind(body.principale)
        .bind(du)
        .bind(body.au)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let payload = json!({ "agent_id": agent_id, "matiere_id": body.matiere_id, "code": code, "principale": body.principale });
        journal_tx(
            &mut tx,
            etab,
            "discipline_ajoutee",
            "Discipline de poste ajoutée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_DISCIPLINES_MODIFIEES,
            agent_id,
            payload,
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(AgentDisciplineDto {
            id,
            matiere_id: body.matiere_id,
            code_discipline_poste: code.map(str::to_string),
            principale: body.principale,
            du,
            au: body.au,
        })
    }

    pub async fn terminer_discipline_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        ligne_id: Uuid,
        body: &TerminerLigneBody,
    ) -> Result<(), EntDbError> {
        self.terminer_ligne_agent(
            etab,
            actor,
            role,
            "ent_agent_discipline",
            ligne_id,
            body.au,
            types::PERSONNEL_DISCIPLINES_MODIFIEES,
            "discipline_terminee",
        )
        .await
    }

    /// Fin datée d'une ligne de fonction ou de discipline (tables internes, jamais une entrée client).
    #[allow(clippy::too_many_arguments)]
    async fn terminer_ligne_agent(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        table: &'static str,
        ligne_id: Uuid,
        au: NaiveDate,
        evenement: &'static str,
        action: &'static str,
    ) -> Result<(), EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ligne: Option<(Uuid, NaiveDate)> = sqlx::query_as(&format!(
            "SELECT agent_id, du FROM {table} WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NULL FOR UPDATE"
        ))
        .bind(etab)
        .bind(ligne_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (agent_id, du) = ligne.ok_or_else(introuvable)?;
        if au < du {
            return Err(invalide("La fin précède le début."));
        }
        sqlx::query(&format!(
            "UPDATE {table} SET au = $3 WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(ligne_id)
        .bind(au)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let payload = json!({ "agent_id": agent_id, "ligne_id": ligne_id, "au": au });
        journal_tx(
            &mut tx,
            etab,
            action,
            "Ligne d'un agent terminée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(&mut tx, etab, evenement, agent_id, payload, actor).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ---- Qualifications (PRS-07) ---------------------------------------------------------------

    pub async fn enregistrer_qualification_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
        body: &QualificationBody,
    ) -> Result<QualificationDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        if !CODES_QUALIFICATION.contains(&body.code.as_str()) {
            return Err(invalide("Qualification inconnue."));
        }
        let precision = body
            .precision
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty());
        if precision.is_some_and(|p| p.chars().count() > 80) {
            return Err(invalide("Précision : 80 caractères au plus."));
        }
        if let (Some(o), Some(v)) = (body.obtenue_le, body.valable_jusqu_au) {
            if v < o {
                return Err(invalide("La validité s'achève avant l'obtention."));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        agent_tx(&mut tx, etab, agent_id, true).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_agent_qualification (id, etablissement_id, agent_id, code, precision, obtenue_le,
                    valable_jusqu_au, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(agent_id)
        .bind(&body.code)
        .bind(precision)
        .bind(body.obtenue_le)
        .bind(body.valable_jusqu_au)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let payload = json!({ "agent_id": agent_id, "code": body.code, "valable_jusqu_au": body.valable_jusqu_au });
        journal_tx(
            &mut tx,
            etab,
            "qualification_ajoutee",
            "Qualification enregistrée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_QUALIFICATION_MODIFIEE,
            agent_id,
            payload,
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(QualificationDto {
            id,
            code: body.code.clone(),
            precision: precision.map(str::to_string),
            obtenue_le: body.obtenue_le,
            valable_jusqu_au: body.valable_jusqu_au,
            expiree: body.valable_jusqu_au.is_some_and(|v| v < jour),
        })
    }

    /// Corriger une qualification saisie par erreur : la ligne est annulée, jamais supprimée.
    pub async fn annuler_qualification_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        qualification_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let row: Option<(Uuid, String)> = sqlx::query_as(
            "UPDATE ent_agent_qualification SET annule_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND annule_le IS NULL RETURNING agent_id, code",
        )
        .bind(etab)
        .bind(qualification_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (agent_id, code) = row.ok_or_else(introuvable)?;
        let payload = json!({ "agent_id": agent_id, "code": code, "nature": "correction" });
        journal_tx(
            &mut tx,
            etab,
            "qualification_corrigee",
            "Qualification erronée annulée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_QUALIFICATION_MODIFIEE,
            agent_id,
            payload,
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ---- Choix de l'agent (PRS-08, 09, 10) -----------------------------------------------------

    /// PRS-08 : l'agent déclare (ou remplace) un contact d'urgence. Facultatif, sur sa décision ;
    /// lisible de lui-même et de `personnels.personnel.lire`.
    pub async fn definir_contact_urgence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &ContactUrgenceAgentBody,
    ) -> Result<ContactUrgenceAgentDto, EntDbError> {
        let nom = body.nom.trim();
        let prenom = body
            .prenom
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty());
        if nom.is_empty()
            || nom.chars().count() > 120
            || prenom.is_some_and(|p| p.chars().count() > 120)
        {
            return Err(invalide("Nom du contact : 1 à 120 caractères."));
        }
        if !(1..=3).contains(&body.ordre) {
            return Err(invalide("Rang du contact : de 1 à 3."));
        }
        if !telephone_valide(&body.telephone) {
            return Err(invalide("Numéro de téléphone mal formé."));
        }
        if body
            .lien
            .as_deref()
            .is_some_and(|l| !LIENS_URGENCE.contains(&l))
        {
            return Err(invalide("Lien avec le contact inconnu."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let agent_id = Self::mon_agent_tx(&mut tx, etab, actor, role).await?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        retirer_rang_tx(&mut tx, etab, agent_id, body.ordre, jour).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_agent_contact_urgence (id, etablissement_id, agent_id, ordre, nom, prenom, lien, telephone,
                    du, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(agent_id)
        .bind(body.ordre)
        .bind(nom)
        .bind(prenom)
        .bind(&body.lien)
        .bind(body.telephone.trim())
        .bind(jour)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        // Ni nom ni numéro dans le journal.
        journal_tx(
            &mut tx,
            etab,
            "contact_urgence_defini",
            "Contact d'urgence déclaré",
            json!({ "agent_id": agent_id, "ordre": body.ordre }),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(ContactUrgenceAgentDto {
            id,
            ordre: body.ordre,
            nom: nom.to_string(),
            prenom: prenom.map(str::to_string),
            lien: body.lien.clone(),
            telephone: body.telephone.trim().to_string(),
        })
    }

    pub async fn retirer_contact_urgence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        ordre: i16,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let agent_id = Self::mon_agent_tx(&mut tx, etab, actor, role).await?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        retirer_rang_tx(&mut tx, etab, agent_id, ordre, jour).await?;
        journal_tx(
            &mut tx,
            etab,
            "contact_urgence_retire",
            "Contact d'urgence retiré",
            json!({ "agent_id": agent_id, "ordre": ordre }),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// PRS-09 : l'agent choisit qui voit sa photo (collègues, élèves, familles séparément). Le fichier
    /// lui-même relève du socle 7 ; ici, le consentement daté et la visibilité.
    pub async fn definir_photo_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &PhotoAgentBody,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let agent_id = Self::mon_agent_tx(&mut tx, etab, actor, role).await?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let au_moins_un = body.visible_collegues || body.visible_eleves || body.visible_familles;
        sqlx::query(
            "UPDATE ent_agent SET photo_visible_collegues = $3, photo_visible_eleves = $4, photo_visible_familles = $5,
                    photo_consentement_le = CASE WHEN $6 THEN $7 ELSE NULL END
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(agent_id)
        .bind(body.visible_collegues)
        .bind(body.visible_eleves)
        .bind(body.visible_familles)
        .bind(au_moins_un)
        .bind(jour)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let payload = json!({ "agent_id": agent_id, "collegues": body.visible_collegues,
                              "eleves": body.visible_eleves, "familles": body.visible_familles });
        journal_tx(
            &mut tx,
            etab,
            "photo_modifiee",
            "Visibilité de la photo modifiée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_PHOTO_MODIFIEE,
            agent_id,
            payload,
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// PRS-10 : l'agent met ses notifications en pause jusqu'à une date (au plus un an). Les avis de
    /// sécurité ne sont pas suspendus (règle des diffusions, socle 8).
    pub async fn definir_pause_notifications_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &PauseNotificationsBody,
    ) -> Result<Option<NaiveDate>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let agent_id = Self::mon_agent_tx(&mut tx, etab, actor, role).await?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        if let Some(d) = body.jusqu_au {
            if d < jour || d > jour + Duration::days(PAUSE_MAX_JOURS) {
                return Err(invalide("La pause court d'aujourd'hui à un an au plus."));
            }
        }
        sqlx::query("UPDATE ent_agent SET notifications_en_pause_jusqu_au = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(agent_id)
            .bind(body.jusqu_au)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            "notifications_pause",
            "Pause des notifications modifiée",
            json!({ "agent_id": agent_id, "jusqu_au": body.jusqu_au }),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(body.jusqu_au)
    }

    // ---- Confidentiel et mineur (PRS-11, PRS-12) -----------------------------------------------

    /// PRS-11 ★ : poser ou lever l'identité restreinte d'un agent menacé. `personnels.confidentiel.gerer`
    /// (direction). Effet : les familles et les élèves ne voient plus son nom (civilité et initiale).
    pub async fn definir_identite_restreinte_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
        body: &IdentiteRestreinteBody,
    ) -> Result<(), EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsConfidentielGerer)
            .await?;
        let motif = if body.restreinte {
            let m = body
                .motif
                .as_deref()
                .ok_or_else(|| invalide("Un motif codé est obligatoire."))?;
            if !MOTIFS_RESTREINTE.contains(&m) {
                return Err(invalide("Motif d'identité restreinte inconnu."));
            }
            Some(m)
        } else {
            None
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        agent_tx(&mut tx, etab, agent_id, true).await?;
        sqlx::query(
            "UPDATE ent_agent SET identite_restreinte = $3, identite_restreinte_motif = $4,
                    identite_restreinte_depuis = CASE WHEN $3 THEN now() ELSE NULL END
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(agent_id)
        .bind(body.restreinte)
        .bind(motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Le motif reste dans la table : ni journal ni événement.
        let payload = json!({ "agent_id": agent_id, "posee": body.restreinte });
        journal_tx(
            &mut tx,
            etab,
            "identite_restreinte",
            "Identité restreinte modifiée",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_IDENTITE_RESTREINTE,
            agent_id,
            payload,
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// PRS-12 : signaler un personnel mineur (restrictions par défaut : jamais dans l'annuaire des
    /// familles ni des élèves).
    pub async fn definir_personnel_mineur_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
        body: &PersonnelMineurBody,
    ) -> Result<(), EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        agent_tx(&mut tx, etab, agent_id, true).await?;
        sqlx::query(
            "UPDATE ent_agent SET personnel_mineur = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(agent_id)
        .bind(body.personnel_mineur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            "personnel_mineur",
            "Statut de personnel mineur modifié",
            json!({ "agent_id": agent_id, "mineur": body.personnel_mineur }),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    // ---- Identifiants externes (361) -------------------------------------------------------------

    pub async fn ajouter_identifiant_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        agent_id: Uuid,
        body: &IdentifiantAgentBody,
    ) -> Result<IdentifiantAgentDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        if !SYSTEMES_IDENTIFIANT.contains(&body.systeme.as_str()) {
            return Err(invalide("Système d'identifiant inconnu."));
        }
        let valeur = body.valeur.trim();
        if valeur.is_empty() || valeur.chars().count() > 200 {
            return Err(invalide("Valeur d'identifiant : 1 à 200 caractères."));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        agent_tx(&mut tx, etab, agent_id, true).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_agent_identifiant (id, etablissement_id, agent_id, systeme, valeur, du, cree_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(agent_id)
        .bind(&body.systeme)
        .bind(valeur)
        .bind(jour)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        // La valeur n'est pas journalisée (elle peut désigner l'agent dans un système national).
        journal_tx(
            &mut tx,
            etab,
            "identifiant_ajoute",
            "Identifiant externe ajouté",
            json!({ "agent_id": agent_id, "systeme": body.systeme }),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(IdentifiantAgentDto {
            id,
            systeme: body.systeme.clone(),
            valeur: valeur.to_string(),
            du: jour,
            au: None,
        })
    }

    pub async fn clore_identifiant_agent_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        identifiant_id: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        let row: Option<(Uuid, String)> = sqlx::query_as(
            "UPDATE ent_agent_identifiant SET au = GREATEST(du, $3)
              WHERE etablissement_id = $1 AND id = $2 AND au IS NULL RETURNING agent_id, systeme",
        )
        .bind(etab)
        .bind(identifiant_id)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (agent_id, systeme) = row.ok_or_else(introuvable)?;
        journal_tx(
            &mut tx,
            etab,
            "identifiant_clos",
            "Identifiant externe clos",
            json!({ "agent_id": agent_id, "systeme": systeme }),
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// L'agent désigné par un identifiant externe actif (jointure STS, AAF, guichet Agents…).
    pub async fn agent_par_identifiant(
        &self,
        etab: Uuid,
        systeme: &str,
        valeur: &str,
    ) -> Result<Option<Uuid>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT agent_id FROM ent_agent_identifiant WHERE etablissement_id = $1 AND systeme = $2 AND valeur = $3 AND au IS NULL",
        )
        .bind(etab)
        .bind(systeme)
        .bind(valeur)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        Ok(row.map(|(id,)| id))
    }

    // ---- Fusion d'agents (PRS-13) ------------------------------------------------------------------

    /// PRS-13 ★ : fusionner deux agents en double (même personne créée deux fois). Passe par la fusion
    /// des personnes du socle 4 (jamais deux comptes différents, D-013), puis déplace vers l'agent
    /// conservé engagements, fonctions, disciplines, qualifications et identifiants. Reprise possible :
    /// si les personnes sont déjà fusionnées (premier appel interrompu), seule la suite est rejouée.
    pub async fn fusionner_agents_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        body: &FusionnerAgentsBody,
    ) -> Result<FusionAgentsDto, EntDbError> {
        self.exiger_personnels(etab, actor, role, Action::PersonnelsGerer)
            .await?;
        self.exiger_personnels(etab, actor, role, Action::PersonnesFusionner)
            .await?;
        let motif = body.motif.trim();
        if motif.chars().count() < 3 || motif.chars().count() > 500 {
            return Err(invalide(
                "Motif de fusion obligatoire (3 à 500 caractères).",
            ));
        }
        if body.source_agent_id == body.cible_agent_id {
            return Err(invalide("Un agent ne se fusionne pas avec lui-même."));
        }
        // 1. Les personnes (hors transaction de l'agent : la commande du socle 4 a la sienne).
        let (src_personne, cible_personne, deja_fusionnees) = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let s = agent_tx(&mut tx, etab, body.source_agent_id, false).await?;
            let c = agent_tx(&mut tx, etab, body.cible_agent_id, false).await?;
            if s.fusionne_dans_agent_id.is_some() || c.fusionne_dans_agent_id.is_some() {
                return Err(conflit("Un des agents est déjà fusionné."));
            }
            let (fusionnee,): (Option<Uuid>,) = sqlx::query_as(
                "SELECT fusionnee_dans FROM ent_personne WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(s.personne_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            (
                s.personne_id,
                c.personne_id,
                fusionnee == Some(c.personne_id),
            )
        };
        if !deja_fusionnees && src_personne != cible_personne {
            self.fusionner_personnes_interne(
                etab,
                actor,
                role,
                src_personne,
                cible_personne,
                motif,
                true,
            )
            .await?;
        }
        // 2. Les agents : une transaction.
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let jour = aujourdhui_tx(&mut tx, etab).await?;
        // Verrous dans un ordre stable.
        let (a, b) = if body.source_agent_id < body.cible_agent_id {
            (body.source_agent_id, body.cible_agent_id)
        } else {
            (body.cible_agent_id, body.source_agent_id)
        };
        let la = agent_tx(&mut tx, etab, a, true).await?;
        let lb = agent_tx(&mut tx, etab, b, true).await?;
        let (source, cible) = if la.id == body.source_agent_id {
            (la, lb)
        } else {
            (lb, la)
        };
        if source.fusionne_dans_agent_id.is_some() || cible.fusionne_dans_agent_id.is_some() {
            return Err(conflit("Un des agents est déjà fusionné."));
        }
        // Doublons de fonctions (même fonction, périodes qui se recouvrent) : la ligne de la source tombe.
        sqlx::query(
            "UPDATE ent_agent_fonction s SET annule_le = now()
              WHERE s.etablissement_id = $1 AND s.agent_id = $2 AND s.annule_le IS NULL
                AND EXISTS (SELECT 1 FROM ent_agent_fonction c
                             WHERE c.etablissement_id = s.etablissement_id AND c.agent_id = $3
                               AND c.annule_le IS NULL AND c.fonction_id = s.fonction_id
                               AND daterange(c.du, c.au, '[]') && daterange(s.du, s.au, '[]'))",
        )
        .bind(etab)
        .bind(source.id)
        .bind(cible.id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let (deplaces,): (i64,) = sqlx::query_as(
            "WITH m AS (UPDATE ent_agent_engagement SET agent_id = $3, agent_origine_id = $2
                         WHERE etablissement_id = $1 AND agent_id = $2 AND annule_le IS NULL RETURNING 1)
             SELECT count(*) FROM m",
        )
        .bind(etab)
        .bind(source.id)
        .bind(cible.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match ecriture(e) {
            EntDbError::Refus(_, _) => conflit("Des engagements des deux agents se chevauchent : corriger l'un d'eux avant de fusionner."),
            autre => autre,
        })?;
        // La fonction et la discipline principales de la cible l'emportent : celles de la source ne le sont plus.
        sqlx::query("UPDATE ent_agent_fonction SET agent_id = $3, principale = false WHERE etablissement_id = $1 AND agent_id = $2 AND annule_le IS NULL")
            .bind(etab).bind(source.id).bind(cible.id).execute(&mut *tx).await.map_err(ecriture)?;
        sqlx::query("UPDATE ent_agent_discipline SET agent_id = $3, principale = false WHERE etablissement_id = $1 AND agent_id = $2 AND annule_le IS NULL")
            .bind(etab).bind(source.id).bind(cible.id).execute(&mut *tx).await.map_err(ecriture)?;
        for table in ["ent_agent_qualification", "ent_agent_identifiant"] {
            sqlx::query(&format!(
                "UPDATE {table} SET agent_id = $3 WHERE etablissement_id = $1 AND agent_id = $2"
            ))
            .bind(etab)
            .bind(source.id)
            .bind(cible.id)
            .execute(&mut *tx)
            .await
            .map_err(ecriture)?;
        }
        // Les références (référent, « remplace ») pointent désormais l'agent conservé.
        sqlx::query("UPDATE ent_agent_engagement SET referent_agent_id = $3 WHERE etablissement_id = $1 AND referent_agent_id = $2")
            .bind(etab).bind(source.id).bind(cible.id).execute(&mut *tx).await.map_err(ecriture)?;
        sqlx::query("UPDATE ent_agent_engagement SET remplace_agent_id = $3 WHERE etablissement_id = $1 AND remplace_agent_id = $2")
            .bind(etab).bind(source.id).bind(cible.id).execute(&mut *tx).await.map_err(ecriture)?;
        // Les protections l'emportent (identité restreinte, personnel mineur).
        sqlx::query(
            "UPDATE ent_agent SET identite_restreinte = identite_restreinte OR $3,
                    identite_restreinte_motif = COALESCE(identite_restreinte_motif, $4),
                    personnel_mineur = personnel_mineur OR $5
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(cible.id)
        .bind(source.identite_restreinte)
        .bind(source.identite_restreinte_motif.clone())
        .bind(source.personnel_mineur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("UPDATE ent_agent SET fusionne_dans_agent_id = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(source.id)
            .bind(cible.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let payload = json!({ "source_agent_id": source.id, "cible_agent_id": cible.id, "engagements_deplaces": deplaces, "le": jour });
        journal_tx(
            &mut tx,
            etab,
            "agents_fusionnes",
            "Agents en doublon fusionnés",
            payload.clone(),
            actor,
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNEL_FUSIONNE,
            cible.id,
            payload,
            actor,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(FusionAgentsDto {
            source_agent_id: source.id,
            cible_agent_id: cible.id,
            engagements_deplaces: deplaces,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Lectures internes
// ---------------------------------------------------------------------------------------------

async fn fonctions_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
) -> Result<Vec<AgentFonctionDto>, EntDbError> {
    let lignes: Vec<LigneFonction> = sqlx::query_as(
        "SELECT af.id, f.code, f.libelle, f.famille, af.principale, af.du, af.au
           FROM ent_agent_fonction af JOIN ent_fonction f ON f.id = af.fonction_id
          WHERE af.etablissement_id = $1 AND af.agent_id = $2 AND af.annule_le IS NULL
          ORDER BY af.principale DESC, af.du",
    )
    .bind(etab)
    .bind(agent_id)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(lignes
        .into_iter()
        .map(
            |(id, fonction_code, libelle, famille, principale, du, au)| AgentFonctionDto {
                id,
                fonction_code,
                libelle,
                famille,
                principale,
                du,
                au,
            },
        )
        .collect())
}

async fn contacts_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
    jour: NaiveDate,
) -> Result<Vec<ContactUrgenceAgentDto>, EntDbError> {
    let lignes: Vec<LigneContact> = sqlx::query_as(
        "SELECT id, ordre, nom, prenom, lien, telephone FROM ent_agent_contact_urgence
          WHERE etablissement_id = $1 AND agent_id = $2 AND du <= $3 AND (au IS NULL OR au >= $3)
          ORDER BY ordre",
    )
    .bind(etab)
    .bind(agent_id)
    .bind(jour)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(lignes
        .into_iter()
        .map(
            |(id, ordre, nom, prenom, lien, telephone)| ContactUrgenceAgentDto {
                id,
                ordre,
                nom,
                prenom,
                lien,
                telephone,
            },
        )
        .collect())
}

/// Libère un rang de contact : une ligne née aujourd'hui est remplacée sur place (rien à garder),
/// une ligne plus ancienne est close la veille.
async fn retirer_rang_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    agent_id: Uuid,
    ordre: i16,
    jour: NaiveDate,
) -> Result<(), EntDbError> {
    sqlx::query(
        "DELETE FROM ent_agent_contact_urgence
          WHERE etablissement_id = $1 AND agent_id = $2 AND ordre = $3 AND au IS NULL AND du >= $4",
    )
    .bind(etab)
    .bind(agent_id)
    .bind(ordre)
    .bind(jour)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "UPDATE ent_agent_contact_urgence SET au = $4::date - 1
          WHERE etablissement_id = $1 AND agent_id = $2 AND ordre = $3 AND au IS NULL AND du < $4",
    )
    .bind(etab)
    .bind(agent_id)
    .bind(ordre)
    .bind(jour)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}
