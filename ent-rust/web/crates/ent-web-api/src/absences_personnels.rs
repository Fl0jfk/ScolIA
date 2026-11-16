//! Absences des personnels (socle 15, écrans A11a à A11c) : client des routes
//! `/personnels/absences/*` d'`ent-api` (`absences_personnels*.rs`). Les types sont ceux d'`ent-types`.
//!
//! Aucun droit n'est décidé ici : le serveur répond 403 / 404 hors du cercle autorisé (R-AP2, §23).
//! Les méthodes portent le préfixe `absp_` pour ne pas se heurter aux absences d'élèves.

use chrono::NaiveDate;
use ent_types::absences_personnels::{
    AbsenceDto, AnnulerSeanceBody, AppelVolontairesBody, ArretRecuBody, ChangerFinBody,
    CompteursTableauDeBordDto, ConfirmerBody, DecisionBody, DecisionExterneBody, DeclarationDto,
    DeclarerAbsenceBody, DeposerPieceBody, EcheanceAbsenceDto, ImpactDto, JournalAbsenceDto,
    LecturePieceDto, LienPieceDto, MessageAbsenceDto, NatureDto, PieceAbsenceDto,
    PosterMessageBody, PresenceDto, ProchaineEtapeDto, PropositionDto, RefuserSollicitationBody,
    RequalifierBody, SignalerAbsenceBody, SollicitationDto, SolliciterBody, TableauJourDto,
};
use ent_types::personnels::AgentResumeDto;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

const BASE: &str = "/personnels/absences";

impl ApiClient {
    // ───────────────────────── A11a : déclarer, mes absences ─────────────────────────

    /// `GET /personnels/absences/natures` : liste courte des natures permises à mon engagement.
    pub async fn absp_natures(&self) -> Result<Vec<NatureDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/natures"), None)
            .await
    }

    /// `GET /personnels/absences/moi` : mes absences (portée `moi`).
    pub async fn absp_mes_absences(&self) -> Result<Vec<AbsenceDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/moi"), None).await
    }

    /// `POST /personnels/absences` : déclarer (soi-même, ou un collègue avec `agent_id`).
    pub async fn absp_declarer(
        &self,
        body: &DeclarerAbsenceBody,
    ) -> Result<DeclarationDto, ApiError> {
        self.appel(Methode::Post, BASE, corps(body)?).await
    }

    /// `POST /personnels/absences/signalements` : absence signalée, sans nature (accueil, vie scolaire).
    pub async fn absp_signaler(
        &self,
        body: &SignalerAbsenceBody,
    ) -> Result<DeclarationDto, ApiError> {
        self.appel(Methode::Post, &format!("{BASE}/signalements"), corps(body)?)
            .await
    }

    /// `GET /personnels/annuaire` : agents que l'on peut désigner pour déclarer à leur place (ABP-101).
    pub async fn absp_annuaire(&self) -> Result<Vec<AgentResumeDto>, ApiError> {
        self.appel(Methode::Get, "/personnels/annuaire?limite=200", None)
            .await
    }

    /// `GET /personnels/absences/presence/{agent}` : présent, absent ou inconnu (jamais la nature).
    pub async fn absp_presence(&self, agent_id: Uuid) -> Result<PresenceDto, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/presence/{agent_id}"), None)
            .await
    }

    // ───────────────────────── Fiche : lecture ─────────────────────────

    /// `GET /personnels/absences/{id}` : cercle RH (404 uniforme hors cercle).
    pub async fn absp_absence(&self, id: Uuid) -> Result<AbsenceDto, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/{id}"), None)
            .await
    }

    /// `GET /personnels/absences/{id}/historique` : journal en ajout seul.
    pub async fn absp_historique(&self, id: Uuid) -> Result<Vec<JournalAbsenceDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/{id}/historique"), None)
            .await
    }

    /// `GET /personnels/absences/{id}/impact` : séances touchées, sans nature.
    pub async fn absp_impact(&self, id: Uuid) -> Result<Vec<ImpactDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/{id}/impact"), None)
            .await
    }

    /// `GET /personnels/absences/{id}/prochaine-etape` : qui doit agir.
    pub async fn absp_prochaine_etape(&self, id: Uuid) -> Result<ProchaineEtapeDto, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/{id}/prochaine-etape"), None)
            .await
    }

    /// `GET /personnels/absences/{id}/retour` : ce qui a été fait pendant l'absence (ABP-159).
    pub async fn absp_retour(&self, id: Uuid) -> Result<Vec<ImpactDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/{id}/retour"), None)
            .await
    }

    // ───────────────────────── Fiche : écritures sur l'absence ─────────────────────────

    /// `POST /personnels/absences/{id}/confirmer` : l'agent confirme une absence signalée.
    pub async fn absp_confirmer(
        &self,
        id: Uuid,
        nature_code: Option<String>,
    ) -> Result<AbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/confirmer"),
            corps(&ConfirmerBody { nature_code })?,
        )
        .await
    }

    /// `POST /personnels/absences/{id}/{action}` pour les décisions à corps commun
    /// (`valider`, `refuser`, `precision`, `traiter`, `clore`, `rouvrir`, `annuler`).
    pub async fn absp_decider(
        &self,
        id: Uuid,
        action: ActionDecision,
        body: &DecisionBody,
    ) -> Result<AbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/{}", action.segment()),
            corps(body)?,
        )
        .await
    }

    /// `POST /personnels/absences/{id}/requalifier`.
    pub async fn absp_requalifier(
        &self,
        id: Uuid,
        body: &RequalifierBody,
    ) -> Result<AbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/requalifier"),
            corps(body)?,
        )
        .await
    }

    /// `POST /personnels/absences/{id}/prolonger` ou `/ecourter` : nouvelle dernière journée.
    pub async fn absp_changer_fin(
        &self,
        id: Uuid,
        ecourter: bool,
        body: &ChangerFinBody,
    ) -> Result<AbsenceDto, ApiError> {
        let segment = if ecourter { "ecourter" } else { "prolonger" };
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/{segment}"),
            corps(body)?,
        )
        .await
    }

    /// `POST /personnels/absences/{id}/decision-externe` : l'employeur décide, l'établissement enregistre.
    pub async fn absp_decision_externe(
        &self,
        id: Uuid,
        body: &DecisionExterneBody,
    ) -> Result<AbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/decision-externe"),
            corps(body)?,
        )
        .await
    }

    /// `POST /personnels/absences/{id}/arret-recu` : l'instructeur note la réception de l'arrêt.
    pub async fn absp_arret_recu(&self, id: Uuid) -> Result<AbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/arret-recu"),
            corps(&ArretRecuBody::default())?,
        )
        .await
    }

    // ───────────────────────── Pièces et fil ─────────────────────────

    /// `GET /personnels/absences/{id}/pieces`.
    pub async fn absp_pieces(&self, id: Uuid) -> Result<Vec<PieceAbsenceDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/{id}/pieces"), None)
            .await
    }

    /// `POST /personnels/absences/{id}/pieces` : dépôt (base64, comme le cahier de textes).
    pub async fn absp_deposer_piece(
        &self,
        id: Uuid,
        body: &DeposerPieceBody,
    ) -> Result<PieceAbsenceDto, ApiError> {
        self.appel(Methode::Post, &format!("{BASE}/{id}/pieces"), corps(body)?)
            .await
    }

    /// `POST /personnels/absences/{id}/pieces/{piece}/lien` : adresse pré-signée de 5 minutes,
    /// lecture journalisée par le serveur.
    pub async fn absp_lien_piece(&self, id: Uuid, piece: Uuid) -> Result<LienPieceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/pieces/{piece}/lien"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `POST /personnels/absences/{id}/pieces/{piece}/retirer`.
    pub async fn absp_retirer_piece(
        &self,
        id: Uuid,
        piece: Uuid,
    ) -> Result<PieceAbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/pieces/{piece}/retirer"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `POST /personnels/absences/{id}/pieces/relancer` : relance de la pièce attendue.
    pub async fn absp_relancer_piece(&self, id: Uuid) -> Result<serde_json::Value, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/pieces/relancer"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `GET /personnels/absences/{id}/pieces/lectures` : qui a lu mes pièces (ABP-144).
    pub async fn absp_lectures(&self, id: Uuid) -> Result<Vec<LecturePieceDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/{id}/pieces/lectures"), None)
            .await
    }

    /// `GET /personnels/absences/{id}/messages` : fil de l'absence (cercle RH).
    pub async fn absp_fil(&self, id: Uuid) -> Result<Vec<MessageAbsenceDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/{id}/messages"), None)
            .await
    }

    /// `POST /personnels/absences/{id}/messages` : ajout seul, 4 000 caractères.
    pub async fn absp_poster(&self, id: Uuid, texte: &str) -> Result<MessageAbsenceDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/{id}/messages"),
            corps(&PosterMessageBody {
                texte: texte.to_string(),
            })?,
        )
        .await
    }

    // ───────────────────────── A11b : files ─────────────────────────

    /// `GET /personnels/absences/a-valider` : file du valideur.
    pub async fn absp_a_valider(&self) -> Result<Vec<AbsenceDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/a-valider"), None)
            .await
    }

    /// `GET /personnels/absences/a-traiter` : file de l'instructeur.
    pub async fn absp_a_traiter(&self) -> Result<Vec<AbsenceDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/a-traiter"), None)
            .await
    }

    /// `GET /personnels/absences/compteurs` : à valider, à traiter, séances à couvrir aujourd'hui.
    pub async fn absp_compteurs(&self) -> Result<CompteursTableauDeBordDto, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/compteurs"), None)
            .await
    }

    /// `GET /personnels/absences/echeances` : 48 h de l'arrêt, pièces attendues…
    pub async fn absp_echeances(&self) -> Result<Vec<EcheanceAbsenceDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/echeances"), None)
            .await
    }

    // ───────────────────────── A11c : ce matin, propositions, réponses ─────────────────────────

    /// `GET /personnels/absences/tableau?jour=` : « ce matin » / « demain » (organisation, sans nature).
    pub async fn absp_tableau(&self, jour: NaiveDate) -> Result<TableauJourDto, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/tableau?jour={jour}"), None)
            .await
    }

    /// `GET /personnels/absences/impacts/{impact}/propositions` : remplaçants possibles, avec leurs raisons.
    pub async fn absp_propositions(&self, impact: Uuid) -> Result<Vec<PropositionDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("{BASE}/impacts/{impact}/propositions"),
            None,
        )
        .await
    }

    /// `POST /personnels/absences/impacts/{impact}/solliciter`.
    pub async fn absp_solliciter(
        &self,
        impact: Uuid,
        agent_id: Uuid,
    ) -> Result<SollicitationDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/impacts/{impact}/solliciter"),
            corps(&SolliciterBody { agent_id })?,
        )
        .await
    }

    /// `POST /personnels/absences/impacts/{impact}/designer` : désignation sans sollicitation (urgence).
    pub async fn absp_designer(
        &self,
        impact: Uuid,
        agent_id: Uuid,
    ) -> Result<SollicitationDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/impacts/{impact}/designer"),
            corps(&SolliciterBody { agent_id })?,
        )
        .await
    }

    /// `POST /personnels/absences/impacts/{impact}/appel-volontaires`.
    pub async fn absp_appel_volontaires(
        &self,
        impact: Uuid,
        agents: Vec<Uuid>,
    ) -> Result<Vec<SollicitationDto>, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/impacts/{impact}/appel-volontaires"),
            corps(&AppelVolontairesBody { agents })?,
        )
        .await
    }

    /// `POST /personnels/absences/impacts/{impact}/lever` : lève la réponse (réversible, fait daté).
    pub async fn absp_lever(&self, impact: Uuid) -> Result<ImpactDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/impacts/{impact}/lever"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `POST /personnels/absences/impacts/{impact}/annuler-seance` : élèves pris en charge.
    pub async fn absp_annuler_seance(
        &self,
        impact: Uuid,
        sortie_anticipee: bool,
    ) -> Result<ImpactDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/impacts/{impact}/annuler-seance"),
            corps(&AnnulerSeanceBody { sortie_anticipee })?,
        )
        .await
    }

    /// `POST /personnels/absences/impacts/{impact}/sans-objet` : la séance n'avait pas lieu.
    pub async fn absp_sans_objet(&self, impact: Uuid) -> Result<ImpactDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/impacts/{impact}/sans-objet"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `GET /personnels/absences/mes-sollicitations` : « ma séance de remplacement » (ABP-158).
    pub async fn absp_mes_sollicitations(&self) -> Result<Vec<SollicitationDto>, ApiError> {
        self.appel(Methode::Get, &format!("{BASE}/mes-sollicitations"), None)
            .await
    }

    /// `POST /personnels/absences/sollicitations/{id}/accepter`.
    pub async fn absp_accepter(&self, id: Uuid) -> Result<SollicitationDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/sollicitations/{id}/accepter"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `POST /personnels/absences/sollicitations/{id}/refuser` : motif légitime codé si volontaire RCD.
    pub async fn absp_refuser_sollicitation(
        &self,
        id: Uuid,
        refus_code: Option<String>,
    ) -> Result<SollicitationDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/sollicitations/{id}/refuser"),
            corps(&RefuserSollicitationBody { refus_code })?,
        )
        .await
    }

    /// `POST /personnels/absences/sollicitations/{id}/accuser` : accusé de réception d'une désignation.
    pub async fn absp_accuser(&self, id: Uuid) -> Result<SollicitationDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("{BASE}/sollicitations/{id}/accuser"),
            Some(serde_json::json!({})),
        )
        .await
    }
}

/// Décision à corps commun sur une absence : le segment d'adresse correspondant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionDecision {
    Valider,
    Refuser,
    /// Demander une pièce ou une précision (`en_instruction`).
    Precision,
    Traiter,
    Clore,
    Rouvrir,
    Annuler,
}

impl ActionDecision {
    pub fn segment(self) -> &'static str {
        match self {
            Self::Valider => "valider",
            Self::Refuser => "refuser",
            Self::Precision => "precision",
            Self::Traiter => "traiter",
            Self::Clore => "clore",
            Self::Rouvrir => "rouvrir",
            Self::Annuler => "annuler",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_segments_suivent_les_routes_du_serveur() {
        assert_eq!(ActionDecision::Valider.segment(), "valider");
        assert_eq!(ActionDecision::Precision.segment(), "precision");
        assert_eq!(ActionDecision::Rouvrir.segment(), "rouvrir");
    }

    #[test]
    fn le_corps_de_requalification_ne_sort_que_ses_champs() {
        let v = corps(&RequalifierBody {
            nature_code: "enfant_malade".into(),
            raison_code: None,
            note: None,
        })
        .unwrap()
        .unwrap();
        assert_eq!(v["nature_code"], "enfant_malade");
    }
}
