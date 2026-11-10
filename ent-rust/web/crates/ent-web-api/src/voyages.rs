//! Sorties et voyages scolaires (socle 42, lot V0a) : routes `/voyages/*` d'`ent-api`. Les droits,
//! les états et les préconditions sont vérifiés par le serveur ; ce client n'en suppose aucun.

use ent_types::voyages::{
    CandidatsClasseDto, CirculaireDto, EncadrantDto, EtapeDto, EtapeSaisieDto, JournalDto,
    ParamsCommandeDto, ParticipantDto, ResultatAjoutDto, SortieDetailDto, SortieDto,
    SortieModifDto, SortieResumeDto, SortieSaisieDto, TableauDirectionDto, VoyageFamilleDto,
};
use serde_json::json;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

/// Encode une valeur de paramètre de requête (lettres, chiffres, `-_.~` conservés).
fn encoder(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

impl ApiClient {
    /// `GET /voyages` : liste des sorties (projection selon le profil).
    pub async fn voyages(
        &self,
        q: &str,
        etat: Option<&str>,
        mes: bool,
        a_venir: bool,
    ) -> Result<Vec<SortieResumeDto>, ApiError> {
        let mut chemin = format!("/voyages?mes={mes}&a_venir={a_venir}");
        if !q.trim().is_empty() {
            chemin.push_str(&format!("&q={}", encoder(q.trim())));
        }
        if let Some(e) = etat.filter(|e| !e.is_empty()) {
            chemin.push_str(&format!("&etat={}", encoder(e)));
        }
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `POST /voyages` : crée une sortie (brouillon).
    pub async fn voyage_creer(&self, saisie: &SortieSaisieDto) -> Result<SortieDto, ApiError> {
        self.appel(Methode::Post, "/voyages", Some(corps(saisie)?))
            .await
    }

    /// `GET /voyages/{id}` : détail, droits calculés, prochaine étape, audit.
    pub async fn voyage(&self, id: Uuid) -> Result<SortieDetailDto, ApiError> {
        self.appel(Methode::Get, &format!("/voyages/{id}"), None)
            .await
    }

    /// `PUT /voyages/{id}` : modifie le projet (verrou optimiste sur `version`).
    pub async fn voyage_modifier(
        &self,
        id: Uuid,
        modif: &SortieModifDto,
    ) -> Result<SortieDto, ApiError> {
        self.appel(Methode::Put, &format!("/voyages/{id}"), Some(corps(modif)?))
            .await
    }

    /// `POST /voyages/{id}/commandes/{commande}` : transition d'état (R-SV1).
    pub async fn voyage_commande(
        &self,
        id: Uuid,
        commande: &str,
        params: &ParamsCommandeDto,
    ) -> Result<SortieDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/voyages/{id}/commandes/{commande}"),
            Some(corps(params)?),
        )
        .await
    }

    /// `PUT /voyages/{id}/programme` : remplace les étapes du programme.
    pub async fn voyage_programme(
        &self,
        id: Uuid,
        etapes: &[EtapeSaisieDto],
    ) -> Result<Vec<EtapeDto>, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/voyages/{id}/programme"),
            Some(corps(&etapes)?),
        )
        .await
    }

    /// `GET /voyages/{id}/journal`.
    pub async fn voyage_journal(&self, id: Uuid) -> Result<Vec<JournalDto>, ApiError> {
        self.appel(Methode::Get, &format!("/voyages/{id}/journal"), None)
            .await
    }

    /// `GET /voyages/{id}/candidats` : élèves proposés (registre, scolarisés à la date).
    pub async fn voyage_candidats(
        &self,
        id: Uuid,
        classe: Option<Uuid>,
    ) -> Result<Vec<CandidatsClasseDto>, ApiError> {
        let chemin = match classe {
            Some(c) => format!("/voyages/{id}/candidats?classe_id={c}"),
            None => format!("/voyages/{id}/candidats"),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `GET /voyages/{id}/participants`.
    pub async fn voyage_participants(&self, id: Uuid) -> Result<Vec<ParticipantDto>, ApiError> {
        self.appel(Methode::Get, &format!("/voyages/{id}/participants"), None)
            .await
    }

    /// `POST /voyages/{id}/participants` : inscrit des élèves choisis dans le registre.
    pub async fn voyage_ajouter_participants(
        &self,
        id: Uuid,
        eleves: &[Uuid],
    ) -> Result<ResultatAjoutDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/voyages/{id}/participants"),
            Some(json!({ "eleve_ids": eleves })),
        )
        .await
    }

    /// `POST /voyages/{id}/participants/{pid}/desistement` ou `/retrait`.
    pub async fn voyage_sortir_participant(
        &self,
        id: Uuid,
        participant: Uuid,
        retrait: bool,
        motif_code: &str,
        motif_texte: Option<&str>,
    ) -> Result<(), ApiError> {
        let fin = if retrait { "retrait" } else { "desistement" };
        self.appel(
            Methode::Post,
            &format!("/voyages/{id}/participants/{participant}/{fin}"),
            Some(json!({ "motif_code": motif_code, "motif_texte": motif_texte })),
        )
        .await
    }

    /// `POST /voyages/{id}/encadrants` : encadrant désigné par agent du personnel.
    pub async fn voyage_ajouter_encadrant(
        &self,
        id: Uuid,
        agent_id: Uuid,
        role: &str,
    ) -> Result<EncadrantDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/voyages/{id}/encadrants"),
            Some(json!({ "agent_id": agent_id, "role": role })),
        )
        .await
    }

    /// `DELETE /voyages/{id}/encadrants/{eid}`.
    pub async fn voyage_retirer_encadrant(
        &self,
        id: Uuid,
        encadrant: Uuid,
    ) -> Result<(), ApiError> {
        self.appel(
            Methode::Delete,
            &format!("/voyages/{id}/encadrants/{encadrant}"),
            None,
        )
        .await
    }

    /// `POST /voyages/{id}/circulaire` : texte de la circulaire aux familles.
    pub async fn voyage_circulaire(&self, id: Uuid) -> Result<CirculaireDto, ApiError> {
        self.appel(Methode::Post, &format!("/voyages/{id}/circulaire"), None)
            .await
    }

    /// `GET /voyages/tableau` : tableau de bord de la direction et de la comptabilité.
    pub async fn voyages_tableau(&self) -> Result<TableauDirectionDto, ApiError> {
        self.appel(Methode::Get, "/voyages/tableau", None).await
    }

    /// `GET /voyages/famille` : sorties où mon enfant (ou moi) est inscrit.
    pub async fn voyages_famille(&self) -> Result<Vec<VoyageFamilleDto>, ApiError> {
        self.appel(Methode::Get, "/voyages/famille", None).await
    }

    /// `GET /personnels/annuaire` : agents du personnel que l'on peut désigner comme encadrants.
    pub async fn agents_pour_encadrement(
        &self,
    ) -> Result<Vec<ent_types::personnels::AgentResumeDto>, ApiError> {
        self.appel(Methode::Get, "/personnels/annuaire?limite=200", None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_les_parametres() {
        assert_eq!(encoder("Rome 2027"), "Rome%202027");
        assert_eq!(encoder("a&b=c"), "a%26b%3Dc");
        assert_eq!(encoder("é"), "%C3%A9");
        assert_eq!(encoder("simple-ok_1.~"), "simple-ok_1.~");
    }
}
