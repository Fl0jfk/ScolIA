//! Assiduité (socle 12, AS14) : appel v2, épisodes d'absence, qualification, justificatifs, avis aux
//! familles, vue de la famille. Routes de `ent-api/src/assiduite.rs` et `assiduite_familles.rs` ; les
//! droits (`appel.faire`, `absences.lire_motif`, `absences.qualifier`, `assiduite.suivre`, lien de
//! famille…) sont décidés par le serveur, qui répond de la même façon à « interdit » et « inexistant ».
//! Les types JSON sont ceux d'`ent-types::assiduite`.

use chrono::NaiveDate;
use ent_types::assiduite::{
    ApercuMasseDto, AppelV2Dto, AvisDemiJourneeDto, BilanMasseDto, CommunicationFamilleDto,
    ContactDto, CorrigerLigneBody, CourrierDto, DeposerJustificatifBody, DetailEpisodeDto,
    EpisodeDto, EpisodeSansReponseDto, EpisodeVueFamilleDto, EpisodeVueProfesseurDto,
    HistoLigneDto, IncoherenceV2Dto, JustificatifDto, LienJustificatifDto, MotifAsdDto,
    NatureMotifDto, ParamFamilleDto, PointerLigneBody, QualifierEpisodeBody, RectificationDto,
    RequalifierEpisodeBody, SelectionMasseBody, SituationAccueilDto, ValiderAppelBody,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

/// Filtre de `GET /assiduite/episodes` (valeurs de la liste fermée du serveur, sans encodage).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FiltreEpisodesApi {
    pub eleve_id: Option<Uuid>,
    pub depuis: Option<NaiveDate>,
    pub jusqu_au: Option<NaiveDate>,
    /// `a_qualifier` (défaut serveur), `qualifie`, `retire`, `tous`.
    pub etat: Option<&'static str>,
    pub avec_justificatif: Option<bool>,
    pub limite: Option<u32>,
}

impl FiltreEpisodesApi {
    /// Chaîne de requête (avec le « ? » initial) ; vide sans paramètre.
    pub fn requete(&self) -> String {
        let mut p: Vec<String> = Vec::new();
        if let Some(e) = self.eleve_id {
            p.push(format!("eleve_id={e}"));
        }
        if let Some(d) = self.depuis {
            p.push(format!("depuis={d}"));
        }
        if let Some(d) = self.jusqu_au {
            p.push(format!("jusqu_au={d}"));
        }
        if let Some(e) = self.etat {
            p.push(format!("etat={e}"));
        }
        if let Some(j) = self.avec_justificatif {
            p.push(format!("avec_justificatif={j}"));
        }
        if let Some(l) = self.limite {
            p.push(format!("limite={l}"));
        }
        if p.is_empty() {
            String::new()
        } else {
            format!("?{}", p.join("&"))
        }
    }
}

impl ApiClient {
    // ── Appel v2 ─────────────────────────────────────────────────────────────────────────────

    /// `GET /appels/{id}/v2` : lignes pré-remplies, récapitulatif, état.
    pub async fn appel_v2(&self, appel_id: Uuid) -> Result<AppelV2Dto, ApiError> {
        self.appel(Methode::Get, &format!("/appels/{appel_id}/v2"), None)
            .await
    }

    /// `POST /appels/{id}/v2/lignes` : pointe un élève (immédiat, historisé).
    pub async fn pointer_ligne(
        &self,
        appel_id: Uuid,
        saisie: &PointerLigneBody,
    ) -> Result<ent_types::assiduite::LigneAppelV2Dto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/appels/{appel_id}/v2/lignes"),
            Some(corps(saisie)?),
        )
        .await
    }

    /// `POST /appels/{id}/v2/valider` : « Appel fait ».
    pub async fn valider_appel_v2(
        &self,
        appel_id: Uuid,
        confirme_non_pointes: bool,
    ) -> Result<AppelV2Dto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/appels/{appel_id}/v2/valider"),
            Some(corps(&ValiderAppelBody {
                confirme_non_pointes,
            })?),
        )
        .await
    }

    /// `POST /appels/{id}/v2/corrections` : correction d'une ligne (l'intervenant le jour même).
    pub async fn corriger_ligne(
        &self,
        appel_id: Uuid,
        correction: &CorrigerLigneBody,
    ) -> Result<ent_types::assiduite::LigneAppelV2Dto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/appels/{appel_id}/v2/corrections"),
            Some(corps(correction)?),
        )
        .await
    }

    /// `GET /appels/{id}/lignes/{eleve}/historique`.
    pub async fn historique_ligne(
        &self,
        appel_id: Uuid,
        eleve_id: Uuid,
    ) -> Result<Vec<HistoLigneDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/appels/{appel_id}/lignes/{eleve_id}/historique"),
            None,
        )
        .await
    }

    /// `GET /appels/{id}/incoherences`.
    pub async fn incoherences_appel(
        &self,
        appel_id: Uuid,
    ) -> Result<Vec<IncoherenceV2Dto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/appels/{appel_id}/incoherences"),
            None,
        )
        .await
    }

    // ── Épisodes, qualification, justificatifs ───────────────────────────────────────────────

    /// `GET /assiduite/episodes` (vie scolaire : `absences.lire_motif`).
    pub async fn episodes(&self, filtre: &FiltreEpisodesApi) -> Result<Vec<EpisodeDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/assiduite/episodes{}", filtre.requete()),
            None,
        )
        .await
    }

    /// `GET /assiduite/episodes/{id}` : épisode, historique, propositions.
    pub async fn episode(&self, id: Uuid) -> Result<DetailEpisodeDto, ApiError> {
        self.appel(Methode::Get, &format!("/assiduite/episodes/{id}"), None)
            .await
    }

    /// `POST /assiduite/episodes/{id}/qualification` (`absences.qualifier`).
    pub async fn qualifier_episode(
        &self,
        id: Uuid,
        q: &QualifierEpisodeBody,
    ) -> Result<EpisodeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/assiduite/episodes/{id}/qualification"),
            Some(corps(q)?),
        )
        .await
    }

    /// `POST /assiduite/episodes/masse/apercu` : rien n'est écrit, rend l'empreinte à confirmer.
    pub async fn apercu_qualification_masse(
        &self,
        s: &SelectionMasseBody,
    ) -> Result<ApercuMasseDto, ApiError> {
        self.appel(
            Methode::Post,
            "/assiduite/episodes/masse/apercu",
            Some(corps(s)?),
        )
        .await
    }

    /// `POST /assiduite/episodes/masse` : seulement avec l'empreinte de l'aperçu.
    pub async fn qualifier_masse(&self, s: &SelectionMasseBody) -> Result<BilanMasseDto, ApiError> {
        self.appel(Methode::Post, "/assiduite/episodes/masse", Some(corps(s)?))
            .await
    }

    /// `GET /assiduite/mes-episodes?jour=` : vue du professeur (régularisation seulement, R-AS21).
    pub async fn mes_episodes(
        &self,
        depuis: Option<NaiveDate>,
    ) -> Result<Vec<EpisodeVueProfesseurDto>, ApiError> {
        let chemin = match depuis {
            Some(d) => format!("/assiduite/mes-episodes?jour={d}"),
            None => "/assiduite/mes-episodes".to_string(),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `GET /assiduite/eleves/{id}/episodes` : vue de la famille ou de l'élève majeur (R-AS22).
    pub async fn episodes_famille(
        &self,
        eleve_id: Uuid,
    ) -> Result<Vec<EpisodeVueFamilleDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/assiduite/eleves/{eleve_id}/episodes"),
            None,
        )
        .await
    }

    /// `GET /assiduite/episodes/{id}/justificatifs`.
    pub async fn justificatifs_episode(&self, id: Uuid) -> Result<Vec<JustificatifDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/assiduite/episodes/{id}/justificatifs"),
            None,
        )
        .await
    }

    /// `POST /assiduite/episodes/{id}/justificatifs` : texte seul (la pièce passe par le corps complet).
    pub async fn deposer_justificatif(
        &self,
        id: Uuid,
        depot: &DeposerJustificatifBody,
    ) -> Result<JustificatifDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/assiduite/episodes/{id}/justificatifs"),
            Some(corps(depot)?),
        )
        .await
    }

    /// `POST /assiduite/episodes/{id}/justificatifs` avec une pièce (contenu en base64).
    pub async fn deposer_piece_justificative(
        &self,
        id: Uuid,
        depot: &DeposerJustificatifBody,
        nom: &str,
        mime_type: &str,
        contenu_base64: &str,
    ) -> Result<JustificatifDto, ApiError> {
        let mut c = corps(depot)?;
        if let Value::Object(m) = &mut c {
            m.insert(
                "fichier".into(),
                serde_json::json!({
                    "nom": nom,
                    "mime_type": mime_type,
                    "contenu_base64": contenu_base64,
                }),
            );
        }
        self.appel(
            Methode::Post,
            &format!("/assiduite/episodes/{id}/justificatifs"),
            Some(c),
        )
        .await
    }

    /// `POST /assiduite/episodes/{id}/justificatifs/{jid}/lien` : lien pré-signé court (lecture journalisée).
    pub async fn lien_justificatif(
        &self,
        id: Uuid,
        justificatif_id: Uuid,
    ) -> Result<LienJustificatifDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/assiduite/episodes/{id}/justificatifs/{justificatif_id}/lien"),
            Some(serde_json::json!({})),
        )
        .await
    }

    /// `GET /assiduite/motifs-famille` : motifs actifs et visibles pour la famille.
    pub async fn motifs_famille(&self) -> Result<Vec<MotifAsdDto>, ApiError> {
        self.appel(Methode::Get, "/assiduite/motifs-famille", None)
            .await
    }

    /// `GET /assiduite/natures-motif` : les cinq motifs de L131-8 et les natures de classement.
    pub async fn natures_motif(&self) -> Result<Vec<NatureMotifDto>, ApiError> {
        self.appel(Methode::Get, "/assiduite/natures-motif", None)
            .await
    }

    // ── Information des familles (AS5) ───────────────────────────────────────────────────────

    /// `GET /assiduite/eleves/{id}/avis?du=` : avis prévus, partis et démentis (`assiduite.suivre`).
    pub async fn avis_eleve(
        &self,
        eleve_id: Uuid,
        du: Option<NaiveDate>,
    ) -> Result<Vec<AvisDemiJourneeDto>, ApiError> {
        let chemin = match du {
            Some(d) => format!("/assiduite/eleves/{eleve_id}/avis?du={d}"),
            None => format!("/assiduite/eleves/{eleve_id}/avis"),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `GET /assiduite/familles/parametres`.
    pub async fn parametres_famille(&self) -> Result<ParamFamilleDto, ApiError> {
        self.appel(Methode::Get, "/assiduite/familles/parametres", None)
            .await
    }

    /// `GET /assiduite/relances/sans-reponse` : file « sans réponse » (J+5) et propositions (J+8).
    pub async fn sans_reponse(&self) -> Result<Vec<EpisodeSansReponseDto>, ApiError> {
        self.appel(Methode::Get, "/assiduite/relances/sans-reponse", None)
            .await
    }

    /// `GET /assiduite/eleves/{id}/communications` : ce que la famille a reçu de la vie scolaire.
    pub async fn communications_famille(
        &self,
        eleve_id: Uuid,
    ) -> Result<Vec<CommunicationFamilleDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/assiduite/eleves/{eleve_id}/communications"),
            None,
        )
        .await
    }

    // ── Accueil (AS-216) ─────────────────────────────────────────────────────────────────────

    /// `GET /assiduite/eleves/{id}/situation-accueil` : lecture unitaire journalisée côté serveur.
    pub async fn situation_accueil(&self, eleve_id: Uuid) -> Result<SituationAccueilDto, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/assiduite/eleves/{eleve_id}/situation-accueil"),
            None,
        )
        .await
    }

    // ── Dossier d'un élève (AS14d) ──────────────────────────────────────────────────────────

    /// `POST /assiduite/episodes/{id}/requalification` (`absences.qualifier`, motif codé obligatoire).
    pub async fn requalifier_episode(
        &self,
        id: Uuid,
        q: &RequalifierEpisodeBody,
    ) -> Result<EpisodeDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/assiduite/episodes/{id}/requalification"),
            Some(corps(q)?),
        )
        .await
    }

    /// `GET /assiduite/eleves/{id}/contacts` (`assiduite.suivre`).
    pub async fn contacts_eleve(&self, eleve_id: Uuid) -> Result<Vec<ContactDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/assiduite/eleves/{eleve_id}/contacts"),
            None,
        )
        .await
    }

    /// `GET /assiduite/eleves/{id}/courriers` (`assiduite.suivre`).
    pub async fn courriers_eleve(&self, eleve_id: Uuid) -> Result<Vec<CourrierDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/assiduite/eleves/{eleve_id}/courriers"),
            None,
        )
        .await
    }

    /// `GET /assiduite/mois/{mois}/rectifications` (`assiduite.signaler`) : celles de tout
    /// l'établissement pour le mois ; l'écran filtre l'élève.
    pub async fn rectifications_mois(
        &self,
        mois: NaiveDate,
    ) -> Result<Vec<RectificationDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/assiduite/mois/{mois}/rectifications"),
            None,
        )
        .await
    }

    /// `POST /assiduite/rectifications/{id}/vue` : « vue » une seule fois.
    pub async fn rectification_vue(&self, id: Uuid) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/assiduite/rectifications/{id}/vue"),
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtre_vide_sans_point_d_interrogation() {
        assert_eq!(FiltreEpisodesApi::default().requete(), "");
    }

    #[test]
    fn filtre_compose_les_parametres() {
        let f = FiltreEpisodesApi {
            etat: Some("tous"),
            avec_justificatif: Some(true),
            limite: Some(50),
            ..Default::default()
        };
        assert_eq!(f.requete(), "?etat=tous&avec_justificatif=true&limite=50");
    }
}
