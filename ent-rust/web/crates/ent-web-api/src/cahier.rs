//! Service du cahier de textes, du travail et des copies : routes `/cahier/*` d'`ent-api`
//! (lot S2-6). Les types sont ceux d'`ent-types::cahier`.
//!
//! Les fichiers sont envoyés en base64 (même convention que les pièces de messagerie) et
//! lus par un **lien signé court** émis après contrôle des droits : le client ne connaît
//! jamais de clé de stockage.

use chrono::NaiveDate;
use ent_types::cahier::{
    AgendaItemDto, ChargeDto, ContenuSeanceBody, CreerTravailBody, FaitBody, FichierDepotBody,
    LienCahierDto, RemiseBody, RemiseDto, RetourBody, SeanceCahierDto, SeanceContenuDto,
    SuiviTravailDto, TravailCreeDto, TravailDto,
};
use uuid::Uuid;

use crate::communication::base64_encoder;
use crate::{ApiClient, ApiError, Methode};

/// Fichier à envoyer : nom, type déclaré et octets (le serveur contrôle les octets).
pub fn depot_fichier(nom: &str, mime_type: &str, octets: &[u8]) -> FichierDepotBody {
    FichierDepotBody {
        nom: nom.to_string(),
        mime_type: mime_type.to_string(),
        contenu_base64: base64_encoder(octets),
    }
}

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Option<serde_json::Value>, ApiError> {
    serde_json::to_value(valeur)
        .map(Some)
        .map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /cahier/seances` : séances datées du professeur connecté (avec leur contenu).
    pub async fn cahier_seances(
        &self,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceCahierDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/seances?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `PUT /cahier/seances/{id}/contenu` : renseigne ou corrige le contenu d'une séance.
    pub async fn cahier_renseigner_seance(
        &self,
        seance_id: Uuid,
        contenu: &str,
        ressources: &str,
    ) -> Result<SeanceContenuDto, ApiError> {
        let b = ContenuSeanceBody {
            contenu: contenu.to_string(),
            ressources: ressources.to_string(),
        };
        self.appel(
            Methode::Put,
            &format!("/cahier/seances/{seance_id}/contenu"),
            corps(&b)?,
        )
        .await
    }

    /// `GET /cahier/travaux` : travaux d'une classe (ou d'un groupe) sur une période.
    pub async fn cahier_travaux(
        &self,
        classe_id: Option<Uuid>,
        groupe_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<TravailDto>, ApiError> {
        let cible = match (classe_id, groupe_id) {
            (Some(c), _) => format!("classe_id={c}"),
            (None, Some(g)) => format!("groupe_id={g}"),
            (None, None) => String::new(),
        };
        self.appel(
            Methode::Get,
            &format!("/cahier/travaux?{cible}&du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `POST /cahier/travaux` : donne un travail (à faire, à rendre, contrôle).
    pub async fn cahier_donner_travail(
        &self,
        b: &CreerTravailBody,
    ) -> Result<TravailCreeDto, ApiError> {
        self.appel(Methode::Post, "/cahier/travaux", corps(b)?)
            .await
    }

    /// `DELETE /cahier/travaux/{id}` : supprime un travail (historique conservé).
    pub async fn cahier_supprimer_travail(&self, id: Uuid) -> Result<(), ApiError> {
        self.appel(Methode::Delete, &format!("/cahier/travaux/{id}"), None)
            .await
    }

    /// `GET /cahier/travaux/{id}/suivi` : élèves de la cible et remises reçues.
    pub async fn cahier_suivi(&self, id: Uuid) -> Result<SuiviTravailDto, ApiError> {
        self.appel(Methode::Get, &format!("/cahier/travaux/{id}/suivi"), None)
            .await
    }

    /// `POST /cahier/remises/{id}/retour` : renvoie la copie corrigée.
    pub async fn cahier_retour(
        &self,
        remise_id: Uuid,
        commentaire: &str,
        fichier: Option<FichierDepotBody>,
    ) -> Result<RemiseDto, ApiError> {
        let b = RetourBody {
            commentaire: commentaire.to_string(),
            note_id: None,
            fichier,
        };
        self.appel(
            Methode::Post,
            &format!("/cahier/remises/{remise_id}/retour"),
            corps(&b)?,
        )
        .await
    }

    /// `POST /cahier/travaux/{id}/remise` : l'élève rend son travail.
    pub async fn cahier_rendre(
        &self,
        travail_id: Uuid,
        commentaire: &str,
        fichier: Option<FichierDepotBody>,
    ) -> Result<RemiseDto, ApiError> {
        let b = RemiseBody {
            commentaire: commentaire.to_string(),
            fichier,
        };
        self.appel(
            Methode::Post,
            &format!("/cahier/travaux/{travail_id}/remise"),
            corps(&b)?,
        )
        .await
    }

    /// `GET /cahier/agenda` : travail à faire d'un élève (`eleve_id` : enfant d'un responsable).
    pub async fn cahier_agenda(
        &self,
        eleve_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<AgendaItemDto>, ApiError> {
        let eleve = eleve_id
            .map(|e| format!("eleve_id={e}&"))
            .unwrap_or_default();
        self.appel(
            Methode::Get,
            &format!("/cahier/agenda?{eleve}du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /cahier/mon-cahier` : contenus des séances de la classe de l'élève.
    pub async fn cahier_de_l_eleve(
        &self,
        eleve_id: Option<Uuid>,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<SeanceCahierDto>, ApiError> {
        let eleve = eleve_id
            .map(|e| format!("eleve_id={e}&"))
            .unwrap_or_default();
        self.appel(
            Methode::Get,
            &format!("/cahier/mon-cahier?{eleve}du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /cahier/travaux/{id}` : détail d'un travail ; consulté par l'élève concerné, il le
    /// marque « vu » (le professeur sait alors qu'une modification a été lue).
    pub async fn cahier_travail(&self, travail_id: Uuid) -> Result<AgendaItemDto, ApiError> {
        self.appel(Methode::Get, &format!("/cahier/travaux/{travail_id}"), None)
            .await
    }

    /// `PUT /cahier/travaux/{id}/fait` : l'élève marque (ou démarque) un travail.
    pub async fn cahier_marquer_fait(&self, travail_id: Uuid, fait: bool) -> Result<(), ApiError> {
        self.appel(
            Methode::Put,
            &format!("/cahier/travaux/{travail_id}/fait"),
            corps(&FaitBody { fait })?,
        )
        .await
    }

    /// `POST /cahier/pieces/{id}/lien` : lien signé court vers une pièce.
    pub async fn cahier_lien_piece(&self, piece_id: Uuid) -> Result<LienCahierDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/cahier/pieces/{piece_id}/lien"),
            None,
        )
        .await
    }

    /// `GET /cahier/charge` : charge de travail d'une classe, alerte de surcharge par jour.
    pub async fn cahier_charge(
        &self,
        classe_id: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<ChargeDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/cahier/charge?classe_id={classe_id}&du={du}&au={au}"),
            None,
        )
        .await
    }

    /// Adresse à ouvrir pour un lien signé : en mode cookie, via le relais de session
    /// (`/web/api`) ; sinon relative à la base du client.
    pub fn adresse_lien(&self, lien: &LienCahierDto) -> String {
        if lien.url.starts_with('/') {
            self.url(&lien.url)
        } else {
            lien.url.clone()
        }
    }
}
