//! Messagerie, lot M3 (`docs/socles/16_MESSAGERIE.md` §11.1, §19.3, R-M11, R-M26, R-M41, R-M45) : boîtes de
//! service, membres **par mission** (socle 6), horaires, réponse automatique, astreinte, traitement des fils
//! (états, prise en charge, transfert, notes internes, échéances, étiquettes), réponse au nom de la boîte,
//! message téléphonique (accueil), messages entrants non vérifiés, boîtes C3, bandeau d'urgence, port de
//! conversion vers N1.
//!
//! Principes tenus ici :
//! * **Lire = participer** (R-M1) : une boîte est participante d'un fil ; ses membres, **relus à chaque
//!   lecture** (`ent_msg_boite_membres`, jour d'établissement), lisent le fil. Aucune liste de membres saisie.
//! * Une boîte **C3** (infirmerie, psychologue, assistante sociale) n'est lue que de ses membres : ni le pont
//!   des rôles réservés, ni la direction, ni la supervision ordinaire (R-M11).
//! * Une **note interne** vit dans sa propre table : aucune lecture de message ne peut la montrer à la famille.
//! * Aucun envoi automatique, aucun effet automatique d'un message entrant (R-M41).
//! * La famille ne voit jamais la liste des membres : seulement le libellé de la boîte, ses horaires et son délai.

// Les services de la messagerie prennent l'établissement, le compte, le rôle et la cible : au-delà de 7 arguments,
// comme `msg_m2.rs`. Les regrouper dans une structure n'ajouterait rien.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use domain_eleve::{DomainError, MsgFil, RepondreFil};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde::Serialize;
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::msg_matrice::{Cible, Decision};
use crate::msg_repo::PorteeFil;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Bandeau d'urgence permanent (R-M26) : affiché à toute famille et à tout élève qui écrit à une boîte ;
/// la messagerie ne promet aucun délai la nuit.
pub const BANDEAU_URGENCE: &str = "En cas d'urgence : 3018 (harcèlement), 119 (enfance en danger), 15 (SAMU), 17 (police). La messagerie n'est pas surveillée la nuit ni le week-end.";

/// Titre neutre d'un fil C3 hors de sa boîte (R-M11).
pub const TITRE_NEUTRE_C3: &str = "Échange confidentiel";

/// Boîtes du catalogue fourni (§11.1) : `(code, libellé, sensibilité)`.
pub const CATALOGUE_BOITES: &[(&str, &str, &str)] = &[
    ("vie_scolaire", "Vie scolaire", "C2"),
    ("secretariat", "Secrétariat", "C2"),
    ("direction", "Direction", "C2"),
    ("infirmerie", "Infirmerie", "C3"),
    ("psychologue", "Psychologue", "C3"),
    ("assistante_sociale", "Assistante sociale", "C3"),
    ("intendance", "Intendance", "C2"),
    ("orientation", "Orientation", "C2"),
    ("cdi", "CDI", "C2"),
    ("stages", "Stages", "C2"),
    ("harcelement", "Harcèlement", "C2"),
];

/// Limite d'étiquettes par boîte (liste courte, S-135).
pub const ETIQUETTES_MAX: i64 = 12;

/// Mots d'alerte du tri **déterministe** (jamais de modèle) : ils ordonnent la file et préviennent
/// l'astreinte ; ils ne décident rien (R-M26, CM-44). Comparés sans accent ni casse.
const MOTS_ALERTE: &[&str] = &[
    "urgent", "suicid", "me tuer", "mourir", "violence", "frapp", "harcel", "abus", "viol ", "menace",
    "danger", "maltrait", "automutil", "scarifi", "me faire du mal", "fugue", "disparu",
];

/// Minuscules sans accents (suffisant pour le français courant).
fn normaliser(t: &str) -> String {
    t.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// Tri déterministe d'urgence (mots d'alerte). Faux positif toléré, faux négatif non garanti : le tri
/// n'est qu'un ordre de lecture, jamais une garantie.
pub fn urgence_deterministe(texte: &str) -> bool {
    let n = format!("{} ", normaliser(texte));
    MOTS_ALERTE.iter().any(|m| n.contains(m))
}

/// Cible de la matrice (§8.2) d'une boîte, d'après son code.
pub fn cible_de_boite(code: &str) -> Cible {
    match code {
        "vie_scolaire" => Cible::BoiteVieScolaire,
        "direction" => Cible::Direction,
        "infirmerie" | "psychologue" | "assistante_sociale" => Cible::Sante,
        _ => Cible::BoiteSecretariat,
    }
}

/// Une boîte, telle que la voit un membre ou la direction.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BoiteVue {
    pub id: Uuid,
    pub code: String,
    pub libelle: String,
    pub sensibilite: String,
    pub actif: bool,
    pub delai_vise_jours: i16,
    pub message_accueil: Option<String>,
    pub reponse_auto: Option<String>,
}

/// Ce que voit une **famille** ou un élève d'une boîte : jamais la liste des membres (R-M26, §21.2).
#[derive(Debug, Clone, Serialize)]
pub struct BoiteAccueil {
    pub boite_id: Uuid,
    pub libelle: String,
    pub delai_vise_jours: i16,
    pub message_accueil: Option<String>,
    pub ouverte_maintenant: bool,
    pub horaires: Vec<HoraireVue>,
    pub bandeau_urgence: &'static str,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct HoraireVue {
    pub jour_iso: i16,
    pub debut: NaiveTime,
    pub fin: NaiveTime,
}

/// Une case d'horaire saisie.
#[derive(Debug, Clone, Copy)]
pub struct HoraireSaisie {
    pub jour_iso: i16,
    pub debut: NaiveTime,
    pub fin: NaiveTime,
}

#[derive(Debug, Clone, Default)]
pub struct SaisieBoite {
    pub code: String,
    /// Boîte du catalogue fourni (`true`) ou boîte personnalisée (`false`, D-022).
    pub catalogue: bool,
    pub libelle: Option<String>,
    /// Sensibilité d'une boîte personnalisée ; ignorée pour le catalogue (infirmerie… : C3).
    pub c3: bool,
    pub message_accueil: Option<String>,
    pub reponse_auto: Option<String>,
    pub delai_vise_jours: Option<i16>,
}

#[derive(Debug, Clone, Default)]
pub struct ParamBoite {
    pub libelle: Option<String>,
    pub message_accueil: Option<Option<String>>,
    pub reponse_auto: Option<Option<String>>,
    pub delai_vise_jours: Option<i16>,
    pub actif: Option<bool>,
}

/// Un membre d'une boîte, vu de la direction ou d'un autre membre (jamais d'une famille).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MembreVue {
    pub personne_id: Uuid,
    pub nom_affiche: String,
    pub source: String,
}

/// États de traitement (§19.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Traitement {
    Nouveau,
    Pris,
    AttenteFamille,
    Traite,
    Transfere,
}

impl Traitement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nouveau => "nouveau",
            Self::Pris => "pris",
            Self::AttenteFamille => "attente_famille",
            Self::Traite => "traite",
            Self::Transfere => "transfere",
        }
    }
}

/// Une ligne de la file d'une boîte (MSG-162).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct LigneFile {
    pub fil_id: Uuid,
    pub eleve_id: Uuid,
    pub sujet: String,
    pub origine: String,
    pub statut: String,
    pub pris_par: Option<String>,
    pub echeance: Option<NaiveDate>,
    pub urgence: bool,
    pub dernier_message_le: DateTime<Utc>,
    pub etiquettes: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct NoteInterne {
    pub id: Uuid,
    pub auteur: String,
    pub corps: String,
    pub transfert: bool,
    pub cree_le: DateTime<Utc>,
}

/// Résultat de l'ouverture d'un fil vers une boîte.
#[derive(Debug, Clone, Serialize)]
pub struct OuvertureBoite {
    pub fil_id: Uuid,
    pub message_id: Uuid,
    /// Texte de réponse automatique à afficher à l'expéditeur (boîte fermée), s'il y en a un.
    pub reponse_auto: Option<String>,
    pub urgence: bool,
    pub accueil: BoiteAccueil,
}

#[derive(Debug, Clone)]
pub struct SaisieTelephone {
    pub boite_id: Uuid,
    pub eleve_id: Uuid,
    pub appelant_personne_id: Option<Uuid>,
    pub appelant_libelle: Option<String>,
    pub motif_code: String,
    pub rappel_demande: bool,
    pub texte: String,
}

/// Un message téléphonique que l'accueil a saisi et qui n'est pas encore pris en charge.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TelephoneSaisi {
    pub message_id: Uuid,
    pub boite_libelle: String,
    pub motif_code: String,
    pub appelant_libelle: Option<String>,
    pub rappel_demande: bool,
    pub cree_le: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanalEntrant {
    Sms,
    Courriel,
}

impl CanalEntrant {
    fn code(self) -> &'static str {
        match self {
            Self::Sms => "sms",
            Self::Courriel => "courriel",
        }
    }
    fn origine(self) -> &'static str {
        match self {
            Self::Sms => "sms_entrant",
            Self::Courriel => "courriel_entrant",
        }
    }
}

/// Port de conversion message ↔ demande vers N1 (`demandes_echanges::PortMessagerie`), **résolu** à l'avance :
/// N1 appelle un port synchrone, la base est asynchrone ; les faits sont donc lus avant l'appel, puis le port
/// ne répond que sur ce qui a été résolu (fail-closed : rien de résolu = rien de convertible).
#[derive(Debug, Clone, Default)]
pub struct MessagerieResolue {
    convertibles: std::collections::HashSet<(Uuid, Uuid, Uuid)>,
}

impl crate::demandes_echanges::PortMessagerie for MessagerieResolue {
    fn peut_convertir_fil(&self, etab: Uuid, personne_id: Uuid, fil_id: Uuid) -> bool {
        self.convertibles.contains(&(etab, personne_id, fil_id))
    }
    /// Reprise du contexte d'une demande dans une conversation : à brancher avec le contexte de N1
    /// (limite connue de M3 ; fail-closed).
    fn conversation_depuis_demande(&self, _: Uuid, _: Uuid, _: Uuid) -> Option<Uuid> {
        None
    }
}

fn refus(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

impl PgEnt {
    // ───────────────────────────── Aides communes ─────────────────────────────

    async fn exiger_action_msg(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        action: Action,
    ) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, user, role, action).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    async fn personne_du_compte_msg(&self, etab: Uuid, user: Uuid) -> Result<Uuid, EntDbError> {
        let (p,): (Option<Uuid>,) = sqlx::query_as("SELECT ent_msg_personne_du_compte($1, $2)")
            .bind(etab)
            .bind(user)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await?;
        p.ok_or(EntDbError::Forbidden)
    }

    async fn charger_boite(&self, etab: Uuid, boite: Uuid) -> Result<BoiteVue, EntDbError> {
        sqlx::query_as::<_, BoiteVue>(
            "SELECT id, code, libelle, sensibilite, actif, delai_vise_jours, message_accueil, reponse_auto
               FROM ent_msg_boite WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(boite)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?
        .ok_or(EntDbError::Domain(DomainError::NotFound))
    }

    /// Le compte est-il membre de la boîte **aujourd'hui** (missions + désignations datées) ?
    pub async fn est_membre_boite(&self, etab: Uuid, boite: Uuid, user: Uuid) -> Result<bool, EntDbError> {
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_msg_boite_membres($1, $2) m WHERE m.user_id = $3)",
        )
        .bind(etab)
        .bind(boite)
        .bind(user)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        Ok(ok)
    }

    /// Membre traitant : droit `messagerie.boite.traiter` **et** membre de la boîte à la date **et**
    /// boîte participante du fil. Rend la fiche personne du membre.
    async fn exiger_membre_traitant(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        boite: Uuid,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieBoiteTraiter).await?;
        if !self.est_membre_boite(etab, boite, user).await? {
            return Err(EntDbError::Forbidden);
        }
        let (porte,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_msg_participant p
                             WHERE p.etablissement_id = $1 AND p.fil_id = $2 AND p.boite_id = $3
                               AND p.au IS NULL)",
        )
        .bind(etab)
        .bind(fil)
        .bind(boite)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        if !porte {
            return Err(EntDbError::Forbidden);
        }
        self.personne_du_compte_msg(etab, user).await
    }

    /// Accès d'un **personnel** à un fil de boîte : `Some(accès)` si le fil est un fil de boîte, `None`
    /// sinon (les règles ordinaires s'appliquent). L'accès est la participation relue par
    /// `ent_msg_fil_lisible` : membre d'une boîte participante, ou participant nominatif (l'auteur d'un
    /// fil interne). Ni le pont des rôles réservés, ni la direction : la supervision motivée est M10.
    pub(crate) async fn acces_personnel_fil_boite(
        &self,
        etab: Uuid,
        user: Uuid,
        fil: Uuid,
    ) -> Result<Option<bool>, EntDbError> {
        let ligne: Option<(bool, bool)> = sqlx::query_as(
            "SELECT f.nature = 'boite',
                    EXISTS (SELECT 1 FROM ent_msg_fil_lisible($1, $3) l WHERE l.fil_id = f.id)
               FROM ent_msg_fil f WHERE f.etablissement_id = $1 AND f.id = $2",
        )
        .bind(etab)
        .bind(fil)
        .bind(user)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        Ok(match ligne {
            Some((true, lisible)) => Some(lisible),
            _ => None,
        })
    }

    /// Boîte participante d'un fil et sa sensibilité (None si ce n'est pas un fil de boîte).
    async fn boite_du_fil(&self, etab: Uuid, fil: Uuid) -> Result<Option<(Uuid, String)>, EntDbError> {
        Ok(sqlx::query_as(
            "SELECT b.id, b.sensibilite FROM ent_msg_participant p
               JOIN ent_msg_boite b ON b.etablissement_id = p.etablissement_id AND b.id = p.boite_id
              WHERE p.etablissement_id = $1 AND p.fil_id = $2 AND p.au IS NULL AND p.boite_id IS NOT NULL
              LIMIT 1",
        )
        .bind(etab)
        .bind(fil)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    async fn publier_msg(
        &self,
        etab: Uuid,
        type_name: &str,
        agregat: Uuid,
        eleve: Option<Uuid>,
        acteur: Uuid,
        payload: serde_json::Value,
    ) -> Result<(), EntDbError> {
        self.events
            .publish(MetierEventDraft {
                etablissement_id: etab,
                type_name: type_name.to_string(),
                aggregate: AggregateKind::MsgFil,
                aggregate_id: agregat,
                eleve_id: eleve,
                payload,
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        Ok(())
    }

    // ───────────────────────────── Gestion des boîtes (MSG-160, 161, 169, 170, 175) ─────────────────────────────

    /// Crée une boîte (MSG-160) : catalogue fourni ou personnalisée. Infirmerie, psychologue et assistante
    /// sociale sont toujours C3 (R-M11).
    pub async fn creer_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        s: SaisieBoite,
    ) -> Result<BoiteVue, EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieBoitesGerer).await?;
        let code = s.code.trim().to_string();
        let (libelle, sens) = if s.catalogue {
            let (_, lib, sens) = CATALOGUE_BOITES
                .iter()
                .find(|(c, _, _)| *c == code)
                .ok_or(refus("boîte absente du catalogue"))?;
            (
                s.libelle.clone().filter(|l| !l.trim().is_empty()).unwrap_or_else(|| (*lib).to_string()),
                (*sens).to_string(),
            )
        } else {
            let ok = code.len() >= 2
                && code.len() <= 40
                && code.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && code.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if !ok || CATALOGUE_BOITES.iter().any(|(c, _, _)| *c == code) {
                return Err(refus("code de boîte invalide"));
            }
            let lib = s.libelle.clone().filter(|l| !l.trim().is_empty()).ok_or(refus("libellé requis"))?;
            (lib, if s.c3 { "C3" } else { "C2" }.to_string())
        };
        if libelle.chars().count() > 80 {
            return Err(refus("libellé trop long"));
        }
        let delai = s.delai_vise_jours.unwrap_or(2);
        if !(1..=30).contains(&delai) {
            return Err(refus("délai visé hors bornes"));
        }
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "INSERT INTO ent_msg_boite
                (id, etablissement_id, code, catalogue, libelle, sensibilite, message_accueil, reponse_auto,
                 delai_vise_jours, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(&code)
        .bind(s.catalogue)
        .bind(libelle.trim())
        .bind(&sens)
        .bind(s.message_accueil.as_deref())
        .bind(s.reponse_auto.as_deref())
        .bind(delai)
        .bind(user)
        .execute(&mut *tx)
        .await;
        match r {
            Ok(_) => {}
            Err(sqlx::Error::Database(d)) if d.is_unique_violation() => {
                return Err(EntDbError::ConflitDossier("cette boîte existe déjà"));
            }
            Err(sqlx::Error::Database(d)) if d.is_check_violation() => {
                return Err(refus("boîte invalide"));
            }
            Err(e) => return Err(e.into()),
        }
        tx.commit().await?;
        self.publier_msg(
            etab,
            types::MESSAGERIE_BOITE_CREEE,
            id,
            None,
            user,
            json!({ "boite_id": id, "code": code, "sensibilite": sens }),
        )
        .await?;
        self.charger_boite(etab, id).await
    }

    /// Paramètre une boîte (accueil, réponse automatique, délai, activation) : direction (MSG-169).
    pub async fn parametrer_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
        p: ParamBoite,
    ) -> Result<BoiteVue, EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieBoitesGerer).await?;
        let actuel = self.charger_boite(etab, boite).await?;
        if let Some(d) = p.delai_vise_jours {
            if !(1..=30).contains(&d) {
                return Err(refus("délai visé hors bornes"));
            }
        }
        let libelle = p.libelle.map(|l| l.trim().to_string()).unwrap_or(actuel.libelle);
        if libelle.is_empty() || libelle.chars().count() > 80 {
            return Err(refus("libellé invalide"));
        }
        let accueil = p.message_accueil.unwrap_or(actuel.message_accueil);
        let auto = p.reponse_auto.unwrap_or(actuel.reponse_auto);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "UPDATE ent_msg_boite SET libelle = $3, message_accueil = $4, reponse_auto = $5,
                    delai_vise_jours = $6, actif = $7, maj_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(boite)
        .bind(&libelle)
        .bind(accueil.as_deref())
        .bind(auto.as_deref())
        .bind(p.delai_vise_jours.unwrap_or(actuel.delai_vise_jours))
        .bind(p.actif.unwrap_or(actuel.actif))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.publier_msg(
            etab,
            types::MESSAGERIE_BOITE_MODIFIEE,
            boite,
            None,
            user,
            json!({ "boite_id": boite }),
        )
        .await?;
        self.charger_boite(etab, boite).await
    }

    /// Rattache (ou détache) un type de mission à une boîte (MSG-161) : les membres sont les porteurs de
    /// la mission, jamais une liste.
    pub async fn rattacher_mission_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
        mission_type: Uuid,
        rattacher: bool,
    ) -> Result<(), EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieBoitesGerer).await?;
        self.charger_boite(etab, boite).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if rattacher {
            let r = sqlx::query(
                "INSERT INTO ent_msg_boite_mission (etablissement_id, boite_id, mission_type_id)
                 VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
            )
            .bind(etab)
            .bind(boite)
            .bind(mission_type)
            .execute(&mut *tx)
            .await;
            match r {
                Ok(_) => {}
                Err(sqlx::Error::Database(d)) if d.is_check_violation() || d.is_foreign_key_violation() => {
                    return Err(refus("type de mission inconnu pour cet établissement"));
                }
                Err(e) => return Err(e.into()),
            }
        } else {
            sqlx::query(
                "DELETE FROM ent_msg_boite_mission
                  WHERE etablissement_id = $1 AND boite_id = $2 AND mission_type_id = $3",
            )
            .bind(etab)
            .bind(boite)
            .bind(mission_type)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Remplace les horaires d'une boîte (heure locale de l'établissement) ; refuse les chevauchements.
    pub async fn definir_horaires_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
        mut horaires: Vec<HoraireSaisie>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieBoitesGerer).await?;
        self.charger_boite(etab, boite).await?;
        horaires.sort_by_key(|h| (h.jour_iso, h.debut));
        for h in &horaires {
            if !(1..=7).contains(&h.jour_iso) || h.fin <= h.debut {
                return Err(refus("horaire invalide"));
            }
        }
        if horaires.windows(2).any(|w| w[0].jour_iso == w[1].jour_iso && w[1].debut < w[0].fin) {
            return Err(refus("horaires qui se chevauchent"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query("DELETE FROM ent_msg_boite_horaire WHERE etablissement_id = $1 AND boite_id = $2")
            .bind(etab)
            .bind(boite)
            .execute(&mut *tx)
            .await?;
        for h in horaires {
            sqlx::query(
                "INSERT INTO ent_msg_boite_horaire (etablissement_id, boite_id, jour_iso, debut, fin)
                 VALUES ($1,$2,$3,$4,$5)",
            )
            .bind(etab)
            .bind(boite)
            .bind(h.jour_iso)
            .bind(h.debut)
            .bind(h.fin)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Désigne un membre ou une remplaçante, à une date (MSG-175). La direction le peut pour toute boîte ;
    /// pour une boîte C3, un membre de la boîte (la professionnelle) le peut aussi. La personne désignée
    /// est un **personnel** (jamais un parent ni un élève).
    #[allow(clippy::too_many_arguments)]
    pub async fn designer_membre_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
        personne: Uuid,
        remplacante: bool,
        du: NaiveDate,
        au: Option<NaiveDate>,
    ) -> Result<(), EntDbError> {
        let b = self.charger_boite(etab, boite).await?;
        let gere = self.a_action_etablissement(etab, user, role, Action::MessagerieBoitesGerer).await?;
        let membre_c3 = b.sensibilite == "C3" && self.est_membre_boite(etab, boite, user).await?;
        if !(gere || membre_c3) {
            return Err(EntDbError::Forbidden);
        }
        if au.is_some_and(|a| a < du) {
            return Err(refus("dates invalides"));
        }
        // La personne doit être un compte de personnel de l'établissement.
        let ok: Option<(String,)> = sqlx::query_as(
            "SELECT mb.role FROM ent_personne pe
               JOIN ent_membership mb ON mb.user_id = pe.user_id AND mb.etablissement_id = pe.etablissement_id
              WHERE pe.etablissement_id = $1 AND pe.id = $2 AND pe.fusionnee_dans IS NULL",
        )
        .bind(etab)
        .bind(personne)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        match ok {
            Some((r,)) if r != "parent" && r != "eleve" => {}
            _ => return Err(refus("la personne désignée doit être un personnel de l'établissement")),
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO ent_msg_boite_designation
                (id, etablissement_id, boite_id, personne_id, qualite, du, au, designe_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(boite)
        .bind(personne)
        .bind(if remplacante { "remplacante" } else { "membre" })
        .bind(du)
        .bind(au)
        .bind(user)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Désigne une astreinte pour les messages urgents d'une boîte (MSG-170, facultatif).
    pub async fn definir_astreinte_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
        personne: Uuid,
        du: DateTime<Utc>,
        au: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieBoitesGerer).await?;
        self.charger_boite(etab, boite).await?;
        if au <= du {
            return Err(refus("plage d'astreinte invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO ent_msg_astreinte (id, etablissement_id, boite_id, personne_id, du, au, designe_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(boite)
        .bind(personne)
        .bind(du)
        .bind(au)
        .bind(user)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Crée une étiquette de la boîte (liste courte).
    pub async fn creer_etiquette_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
        code: &str,
        libelle: &str,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieBoitesGerer).await?;
        self.charger_boite(etab, boite).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_msg_etiquette WHERE etablissement_id = $1 AND boite_id = $2",
        )
        .bind(etab)
        .bind(boite)
        .fetch_one(&mut *tx)
        .await?;
        if n >= ETIQUETTES_MAX {
            return Err(EntDbError::ConflitDossier("liste d'étiquettes complète"));
        }
        let id = Uuid::new_v4();
        let r = sqlx::query(
            "INSERT INTO ent_msg_etiquette (id, etablissement_id, boite_id, code, libelle)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id)
        .bind(etab)
        .bind(boite)
        .bind(code)
        .bind(libelle.trim())
        .execute(&mut *tx)
        .await;
        match r {
            Ok(_) => {}
            Err(sqlx::Error::Database(d)) if d.is_unique_violation() => {
                return Err(EntDbError::ConflitDossier("étiquette déjà créée"));
            }
            Err(sqlx::Error::Database(d)) if d.is_check_violation() => return Err(refus("étiquette invalide")),
            Err(e) => return Err(e.into()),
        }
        tx.commit().await?;
        Ok(id)
    }

    /// Membres d'une boîte à la date : direction (gestion) ou membre de la boîte. **Jamais une famille.**
    pub async fn membres_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
    ) -> Result<Vec<MembreVue>, EntDbError> {
        let gere = self.a_action_etablissement(etab, user, role, Action::MessagerieBoitesGerer).await?;
        if !(gere || self.est_membre_boite(etab, boite, user).await?) {
            return Err(EntDbError::Forbidden);
        }
        Ok(sqlx::query_as::<_, MembreVue>(
            "SELECT m.personne_id,
                    btrim(COALESCE(pe.prenom, '') || ' ' || pe.nom) AS nom_affiche,
                    m.source
               FROM ent_msg_boite_membres($1, $2) m
               JOIN ent_personne pe ON pe.etablissement_id = $1 AND pe.id = m.personne_id
              ORDER BY pe.nom, pe.prenom",
        )
        .bind(etab)
        .bind(boite)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    /// Boîtes que le compte **traite** (membre à la date) ; avec `gestion`, toutes celles de l'établissement.
    pub async fn boites_du_personnel_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<Vec<BoiteVue>, EntDbError> {
        if !Role::parse(role).is_some_and(|r| r.is_staff()) {
            return Err(EntDbError::Forbidden);
        }
        let gere = self.a_action_etablissement(etab, user, role, Action::MessagerieBoitesGerer).await?;
        Ok(sqlx::query_as::<_, BoiteVue>(
            "SELECT b.id, b.code, b.libelle, b.sensibilite, b.actif, b.delai_vise_jours, b.message_accueil,
                    b.reponse_auto
               FROM ent_msg_boite b
              WHERE b.etablissement_id = $1
                AND ($3 OR EXISTS (SELECT 1 FROM ent_msg_boite_membres($1, b.id) m WHERE m.user_id = $2))
              ORDER BY b.libelle",
        )
        .bind(etab)
        .bind(user)
        .bind(gere)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    // ───────────────────────────── Horaires, accueil, réponse automatique ─────────────────────────────

    /// La boîte est-elle ouverte à `instant` (horaires, heure locale de l'établissement) ? Une boîte sans
    /// horaire est ouverte en permanence.
    pub async fn boite_ouverte_a(
        &self,
        etab: Uuid,
        boite: Uuid,
        instant: DateTime<Utc>,
    ) -> Result<bool, EntDbError> {
        let (ouverte,): (bool,) = sqlx::query_as(
            "WITH loc AS (
                SELECT ($3::timestamptz AT TIME ZONE COALESCE(
                          (SELECT p.fuseau FROM ent_etablissement_parametre p WHERE p.etablissement_id = $1),
                          'Europe/Paris')) AS t)
             SELECT NOT EXISTS (SELECT 1 FROM ent_msg_boite_horaire h WHERE h.etablissement_id = $1 AND h.boite_id = $2)
                 OR EXISTS (SELECT 1 FROM ent_msg_boite_horaire h, loc
                             WHERE h.etablissement_id = $1 AND h.boite_id = $2
                               AND h.jour_iso = extract(isodow FROM loc.t)::int
                               AND loc.t::time >= h.debut AND loc.t::time < h.fin)",
        )
        .bind(etab)
        .bind(boite)
        .bind(instant)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        Ok(ouverte)
    }

    async fn accueil_boite(&self, etab: Uuid, b: &BoiteVue, instant: DateTime<Utc>) -> Result<BoiteAccueil, EntDbError> {
        let horaires = sqlx::query_as::<_, HoraireVue>(
            "SELECT jour_iso, debut, fin FROM ent_msg_boite_horaire
              WHERE etablissement_id = $1 AND boite_id = $2 ORDER BY jour_iso, debut",
        )
        .bind(etab)
        .bind(b.id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        Ok(BoiteAccueil {
            boite_id: b.id,
            libelle: b.libelle.clone(),
            delai_vise_jours: b.delai_vise_jours,
            message_accueil: b.message_accueil.clone(),
            ouverte_maintenant: self.boite_ouverte_a(etab, b.id, instant).await?,
            horaires,
            bandeau_urgence: BANDEAU_URGENCE,
        })
    }

    /// Boîtes auxquelles **cet expéditeur** a le droit d'écrire à propos de cet élève (la matrice du §8.2,
    /// recalculée) : une boîte fermée par la matrice n'est pas listée. Jamais la liste des membres.
    pub async fn boites_ecrivables_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<Vec<BoiteAccueil>, EntDbError> {
        if matches!(Role::parse(role), Some(Role::Parent | Role::Eleve))
            && !self.actor_can_access_msg_fil(etab, user, role, eleve).await?
        {
            return Ok(Vec::new());
        }
        let boites = sqlx::query_as::<_, BoiteVue>(
            "SELECT id, code, libelle, sensibilite, actif, delai_vise_jours, message_accueil, reponse_auto
               FROM ent_msg_boite WHERE etablissement_id = $1 AND actif ORDER BY libelle",
        )
        .bind(etab)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let mut out = Vec::new();
        for b in boites {
            let d = self
                .peut_ecrire_for_actor(etab, user, role, cible_de_boite(&b.code), eleve, None, true)
                .await?;
            if matches!(d, Decision::Permis(_)) {
                out.push(self.accueil_boite(etab, &b, Utc::now()).await?);
            }
        }
        Ok(out)
    }

    // ───────────────────────────── Ouvrir un fil vers une boîte ─────────────────────────────

    /// Ouvre un fil vers une boîte (famille, élève ou personnel) : la matrice est recalculée côté serveur
    /// (R-M2) ; la boîte devient participante ; le traitement naît « nouveau » ; la réponse automatique et le
    /// bandeau d'urgence sont rendus ; un message marqué urgent par le tri déterministe prévient l'astreinte
    /// quand la boîte est fermée (CM-44). Les notifications sont neutres (jamais de sujet ni d'enfant pour C3).
    pub async fn ouvrir_fil_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
        eleve: Uuid,
        sujet: &str,
        corps: &str,
    ) -> Result<OuvertureBoite, EntDbError> {
        self.ouvrir_fil_boite_a(etab, user, role, boite, eleve, sujet, corps, Utc::now()).await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn ouvrir_fil_boite_a(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
        eleve: Uuid,
        sujet: &str,
        corps: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<OuvertureBoite, EntDbError> {
        let b = self.charger_boite(etab, boite).await?;
        if !b.actif {
            return Err(EntDbError::Forbidden);
        }
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        // La matrice n'exige pas de lien pour écrire à une boîte (§8.2) : une famille ou un élève ne peut
        // pourtant écrire qu'à propos d'un élève avec lequel elle a un lien communicant (ou de lui-même).
        if matches!(parsed, Role::Parent | Role::Eleve)
            && !self.actor_can_access_msg_fil(etab, user, role, eleve).await?
        {
            return Err(EntDbError::Forbidden);
        }
        match self
            .peut_ecrire_for_actor(etab, user, role, cible_de_boite(&b.code), eleve, None, true)
            .await?
        {
            Decision::Permis(_) => {}
            Decision::Refuse(_) => return Err(EntDbError::Forbidden),
        }
        let (mut fil, msg) = domain_eleve::plan_ouvrir_fil(domain_eleve::OuvrirFil {
            etablissement_id: etab,
            eleve_id: eleve,
            sujet: sujet.to_string(),
            premier_message: corps.to_string(),
            actor_user_id: user,
        })?;
        fil.modele_id = None;
        let portee = match parsed {
            Role::Parent => PorteeFil::prive_parent(user),
            Role::Eleve => PorteeFil::eleve(user),
            _ => PorteeFil::responsables(),
        };
        self.messages.insert_fil_portee(&fil, portee).await?;
        let urgence = urgence_deterministe(&format!("{sujet} {corps}"));
        {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            sqlx::query(
                "UPDATE ent_msg_fil SET nature = 'boite', sensibilite = $3,
                        origine = CASE $4::text WHEN 'parent' THEN 'famille' WHEN 'eleve' THEN 'eleve' ELSE 'personnel' END
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(fil.id)
            .bind(&b.sensibilite)
            .bind(parsed.as_str())
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO ent_msg_participant
                    (id, etablissement_id, fil_id, boite_id, cote, qualite, raison)
                 VALUES ($1,$2,$3,$4,'etablissement','destinataire','boite')",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(fil.id)
            .bind(boite)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO ent_msg_traitement (etablissement_id, fil_id, boite_id, urgence)
                 VALUES ($1,$2,$3,$4)",
            )
            .bind(etab)
            .bind(fil.id)
            .bind(boite)
            .bind(urgence)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        // Un message vers une boîte n'est jamais différé par la plage calme : la plage protège les
        // familles et les élèves des messages des adultes, pas l'inverse (R-M20).
        self.messages.insert_message_remise(&msg, None, None).await?;
        self.messages.touch_fil_last(etab, fil.id, msg.created_at).await?;
        let ouverte = self.boite_ouverte_a(etab, boite, maintenant).await?;
        let reponse_auto = if !ouverte { b.reponse_auto.clone() } else { None };
        if reponse_auto.is_some() {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            sqlx::query(
                "UPDATE ent_msg_traitement SET reponse_auto_le = now()
                  WHERE etablissement_id = $1 AND fil_id = $2 AND boite_id = $3",
            )
            .bind(etab)
            .bind(fil.id)
            .bind(boite)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        self.publier_msg(
            etab,
            types::MESSAGERIE_FIL_OUVERT,
            fil.id,
            if b.sensibilite == "C3" { None } else { Some(eleve) },
            user,
            json!({ "message_id": msg.id, "nature": "boite", "boite_id": boite, "sensibilite": b.sensibilite,
                    "urgence": urgence }),
        )
        .await?;
        self.avertir_boite(etab, boite, &b.sensibilite, fil.id, msg.id, eleve, urgence && !ouverte, user, maintenant)
            .await;
        let accueil = self.accueil_boite(etab, &b, maintenant).await?;
        Ok(OuvertureBoite { fil_id: fil.id, message_id: msg.id, reponse_auto, urgence, accueil })
    }

    /// Avis **neutres** aux membres de la boîte (ou à l'astreinte quand le message est urgent et la boîte
    /// fermée). Jamais de sujet ; jamais l'enfant ni la boîte pour une boîte C3. Meilleur effort : une
    /// erreur est journalisée, jamais bloquante (D14).
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn avertir_boite(
        &self,
        etab: Uuid,
        boite: Uuid,
        sensibilite: &str,
        fil: Uuid,
        message: Uuid,
        eleve: Uuid,
        vers_astreinte: bool,
        sauf: Uuid,
        maintenant: DateTime<Utc>,
    ) {
        let r: Result<(), EntDbError> = async {
            let destinataires: Vec<(Uuid,)> = if vers_astreinte {
                let a: Vec<(Uuid,)> = sqlx::query_as(
                    "SELECT DISTINCT pe.user_id FROM ent_msg_astreinte a
                       JOIN ent_personne pe ON pe.etablissement_id = a.etablissement_id AND pe.id = a.personne_id
                      WHERE a.etablissement_id = $1 AND a.boite_id = $2 AND a.du <= $3 AND a.au > $3
                        AND pe.user_id IS NOT NULL AND pe.user_id <> $4",
                )
                .bind(etab)
                .bind(boite)
                .bind(maintenant)
                .bind(sauf)
                .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
                .await?;
                if a.is_empty() {
                    self.membres_a_prevenir(etab, boite, sauf).await?
                } else {
                    a
                }
            } else {
                self.membres_a_prevenir(etab, boite, sauf).await?
            };
            let mut c = TenantTx::begin(self.pool(), etab).await?;
            for (u,) in destinataires {
                crate::notifications::notifier_personne(
                    &mut c,
                    crate::notifications::EnvoiAPlanifier {
                        etablissement_id: etab,
                        user_id: u,
                        kind: "messagerie_boite",
                        titre: "Nouveau message",
                        corps: "Un message attend dans une boîte de service de la messagerie.",
                        eleve_id: if sensibilite == "C3" { None } else { Some(eleve) },
                        notification_id: None,
                        source_type: "systeme",
                        source_id: message,
                        cle_dedup: format!("msgboite:{message}"),
                        droit_requis: None,
                        regroupable: true,
                        canaux_voulus: &["push"],
                    },
                    message,
                    maintenant,
                )
                .await?;
            }
            c.commit().await?;
            Ok(())
        }
        .await;
        if let Err(e) = r {
            tracing::error!(canal = "messagerie", fil = %fil, error = %e, "avis de boîte non émis");
        }
    }

    pub(crate) async fn membres_a_prevenir(
        &self,
        etab: Uuid,
        boite: Uuid,
        sauf: Uuid,
    ) -> Result<Vec<(Uuid,)>, EntDbError> {
        Ok(sqlx::query_as(
            "SELECT DISTINCT m.user_id FROM ent_msg_boite_membres($1, $2) m WHERE m.user_id <> $3",
        )
        .bind(etab)
        .bind(boite)
        .bind(sauf)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    /// Membres de la boîte participante d'un fil, hors `sauf` : les seuls personnels à prévenir quand une
    /// famille écrit dans un fil de boîte (jamais le pont des rôles réservés). `None` si ce n'est pas un fil de boîte.
    pub(crate) async fn personnels_a_prevenir_fil_boite(
        &self,
        etab: Uuid,
        fil: Uuid,
        sauf: Uuid,
    ) -> Result<Option<Vec<Uuid>>, EntDbError> {
        let Some((boite, _)) = self.boite_du_fil(etab, fil).await? else {
            return Ok(None);
        };
        Ok(Some(self.membres_a_prevenir(etab, boite, sauf).await?.into_iter().map(|(u,)| u).collect()))
    }

    /// Réponse d'un membre **au nom de la boîte** (MSG-168) : « Vie scolaire — Paul D. ». Passe par le
    /// chemin ordinaire de réponse (plage calme, avis à la famille). Le membre n'y devient pas participant
    /// nominatif : il quitte la lecture en quittant la mission.
    pub async fn repondre_au_nom_de_boite_for_actor(
        &self,
        role: &str,
        cmd: RepondreFil,
        boite: Uuid,
    ) -> Result<domain_eleve::Message, EntDbError> {
        let (etab, user, fil) = (cmd.etablissement_id, cmd.actor_user_id, cmd.fil_id);
        let personne = self.exiger_membre_traitant(etab, user, role, fil, boite).await?;
        let envoye = self.repondre_fil_avec_options(role, cmd, Default::default()).await?;
        let msg = envoye.message;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "UPDATE ent_msg SET au_nom_de_boite_id = $3 WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(msg.id)
        .bind(boite)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_msg_traitement SET statut = 'pris', pris_par_personne_id = $4, pris_le = now(), maj_le = now()
              WHERE etablissement_id = $1 AND fil_id = $2 AND boite_id = $3 AND statut = 'nouveau'",
        )
        .bind(etab)
        .bind(fil)
        .bind(boite)
        .bind(personne)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(msg)
    }

    /// Libellé d'expéditeur « Boîte — Prénom I. » des messages envoyés au nom d'une boîte, pour un fil que
    /// l'acteur peut lire. La famille voit le prénom et l'initiale, jamais la liste des membres.
    pub async fn expediteurs_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>, EntDbError> {
        let fil: MsgFil = self
            .messages
            .get_fil(etab, fil_id)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if !self.peut_lire_fil(etab, user, role, &fil).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(sqlx::query_as(
            "SELECT m.id, b.libelle || ' — ' || COALESCE(left(pe.prenom, 40), '') ||
                    CASE WHEN pe.nom <> '' THEN ' ' || upper(left(pe.nom, 1)) || '.' ELSE '' END
               FROM ent_msg m
               JOIN ent_msg_boite b ON b.etablissement_id = m.etablissement_id AND b.id = m.au_nom_de_boite_id
               JOIN ent_personne pe ON pe.etablissement_id = m.etablissement_id AND pe.id = m.auteur_personne_id
              WHERE m.etablissement_id = $1 AND m.fil_id = $2",
        )
        .bind(etab)
        .bind(fil.id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    /// Réouverture automatique : une réponse de la famille ou de l'élève rouvre le traitement (« nouveau »)
    /// quand il était « traité » ou « en attente de la famille ». Meilleur effort, appelé après une réponse.
    pub(crate) async fn rouvrir_traitement_apres_reponse(&self, etab: Uuid, fil: Uuid) {
        let r: Result<(), EntDbError> = async {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            sqlx::query(
                "UPDATE ent_msg_traitement SET statut = 'nouveau', traite_le = NULL, traite_par_personne_id = NULL,
                        maj_le = now()
                  WHERE etablissement_id = $1 AND fil_id = $2 AND statut IN ('traite', 'attente_famille')",
            )
            .bind(etab)
            .bind(fil)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(())
        }
        .await;
        if let Err(e) = r {
            tracing::error!(canal = "messagerie", fil = %fil, error = %e, "réouverture du traitement non posée");
        }
    }

    // ───────────────────────────── Traitement (MSG-162 à 167) ─────────────────────────────

    /// File d'une boîte (MSG-162) : urgents d'abord, puis échéance, puis le plus ancien. Réservée aux
    /// membres à la date.
    pub async fn file_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        boite: Uuid,
        statut: Option<Traitement>,
        limite: i64,
    ) -> Result<Vec<LigneFile>, EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieBoiteTraiter).await?;
        if !self.est_membre_boite(etab, boite, user).await? {
            return Err(EntDbError::Forbidden);
        }
        Ok(sqlx::query_as::<_, LigneFile>(
            "SELECT f.id AS fil_id, f.eleve_id, f.sujet, f.origine, t.statut,
                    btrim(COALESCE(pp.prenom, '') || ' ' || COALESCE(pp.nom, '')) AS pris_par,
                    t.echeance, t.urgence, f.last_message_at AS dernier_message_le,
                    COALESCE((SELECT string_agg(e.libelle, ', ' ORDER BY e.libelle)
                                FROM ent_msg_traitement_etiquette te
                                JOIN ent_msg_etiquette e ON e.etablissement_id = te.etablissement_id AND e.id = te.etiquette_id
                               WHERE te.fil_id = t.fil_id AND te.boite_id = t.boite_id), '') AS etiquettes
               FROM ent_msg_traitement t
               JOIN ent_msg_fil f ON f.etablissement_id = t.etablissement_id AND f.id = t.fil_id
               LEFT JOIN ent_personne pp ON pp.etablissement_id = t.etablissement_id AND pp.id = t.pris_par_personne_id
              WHERE t.etablissement_id = $1 AND t.boite_id = $2 AND t.statut <> 'transfere'
                AND ($3::text IS NULL OR t.statut = $3)
                AND EXISTS (SELECT 1 FROM ent_msg_participant p
                             WHERE p.etablissement_id = t.etablissement_id AND p.fil_id = t.fil_id
                               AND p.boite_id = t.boite_id AND p.au IS NULL)
              ORDER BY t.urgence DESC, t.echeance ASC NULLS LAST, f.last_message_at ASC, f.id
              LIMIT $4",
        )
        .bind(etab)
        .bind(boite)
        .bind(statut.map(|s| s.as_str()))
        .bind(limite.clamp(1, 200))
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    /// Prendre en charge (MSG-163) : « Paul s'en occupe », visible des membres. Un fil traité se rouvre
    /// d'abord ; un fil transféré n'est plus à cette boîte.
    pub async fn prendre_en_charge_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        boite: Uuid,
    ) -> Result<(), EntDbError> {
        let personne = self.exiger_membre_traitant(etab, user, role, fil, boite).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "UPDATE ent_msg_traitement SET statut = 'pris', pris_par_personne_id = $4, pris_le = now(),
                    traite_le = NULL, traite_par_personne_id = NULL, maj_le = now()
              WHERE etablissement_id = $1 AND fil_id = $2 AND boite_id = $3 AND statut <> 'transfere'",
        )
        .bind(etab)
        .bind(fil)
        .bind(boite)
        .bind(personne)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("ce fil n'est plus à cette boîte"));
        }
        tx.commit().await?;
        self.publier_msg(etab, types::MESSAGERIE_FIL_PRIS, fil, None, user, json!({ "boite_id": boite }))
            .await?;
        Ok(())
    }

    /// Poser « en attente de la famille » (MSG-166).
    pub async fn attendre_famille_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        boite: Uuid,
    ) -> Result<(), EntDbError> {
        let personne = self.exiger_membre_traitant(etab, user, role, fil, boite).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "UPDATE ent_msg_traitement SET statut = 'attente_famille',
                    pris_par_personne_id = COALESCE(pris_par_personne_id, $4),
                    pris_le = COALESCE(pris_le, now()), maj_le = now()
              WHERE etablissement_id = $1 AND fil_id = $2 AND boite_id = $3 AND statut IN ('nouveau', 'pris')",
        )
        .bind(etab)
        .bind(fil)
        .bind(boite)
        .bind(personne)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("état de traitement incompatible"));
        }
        tx.commit().await?;
        Ok(())
    }

    /// Passer « traité ». **R-M45** : un fil ouvert par une famille ou un élève (demande d'information ou
    /// d'entrevue) ne passe pas « traité » sans une réponse écrite d'un personnel ; un refus est motivé par
    /// écrit (C. éduc. D. 111-4). Un message entrant (SMS, courriel, téléphone) peut être classé sans réponse.
    pub async fn traiter_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        boite: Uuid,
    ) -> Result<(), EntDbError> {
        let personne = self.exiger_membre_traitant(etab, user, role, fil, boite).await?;
        let (origine, repondu): (String, bool) = sqlx::query_as(
            "SELECT f.origine,
                    EXISTS (SELECT 1 FROM ent_msg m
                             JOIN ent_membership mb ON mb.user_id = m.auteur_user_id AND mb.etablissement_id = m.etablissement_id
                            WHERE m.etablissement_id = f.etablissement_id AND m.fil_id = f.id
                              AND mb.role NOT IN ('parent', 'eleve') AND m.canal_entrant IS NULL
                              AND m.retire_le IS NULL)
               FROM ent_msg_fil f WHERE f.etablissement_id = $1 AND f.id = $2",
        )
        .bind(etab)
        .bind(fil)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if matches!(origine.as_str(), "famille" | "eleve") && !repondu {
            return Err(EntDbError::ConflitDossier(
                "une demande d'une famille ou d'un élève reçoit une réponse écrite avant d'être traitée",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let r = sqlx::query(
            "UPDATE ent_msg_traitement SET statut = 'traite', traite_le = now(), traite_par_personne_id = $4,
                    maj_le = now()
              WHERE etablissement_id = $1 AND fil_id = $2 AND boite_id = $3 AND statut <> 'transfere'",
        )
        .bind(etab)
        .bind(fil)
        .bind(boite)
        .bind(personne)
        .execute(&mut *tx)
        .await?;
        if r.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("ce fil n'est plus à cette boîte"));
        }
        tx.commit().await?;
        self.publier_msg(etab, types::MESSAGERIE_FIL_TRAITE, fil, None, user, json!({ "boite_id": boite }))
            .await?;
        Ok(())
    }

    /// Poser ou retirer une échéance (MSG-167) : un jour d'établissement, jamais dans le passé.
    pub async fn poser_echeance_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        boite: Uuid,
        echeance: Option<NaiveDate>,
    ) -> Result<(), EntDbError> {
        self.exiger_membre_traitant(etab, user, role, fil, boite).await?;
        if let Some(e) = echeance {
            if e < self.aujourdhui_etablissement(etab).await? {
                return Err(refus("échéance dans le passé"));
            }
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "UPDATE ent_msg_traitement SET echeance = $4, maj_le = now()
              WHERE etablissement_id = $1 AND fil_id = $2 AND boite_id = $3",
        )
        .bind(etab)
        .bind(fil)
        .bind(boite)
        .bind(echeance)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Pose (ou retire) une étiquette de la boîte sur le fil.
    pub async fn etiqueter_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        boite: Uuid,
        etiquette_code: &str,
        poser: bool,
    ) -> Result<(), EntDbError> {
        self.exiger_membre_traitant(etab, user, role, fil, boite).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let e: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_msg_etiquette WHERE etablissement_id = $1 AND boite_id = $2 AND code = $3",
        )
        .bind(etab)
        .bind(boite)
        .bind(etiquette_code)
        .fetch_optional(&mut *tx)
        .await?;
        let (eid,) = e.ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if poser {
            sqlx::query(
                "INSERT INTO ent_msg_traitement_etiquette (etablissement_id, fil_id, boite_id, etiquette_id)
                 VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
            )
            .bind(etab)
            .bind(fil)
            .bind(boite)
            .bind(eid)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "DELETE FROM ent_msg_traitement_etiquette
                  WHERE etablissement_id = $1 AND fil_id = $2 AND boite_id = $3 AND etiquette_id = $4",
            )
            .bind(etab)
            .bind(fil)
            .bind(boite)
            .bind(eid)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Note interne (MSG-165) : invisible de la famille, par construction (table à part).
    pub async fn ecrire_note_interne_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        boite: Uuid,
        corps: &str,
    ) -> Result<Uuid, EntDbError> {
        let personne = self.exiger_membre_traitant(etab, user, role, fil, boite).await?;
        let corps = corps.trim();
        if corps.is_empty() || corps.chars().count() > 5000 {
            return Err(refus("note vide ou trop longue"));
        }
        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO ent_msg_note_interne (id, etablissement_id, fil_id, boite_id, auteur_personne_id, corps)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(etab)
        .bind(fil)
        .bind(boite)
        .bind(personne)
        .bind(corps)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Notes internes d'un fil : membres de la boîte seulement.
    pub async fn notes_internes_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        boite: Uuid,
    ) -> Result<Vec<NoteInterne>, EntDbError> {
        self.exiger_membre_traitant(etab, user, role, fil, boite).await?;
        Ok(sqlx::query_as::<_, NoteInterne>(
            "SELECT n.id, btrim(COALESCE(pe.prenom, '') || ' ' || pe.nom) AS auteur, n.corps, n.transfert, n.cree_le
               FROM ent_msg_note_interne n
               JOIN ent_personne pe ON pe.etablissement_id = n.etablissement_id AND pe.id = n.auteur_personne_id
              WHERE n.etablissement_id = $1 AND n.fil_id = $2 AND n.boite_id = $3
              ORDER BY n.cree_le, n.id",
        )
        .bind(etab)
        .bind(fil)
        .bind(boite)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    /// Transfert vers une autre boîte (MSG-164) avec note interne : la boîte d'origine sort du fil, la
    /// nouvelle y entre, le traitement repart « nouveau ». La famille voit seulement le libellé de la nouvelle
    /// boîte. Un fil ne change **jamais** de sensibilité par un transfert : de ou vers une boîte C3, refusé
    /// (R-M11 : on ne déverse pas un échange confidentiel dans une boîte ordinaire, ni l'inverse).
    pub async fn transferer_fil_boite_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        de: Uuid,
        vers: Uuid,
        note: Option<&str>,
    ) -> Result<(), EntDbError> {
        let personne = self.exiger_membre_traitant(etab, user, role, fil, de).await?;
        if de == vers {
            return Err(refus("la boîte de destination est la boîte actuelle"));
        }
        let (b_de, b_vers) = (self.charger_boite(etab, de).await?, self.charger_boite(etab, vers).await?);
        if !b_vers.actif {
            return Err(refus("boîte de destination inactive"));
        }
        if b_de.sensibilite != b_vers.sensibilite {
            return Err(EntDbError::ConflitDossier(
                "un échange confidentiel ne se transfère pas vers une boîte ordinaire, ni l'inverse",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "UPDATE ent_msg_participant SET au = now(), motif_sortie = 'autre'
              WHERE etablissement_id = $1 AND fil_id = $2 AND boite_id = $3 AND au IS NULL",
        )
        .bind(etab)
        .bind(fil)
        .bind(de)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_msg_participant (id, etablissement_id, fil_id, boite_id, cote, qualite, raison)
             VALUES ($1,$2,$3,$4,'etablissement','destinataire','boite')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(fil)
        .bind(vers)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_msg_traitement SET statut = 'transfere', maj_le = now()
              WHERE etablissement_id = $1 AND fil_id = $2 AND boite_id = $3",
        )
        .bind(etab)
        .bind(fil)
        .bind(de)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_msg_traitement (etablissement_id, fil_id, boite_id)
             VALUES ($1,$2,$3)
             ON CONFLICT (fil_id, boite_id) DO UPDATE SET statut = 'nouveau', pris_par_personne_id = NULL,
                    pris_le = NULL, traite_le = NULL, traite_par_personne_id = NULL, maj_le = now()",
        )
        .bind(etab)
        .bind(fil)
        .bind(vers)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_msg_transfert (id, etablissement_id, fil_id, de_boite_id, vers_boite_id, par_personne_id)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(fil)
        .bind(de)
        .bind(vers)
        .bind(personne)
        .execute(&mut *tx)
        .await?;
        // Note interne de transfert, lisible de la nouvelle boîte (et de l'ancienne tant qu'elle a le fil).
        let texte = note.map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Transféré.");
        sqlx::query(
            "INSERT INTO ent_msg_note_interne (id, etablissement_id, fil_id, boite_id, auteur_personne_id, corps, transfert)
             VALUES ($1,$2,$3,$4,$5,$6,true)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(fil)
        .bind(vers)
        .bind(personne)
        .bind(texte)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE ent_msg_fil SET sensibilite = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(fil)
            .bind(&b_vers.sensibilite)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.publier_msg(
            etab,
            types::MESSAGERIE_FIL_TRANSFERE,
            fil,
            None,
            user,
            json!({ "de": de, "vers": vers }),
        )
        .await?;
        Ok(())
    }

    // ───────────────────────────── Message téléphonique (MSG-171, CM-49) ─────────────────────────────

    /// Saisie d'un message téléphonique par l'accueil ou la vie scolaire : un fil de boîte, origine
    /// « téléphone », message **non vérifié** (la personne présumée n'est pas authentifiée), sans effet
    /// automatique. L'accueil **ne lit pas la suite du fil** : sa participation nominative est refermée
    /// aussitôt ; il ne voit que ses saisies non encore prises en charge, sans leur contenu de suivi.
    pub async fn saisir_message_telephonique_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        s: SaisieTelephone,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieMessageTelSaisir).await?;
        if !matches!(s.motif_code.as_str(), "absence_signalee" | "information" | "rendez_vous" | "autre") {
            return Err(refus("motif de message téléphonique inconnu"));
        }
        if s.appelant_personne_id.is_none()
            && s.appelant_libelle.as_deref().map(str::trim).unwrap_or("").is_empty()
        {
            return Err(refus("indiquer la personne présumée"));
        }
        let b = self.charger_boite(etab, s.boite_id).await?;
        if !b.actif {
            return Err(refus("boîte inactive"));
        }
        let (mut fil, msg) = domain_eleve::plan_ouvrir_fil(domain_eleve::OuvrirFil {
            etablissement_id: etab,
            eleve_id: s.eleve_id,
            sujet: "Message téléphonique".to_string(),
            premier_message: s.texte.clone(),
            actor_user_id: user,
        })?;
        fil.modele_id = None;
        self.messages.insert_fil_portee(&fil, PorteeFil::responsables()).await?;
        self.messages.insert_message_remise(&msg, None, None).await?;
        self.messages.touch_fil_last(etab, fil.id, msg.created_at).await?;
        let urgence = urgence_deterministe(&s.texte);
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "UPDATE ent_msg_fil SET nature = 'boite', sensibilite = $3, origine = 'telephone'
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(fil.id)
        .bind(&b.sensibilite)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_msg_participant (id, etablissement_id, fil_id, boite_id, cote, qualite, raison)
             VALUES ($1,$2,$3,$4,'etablissement','destinataire','boite')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(fil.id)
        .bind(s.boite_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_msg_traitement (etablissement_id, fil_id, boite_id, urgence) VALUES ($1,$2,$3,$4)",
        )
        .bind(etab)
        .bind(fil.id)
        .bind(s.boite_id)
        .bind(urgence)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_msg SET verifie = false, canal_entrant = 'telephone', type = 'telephone',
                    expediteur_presume_personne_id = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(msg.id)
        .bind(s.appelant_personne_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_msg_telephone
                (message_id, etablissement_id, fil_id, boite_id, appelant_personne_id, appelant_libelle,
                 motif_code, rappel_demande, saisi_par_user_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(msg.id)
        .bind(etab)
        .bind(fil.id)
        .bind(s.boite_id)
        .bind(s.appelant_personne_id)
        .bind(s.appelant_libelle.as_deref().map(str::trim).filter(|l| !l.is_empty()))
        .bind(&s.motif_code)
        .bind(s.rappel_demande)
        .bind(user)
        .execute(&mut *tx)
        .await?;
        // L'accueil ne lit pas la suite : sa participation nominative est refermée (CM-49).
        sqlx::query(
            "UPDATE ent_msg_participant p SET au = now(), motif_sortie = 'autre'
              WHERE p.etablissement_id = $1 AND p.fil_id = $2 AND p.au IS NULL AND p.personne_id IN
                    (SELECT pe.id FROM ent_personne pe WHERE pe.etablissement_id = $1 AND pe.user_id = $3)",
        )
        .bind(etab)
        .bind(fil.id)
        .bind(user)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.publier_msg(
            etab,
            types::MESSAGERIE_MESSAGE_TEL_SAISI,
            fil.id,
            if b.sensibilite == "C3" { None } else { Some(s.eleve_id) },
            user,
            json!({ "message_id": msg.id, "boite_id": s.boite_id, "motif": s.motif_code }),
        )
        .await?;
        self.avertir_boite(etab, s.boite_id, &b.sensibilite, fil.id, msg.id, s.eleve_id, false, user, Utc::now())
            .await;
        Ok(fil.id)
    }

    /// Ce que l'accueil retrouve de ses saisies **tant qu'elles ne sont pas prises en charge** : aucun
    /// contenu de suivi, aucune lecture du fil.
    pub async fn mes_messages_telephoniques_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
    ) -> Result<Vec<TelephoneSaisi>, EntDbError> {
        self.exiger_action_msg(etab, user, role, Action::MessagerieMessageTelSaisir).await?;
        Ok(sqlx::query_as::<_, TelephoneSaisi>(
            "SELECT t.message_id, b.libelle AS boite_libelle, t.motif_code, t.appelant_libelle,
                    t.rappel_demande, t.cree_le
               FROM ent_msg_telephone t
               JOIN ent_msg_boite b ON b.etablissement_id = t.etablissement_id AND b.id = t.boite_id
               JOIN ent_msg_traitement tr ON tr.etablissement_id = t.etablissement_id
                                         AND tr.fil_id = t.fil_id AND tr.boite_id = t.boite_id
              WHERE t.etablissement_id = $1 AND t.saisi_par_user_id = $2 AND tr.statut = 'nouveau'
              ORDER BY t.cree_le DESC",
        )
        .bind(etab)
        .bind(user)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?)
    }

    // ───────────────────────────── Messages entrants non vérifiés (R-M41, MSG-172) ─────────────────────────────

    /// Port `messagerie::deposer_message_entrant` (§6.3), appelé par le socle 8 (SMS, courriel). Le texte est
    /// **non vérifié et non fiable** ; l'expéditeur n'est que présumé ; **aucun effet automatique** (aucune
    /// réponse, aucune action, aucune participation de la famille). `depose_par` est le compte de service ou
    /// l'agent qui dépose (auteur technique du message). Sans élève identifiable, le message va à la file de
    /// tri du socle 8, pas ici.
    #[allow(clippy::too_many_arguments)]
    pub async fn deposer_message_entrant(
        &self,
        etab: Uuid,
        canal: CanalEntrant,
        boite: Uuid,
        eleve: Uuid,
        expediteur_presume: Option<Uuid>,
        texte: &str,
        depose_par: Uuid,
    ) -> Result<Uuid, EntDbError> {
        let b = self.charger_boite(etab, boite).await?;
        if !b.actif {
            return Err(refus("boîte inactive"));
        }
        let (mut fil, msg) = domain_eleve::plan_ouvrir_fil(domain_eleve::OuvrirFil {
            etablissement_id: etab,
            eleve_id: eleve,
            sujet: match canal {
                CanalEntrant::Sms => "SMS reçu".to_string(),
                CanalEntrant::Courriel => "Courriel reçu".to_string(),
            },
            premier_message: texte.to_string(),
            actor_user_id: depose_par,
        })?;
        fil.modele_id = None;
        self.messages.insert_fil_portee(&fil, PorteeFil::responsables()).await?;
        self.messages.insert_message_remise(&msg, None, None).await?;
        self.messages.touch_fil_last(etab, fil.id, msg.created_at).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "UPDATE ent_msg_fil SET nature = 'boite', sensibilite = $3, origine = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(fil.id)
        .bind(&b.sensibilite)
        .bind(canal.origine())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_msg_participant (id, etablissement_id, fil_id, boite_id, cote, qualite, raison)
             VALUES ($1,$2,$3,$4,'etablissement','destinataire','boite')",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(fil.id)
        .bind(boite)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO ent_msg_traitement (etablissement_id, fil_id, boite_id, urgence) VALUES ($1,$2,$3,$4)",
        )
        .bind(etab)
        .bind(fil.id)
        .bind(boite)
        .bind(urgence_deterministe(texte))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_msg SET verifie = false, canal_entrant = $3, expediteur_presume_personne_id = $4
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(msg.id)
        .bind(canal.code())
        .bind(expediteur_presume)
        .execute(&mut *tx)
        .await?;
        // Le déposant technique n'est pas un participant nominatif (il ne relit pas le fil).
        sqlx::query(
            "UPDATE ent_msg_participant p SET au = now(), motif_sortie = 'autre'
              WHERE p.etablissement_id = $1 AND p.fil_id = $2 AND p.au IS NULL AND p.personne_id IN
                    (SELECT pe.id FROM ent_personne pe WHERE pe.etablissement_id = $1 AND pe.user_id = $3)",
        )
        .bind(etab)
        .bind(fil.id)
        .bind(depose_par)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.publier_msg(
            etab,
            types::MESSAGERIE_ENTRANT_DEPOSE,
            fil.id,
            if b.sensibilite == "C3" { None } else { Some(eleve) },
            depose_par,
            json!({ "message_id": msg.id, "boite_id": boite, "canal": canal.code() }),
        )
        .await?;
        self.avertir_boite(etab, boite, &b.sensibilite, fil.id, msg.id, eleve, false, depose_par, Utc::now())
            .await;
        Ok(fil.id)
    }

    /// Rattacher un message entrant à une personne identifiée (MSG-172) : un membre vérifie l'expéditeur ;
    /// la personne doit avoir un lien **communicant** avec l'élève du fil. Elle devient participante
    /// (famille) et le message est marqué vérifié. Sans cette action, rien n'est jamais rattaché.
    pub async fn rattacher_entrant_for_actor(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
        boite: Uuid,
        personne: Uuid,
    ) -> Result<(), EntDbError> {
        self.exiger_membre_traitant(etab, user, role, fil, boite).await?;
        let (origine, eleve): (String, Uuid) = sqlx::query_as(
            "SELECT origine, eleve_id FROM ent_msg_fil WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(fil)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?
        .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if !matches!(origine.as_str(), "sms_entrant" | "courriel_entrant" | "telephone") {
            return Err(refus("ce fil n'est pas un message entrant"));
        }
        let (ok,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_communicant l
                             JOIN ent_personne pe ON pe.etablissement_id = l.etablissement_id AND pe.user_id = l.user_id
                            WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND pe.id = $3)",
        )
        .bind(etab)
        .bind(eleve)
        .bind(personne)
        .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        if !ok {
            return Err(refus("cette personne n'a pas de lien communicant avec l'élève"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO ent_msg_participant
                (id, etablissement_id, fil_id, personne_id, cote, qualite, raison)
             VALUES ($1,$2,$3,$4,'famille','destinataire','titulaire_foyer') ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(fil)
        .bind(personne)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE ent_msg SET verifie = true, expediteur_presume_personne_id = $3
              WHERE etablissement_id = $1 AND fil_id = $2 AND canal_entrant IS NOT NULL",
        )
        .bind(etab)
        .bind(fil)
        .bind(personne)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    // ───────────────────────────── Port vers N1 (conversion message ↔ demande) ─────────────────────────────

    /// Résout, pour N1, le droit de convertir un fil en demande (I-008, §37.3 de N1) : celui du socle 16.
    /// Convertible : un fil que la personne **lit côté établissement** (membre de la boîte traitante, avec le
    /// droit de traitement) ; **jamais** un fil C3 (le contenu confidentiel ne passe pas dans une demande),
    /// jamais un fil lu seulement par une famille. Fail-closed.
    pub async fn resoudre_port_messagerie(
        &self,
        etab: Uuid,
        user: Uuid,
        role: &str,
        fil: Uuid,
    ) -> Result<MessagerieResolue, EntDbError> {
        let mut r = MessagerieResolue::default();
        if !Role::parse(role).is_some_and(|x| x.is_staff()) {
            return Ok(r);
        }
        if !self.a_action_etablissement(etab, user, role, Action::MessagerieBoiteTraiter).await? {
            return Ok(r);
        }
        let Some((boite, sens)) = self.boite_du_fil(etab, fil).await? else {
            return Ok(r);
        };
        if sens == "C3" || !self.est_membre_boite(etab, boite, user).await? {
            return Ok(r);
        }
        let personne = self.personne_du_compte_msg(etab, user).await?;
        r.convertibles.insert((etab, personne, fil));
        Ok(r)
    }
}
