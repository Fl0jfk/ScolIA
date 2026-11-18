//! Messagerie, lot M6 (`docs/socles/16_MESSAGERIE.md` §13, §23 ; R-M15, R-M16, R-M19, R-M22 ; CM-46, CM-61, CM-62,
//! Q-M8) : lecture et avis.
//!
//! * **« Lu jusqu'à » par participant** (`ent_msg_lecture`) : une ligne par participant (personne, élève ou boîte) ;
//!   pour une boîte, la lecture est celle du **premier membre** qui ouvre. Elle ne recule jamais.
//! * **R-M19 — lu par le destinataire** : seule une **participation active** pose « lu jusqu'à ». Une lecture par
//!   supervision, par un compte qui n'est pas participant (direction hors boîte, membre d'une autre boîte) ne pose
//!   **rien** ; un auteur n'accuse pas ses propres messages (seuls les pointeurs de l'autre côté comptent). Il n'existe
//!   pas de réglage par lequel un destinataire **refuse** l'accusé : la règle R-M19 ne le prévoit pas (S-145 « accusé
//!   configurable » est écarté, voir `docs/socles/16_MESSAGERIE.md` §29) ; « refus » = refus de poser un accusé pour
//!   une lecture qui n'est pas celle d'un destinataire.
//! * **Ce que voit chacun** (§13, Q-M8, R-M22) : la famille voit « lu par l'établissement » (boîte) ou « lu par
//!   Prénom X. » (fil individuel avec un seul interlocuteur), **sans heure**, jamais le nom d'un membre de boîte ;
//!   le personnel voit, par participant famille ou élève, jusqu'où il a lu et depuis quand, et « non lu après 3 jours »
//!   (CM-61). Jamais d'indicateur de présence ni d'heure de lecture des personnels côté familles (R-M22).
//! * **Prise en charge visible** (boîtes) : la famille voit un état neutre (« pris en charge », « en attente de votre
//!   réponse », « traité ») sans nom ; les membres de la boîte voient qui s'en occupe.
//! * **Disponibilité** : « lira votre message à partir de lundi à 7 h 30 », d'après les horaires de la boîte (M3) ou les
//!   plages de joignabilité de la personne (socle 6), à l'heure de l'établissement.
//! * **Avis** : les natures du §23 sont déclarées au socle 8 (`ntf::catalogue`) ; les textes sont **ceux du gabarit**, jamais
//!   un contenu de message, un objet ou un nom (R-M16).

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use chrono::{DateTime, Datelike, Duration, NaiveDateTime, NaiveTime, Utc};
use domain_eleve::{DomainError, Message};
use ent_auth::Role;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::ntf::catalogue::MESSAGERIE_REPONSE_ATTENDUE;
use crate::ntf::reglages::{local_de, lire_parametre};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Au-delà de ce délai sans lecture, le personnel voit « non lu après 3 jours » (CM-61) ; aucune relance infinie : l'escalade
/// est celle du socle 8.
pub const NON_LU_APRES_JOURS: i64 = 3;

fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::MsgFilIntrouvable)
}

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

// ───────────────────────── Disponibilité (pur) ─────────────────────────

/// Une plage d'ouverture : jour ISO (1 = lundi … 7 = dimanche), début, fin.
pub type PlageOuverture = (i16, NaiveTime, NaiveTime);

/// Où en est la disponibilité à un instant local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disponibilite {
    /// Dans une plage (ou aucune plage déclarée : on ne promet ni n'interdit rien).
    Ouverte,
    /// Fermée : la prochaine ouverture, à l'heure locale de l'établissement.
    ProchaineOuverture(NaiveDateTime),
}

/// Prochaine ouverture d'un service ou d'une personne, à l'heure **locale** `local`. Sans plage déclarée : `Ouverte`
/// (aucune promesse de délai, R-M26). Cherche sur les huit jours qui suivent.
pub fn disponibilite_a(plages: &[PlageOuverture], local: NaiveDateTime) -> Disponibilite {
    if plages.is_empty() {
        return Disponibilite::Ouverte;
    }
    let jour_iso = |d: NaiveDateTime| d.date().weekday().number_from_monday() as i16;
    for decalage in 0..=8i64 {
        let date = local.date() + Duration::days(decalage);
        let jour = jour_iso(date.and_hms_opt(0, 0, 0).expect("minuit"));
        let mut du_jour: Vec<&PlageOuverture> = plages.iter().filter(|p| p.0 == jour).collect();
        du_jour.sort_by_key(|p| p.1);
        for (_, debut, fin) in du_jour {
            let (d, f) = (date.and_time(*debut), date.and_time(*fin));
            if decalage == 0 && local >= d && local < f {
                return Disponibilite::Ouverte;
            }
            if d > local {
                return Disponibilite::ProchaineOuverture(d);
            }
        }
    }
    Disponibilite::Ouverte
}

fn jour_francais(i: u32) -> &'static str {
    ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"][(i as usize).min(6)]
}

/// « lundi à 7 h 30 » (le jour de la semaine suffit : la prochaine ouverture est dans la semaine).
pub fn libelle_ouverture(ouverture: NaiveDateTime, local: NaiveDateTime) -> String {
    use chrono::Timelike;
    let h = ouverture.hour();
    let m = ouverture.minute();
    let heure = if m == 0 { format!("{h} h") } else { format!("{h} h {m:02}") };
    if ouverture.date() == local.date() {
        format!("aujourd'hui à {heure}")
    } else if ouverture.date() == local.date() + Duration::days(1) {
        format!("demain à {heure}")
    } else {
        format!("{} à {heure}", jour_francais(ouverture.date().weekday().num_days_from_monday()))
    }
}

// ───────────────────────── Types publiés ─────────────────────────

/// Ce que le personnel voit d'un participant famille ou élève.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LectureParticipant {
    pub participant_id: Uuid,
    /// `famille` ou `eleve`.
    pub cote: String,
    /// Prénom et initiale (jamais une coordonnée).
    pub libelle: String,
    pub lu_jusqua_message_id: Option<Uuid>,
    /// Quand il a lu (première lecture) : visible **du personnel seulement** (R-M22).
    pub lu_depuis: Option<DateTime<Utc>>,
    /// Un message du personnel plus ancien que trois jours n'est pas lu (CM-61).
    pub non_lu_apres_3_jours: bool,
}

/// Ce que la famille voit d'un de **ses** messages : jamais l'heure, jamais le nom d'un membre de boîte.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LectureMessage {
    pub message_id: Uuid,
    pub lu: bool,
    /// « l'établissement » ou « Prénom X. » (fil individuel, un seul interlocuteur) ; absent si non lu.
    pub lu_par: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PriseEnCharge {
    /// `nouveau`, `pris`, `attente_famille`, `traite` (jamais `transfere` : un transfert est interne).
    pub statut: &'static str,
    pub libelle: &'static str,
    /// Nom du membre qui s'en occupe : **membres de la boîte seulement**.
    pub pris_par: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DisponibiliteVue {
    pub ouverte: bool,
    /// « La vie scolaire lira votre message à partir de lundi à 7 h 30. » ; absent si ouverte.
    pub libelle: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VueLecture {
    pub fil_id: Uuid,
    /// `etablissement`, `famille`, `eleve`, ou `hors_participation` (accès sans participation : rien n'est posé).
    pub cote_lecteur: &'static str,
    /// Famille et élève : l'état de lecture de **leurs** messages.
    pub messages: Vec<LectureMessage>,
    /// Personnel : l'état de lecture de chaque participant famille ou élève.
    pub participants: Vec<LectureParticipant>,
    pub prise_en_charge: Option<PriseEnCharge>,
    pub disponibilite: Option<DisponibiliteVue>,
}

#[derive(Debug, FromRow)]
struct Participation {
    participant_id: Uuid,
    cote: String,
    boite_id: Option<Uuid>,
    personne_id: Option<Uuid>,
}

#[derive(Debug, FromRow)]
struct LigneLecture {
    participant_id: Uuid,
    cote: String,
    boite_id: Option<Uuid>,
    personne_id: Option<Uuid>,
    libelle: Option<String>,
    lu_jusqua_message_id: Option<Uuid>,
    lu_jusqua_message_le: Option<DateTime<Utc>>,
    premiere_lecture_le: Option<DateTime<Utc>>,
}

fn libelle_statut(statut: &str) -> (&'static str, &'static str) {
    match statut {
        "pris" | "transfere" => ("pris", "pris en charge"),
        "attente_famille" => ("attente_famille", "en attente de votre réponse"),
        "traite" => ("traite", "traité"),
        _ => ("nouveau", "reçu par l'établissement"),
    }
}

impl PgEnt {
    async fn participations_du_compte(
        &self,
        etab: Uuid,
        user: Uuid,
        fil: Uuid,
    ) -> Result<Vec<Participation>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let v = sqlx::query_as::<_, Participation>(
            "SELECT participant_id, cote, boite_id, personne_id FROM ent_msg_mes_participations($1, $2, $3)",
        )
        .bind(etab)
        .bind(user)
        .bind(fil)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(v)
    }

    /// Pose « lu jusqu'à » pour les participations **actives** du lecteur (R-M19). `msgs` : ce que le lecteur voit déjà
    /// (la vue filtrée). Rend `true` si au moins un pointeur a avancé. Sans participation : ne pose rien.
    pub(crate) async fn poser_lu_jusqua(
        &self,
        etab: Uuid,
        user: Uuid,
        fil: Uuid,
        msgs: &[Message],
        jusqua: Option<Uuid>,
    ) -> Result<bool, EntDbError> {
        let cible = match jusqua {
            Some(id) => msgs.iter().find(|m| m.id == id).ok_or(EntDbError::SaisieInvalide("message inconnu dans ce fil"))?,
            None => match msgs.iter().max_by_key(|m| (m.created_at, m.id)) {
                Some(m) => m,
                None => return Ok(false),
            },
        };
        let participations = self.participations_du_compte(etab, user, fil).await?;
        if participations.is_empty() {
            // Supervision, direction hors participation, membre d'une autre boîte : aucune lecture de destinataire.
            return Ok(false);
        }
        let personne: Option<Uuid> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let p: Option<Uuid> = sqlx::query_scalar("SELECT ent_msg_personne_du_compte($1, $2)")
                .bind(etab)
                .bind(user)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
            tx.commit().await.map_err(db)?;
            p
        };
        let mut avance = false;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        for p in &participations {
            let r = sqlx::query(
                "INSERT INTO ent_msg_lecture
                    (etablissement_id, fil_id, participant_id, lu_jusqua_message_id, lu_jusqua_message_le, lu_par_personne_id)
                 VALUES ($1,$2,$3,$4,$5,$6)
                 ON CONFLICT (fil_id, participant_id) DO UPDATE
                    SET lu_jusqua_message_id = EXCLUDED.lu_jusqua_message_id,
                        lu_jusqua_message_le = EXCLUDED.lu_jusqua_message_le,
                        lu_par_personne_id = EXCLUDED.lu_par_personne_id,
                        maj_le = now()
                  WHERE EXCLUDED.lu_jusqua_message_le > ent_msg_lecture.lu_jusqua_message_le",
            )
            .bind(etab)
            .bind(fil)
            .bind(p.participant_id)
            .bind(cible.id)
            .bind(cible.created_at)
            // Le membre qui a ouvert une boîte : visible du personnel de la boîte seulement.
            .bind(if p.boite_id.is_some() { personne } else { p.personne_id })
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            avance |= r.rows_affected() > 0;
        }
        tx.commit().await.map_err(db)?;
        if avance {
            let eleve = self.messages.get_fil(etab, fil).await?.map(|f| f.eleve_id);
            self.events
                .publish(MetierEventDraft {
                    etablissement_id: etab,
                    type_name: types::MESSAGERIE_LU_JUSQUA.to_string(),
                    aggregate: AggregateKind::MsgFil,
                    aggregate_id: fil,
                    eleve_id: eleve,
                    // R-M14 : le fil et le message, jamais le contenu, le sujet, ni qui a lu.
                    payload: json!({ "message_id": cible.id }),
                    actor_user_id: None,
                })
                .await?;
        }
        Ok(avance)
    }

    /// Marque le fil lu **par ce compte** jusqu'à un message (ou jusqu'au dernier). Le fil doit être lisible par lui
    /// (404 uniforme sinon). Sans participation active, ne pose rien (R-M19) et rend `false`.
    pub async fn marquer_lu_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil_id: Uuid,
        jusqua: Option<Uuid>,
    ) -> Result<bool, EntDbError> {
        let fil = self.messages.get_fil(etab, fil_id).await?.ok_or_else(introuvable)?;
        if !self.actor_can_access_fil(etab, user, role, &fil).await? {
            return Err(introuvable());
        }
        let msgs = self.messages.list_messages(etab, fil_id, user).await?;
        self.poser_lu_jusqua(etab, user, fil_id, &msgs, jusqua).await
    }

    /// L'état de lecture, la prise en charge et la disponibilité d'un fil **tels que ce compte les voit**.
    pub async fn lecture_du_fil_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil_id: Uuid,
    ) -> Result<VueLecture, EntDbError> {
        self.lecture_du_fil_a(etab, user, role, fil_id, Utc::now()).await
    }

    pub(crate) async fn lecture_du_fil_a(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil_id: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<VueLecture, EntDbError> {
        let fil = self.messages.get_fil(etab, fil_id).await?.ok_or_else(introuvable)?;
        if !self.actor_can_access_fil(etab, user, role, &fil).await? {
            return Err(introuvable());
        }
        let msgs = self.messages.list_messages(etab, fil_id, user).await?;
        let moi = self.participations_du_compte(etab, user, fil_id).await?;
        let cote_lecteur: &'static str = if moi.iter().any(|p| p.cote == "etablissement") {
            "etablissement"
        } else if moi.iter().any(|p| p.cote == "famille") {
            "famille"
        } else if moi.iter().any(|p| p.cote == "eleve") {
            "eleve"
        } else {
            "hors_participation"
        };
        let est_personnel = Role::parse(role).is_some_and(|r| r.is_staff());
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let lignes: Vec<LigneLecture> = sqlx::query_as(
            "SELECT p.id AS participant_id, p.cote, p.boite_id, p.personne_id,
                    COALESCE(
                        (SELECT btrim(coalesce(left(pe.prenom, 40), '') || ' ' || upper(left(pe.nom, 1)) || '.')
                           FROM ent_personne pe WHERE pe.etablissement_id = p.etablissement_id AND pe.id = p.personne_id),
                        (SELECT btrim(coalesce(left(e.prenom, 40), '') || ' ' || upper(left(e.nom, 1)) || '.')
                           FROM ent_eleve e WHERE e.etablissement_id = p.etablissement_id AND e.id = p.eleve_id)
                    ) AS libelle,
                    l.lu_jusqua_message_id, l.lu_jusqua_message_le, l.premiere_lecture_le
               FROM ent_msg_participant p
               LEFT JOIN ent_msg_lecture l ON l.fil_id = p.fil_id AND l.participant_id = p.id
                                          AND l.etablissement_id = p.etablissement_id
              WHERE p.etablissement_id = $1 AND p.fil_id = $2 AND p.au IS NULL AND p.suspendu_le IS NULL
              ORDER BY p.cree_le, p.id",
        )
        .bind(etab)
        .bind(fil_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;

        let auteurs: Vec<Uuid> = msgs.iter().map(|m| m.auteur_user_id).collect();
        let roles = self.messages.roles_de(etab, &auteurs).await?;
        let est_famille_auteur = |m: &Message| {
            roles
                .get(&m.auteur_user_id)
                .is_some_and(|r| matches!(Role::parse(r), Some(Role::Parent | Role::Eleve)))
        };
        let boite = lignes.iter().find_map(|l| l.boite_id);

        let mut messages = Vec::new();
        let mut participants = Vec::new();
        if est_personnel {
            // Le personnel : jusqu'où chaque participant famille ou élève a lu, depuis quand, et « non lu après 3 jours ».
            let seuil = maintenant - Duration::days(NON_LU_APRES_JOURS);
            for l in lignes.iter().filter(|l| l.cote == "famille" || l.cote == "eleve") {
                let non_lu = msgs.iter().filter(|m| !est_famille_auteur(m) && m.created_at <= seuil).any(|m| {
                    l.lu_jusqua_message_le.is_none_or(|lu| lu < m.created_at)
                });
                participants.push(LectureParticipant {
                    participant_id: l.participant_id,
                    cote: l.cote.clone(),
                    libelle: l.libelle.clone().unwrap_or_else(|| "participant".into()),
                    lu_jusqua_message_id: l.lu_jusqua_message_id,
                    lu_depuis: l.premiere_lecture_le,
                    non_lu_apres_3_jours: non_lu,
                });
            }
        } else if cote_lecteur != "hors_participation" {
            // La famille ou l'élève : « lu par l'établissement » ou « lu par Prénom X. », **sans l'heure**, jamais le nom d'un membre
            // d'une boîte ni d'un autre membre du personnel qui aurait ouvert le fil (§21.3 point 4).
            let cote_etab: Vec<&LigneLecture> = lignes.iter().filter(|l| l.cote == "etablissement").collect();
            let un_seul_nomme = boite.is_none() && cote_etab.len() == 1 && cote_etab[0].personne_id.is_some();
            for m in msgs.iter().filter(|m| est_famille_auteur(m)) {
                let lu = cote_etab.iter().any(|l| l.lu_jusqua_message_le.is_some_and(|le| le >= m.created_at));
                let lu_par = lu.then(|| {
                    if un_seul_nomme {
                        cote_etab[0].libelle.clone().unwrap_or_else(|| "l'établissement".into())
                    } else {
                        "l'établissement".to_string()
                    }
                });
                messages.push(LectureMessage { message_id: m.id, lu, lu_par });
            }
        }

        // Prise en charge (boîtes) : état neutre pour la famille, le nom seulement pour les membres.
        let mut prise = None;
        if let Some(boite_id) = boite {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let t: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT t.statut,
                        (SELECT btrim(coalesce(left(pe.prenom, 40), '') || ' ' || upper(left(pe.nom, 1)) || '.')
                           FROM ent_personne pe WHERE pe.etablissement_id = t.etablissement_id
                            AND pe.id = t.pris_par_personne_id)
                   FROM ent_msg_traitement t
                  WHERE t.etablissement_id = $1 AND t.fil_id = $2 AND t.boite_id = $3",
            )
            .bind(etab)
            .bind(fil_id)
            .bind(boite_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            if let Some((statut, nom)) = t {
                let (code, libelle) = libelle_statut(&statut);
                // Les membres de la boîte (côté établissement, participation active) voient qui s'en occupe.
                let pris_par = if cote_lecteur == "etablissement" && code != "nouveau" { nom } else { None };
                prise = Some(PriseEnCharge { statut: code, libelle, pris_par });
            }
        }

        // Disponibilité : annoncée à la famille et à l'élève (CM-46), jamais au personnel sur lui-même.
        let mut disponibilite = None;
        if !est_personnel && cote_lecteur != "hors_participation" {
            let plages = self.plages_du_destinataire(etab, &lignes).await?;
            if !plages.is_empty() {
                let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
                let param = lire_parametre(&mut *tx, etab).await.map_err(db)?;
                let local = local_de(&mut *tx, &param.fuseau, maintenant).await.map_err(db)?;
                tx.commit().await.map_err(db)?;
                disponibilite = Some(match disponibilite_a(&plages, local) {
                    Disponibilite::Ouverte => DisponibiliteVue { ouverte: true, libelle: None },
                    Disponibilite::ProchaineOuverture(o) => DisponibiliteVue {
                        ouverte: false,
                        libelle: Some(format!(
                            "{} lira votre message à partir de {}.",
                            if boite.is_some() { "Le service" } else { "Votre interlocuteur" },
                            libelle_ouverture(o, local)
                        )),
                    },
                });
            }
        }
        Ok(VueLecture { fil_id, cote_lecteur, messages, participants, prise_en_charge: prise, disponibilite })
    }

    /// Plages d'ouverture de l'interlocuteur côté établissement : celles de la boîte, sinon celles de l'**unique** personnel
    /// participant (plages de joignabilité du socle 6). Plusieurs interlocuteurs individuels : rien n'est promis.
    async fn plages_du_destinataire(
        &self,
        etab: Uuid,
        lignes: &[LigneLecture],
    ) -> Result<Vec<PlageOuverture>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let plages = if let Some(b) = lignes.iter().find_map(|l| l.boite_id) {
            sqlx::query_as::<_, (i16, NaiveTime, NaiveTime)>(
                "SELECT jour_iso, debut, fin FROM ent_msg_boite_horaire WHERE etablissement_id = $1 AND boite_id = $2",
            )
            .bind(etab)
            .bind(b)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
        } else {
            let seuls: Vec<Uuid> = lignes
                .iter()
                .filter(|l| l.cote == "etablissement")
                .filter_map(|l| l.personne_id)
                .collect();
            if seuls.len() != 1 {
                Vec::new()
            } else {
                let lignes: Vec<(Vec<i16>, NaiveTime, NaiveTime)> = sqlx::query_as(
                    "SELECT jours, de, a FROM ent_personne_plage_joignable WHERE etablissement_id = $1 AND personne_id = $2",
                )
                .bind(etab)
                .bind(seuls[0])
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
                lignes
                    .into_iter()
                    .flat_map(|(jours, de, a)| jours.into_iter().map(move |j| (j, de, a)))
                    .collect()
            }
        };
        tx.commit().await.map_err(db)?;
        Ok(plages)
    }

    // ───────────────────────── Avis (§23) ─────────────────────────

    /// `messagerie.reponse_attendue` (U1, C2, pas de plancher) : la boîte attend la réponse de la famille. Les participants
    /// **famille** actifs du fil reçoivent l'enveloppe neutre du gabarit déclaré au socle 8 : ni sujet, ni contenu, ni
    /// nom (R-M16). Une relance au plus (clé de dédoublonnage par fil). Sans compte : rien (le socle 8 prévient les
    /// personnes sans compte ailleurs). Au mieux : un échec est journalisé, jamais remonté à l'action de traitement.
    pub(crate) async fn avertir_reponse_attendue(&self, etab: Uuid, fil: Uuid) {
        let r: Result<(), EntDbError> = async {
            let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
            let familles: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT DISTINCT pe.user_id
                   FROM ent_msg_participant p
                   JOIN ent_personne pe ON pe.etablissement_id = p.etablissement_id AND pe.id = p.personne_id
                  WHERE p.etablissement_id = $1 AND p.fil_id = $2 AND p.cote = 'famille'
                    AND p.au IS NULL AND p.suspendu_le IS NULL AND pe.user_id IS NOT NULL",
            )
            .bind(etab)
            .bind(fil)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let eleve: Option<(Uuid,)> =
                sqlx::query_as("SELECT eleve_id FROM ent_msg_fil WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(fil)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db)?;
            let g = MESSAGERIE_REPONSE_ATTENDUE.gabarit;
            for (user,) in familles {
                crate::notifications::notifier_personne(
                    &mut *tx,
                    crate::notifications::EnvoiAPlanifier {
                        etablissement_id: etab,
                        user_id: user,
                        // Famille : texte neutre sur tous les canaux externes (`canal_externe_neutre`).
                        kind: "messagerie_famille",
                        titre: g.titre,
                        corps: g.corps,
                        eleve_id: eleve.map(|e| e.0),
                        notification_id: None,
                        source_type: "systeme",
                        source_id: fil,
                        cle_dedup: format!("msgreponse:{fil}:{user}"),
                        droit_requis: None,
                        regroupable: false,
                        canaux_voulus: &["push"],
                    },
                    Uuid::new_v4(),
                    Utc::now(),
                )
                .await?;
            }
            tx.commit().await.map_err(db)?;
            Ok(())
        }
        .await;
        if let Err(e) = r {
            tracing::error!(canal = "messagerie", fil = %fil, error = %e, "avis « réponse attendue » non émis");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }
    fn dt(j: u32, h: u32, m: u32) -> NaiveDateTime {
        // 2030-04-xx : le 1er avril 2030 est un lundi.
        NaiveDate::from_ymd_opt(2030, 4, j).unwrap().and_time(t(h, m))
    }
    /// Lundi à vendredi, 7 h 30 – 19 h 00.
    fn semaine() -> Vec<PlageOuverture> {
        (1..=5).map(|j| (j, t(7, 30), t(19, 0))).collect()
    }

    #[test]
    fn ouverte_dans_une_plage_et_sans_plage() {
        assert_eq!(disponibilite_a(&semaine(), dt(1, 10, 0)), Disponibilite::Ouverte);
        assert_eq!(disponibilite_a(&[], dt(1, 3, 0)), Disponibilite::Ouverte, "aucune plage : aucune promesse");
    }

    #[test]
    fn prochaine_ouverture_de_nuit_de_soiree_et_de_week_end() {
        // Lundi 6 h : le jour même à 7 h 30.
        assert_eq!(disponibilite_a(&semaine(), dt(1, 6, 0)), Disponibilite::ProchaineOuverture(dt(1, 7, 30)));
        // Lundi 20 h : mardi 7 h 30.
        assert_eq!(disponibilite_a(&semaine(), dt(1, 20, 0)), Disponibilite::ProchaineOuverture(dt(2, 7, 30)));
        // Vendredi 19 h pile (fin exclue) puis samedi : lundi 7 h 30.
        assert_eq!(disponibilite_a(&semaine(), dt(5, 19, 0)), Disponibilite::ProchaineOuverture(dt(8, 7, 30)));
        assert_eq!(disponibilite_a(&semaine(), dt(6, 11, 0)), Disponibilite::ProchaineOuverture(dt(8, 7, 30)));
        // Plusieurs plages le même jour : la pause du midi.
        let midi = vec![(1, t(8, 0), t(12, 0)), (1, t(14, 0), t(17, 0))];
        assert_eq!(disponibilite_a(&midi, dt(1, 12, 30)), Disponibilite::ProchaineOuverture(dt(1, 14, 0)));
        assert_eq!(disponibilite_a(&midi, dt(1, 9, 0)), Disponibilite::Ouverte);
    }

    #[test]
    fn libelles_en_francais() {
        assert_eq!(libelle_ouverture(dt(1, 7, 30), dt(1, 6, 0)), "aujourd'hui à 7 h 30");
        assert_eq!(libelle_ouverture(dt(2, 8, 0), dt(1, 20, 0)), "demain à 8 h");
        assert_eq!(libelle_ouverture(dt(8, 7, 30), dt(5, 19, 0)), "lundi à 7 h 30");
    }

    #[test]
    fn statuts_neutres_pour_la_famille() {
        assert_eq!(libelle_statut("pris"), ("pris", "pris en charge"));
        assert_eq!(libelle_statut("transfere"), ("pris", "pris en charge"), "un transfert est interne");
        assert_eq!(libelle_statut("attente_famille").1, "en attente de votre réponse");
        assert_eq!(libelle_statut("traite").1, "traité");
        assert_eq!(libelle_statut("nouveau").0, "nouveau");
    }
}
