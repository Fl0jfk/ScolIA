//! Agenda (socle 19, lot AG1) : routes `/agenda/*` d'`ent-api/src/agenda.rs`. Les droits, la place du
//! lecteur (R-AG5) et l'audience sous les droits de l'auteur (R-AG8) sont décidés par le serveur.

use chrono::NaiveDate;
use ent_types::agenda::{
    AnnulationEvenement, AnnulationReservation, ApercuPublication, CreneauSemaine,
    DemandePublication, EvenementCree, EvenementDetail, ModifEvenement, NouvelEvenement,
    NouvelleReservation, PlanningSalle, RapportReservation, RechercheSalle, ReponseCoupon,
    ReponseInvitation, ReservationVue, SalleDispo, VueAgenda,
};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /agenda?du=&au=&sources=&eleve_id=` : mon agenda sur une plage de 42 jours au plus.
    pub async fn agenda_vue(
        &self,
        du: NaiveDate,
        au: NaiveDate,
        sources: &[&str],
        eleve_id: Option<Uuid>,
    ) -> Result<VueAgenda, ApiError> {
        let mut chemin = format!("/agenda?du={du}&au={au}");
        if !sources.is_empty() {
            chemin.push_str(&format!("&sources={}", sources.join(",")));
        }
        if let Some(e) = eleve_id {
            chemin.push_str(&format!("&eleve_id={e}"));
        }
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `GET /agenda/evenements/{id}`.
    pub async fn agenda_evenement(&self, id: Uuid) -> Result<EvenementDetail, ApiError> {
        self.appel(Methode::Get, &format!("/agenda/evenements/{id}"), None)
            .await
    }

    /// `POST /agenda/evenements` : crée un brouillon.
    pub async fn agenda_creer_evenement(
        &self,
        n: &NouvelEvenement,
    ) -> Result<EvenementCree, ApiError> {
        self.appel(Methode::Post, "/agenda/evenements", Some(corps(n)?))
            .await
    }

    /// `PUT /agenda/evenements/{id}`.
    pub async fn agenda_modifier_evenement(
        &self,
        id: Uuid,
        m: &ModifEvenement,
    ) -> Result<EvenementDetail, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/agenda/evenements/{id}"),
            Some(corps(m)?),
        )
        .await
    }

    /// `POST /agenda/evenements/{id}/apercu-publication` : combien de personnes, sans rien écrire.
    pub async fn agenda_apercu_publication(
        &self,
        id: Uuid,
        d: &DemandePublication,
    ) -> Result<ApercuPublication, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/agenda/evenements/{id}/apercu-publication"),
            Some(corps(d)?),
        )
        .await
    }

    /// `POST /agenda/evenements/{id}/publier` (avec l'empreinte de l'aperçu confirmé).
    pub async fn agenda_publier(
        &self,
        id: Uuid,
        d: &DemandePublication,
    ) -> Result<EvenementDetail, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/agenda/evenements/{id}/publier"),
            Some(corps(d)?),
        )
        .await
    }

    /// `POST /agenda/evenements/{id}/annuler`.
    pub async fn agenda_annuler(
        &self,
        id: Uuid,
        a: &AnnulationEvenement,
    ) -> Result<EvenementDetail, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/agenda/evenements/{id}/annuler"),
            Some(corps(a)?),
        )
        .await
    }

    /// `POST /agenda/evenements/{id}/reponse`.
    pub async fn agenda_repondre(
        &self,
        id: Uuid,
        r: &ReponseInvitation,
    ) -> Result<EvenementDetail, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/agenda/evenements/{id}/reponse"),
            Some(corps(r)?),
        )
        .await
    }

    /// `POST /agenda/evenements/{id}/coupon`.
    pub async fn agenda_repondre_coupon(
        &self,
        id: Uuid,
        r: &ReponseCoupon,
    ) -> Result<EvenementDetail, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/agenda/evenements/{id}/coupon"),
            Some(corps(r)?),
        )
        .await
    }

    /// `POST /agenda/semaine` : ajoute un créneau à la feuille de semaine (`agenda.semaine.gerer`).
    pub async fn agenda_semaine_ajouter(
        &self,
        c: &CreneauSemaine,
    ) -> Result<EvenementCree, ApiError> {
        self.appel(Methode::Post, "/agenda/semaine", Some(corps(c)?))
            .await
    }
}

impl ApiClient {
    /// `POST /agenda/lieux/recherche` : salles libres sur un créneau (règle, emploi du temps, réservations).
    pub async fn agenda_rechercher_salles(
        &self,
        q: &RechercheSalle,
    ) -> Result<Vec<SalleDispo>, ApiError> {
        self.appel(Methode::Post, "/agenda/lieux/recherche", Some(corps(q)?))
            .await
    }

    /// `GET /agenda/lieux/{id}/planning?du=&au=` : plage de 42 jours au plus.
    pub async fn agenda_planning_salle(
        &self,
        salle: Uuid,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<PlanningSalle, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/agenda/lieux/{salle}/planning?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `GET /agenda/reservations?du=&au=` : mes réservations.
    pub async fn agenda_mes_reservations(
        &self,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<ReservationVue>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/agenda/reservations?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `POST /agenda/reservations` : réserve (unitaire ou série) ; conflits dits par occurrence.
    pub async fn agenda_reserver_lieu(
        &self,
        n: &NouvelleReservation,
    ) -> Result<RapportReservation, ApiError> {
        self.appel(Methode::Post, "/agenda/reservations", Some(corps(n)?))
            .await
    }

    /// `POST /agenda/reservations/{id}/annuler`.
    pub async fn agenda_annuler_reservation(
        &self,
        id: Uuid,
        a: &AnnulationReservation,
    ) -> Result<usize, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/agenda/reservations/{id}/annuler"),
            Some(corps(a)?),
        )
        .await
    }
}
