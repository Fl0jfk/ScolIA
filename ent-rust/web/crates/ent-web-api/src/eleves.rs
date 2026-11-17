//! Service « élèves » (socle 2, lot E9) : liste et recherche, synthèse de la fiche (tuiles et
//! pastilles d'E8), historique d'identité, identifiants, consultations. Routes `/eleves/*` d'`ent-api` ;
//! les droits, le masquage et le cloisonnement (élève protégé compris) sont décidés par le serveur,
//! ce client transporte.

use ent_types::eleve_protection::ConsultationDto;
use ent_types::eleve_synthese::{CompteurPastilleDto, EleveFicheDto, EleveListeDto, SyntheseDto};
use ent_types::identifiants::IdentifiantDto;
use ent_types::identite::IdentiteVersionDto;
use uuid::Uuid;

use crate::communication::encoder_requete;
use crate::{ApiClient, ApiError, Methode};

/// Filtres de la liste des élèves. Une valeur vide n'est pas envoyée.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FiltreListeEleves {
    pub q: String,
    pub classe: String,
    /// `actif`, `preinscrit`, `sorti` ou vide (tous).
    pub statut: String,
    /// Code d'une pastille du catalogue ou vide.
    pub pastille: String,
    pub limite: u32,
    pub decalage: u32,
}

impl FiltreListeEleves {
    /// Chemin de la requête (`/eleves/liste?…`), valeurs encodées.
    pub fn chemin(&self) -> String {
        let mut p: Vec<String> = Vec::new();
        for (nom, valeur) in [
            ("q", self.q.trim()),
            ("classe", self.classe.trim()),
            ("statut", self.statut.trim()),
            ("pastille", self.pastille.trim()),
        ] {
            if !valeur.is_empty() {
                p.push(format!("{nom}={}", encoder_requete(valeur)));
            }
        }
        if self.limite > 0 {
            p.push(format!("limite={}", self.limite));
        }
        if self.decalage > 0 {
            p.push(format!("decalage={}", self.decalage));
        }
        if p.is_empty() {
            "/eleves/liste".to_string()
        } else {
            format!("/eleves/liste?{}", p.join("&"))
        }
    }
}

/// Adresse de la photo d'un élève (GET avec le cookie de session ; 404 si absente ou masquée).
pub fn adresse_photo_eleve(eleve: Uuid) -> String {
    format!("/eleves/{eleve}/photo")
}

impl ApiClient {
    /// `GET /eleves/liste` : une page de la liste des élèves visibles du lecteur (personnel).
    pub async fn eleves_liste(&self, f: &FiltreListeEleves) -> Result<EleveListeDto, ApiError> {
        self.appel(Methode::Get, &f.chemin(), None).await
    }

    /// `GET /eleves/pastilles` : combien d'élèves par pastille (les pastilles que le lecteur peut voir).
    pub async fn eleves_compteurs_pastilles(&self) -> Result<Vec<CompteurPastilleDto>, ApiError> {
        self.appel(Methode::Get, "/eleves/pastilles", None).await
    }

    /// `GET /eleves/{id}` : l'élève minimisé selon les droits du lecteur (404 s'il n'existe pas pour lui).
    pub async fn eleve_fiche(&self, eleve: Uuid) -> Result<EleveFicheDto, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}"), None).await
    }

    /// `GET /eleves/{id}/synthese` : pastilles visibles et tuiles par domaine (masquées avec leur raison).
    pub async fn eleve_synthese(&self, eleve: Uuid) -> Result<SyntheseDto, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}/synthese"), None).await
    }

    /// `GET /eleves/{id}/identite/historique` : versions d'identité visibles du lecteur.
    pub async fn eleve_historique_identite(&self, eleve: Uuid) -> Result<Vec<IdentiteVersionDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}/identite/historique"), None).await
    }

    /// `GET /eleves/{id}/identifiants` : identifiants externes (INE, badge…) visibles du lecteur.
    pub async fn eleve_identifiants(&self, eleve: Uuid) -> Result<Vec<IdentifiantDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}/identifiants"), None).await
    }

    /// `GET /eleves/{id}/consultations` : qui a consulté ce dossier (audit, ajout seul).
    pub async fn eleve_consultations(&self, eleve: Uuid) -> Result<Vec<ConsultationDto>, ApiError> {
        self.appel(Methode::Get, &format!("/eleves/{eleve}/consultations?limite=50"), None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_chemin_n_envoie_que_les_filtres_renseignes_et_les_encode() {
        assert_eq!(FiltreListeEleves::default().chemin(), "/eleves/liste");
        let f = FiltreListeEleves {
            q: "  Léa & Co ".into(),
            classe: "6A".into(),
            statut: "".into(),
            pastille: "sans_ine".into(),
            limite: 50,
            decalage: 100,
        };
        assert_eq!(
            f.chemin(),
            "/eleves/liste?q=L%C3%A9a%20%26%20Co&classe=6A&pastille=sans_ine&limite=50&decalage=100"
        );
        // Rien d'injectable : un « & » ou un « = » saisi reste dans la valeur.
        let f = FiltreListeEleves { q: "a=b&statut=sorti".into(), ..Default::default() };
        assert_eq!(f.chemin(), "/eleves/liste?q=a%3Db%26statut%3Dsorti");
    }

    #[test]
    fn la_photo_se_demande_par_l_identifiant_seul() {
        let id = Uuid::nil();
        assert_eq!(adresse_photo_eleve(id), format!("/eleves/{id}/photo"));
    }
}
