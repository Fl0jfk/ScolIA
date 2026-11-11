//! Séries hebdomadaires de sorties (socle 42) : routes `/voyages/series`. Le calcul des dates (jours
//! fériés et vacances écartés) et le droit de valider, séance par séance, sont ceux du serveur.

use ent_types::voyages::{ResultatSeanceDto, ResultatSerieDto, SerieSaisieDto};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

impl ApiClient {
    /// `POST /voyages/series` : crée les séances d'une série hebdomadaire (brouillons) et renvoie les
    /// séances créées et les dates écartées.
    pub async fn voyage_serie_creer(
        &self,
        serie: &SerieSaisieDto,
    ) -> Result<ResultatSerieDto, ApiError> {
        let corps = serde_json::to_value(serie).map_err(|e| ApiError::Decodage(e.to_string()))?;
        self.appel(Methode::Post, "/voyages/series", Some(corps))
            .await
    }

    /// `POST /voyages/series/{serie_id}/valider` : valide la pédagogie de toutes les séances ; le droit
    /// est contrôlé séance par séance, le résultat dit lesquelles n'ont pas pu l'être.
    pub async fn voyage_serie_valider(
        &self,
        serie_id: Uuid,
    ) -> Result<Vec<ResultatSeanceDto>, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/voyages/series/{serie_id}/valider"),
            None,
        )
        .await
    }
}
