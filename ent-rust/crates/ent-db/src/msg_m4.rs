//! Messagerie, lot M4 (`docs/socles/16_MESSAGERIE.md` §19.2, §20 R-M13, R-M20, R-M21, R-M27, R-M28 ; CM-26,
//! CM-45, CM-54 à CM-56, CM-79) : « écrire mieux ».
//!
//! * **Lots individuels** : 30 fils au plus (R-M13), aperçu chiffré puis confirmation sur la **même empreinte**
//!   (le texte confirmé est exactement celui qui a été prévisualisé) ; au-delà, publication ou diffusion.
//! * **Modèles** à trois portées : personnel, boîte, établissement.
//! * **Programmation** : un envoi programmé ne contourne jamais la plage calme (le plus tardif des deux gagne).
//! * **Correction** (15 min, si personne n'a lu) et **retrait** (10 min) : versionnés, tracés, jamais de
//!   suppression physique (R-M28).
//! * **Brouillons** strictement personnels, bornés et périmés.
//! * **Fusion** et **scission** de fils, avec les mêmes garde-fous de participants.
//!
//! Le chemin d'envoi reste celui de M0b/M1/M2 (`ouvrir_fils_a`, `repondre_fil_a`) : plage calme, matrice,
//! avis, participants. Ce module n'en recopie rien.

// Les services de la messagerie prennent l'établissement, le compte, le rôle et la cible : au-delà de 7 arguments,
// comme `msg_m2.rs`. Les regrouper dans une structure n'ajouterait rien.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use chrono::{DateTime, Duration, Utc};
use domain_eleve::{DomainError, MsgFil, OuvrirFil, RepondreFil};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::messagerie_m0b::OptionsEnvoiMessage;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Plus grand lot individuel (R-M13, S-139).
pub const LOT_MAX_FILS: usize = 30;
/// Durée de validité d'un aperçu de lot.
pub const LOT_VALIDITE_MINUTES: i64 = 30;
/// Fenêtre de correction d'un message non lu (R-M27).
pub const CORRECTION_MINUTES: i64 = 15;
/// Fenêtre de retrait par l'auteur (R-M28, CM-54, CM-56).
pub const RETRAIT_MINUTES: i64 = 10;
/// Horizon maximal d'une programmation.
pub const PROGRAMMATION_JOURS: i64 = 90;
/// Nombre maximal de brouillons par compte.
pub const BROUILLONS_MAX: i64 = 50;

fn refus(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

fn conflit(m: &'static str) -> EntDbError {
    EntDbError::ConflitDossier(m)
}

/// Aperçu d'un lot individuel : **aucun message n'est envoyé**.
#[derive(Debug, Clone, Serialize)]
pub struct ApercuLot {
    pub lot_id: Uuid,
    pub empreinte: String,
    pub nb_eleves: usize,
    /// Conversations qui seront créées (un fil par foyer, R-M3).
    pub nb_fils: usize,
    /// Élèves que l'expéditeur n'a pas le droit de joindre (matrice), sans détail.
    pub eleves_hors_droits: usize,
    /// Responsables exclus par décision ou « ne pas solliciter » : **un nombre**, jamais un nom ni une cause.
    pub exclus: usize,
    pub expire_le: DateTime<Utc>,
}

/// Portée d'un modèle de message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PorteeModele {
    Personnel,
    Boite(Uuid),
    Etablissement,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ModeleVu {
    pub id: Uuid,
    pub titre: String,
    pub sujet: String,
    pub corps: String,
    pub portee: String,
    pub boite_id: Option<Uuid>,
}

#[derive(Debug, Clone, Default)]
pub struct BrouillonSaisie {
    pub id: Option<Uuid>,
    pub fil_id: Option<Uuid>,
    pub eleve_id: Option<Uuid>,
    pub sujet: String,
    pub corps: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BrouillonVu {
    pub id: Uuid,
    pub fil_id: Option<Uuid>,
    pub eleve_id: Option<Uuid>,
    pub sujet: String,
    pub corps: String,
    pub maj_le: DateTime<Utc>,
}

/// Empreinte (SHA-256 hex) du contenu d'un lot : texte et élèves (triés).
fn contenu_lot(sujet: &str, corps: &str, eleves: &[Uuid]) -> String {
    let mut ids: Vec<String> = eleves.iter().map(|e| e.to_string()).collect();
    ids.sort();
    format!("{}\n{}\n{}", sujet.trim(), corps.trim(), ids.join(","))
}

impl PgEnt {
    // ───────────────────────────── Lots individuels (R-M13, CM-79) ─────────────────────────────

    async fn empreinte_sql(&self, etab: Uuid, contenu: &str) -> Result<String, EntDbError> {
        let (h,): (String,) = sqlx::query_as("SELECT encode(sha256(convert_to($1, 'UTF8')), 'hex')")
            .bind(contenu)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await?;
        Ok(h)
    }

    /// Aperçu d'un lot (« écrire aux responsables de ces élèves ») : calcule les conversations (un fil par
    /// foyer), refuse au-delà de 30 (bascule vers une annonce ou une diffusion, CM-79), ne sort aucune
    /// coordonnée, agrège les exclusions. Ne crée aucun fil et n'envoie rien ; le lot expire en 30 minutes.
    pub async fn apercu_lot_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        eleves: Vec<Uuid>,
        sujet: &str,
        corps: &str,
    ) -> Result<ApercuLot, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        if !self.a_action_etablissement(etab, user, role, Action::MessagerieLotEnvoyer).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut eleves = eleves;
        eleves.sort();
        eleves.dedup();
        if eleves.is_empty() {
            return Err(refus("aucun élève dans le lot"));
        }
        if sujet.trim().is_empty() || corps.trim().is_empty() {
            return Err(EntDbError::Domain(DomainError::MessageVide));
        }
        if sujet.trim().chars().count() > 200 || corps.trim().chars().count() > 20_000 {
            return Err(EntDbError::Domain(DomainError::MessageTropLong));
        }
        if eleves.len() > LOT_MAX_FILS {
            return Err(conflit(
                "au-delà de 30 conversations, ce n'est plus un lot de messages : utilisez une annonce ou une diffusion",
            ));
        }
        let (mut nb_fils, mut hors_droits, mut exclus) = (0usize, 0usize, 0usize);
        for e in &eleves {
            match self.apercu_destinataires_for_actor(etab, user, role, *e).await {
                Ok(a) if a.decision.est_permis() => {
                    nb_fils += if a.destinataire_est_l_eleve { 1 } else { a.conversations.len() };
                    exclus += a.exclus;
                }
                Ok(_) | Err(EntDbError::Forbidden) => hors_droits += 1,
                Err(e) => return Err(e),
            }
        }
        if nb_fils == 0 {
            return Err(conflit("aucun destinataire joignable dans ce lot"));
        }
        if nb_fils > LOT_MAX_FILS {
            return Err(conflit(
                "ce lot créerait plus de 30 conversations (un fil par foyer) : utilisez une annonce ou une diffusion",
            ));
        }
        let empreinte = self.empreinte_sql(etab, &contenu_lot(sujet, corps, &eleves)).await?;
        let id = Uuid::new_v4();
        // F7 : une seule horloge, celle de la base (l'expiration est relue contre `now()` de la base à la confirmation).
        let expire = self.maintenant_base(etab).await? + Duration::minutes(LOT_VALIDITE_MINUTES);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO ent_msg_lot
                (id, etablissement_id, auteur_user_id, empreinte, nb_eleves, nb_fils, nb_exclus, expire_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(user)
        .bind(&empreinte)
        .bind(eleves.len() as i16)
        .bind(nb_fils as i16)
        .bind(exclus.min(i16::MAX as usize) as i16)
        .bind(expire)
        .execute(&mut *tx)
        .await?;
        for e in &eleves {
            sqlx::query("INSERT INTO ent_msg_lot_eleve (etablissement_id, lot_id, eleve_id) VALUES ($1,$2,$3)")
                .bind(etab)
                .bind(id)
                .bind(e)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(ApercuLot {
            lot_id: id,
            empreinte,
            nb_eleves: eleves.len(),
            nb_fils,
            eleves_hors_droits: hors_droits,
            exclus,
            expire_le: expire,
        })
    }

    /// Confirme un lot : le texte rendu doit avoir **la même empreinte** que celui de l'aperçu, l'aperçu
    /// doit être celui de l'acteur, non expiré, non déjà confirmé (une seule confirmation). Chaque élève est
    /// ensuite **rejugé** (matrice recalculée au moment de l'envoi, R-M2) : un élève devenu hors droits est
    /// passé sous silence, jamais joint. Rend les fils créés.
    pub async fn confirmer_lot_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        lot: Uuid,
        sujet: &str,
        corps: &str,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let maintenant = self.maintenant_base(etab).await?;
        self.confirmer_lot_a(etab, user, role, lot, sujet, corps, maintenant).await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn confirmer_lot_a(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        lot: Uuid,
        sujet: &str,
        corps: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<Vec<Uuid>, EntDbError> {
        if !self.a_action_etablissement(etab, user, role, Action::MessagerieLotEnvoyer).await? {
            return Err(EntDbError::Forbidden);
        }
        let ligne: Option<(String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT empreinte, etat, expire_le FROM ent_msg_lot
              WHERE etablissement_id = $1 AND id = $2 AND auteur_user_id = $3",
        )
        .bind(etab)
        .bind(lot)
        .bind(user)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let (empreinte, etat, expire) = ligne.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if etat != "apercu" {
            return Err(conflit("ce lot a déjà été confirmé ou a expiré"));
        }
        if expire <= maintenant {
            return Err(conflit("l'aperçu a expiré : refaites l'aperçu"));
        }
        let eleves: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT eleve_id FROM ent_msg_lot_eleve WHERE etablissement_id = $1 AND lot_id = $2 ORDER BY eleve_id",
        )
        .bind(etab)
        .bind(lot)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let eleves: Vec<Uuid> = eleves.into_iter().map(|(e,)| e).collect();
        if self.empreinte_sql(etab, &contenu_lot(sujet, corps, &eleves)).await? != empreinte {
            return Err(conflit("le texte a changé depuis l'aperçu : refaites l'aperçu"));
        }
        // Une seule confirmation : on réserve le lot avant d'écrire quoi que ce soit.
        let pris = {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            let r = sqlx::query(
                "UPDATE ent_msg_lot SET etat = 'confirme', confirme_le = now()
                  WHERE etablissement_id = $1 AND id = $2 AND etat = 'apercu' AND expire_le > $3",
            )
            .bind(etab)
            .bind(lot)
            .bind(maintenant)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            r.rows_affected() == 1
        };
        if !pris {
            return Err(conflit("ce lot a déjà été confirmé ou a expiré"));
        }
        // Le plafond est revérifié avant d'écrire (la matrice a pu changer depuis l'aperçu).
        let mut a_joindre = Vec::new();
        let mut total = 0usize;
        for e in &eleves {
            if let Ok(a) = self.apercu_destinataires_for_actor(etab, user, role, *e).await {
                if a.decision.est_permis() {
                    let n = if a.destinataire_est_l_eleve { 1 } else { a.conversations.len() };
                    if n > 0 {
                        total += n;
                        a_joindre.push(*e);
                    }
                }
            }
        }
        if total > LOT_MAX_FILS {
            return Err(conflit("ce lot dépasse 30 conversations : refaites l'aperçu"));
        }
        let mut crees: Vec<Uuid> = Vec::new();
        for e in a_joindre {
            let fils = self
                .ouvrir_fils_a(
                    role,
                    OuvrirFil {
                        etablissement_id: etab,
                        eleve_id: e,
                        sujet: sujet.to_string(),
                        premier_message: corps.to_string(),
                        actor_user_id: user,
                    },
                    OptionsEnvoiMessage::default(),
                    None,
                    maintenant,
                )
                .await?;
            crees.extend(fils.into_iter().map(|f| f.fil.id));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query("UPDATE ent_msg_fil SET lot_id = $3 WHERE etablissement_id = $1 AND id = ANY($2)")
            .bind(etab)
            .bind(&crees)
            .bind(lot)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.publier_m4(
            etab,
            types::MESSAGERIE_LOT_ENVOYE,
            lot,
            user,
            json!({ "lot_id": lot, "nb_fils": crees.len() }),
        )
        .await?;
        Ok(crees)
    }

    async fn publier_m4(
        &self,
        etab: Uuid,
        type_name: &str,
        agregat: Uuid,
        acteur: Uuid,
        payload: serde_json::Value,
    ) -> Result<(), EntDbError> {
        self.events
            .publish(MetierEventDraft {
                etablissement_id: etab,
                type_name: type_name.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: agregat,
                eleve_id: None,
                payload,
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        Ok(())
    }

    // ───────────────────────────── Modèles à trois portées ─────────────────────────────

    /// Crée un modèle : personnel (tout personnel qui écrit), de boîte (un **membre** de la boîte),
    /// d'établissement (la direction, `messagerie.boites.gerer`).
    pub async fn creer_modele_portee_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        titre: &str,
        sujet: &str,
        corps: &str,
        portee: PorteeModele,
    ) -> Result<Uuid, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        match portee {
            PorteeModele::Etablissement => {
                if !self.a_action_etablissement(etab, user, role, Action::MessagerieBoitesGerer).await? {
                    return Err(EntDbError::Forbidden);
                }
            }
            PorteeModele::Boite(b) => {
                if !self.est_membre_boite(etab, b, user).await? {
                    return Err(EntDbError::Forbidden);
                }
            }
            PorteeModele::Personnel => {}
        }
        let (titre, sujet, corps) = (titre.trim(), sujet.trim(), corps.trim());
        if titre.is_empty() || sujet.is_empty() || corps.is_empty() {
            return Err(EntDbError::Domain(DomainError::MessageVide));
        }
        if titre.chars().count() > 120 || sujet.chars().count() > 200 || corps.chars().count() > 20_000 {
            return Err(EntDbError::Domain(DomainError::MessageTropLong));
        }
        let (code, boite) = match portee {
            PorteeModele::Personnel => ("personnel", None),
            PorteeModele::Boite(b) => ("boite", Some(b)),
            PorteeModele::Etablissement => ("etablissement", None),
        };
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO ent_msg_modele
                (id, etablissement_id, titre, sujet, corps, actif, created_by_user_id, portee, boite_id)
             VALUES ($1,$2,$3,$4,$5,true,$6,$7,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(titre)
        .bind(sujet)
        .bind(corps)
        .bind(user)
        .bind(code)
        .bind(boite)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Modèles que le compte voit : les siens, ceux de ses boîtes (membre à la date), ceux de l'établissement.
    pub async fn modeles_visibles_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<Vec<ModeleVu>, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        Ok(sqlx::query_as::<_, ModeleVu>(
            "SELECT m.id, m.titre, m.sujet, m.corps, m.portee, m.boite_id
               FROM ent_msg_modele m
              WHERE m.etablissement_id = $1 AND m.actif
                AND (m.portee = 'etablissement'
                     OR (m.portee = 'personnel' AND m.created_by_user_id = $2)
                     OR (m.portee = 'boite' AND EXISTS (
                            SELECT 1 FROM ent_msg_boite_membres($1, m.boite_id) bm WHERE bm.user_id = $2)))
              ORDER BY m.portee, m.titre",
        )
        .bind(etab)
        .bind(user)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    // ───────────────────────────── Aperçu de remise, programmation (R-M20, R-M21) ─────────────────────────────

    /// Quand un message écrit **maintenant** sera-t-il lu ? `None` : tout de suite ; `Some(instant)` : à la fin
    /// de la plage calme. L'auteur le voit **avant** d'envoyer (personnel → famille ou élève : « partira lundi
    /// 7 h » ; famille ou élève → personnel : « sera lu à partir de demain 7 h », CM-26).
    pub async fn lecture_prevue(
        &self,
        etab: Uuid,
        instant: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, EntDbError> {
        let (r,): (Option<DateTime<Utc>>,) = sqlx::query_as("SELECT ent_msg_remise_calme($1, $2)")
            .bind(etab)
            .bind(instant)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await?;
        Ok(r)
    }

    /// Répondre avec un envoi **programmé** (personnel seulement). La remise est le plus tardif de la
    /// programmation et de la fin de la plage calme : on ne contourne pas la plage en programmant. Annulable
    /// par l'auteur jusqu'à la remise (`annuler_message_differe_for_actor`).
    pub async fn repondre_programme_for_actor(
        &self,
        role: &str,
        cmd: RepondreFil,
        programme_le: DateTime<Utc>,
    ) -> Result<(domain_eleve::Message, Option<DateTime<Utc>>), EntDbError> {
        // F7 : la remise est comparée ensuite à `now()` de la base : même horloge.
        let maintenant = self.maintenant_base(cmd.etablissement_id).await?;
        self.repondre_programme_a(role, cmd, programme_le, maintenant).await
    }

    pub(crate) async fn repondre_programme_a(
        &self,
        role: &str,
        cmd: RepondreFil,
        programme_le: DateTime<Utc>,
        maintenant: DateTime<Utc>,
    ) -> Result<(domain_eleve::Message, Option<DateTime<Utc>>), EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        if programme_le <= maintenant + Duration::minutes(1) {
            return Err(refus("la programmation doit être dans le futur"));
        }
        if programme_le > maintenant + Duration::days(PROGRAMMATION_JOURS) {
            return Err(refus("programmation trop lointaine (90 jours au plus)"));
        }
        let etab = cmd.etablissement_id;
        let envoye = self
            .repondre_fil_programme_a(role, cmd, OptionsEnvoiMessage::default(), maintenant, Some(programme_le))
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query("UPDATE ent_msg SET programme_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(envoye.message.id)
            .bind(programme_le)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok((envoye.message, envoye.remise_prevue_le))
    }

    // ───────────────────────────── Correction et retrait (R-M27, R-M28) ─────────────────────────────

    /// Règle 4 : l'instant des délais de la messagerie (15 min de correction, 10 min de retrait) est celui de la
    /// base, comme `created_at` et `remise_prevue_le` (jamais l'horloge de l'application).
    async fn maintenant_base(&self, etab: Uuid) -> Result<DateTime<Utc>, EntDbError> {
        Ok(sqlx::query_scalar("SELECT now()").fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?).await?)
    }

    async fn message_de_l_auteur(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        message: Uuid,
    ) -> Result<(Uuid, String, i32, DateTime<Utc>, Option<DateTime<Utc>>, bool), EntDbError> {
        let ligne: Option<(Uuid, Uuid, String, i32, DateTime<Utc>, Option<DateTime<Utc>>, bool)> = sqlx::query_as(
            "SELECT fil_id, auteur_user_id, corps, version, created_at, remise_prevue_le, retire_le IS NOT NULL
               FROM ent_msg WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(message)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let (fil_id, auteur, corps, version, cree, remise, retire) =
            ligne.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        // Règle 3 : un différé d'autrui est un 404, identique à un identifiant inconnu, avant tout jugement d'auteur.
        if self.messages.message_differe_d_autrui(etab, message, user).await? {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        // Le message n'existe pour l'acteur que si le fil lui est lisible.
        let fil: MsgFil = self
            .messages
            .get_fil(etab, fil_id)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if !self.peut_lire_fil(etab, user, role, &fil).await? {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        if auteur != user {
            return Err(EntDbError::Forbidden);
        }
        if retire {
            return Err(conflit("ce message est retiré"));
        }
        Ok((fil_id, corps, version, cree, remise, retire))
    }

    /// Corriger son message (R-M27, CM-55) : **15 minutes**, **si personne d'autre ne l'a lu**. La version
    /// remplacée est conservée (`ent_msg_version`, ajout seul) ; le message est marqué « modifié ».
    pub async fn corriger_message_fil_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        message: Uuid,
        nouveau_corps: &str,
    ) -> Result<(), EntDbError> {
        let maintenant = self.maintenant_base(etab).await?;
        self.corriger_message_fil_a(etab, user, role, message, nouveau_corps, maintenant).await
    }

    pub(crate) async fn corriger_message_fil_a(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        message: Uuid,
        nouveau_corps: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (fil_id, ancien, version, cree, remise, _) = self.message_de_l_auteur(etab, user, role, message).await?;
        let corps = nouveau_corps.trim();
        if corps.is_empty() {
            return Err(EntDbError::Domain(DomainError::MessageVide));
        }
        if corps.chars().count() > 20_000 {
            return Err(EntDbError::Domain(DomainError::MessageTropLong));
        }
        if corps == ancien.trim() {
            return Err(refus("aucune modification"));
        }
        // Différé : personne n'a pu le lire, la correction reste libre jusqu'à la remise.
        let non_remis = remise.is_some_and(|r| r > maintenant);
        if !non_remis && maintenant - cree > Duration::minutes(CORRECTION_MINUTES) {
            return Err(conflit("le délai de correction (15 minutes) est dépassé : envoyez un nouveau message"));
        }
        let (lu,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_msg_accuse a
                             WHERE a.etablissement_id = $1 AND a.message_id = $2 AND a.lecteur_user_id <> $3)",
        )
        .bind(etab)
        .bind(message)
        .bind(user)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        if lu {
            return Err(conflit("ce message a déjà été lu : envoyez un nouveau message"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO ent_msg_version (id, etablissement_id, message_id, version, corps, par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(message)
        .bind(version)
        .bind(&ancien)
        .bind(user)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_msg SET corps = $3, version = version + 1, modifie_le = now(),
                    empreinte = encode(sha256(convert_to($3, 'UTF8')), 'hex')
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(message)
        .bind(corps)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.publier_m4(
            etab,
            types::MESSAGERIE_MESSAGE_CORRIGE,
            fil_id,
            user,
            json!({ "message_id": message, "version": version + 1 }),
        )
        .await?;
        Ok(())
    }

    /// Retirer son message (R-M28, CM-54, CM-56) : **10 minutes** après la remise ; au-delà, retrait par
    /// modération seulement (M10). C'est un **masquage tracé** : le contenu reste conservé (preuve), il n'est
    /// plus servi ; jamais de suppression physique.
    pub async fn retirer_message_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        message: Uuid,
        motif_code: Option<&str>,
    ) -> Result<(), EntDbError> {
        let maintenant = self.maintenant_base(etab).await?;
        self.retirer_message_a(etab, user, role, message, motif_code, maintenant).await
    }

    pub(crate) async fn retirer_message_a(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        message: Uuid,
        motif_code: Option<&str>,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let (fil_id, _, _, cree, remise, _) = self.message_de_l_auteur(etab, user, role, message).await?;
        let effectif = remise.map_or(cree, |r| r.max(cree));
        if maintenant - effectif > Duration::minutes(RETRAIT_MINUTES) {
            return Err(conflit(
                "le délai de retrait (10 minutes) est dépassé : demandez le retrait à l'établissement",
            ));
        }
        let motif = motif_code.unwrap_or("erreur_auteur");
        if !motif.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            || !motif.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            || motif.len() > 40
        {
            return Err(refus("motif de retrait invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "UPDATE ent_msg SET retire_le = now(), retire_par = $3, retrait_par = 'auteur', retrait_motif_code = $4
              WHERE etablissement_id = $1 AND id = $2 AND retire_le IS NULL",
        )
        .bind(etab)
        .bind(message)
        .bind(user)
        .bind(motif)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            return Err(conflit("ce message est déjà retiré"));
        }
        tx.commit().await?;
        self.publier_m4(
            etab,
            types::MESSAGERIE_MESSAGE_RETIRE,
            fil_id,
            user,
            json!({ "message_id": message, "par": "auteur" }),
        )
        .await?;
        Ok(())
    }

    // ───────────────────────────── Brouillons ─────────────────────────────

    /// Enregistre (ou met à jour) un brouillon : réponse dans un fil lisible de l'auteur, ou premier message à
    /// propos d'un élève. Strictement personnel ; 50 au plus ; périmé après 90 jours.
    pub async fn enregistrer_brouillon_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        s: BrouillonSaisie,
    ) -> Result<Uuid, EntDbError> {
        if s.fil_id.is_none() && s.eleve_id.is_none() {
            return Err(refus("un brouillon vise un fil ou un élève"));
        }
        if s.sujet.chars().count() > 200 || s.corps.chars().count() > 20_000 {
            return Err(EntDbError::Domain(DomainError::MessageTropLong));
        }
        if let Some(f) = s.fil_id {
            let fil: MsgFil = self
                .messages
                .get_fil(etab, f)
                .await?
                .ok_or(EntDbError::Domain(DomainError::NotFound))?;
            if !self.peut_lire_fil(etab, user, role, &fil).await? {
                return Err(EntDbError::Domain(DomainError::NotFound));
            }
        } else if let Some(e) = s.eleve_id {
            if !self.actor_can_access_msg_fil(etab, user, role, e).await? {
                return Err(EntDbError::Forbidden);
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if let Some(id) = s.id {
            let r = sqlx::query(
                "UPDATE ent_msg_brouillon SET sujet = $4, corps = $5, maj_le = now(),
                        expire_le = now() + interval '90 days'
                  WHERE etablissement_id = $1 AND id = $2 AND auteur_user_id = $3",
            )
            .bind(etab)
            .bind(id)
            .bind(user)
            .bind(&s.sujet)
            .bind(&s.corps)
            .execute(&mut *tx)
            .await?;
            if r.rows_affected() == 0 {
                return Err(EntDbError::Domain(DomainError::NotFound));
            }
            tx.commit().await?;
            return Ok(id);
        }
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_msg_brouillon WHERE etablissement_id = $1 AND auteur_user_id = $2",
        )
        .bind(etab)
        .bind(user)
        .fetch_one(&mut *tx)
        .await?;
        if n >= BROUILLONS_MAX {
            return Err(conflit("trop de brouillons : supprimez-en"));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_msg_brouillon (id, etablissement_id, auteur_user_id, fil_id, eleve_id, sujet, corps)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(etab)
        .bind(user)
        .bind(s.fil_id)
        .bind(s.eleve_id)
        .bind(&s.sujet)
        .bind(&s.corps)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Brouillons de l'auteur, du plus récent au plus ancien. Les brouillons périmés sont purgés au passage.
    pub async fn lister_brouillons_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
    ) -> Result<Vec<BrouillonVu>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query("DELETE FROM ent_msg_brouillon WHERE etablissement_id = $1 AND expire_le < now()")
            .bind(etab)
            .execute(&mut *tx)
            .await?;
        let v = sqlx::query_as::<_, BrouillonVu>(
            "SELECT id, fil_id, eleve_id, sujet, corps, maj_le FROM ent_msg_brouillon
              WHERE etablissement_id = $1 AND auteur_user_id = $2 ORDER BY maj_le DESC",
        )
        .bind(etab)
        .bind(user)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(v)
    }

    pub async fn supprimer_brouillon_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        brouillon: Uuid,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "DELETE FROM ent_msg_brouillon WHERE etablissement_id = $1 AND id = $2 AND auteur_user_id = $3",
        )
        .bind(etab)
        .bind(brouillon)
        .bind(user)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        tx.commit().await?;
        Ok(())
    }

    /// Envoie un brouillon par le chemin ordinaire (plage calme, matrice, avis) puis le supprime. Rend le
    /// fil concerné.
    pub async fn envoyer_brouillon_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        brouillon: Uuid,
    ) -> Result<Uuid, EntDbError> {
        let b: Option<BrouillonVu> = sqlx::query_as(
            "SELECT id, fil_id, eleve_id, sujet, corps, maj_le FROM ent_msg_brouillon
              WHERE etablissement_id = $1 AND id = $2 AND auteur_user_id = $3",
        )
        .bind(etab)
        .bind(brouillon)
        .bind(user)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let b = b.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let fil = if let Some(f) = b.fil_id {
            self.repondre_fil_for_actor(
                role,
                RepondreFil { etablissement_id: etab, fil_id: f, corps: b.corps.clone(), actor_user_id: user },
            )
            .await?;
            f
        } else {
            let eleve = b.eleve_id.ok_or(refus("brouillon sans cible"))?;
            let ouverts = self
                .ouvrir_fils_for_actor(
                    role,
                    OuvrirFil {
                        etablissement_id: etab,
                        eleve_id: eleve,
                        sujet: b.sujet.clone(),
                        premier_message: b.corps.clone(),
                        actor_user_id: user,
                    },
                    OptionsEnvoiMessage::default(),
                )
                .await?;
            ouverts.first().map(|o| o.fil.id).ok_or(refus("aucun fil créé"))?
        };
        self.supprimer_brouillon_for_actor(etab, user, brouillon).await?;
        Ok(fil)
    }

    // ───────────────────────────── Fusion et scission de fils (MSG-174) ─────────────────────────────

    /// Un personnel qui lit **et écrit** dans ce fil (membre traitant s'il s'agit d'un fil de boîte).
    async fn exiger_editeur_fil(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil_id: Uuid,
    ) -> Result<MsgFil, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        let fil: MsgFil = self
            .messages
            .get_fil(etab, fil_id)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if !self.peut_lire_fil(etab, user, role, &fil).await? {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        if !self.peut_ecrire_fil(etab, user, role, &fil).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(fil)
    }

    /// Fusionne deux fils d'une **même famille** sur un même sujet : même élève, même nature et sensibilité,
    /// mêmes participants famille, élève et boîtes ; sinon refus. Les messages sont déplacés (jamais copiés), avec
    /// leurs pièces et accusés ; le fil absorbé est clos et archivé. Rend le nombre de messages déplacés.
    pub async fn fusionner_fils_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        garde: Uuid,
        absorbe: Uuid,
    ) -> Result<usize, EntDbError> {
        if garde == absorbe {
            return Err(refus("deux fils différents sont nécessaires"));
        }
        self.exiger_editeur_fil(etab, user, role, garde).await?;
        self.exiger_editeur_fil(etab, user, role, absorbe).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (n,): (Option<i32>,) = sqlx::query_as("SELECT ent_msg_fusionner($1, $2)")
            .bind(garde)
            .bind(absorbe)
            .fetch_one(&mut *tx)
            .await?;
        let n = n.ok_or(conflit(
            "ces fils ne sont pas fusionnables : même élève, même famille et mêmes participants sont exigés",
        ))?;
        tx.commit().await?;
        self.publier_m4(
            etab,
            types::MESSAGERIE_FILS_FUSIONNES,
            garde,
            user,
            json!({ "garde": garde, "absorbe": absorbe, "messages": n }),
        )
        .await?;
        Ok(n as usize)
    }

    /// Scinde un fil à partir d'un message : le message et les suivants passent dans un fil frère (mêmes
    /// participants, même portée). Le premier message ne se scinde pas (rien ne resterait). Rend le nouveau fil.
    pub async fn scinder_fil_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        a_partir_du_message: Uuid,
        nouveau_sujet: &str,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_editeur_fil(etab, user, role, fil).await?;
        let sujet = nouveau_sujet.trim();
        if sujet.is_empty() {
            return Err(EntDbError::Domain(DomainError::MessageVide));
        }
        if sujet.chars().count() > 200 {
            return Err(EntDbError::Domain(DomainError::MessageTropLong));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (nouveau,): (Option<Uuid>,) = sqlx::query_as("SELECT ent_msg_scinder($1, $2, $3)")
            .bind(fil)
            .bind(a_partir_du_message)
            .bind(sujet)
            .fetch_one(&mut *tx)
            .await?;
        let nouveau = nouveau.ok_or(conflit("ce message ne peut pas servir de point de scission"))?;
        tx.commit().await?;
        self.publier_m4(
            etab,
            types::MESSAGERIE_FIL_SCINDE,
            fil,
            user,
            json!({ "fil": fil, "nouveau": nouveau }),
        )
        .await?;
        Ok(nouveau)
    }
}
