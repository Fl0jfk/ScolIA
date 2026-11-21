//! Service emploi du temps : routes existantes d'`ent-api`.

use chrono::NaiveDate;

use crate::{ApiClient, ApiError, Creneau, Methode};

impl ApiClient {
    /// `GET /edt/mes-creneaux?jour=` : séances de la personne connectée ce jour-là
    /// (remplacements compris, filtrés par le serveur selon le rôle).
    pub async fn mes_creneaux(&self, jour: NaiveDate) -> Result<Vec<Creneau>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/edt/mes-creneaux?jour={jour}"),
            None,
        )
        .await
    }
}
