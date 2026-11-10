//! Messagerie, lot M0b (`docs/socles/16_MESSAGERIE.md` §27.1 et §33.2) : correctifs de sécurité
//! résiduelle et de fiabilité.
//!
//! - D8, D18, D27 : annonces en deux temps (aperçu chiffré, confirmation), retrait, envoi par la
//!   chaîne S2-8 (jamais de boucle d'envoi dans la requête), ciblage par identifiant de classe,
//!   enveloppe neutre ;
//! - D24 : réglage de la plage calme (défaut national, motif à la réduction) ;
//! - D25 : liens créés après une interdiction de communication saisie par libellé, à vérifier
//!   avant de redevenir communicants.
//!
//! Les types d'échange des fonctions de `service.rs` (ouverture, réponse, pagination) sont aussi
//! définis ici.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate, Utc};
use domain_eleve::{
    plan_creer_annonce, Annonce, AudienceAnnonce, CreerAnnonce, DomainError, Message, MsgFil,
    StatutAnnonce,
};
use ent_auth::{Action, Role};
use ent_events::types;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::annonce_repo::PgAnnonceRepo;
use crate::eleve_repo::PgEleveRepo;
use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::service::{EntDbError, PgEnt};

/// Options d'envoi d'un message (M0b, D24).
#[derive(Debug, Clone, Default)]
pub struct OptionsEnvoiMessage {
    /// Motif d'une remise immédiate pendant la plage calme (`messagerie.urgence_hors_plage`).
    pub urgence_motif: Option<String>,
}

/// Un fil créé, avec son premier message et sa remise éventuelle.
#[derive(Debug, Clone)]
pub struct FilOuvert {
    pub fil: MsgFil,
    pub message: Message,
    /// `Some` : le message est différé par la plage calme jusqu'à cet instant.
    pub remise_prevue_le: Option<DateTime<Utc>>,
}

/// Un message envoyé, avec sa remise éventuelle (l'auteur la voit avant d'envoyer, R-M20).
#[derive(Debug, Clone)]
pub struct MessageEnvoye {
    pub message: Message,
    pub remise_prevue_le: Option<DateTime<Utc>>,
}

/// Paramètres d'une page de fils (D12).
#[derive(Debug, Clone, Default)]
pub struct ParamsListeFils {
    pub eleve_id: Option<Uuid>,
    pub include_archives: bool,
    pub dossier: Option<String>,
    /// Recherche sur le sujet, côté serveur.
    pub recherche: Option<String>,
    /// Curseur rendu par la page précédente.
    pub apres: Option<String>,
    /// 1 à 200 ; 50 par défaut.
    pub limite: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct PageFils {
    pub fils: Vec<MsgFil>,
    /// Curseur de la page suivante, `None` à la dernière page.
    pub suivant: Option<String>,
}

/// Aperçu chiffré d'une annonce (D8) : ce qui partira si l'auteur confirme.
#[derive(Debug, Clone)]
pub struct ApercuAnnonce {
    pub annonce: Annonce,
    pub nb_eleves: i32,
    pub nb_responsables: i32,
    /// Élèves de l'audience dont aucun responsable ne peut être joint (aucun lien communicant).
    pub nb_eleves_sans_responsable: i32,
}

/// Réglage de la plage calme d'un établissement (D24).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReglagePlageCalme {
    pub plage_calme: bool,
    pub debut_heure: i16,
    pub fin_heure: i16,
    pub weekend: bool,
    pub motif: Option<String>,
}

impl Default for ReglagePlageCalme {
    /// Défaut national (référentiel du numérique responsable, exigences 8 et 9).
    fn default() -> Self {
        Self { plage_calme: true, debut_heure: 20, fin_heure: 7, weekend: true, motif: None }
    }
}

#[derive(Debug, Clone)]
pub struct ReglagePlageCalmeSaisie {
    pub plage_calme: bool,
    pub debut_heure: i16,
    pub fin_heure: i16,
    pub weekend: bool,
    /// Obligatoire dès que la plage est désactivée ou plus courte que le défaut national.
    pub motif: Option<String>,
}

/// Lien créé après une interdiction de communication par libellé (D25).
#[derive(Debug, Clone, Serialize)]
pub struct LienAVerifier {
    pub lien_id: Uuid,
    pub eleve_id: Uuid,
    pub interdiction_id: Uuid,
    pub cree_le: DateTime<Utc>,
}

impl PgEnt {
    // ───────────────────────── Plage calme (D24) ─────────────────────────

    /// Réglage en vigueur (défaut national si l'établissement n'en a jamais enregistré).
    pub async fn reglage_plage_calme(
        &self,
        etablissement_id: Uuid,
    ) -> Result<ReglagePlageCalme, EntDbError> {
        let l: Option<(bool, i16, i16, bool, Option<String>)> = sqlx::query_as(
            "SELECT plage_calme, plage_debut_heure, plage_fin_heure, plage_weekend, motif
               FROM ent_msg_reglage WHERE etablissement_id = $1",
        )
        .bind(etablissement_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(l.map(|(plage_calme, debut_heure, fin_heure, weekend, motif)| ReglagePlageCalme {
            plage_calme,
            debut_heure,
            fin_heure,
            weekend,
            motif,
        })
        .unwrap_or_default())
    }

    /// Règle la plage calme (MSG-262) : `notifications.gerer` (établissement entier). Désactiver
    /// ou raccourcir la plage exige un motif, conservé avec l'auteur.
    pub async fn definir_reglage_plage_calme_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        saisie: ReglagePlageCalmeSaisie,
    ) -> Result<ReglagePlageCalme, EntDbError> {
        if !self
            .a_action_etablissement(etablissement_id, actor_user_id, role, Action::NotificationsGerer)
            .await?
        {
            return Err(EntDbError::Forbidden);
        }
        if !(12..=23).contains(&saisie.debut_heure) || !(0..=11).contains(&saisie.fin_heure) {
            return Err(EntDbError::SaisieInvalide("plage calme : heures hors limites"));
        }
        let motif: Option<String> = saisie
            .motif
            .as_deref()
            .map(|m| m.trim().chars().filter(|c| !c.is_control()).take(300).collect::<String>())
            .filter(|m| !m.is_empty());
        let reduite = !saisie.plage_calme
            || saisie.debut_heure > 20
            || saisie.fin_heure < 7
            || !saisie.weekend;
        if reduite && motif.is_none() {
            return Err(EntDbError::SaisieInvalide(
                "plage calme réduite ou désactivée : motif obligatoire",
            ));
        }
        sqlx::query(
            "INSERT INTO ent_msg_reglage
                (etablissement_id, plage_calme, plage_debut_heure, plage_fin_heure, plage_weekend,
                 motif, modifie_par, modifie_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,now())
             ON CONFLICT (etablissement_id) DO UPDATE
                SET plage_calme = EXCLUDED.plage_calme,
                    plage_debut_heure = EXCLUDED.plage_debut_heure,
                    plage_fin_heure = EXCLUDED.plage_fin_heure,
                    plage_weekend = EXCLUDED.plage_weekend,
                    motif = EXCLUDED.motif, modifie_par = EXCLUDED.modifie_par,
                    modifie_le = now()",
        )
        .bind(etablissement_id)
        .bind(saisie.plage_calme)
        .bind(saisie.debut_heure)
        .bind(saisie.fin_heure)
        .bind(saisie.weekend)
        .bind(if reduite { motif.clone() } else { None })
        .bind(actor_user_id)
        .execute(self.pool())
        .await?;
        self.publier_msg_event(
            etablissement_id,
            types::MSG_REGLAGE,
            etablissement_id,
            None,
            json!({
                "plage_calme": saisie.plage_calme,
                "debut_heure": saisie.debut_heure,
                "fin_heure": saisie.fin_heure,
                "weekend": saisie.weekend,
            }),
            actor_user_id,
        )
        .await?;
        self.reglage_plage_calme(etablissement_id).await
    }

    async fn publier_msg_event(
        &self,
        etablissement_id: Uuid,
        type_name: &str,
        aggregate_id: Uuid,
        eleve_id: Option<Uuid>,
        payload: serde_json::Value,
        actor_user_id: Uuid,
    ) -> Result<(), EntDbError> {
        use ent_events::{AggregateKind, MetierEventDraft};
        self.events_bus()
            .publish(MetierEventDraft {
                etablissement_id,
                type_name: type_name.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id,
                eleve_id,
                payload,
                actor_user_id: Some(actor_user_id.to_string()),
            })
            .await?;
        Ok(())
    }

    // ───────────────────── Liens à vérifier (D25) ─────────────────────

    /// Liens créés après une interdiction de communication saisie par libellé, encore à décider
    /// (`remise.interdire`, établissement entier).
    pub async fn liens_a_verifier_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<Vec<LienAVerifier>, EntDbError> {
        self.exiger_remise_interdire(etablissement_id, actor_user_id, role).await?;
        let rows: Vec<(Uuid, Uuid, Uuid, DateTime<Utc>)> = sqlx::query_as(
            "SELECT l.id, l.eleve_id, i.id, l.created_at
               FROM ent_eleve_lien l
               JOIN ent_eleve_interdiction i
                 ON i.etablissement_id = l.etablissement_id AND i.eleve_id = l.eleve_id
              WHERE l.etablissement_id = $1
                AND i.levee_le IS NULL AND 'communication' = ANY (i.objet)
                AND i.personne_id IS NULL AND l.created_at >= i.created_at
                AND i.du <= ent_aujourdhui_etab($1)
                AND (i.au IS NULL OR i.au >= ent_aujourdhui_etab($1))
                AND NOT EXISTS (SELECT 1 FROM ent_eleve_lien_verification v
                                 WHERE v.lien_id = l.id AND v.interdiction_id = i.id)
              ORDER BY l.created_at",
        )
        .bind(etablissement_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(lien_id, eleve_id, interdiction_id, cree_le)| LienAVerifier {
                lien_id,
                eleve_id,
                interdiction_id,
                cree_le,
            })
            .collect())
    }

    /// Décide d'un lien à vérifier : `confirmer` (la personne n'est pas visée : le lien redevient
    /// communicant) ou `maintenir_inactif`. Le motif est conservé avec l'auteur ; rien n'est
    /// jamais effacé.
    pub async fn verifier_lien_for_actor(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
        lien_id: Uuid,
        interdiction_id: Uuid,
        confirmer: bool,
        motif: Option<&str>,
    ) -> Result<(), EntDbError> {
        self.exiger_remise_interdire(etablissement_id, actor_user_id, role).await?;
        let motif: Option<String> = motif
            .map(|m| m.trim().chars().filter(|c| !c.is_control()).take(300).collect::<String>())
            .filter(|m| !m.is_empty());
        if motif.is_none() {
            return Err(EntDbError::SaisieInvalide("motif de la décision obligatoire"));
        }
        let eleve: Option<(Uuid,)> = sqlx::query_as(
            "SELECT l.eleve_id FROM ent_eleve_lien l
               JOIN ent_eleve_interdiction i
                 ON i.etablissement_id = l.etablissement_id AND i.eleve_id = l.eleve_id
              WHERE l.etablissement_id = $1 AND l.id = $2 AND i.id = $3
                AND i.personne_id IS NULL AND i.levee_le IS NULL",
        )
        .bind(etablissement_id)
        .bind(lien_id)
        .bind(interdiction_id)
        .fetch_optional(self.pool())
        .await?;
        let Some((eleve_id,)) = eleve else {
            return Err(DomainError::NotFound.into());
        };
        let r = sqlx::query(
            "INSERT INTO ent_eleve_lien_verification
                (etablissement_id, lien_id, interdiction_id, decision, motif, decide_par)
             VALUES ($1,$2,$3,$4,$5,$6)
             ON CONFLICT (lien_id, interdiction_id) DO UPDATE
                SET decision = EXCLUDED.decision, motif = EXCLUDED.motif,
                    decide_par = EXCLUDED.decide_par, decide_le = now()",
        )
        .bind(etablissement_id)
        .bind(lien_id)
        .bind(interdiction_id)
        .bind(if confirmer { "confirme" } else { "maintenu_inactif" })
        .bind(&motif)
        .bind(actor_user_id)
        .execute(self.pool())
        .await?;
        let _ = r;
        self.publier_msg_event(
            etablissement_id,
            types::LIEN_VERIFIE,
            lien_id,
            Some(eleve_id),
            json!({ "decision": if confirmer { "confirme" } else { "maintenu_inactif" } }),
            actor_user_id,
        )
        .await
    }

    async fn exiger_remise_interdire(
        &self,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        role: &str,
    ) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etablissement_id, actor_user_id, role, Action::RemiseInterdire)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    // ─────────────────────────── Annonces (D8, D18, D27) ───────────────────────────

    /// Crée l'**aperçu** d'une annonce : rien n'est envoyé. Retourne le nombre d'élèves et de
    /// responsables qui la recevraient. L'auteur confirme ensuite avec
    /// [`Self::confirmer_annonce_for_actor`].
    pub async fn apercu_annonce_for_actor(
        &self,
        role: &str,
        cmd: CreerAnnonce,
    ) -> Result<ApercuAnnonce, EntDbError> {
        if !Self::role_can_publier_annonces(role) {
            return Err(EntDbError::Forbidden);
        }
        let mut a = plan_creer_annonce(cmd)?;
        let repo = PgAnnonceRepo::new(self.pool().clone());
        if a.audience == AudienceAnnonce::Classe {
            // D18 : la classe est désignée par son identifiant ; une classe renommée ensuite
            // continue d'être atteinte. Un libellé qui ne désigne aucune classe du référentiel
            // garde l'ancien ciblage par libellé.
            let libelle = a.cible_classe.clone().unwrap_or_default();
            a.classe_id = self.resoudre_classe(a.etablissement_id, &libelle).await?;
        }
        let (eleves, par_eleve) = self.destinataires_annonce(&a).await?;
        let responsables: BTreeSet<Uuid> = par_eleve.values().flatten().copied().collect();
        let sans = eleves.iter().filter(|e| par_eleve.get(e).is_none_or(|u| u.is_empty())).count();
        a.apercu_nb_eleves = Some(eleves.len() as i32);
        a.apercu_nb_responsables = Some(responsables.len() as i32);
        repo.insert(&a).await?;
        self.publier_msg_event_annonce(&a, types::ANNONCE_APERCU, a.auteur_user_id).await?;
        Ok(ApercuAnnonce {
            nb_eleves: eleves.len() as i32,
            nb_responsables: responsables.len() as i32,
            nb_eleves_sans_responsable: sans as i32,
            annonce: a,
        })
    }

    /// Confirme un aperçu : l'annonce est publiée et ses envois sont **mis en file** (S2-8) ; le
    /// fournisseur n'est jamais appelé dans la requête.
    pub async fn confirmer_annonce_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        annonce_id: Uuid,
    ) -> Result<Annonce, EntDbError> {
        if !Self::role_can_publier_annonces(role) {
            return Err(EntDbError::Forbidden);
        }
        let repo = PgAnnonceRepo::new(self.pool().clone());
        let a = repo
            .get(etablissement_id, annonce_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if a.statut != StatutAnnonce::Apercu {
            return Err(EntDbError::ConflitDossier("annonce déjà confirmée ou retirée"));
        }
        if !repo.confirmer(etablissement_id, annonce_id, actor_user_id).await? {
            return Err(EntDbError::ConflitDossier("annonce déjà confirmée ou retirée"));
        }
        let a = Annonce { statut: StatutAnnonce::Publiee, ..a };
        self.publier_msg_event_annonce(&a, types::ANNONCE_PUBLIEE, actor_user_id).await?;
        self.enregistrer_journal_interne(domain_eleve::EnregistrerJournal {
            etablissement_id,
            eleve_id: None,
            domaine: "communication".into(),
            action: "annonce_publiee".into(),
            libelle: format!(
                "Annonce « {} » diffusée ({})",
                a.titre,
                a.audience.as_str()
            ),
            payload: json!({
                "annonce_id": a.id,
                "audience": a.audience.as_str(),
                "cible_classe": a.cible_classe,
            }),
            visible_famille: a.audience.touche_parents(),
            actor_user_id: Some(actor_user_id),
        })
        .await?;
        if a.audience.touche_parents() {
            if let Err(e) = self.mettre_en_file_annonce(&a).await {
                // La publication est acquise ; la mise en file se rejoue (idempotente par
                // destinataire) : on journalise, on ne défait pas.
                tracing::error!(canal = "annonce", annonce = %a.id, error = %e, "mise en file incomplète");
                return Err(e);
            }
        }
        Ok(a)
    }

    /// Aperçu puis confirmation en un seul geste (appels internes, labo) : la route HTTP
    /// impose les deux temps.
    pub async fn publier_annonce_for_actor(
        &self,
        role: &str,
        cmd: CreerAnnonce,
    ) -> Result<Annonce, EntDbError> {
        let etab = cmd.etablissement_id;
        let actor = cmd.actor_user_id;
        let apercu = self.apercu_annonce_for_actor(role, cmd).await?;
        self.confirmer_annonce_for_actor(role, etab, actor, apercu.annonce.id).await
    }

    /// Retire une annonce publiée : plus visible des familles, envois en attente annulés,
    /// notifications dans l'ENT non lues retirées. Motif obligatoire, tracé.
    pub async fn retirer_annonce_for_actor(
        &self,
        role: &str,
        etablissement_id: Uuid,
        actor_user_id: Uuid,
        annonce_id: Uuid,
        motif: &str,
    ) -> Result<Annonce, EntDbError> {
        if !Self::role_can_publier_annonces(role) {
            return Err(EntDbError::Forbidden);
        }
        let motif: String = motif.trim().chars().filter(|c| !c.is_control()).take(300).collect();
        if motif.is_empty() {
            return Err(EntDbError::SaisieInvalide("motif du retrait obligatoire"));
        }
        let repo = PgAnnonceRepo::new(self.pool().clone());
        let a = repo
            .get(etablissement_id, annonce_id)
            .await?
            .ok_or(DomainError::NotFound)?;
        if !repo.retirer(etablissement_id, annonce_id, actor_user_id, &motif).await? {
            return Err(EntDbError::ConflitDossier("annonce non publiée ou déjà retirée"));
        }
        sqlx::query(
            "UPDATE ent_notification_envoi SET statut = 'annule', traite_le = now()
              WHERE etablissement_id = $1 AND source_type = 'systeme' AND source_id = $2
                AND statut = 'en_attente'",
        )
        .bind(etablissement_id)
        .bind(annonce_id)
        .execute(self.pool())
        .await?;
        sqlx::query(
            "DELETE FROM ent_notification
              WHERE etablissement_id = $1 AND source_event_id = $2 AND kind = 'annonce'
                AND lu_le IS NULL",
        )
        .bind(etablissement_id)
        .bind(annonce_id)
        .execute(self.pool())
        .await?;
        let a = Annonce { statut: StatutAnnonce::Retiree, ..a };
        self.publier_msg_event_annonce(&a, types::ANNONCE_RETIREE, actor_user_id).await?;
        Ok(a)
    }

    async fn publier_msg_event_annonce(
        &self,
        a: &Annonce,
        type_name: &str,
        actor: Uuid,
    ) -> Result<(), EntDbError> {
        use ent_events::{AggregateKind, MetierEventDraft};
        self.events_bus()
            .publish(MetierEventDraft {
                etablissement_id: a.etablissement_id,
                type_name: type_name.to_string(),
                aggregate: AggregateKind::Annonce,
                aggregate_id: a.id,
                eleve_id: None,
                payload: json!({
                    "titre": a.titre,
                    "audience": a.audience.as_str(),
                    "cible_classe": a.cible_classe,
                }),
                actor_user_id: Some(actor.to_string()),
            })
            .await?;
        Ok(())
    }

    /// Identifiant de la classe désignée par `libelle` (libellé ou code, sans tenir compte de la
    /// casse). Plusieurs classes de ce nom : celle qui a des élèves affectés aujourd'hui ; si
    /// c'est ambigu, la saisie est refusée.
    async fn resoudre_classe(
        &self,
        etablissement_id: Uuid,
        libelle: &str,
    ) -> Result<Option<Uuid>, EntDbError> {
        let jour = self.aujourdhui_etablissement(etablissement_id).await?;
        let rows: Vec<(Uuid, bool)> = sqlx::query_as(
            "SELECT c.id,
                    EXISTS (SELECT 1 FROM ent_affectation a
                             WHERE a.etablissement_id = c.etablissement_id AND a.classe_id = c.id
                               AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3))
               FROM ent_classe c
              WHERE c.etablissement_id = $1
                AND (lower(btrim(c.libelle)) = lower(btrim($2)) OR lower(btrim(c.code)) = lower(btrim($2)))",
        )
        .bind(etablissement_id)
        .bind(libelle)
        .bind(jour)
        .fetch_all(self.pool())
        .await?;
        match rows.as_slice() {
            [] => Ok(None),
            [(id, _)] => Ok(Some(*id)),
            many => {
                let actives: Vec<Uuid> =
                    many.iter().filter(|(_, active)| *active).map(|(id, _)| *id).collect();
                match actives.as_slice() {
                    [id] => Ok(Some(*id)),
                    _ => Err(EntDbError::SaisieInvalide("classe ambiguë : précisez-la")),
                }
            }
        }
    }

    /// Élèves concernés par l'annonce et, pour chacun, les responsables **joignables** (lien actif,
    /// non visé par une interdiction de communication, non « ne pas solliciter »).
    async fn destinataires_annonce(
        &self,
        a: &Annonce,
    ) -> Result<(Vec<Uuid>, BTreeMap<Uuid, Vec<Uuid>>), EntDbError> {
        let etab = a.etablissement_id;
        let jour: NaiveDate = self.aujourdhui_etablissement(etab).await?;
        let eleves: Vec<Uuid> = match (a.audience, a.classe_id) {
            (AudienceAnnonce::Personnels, _) => Vec::new(),
            (AudienceAnnonce::Classe, Some(classe_id)) => {
                let rows: Vec<(Uuid,)> = sqlx::query_as(
                    "SELECT DISTINCT a.eleve_id
                       FROM ent_affectation a
                       JOIN ent_eleve e ON e.id = a.eleve_id AND e.etablissement_id = a.etablissement_id
                      WHERE a.etablissement_id = $1 AND a.classe_id = $2
                        AND a.date_debut <= $3 AND (a.date_fin IS NULL OR a.date_fin >= $3)
                        AND (e.date_sortie IS NULL OR e.date_sortie > $3)",
                )
                .bind(etab)
                .bind(classe_id)
                .bind(jour)
                .fetch_all(self.pool())
                .await?;
                rows.into_iter().map(|(id,)| id).collect()
            }
            (AudienceAnnonce::Classe, None) => {
                // Libellé sans classe du référentiel : ancien ciblage par libellé.
                let libelle = a.cible_classe.clone().unwrap_or_default();
                PgEleveRepo::new(self.pool().clone())
                    .list_en_classe_au(etab, &libelle, jour)
                    .await?
                    .into_iter()
                    .map(|e| e.id)
                    .collect()
            }
            _ => PgEleveRepo::new(self.pool().clone())
                .list_actifs_au(etab, jour)
                .await?
                .into_iter()
                .map(|e| e.id)
                .collect(),
        };
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT DISTINCT l.eleve_id, l.user_id FROM ent_eleve_lien_sollicitable l
              WHERE l.etablissement_id = $1 AND l.eleve_id = ANY($2) AND l.user_id IS NOT NULL
              ORDER BY l.eleve_id, l.user_id",
        )
        .bind(etab)
        .bind(&eleves)
        .fetch_all(self.pool())
        .await?;
        let mut par_eleve: BTreeMap<Uuid, Vec<Uuid>> = BTreeMap::new();
        for (e, u) in rows {
            par_eleve.entry(e).or_default().push(u);
        }
        Ok((eleves, par_eleve))
    }

    /// Une notification par responsable (dans l'ENT, plus ses canaux externes selon ses
    /// préférences) : e-mail, SMS et push **neutres** (aucun texte de l'annonce, D27), le contenu
    /// restant dans l'ENT. Dédoublonnée par annonce et par responsable.
    async fn mettre_en_file_annonce(&self, a: &Annonce) -> Result<(), EntDbError> {
        let (_, par_eleve) = self.destinataires_annonce(a).await?;
        // Un responsable de plusieurs élèves ne reçoit qu'un avis, rattaché à son premier élève.
        let mut premier: BTreeMap<Uuid, Uuid> = BTreeMap::new();
        for (eleve, users) in &par_eleve {
            for u in users {
                premier.entry(*u).or_insert(*eleve);
            }
        }
        let maintenant = Utc::now();
        let mut c = self.pool().acquire().await?;
        for (user, eleve) in premier {
            notifier_personne(
                &mut c,
                EnvoiAPlanifier {
                    etablissement_id: a.etablissement_id,
                    user_id: user,
                    kind: "annonce",
                    titre: &a.titre,
                    corps: &a.corps,
                    eleve_id: Some(eleve),
                    notification_id: None,
                    source_type: "systeme",
                    source_id: a.id,
                    cle_dedup: format!("annonce:{}", a.id),
                    droit_requis: Some("lien"),
                    regroupable: false,
                    canaux_voulus: &["email", "push"],
                },
                a.id,
                maintenant,
            )
            .await?;
        }
        Ok(())
    }

    /// Annonces visibles par la famille d'un élève (publiées seulement ; ciblage par classe).
    pub(crate) async fn classes_de_eleve(
        &self,
        etablissement_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let jour = self.aujourdhui_etablissement(etablissement_id).await?;
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT classe_id FROM ent_affectation
              WHERE etablissement_id = $1 AND eleve_id = $2 AND classe_id IS NOT NULL
                AND date_debut <= $3 AND (date_fin IS NULL OR date_fin >= $3)",
        )
        .bind(etablissement_id)
        .bind(eleve_id)
        .bind(jour)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}

/// Rôles de base qui lisent les fils réservés par défaut (documentation et tests).
pub const ROLES_FILS_RESERVES_PAR_DEFAUT: [Role; 4] =
    [Role::Admin, Role::Direction, Role::Administratif, Role::Cpe];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglage_par_defaut_est_le_defaut_national() {
        let r = ReglagePlageCalme::default();
        assert!(r.plage_calme && r.weekend);
        assert_eq!((r.debut_heure, r.fin_heure), (20, 7));
    }
}
