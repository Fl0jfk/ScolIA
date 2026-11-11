//! Sorties et voyages scolaires (socle 42, lot V0a) : routes `/voyages/*` d'`ent-api`. Les droits,
//! les états et les préconditions sont vérifiés par le serveur ; ce client n'en suppose aucun.

use ent_types::voyages::{
    BudgetDto, CandidatsClasseDto, CirculaireDto, DocumentDto, EncadrantDto, EtapeDto,
    EtapeSaisieDto, JournalDto, LigneBudgetSaisieDto, ParamsBudgetDto, ParamsCommandeDto,
    ParticipantDto, RappelDto, ResultatAjoutDto, SortieDetailDto, SortieDto, SortieModifDto,
    SortieResumeDto, SortieSaisieDto, TableauDirectionDto, VoyageFamilleDto,
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

// ───────────────────────── Lot V0b : budget, prix annoncé, documents, rappels ─────────────────────────

impl ApiClient {
    /// `GET /voyages/{id}/budget` : versions, lignes, calcul de la part des familles, prix annoncé.
    pub async fn voyage_budget(&self, id: Uuid) -> Result<BudgetDto, ApiError> {
        self.appel(Methode::Get, &format!("/voyages/{id}/budget"), None)
            .await
    }

    /// `POST /voyages/{id}/budget/versions` : version suivante (révision motivée si une version est validée).
    pub async fn voyage_budget_version_creer(
        &self,
        id: Uuid,
        motif: Option<&str>,
    ) -> Result<BudgetDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/voyages/{id}/budget/versions"),
            Some(json!({ "motif": motif })),
        )
        .await
    }

    /// `DELETE /voyages/{id}/budget/versions/{numero}?rev=` : abandonne le brouillon.
    pub async fn voyage_budget_abandonner(
        &self,
        id: Uuid,
        numero: i32,
        rev: i32,
    ) -> Result<BudgetDto, ApiError> {
        self.appel(
            Methode::Delete,
            &format!("/voyages/{id}/budget/versions/{numero}?rev={rev}"),
            None,
        )
        .await
    }

    /// `PUT /voyages/{id}/budget/versions/{numero}` : effectif payant et marge de sécurité.
    pub async fn voyage_budget_parametrer(
        &self,
        id: Uuid,
        numero: i32,
        params: &ParamsBudgetDto,
    ) -> Result<BudgetDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/voyages/{id}/budget/versions/{numero}"),
            Some(corps(params)?),
        )
        .await
    }

    /// `POST …/lignes` : ajoute une dépense ou une recette (montant au format français).
    pub async fn voyage_budget_ligne_ajouter(
        &self,
        id: Uuid,
        numero: i32,
        ligne: &LigneBudgetSaisieDto,
    ) -> Result<BudgetDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/voyages/{id}/budget/versions/{numero}/lignes"),
            Some(corps(ligne)?),
        )
        .await
    }

    /// `PUT …/lignes/{lid}` : modifie une ligne du brouillon.
    pub async fn voyage_budget_ligne_modifier(
        &self,
        id: Uuid,
        numero: i32,
        ligne_id: Uuid,
        ligne: &LigneBudgetSaisieDto,
    ) -> Result<BudgetDto, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/voyages/{id}/budget/versions/{numero}/lignes/{ligne_id}"),
            Some(corps(ligne)?),
        )
        .await
    }

    /// `DELETE …/lignes/{lid}?rev=` : supprime une ligne du brouillon.
    pub async fn voyage_budget_ligne_supprimer(
        &self,
        id: Uuid,
        numero: i32,
        ligne_id: Uuid,
        rev: i32,
    ) -> Result<BudgetDto, ApiError> {
        self.appel(
            Methode::Delete,
            &format!("/voyages/{id}/budget/versions/{numero}/lignes/{ligne_id}?rev={rev}"),
            None,
        )
        .await
    }

    /// `POST …/valider-revision` : valide la version n+1 d'un budget déjà validé.
    pub async fn voyage_budget_valider_revision(
        &self,
        id: Uuid,
        numero: i32,
        rev: i32,
    ) -> Result<BudgetDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/voyages/{id}/budget/versions/{numero}/valider-revision"),
            Some(json!({ "rev": rev })),
        )
        .await
    }

    /// `POST /voyages/{id}/prix/hausse` : décision de la direction d'augmenter le prix annoncé.
    pub async fn voyage_prix_hausser(&self, id: Uuid, motif: &str) -> Result<BudgetDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/voyages/{id}/prix/hausse"),
            Some(json!({ "motif": motif })),
        )
        .await
    }

    /// `POST /voyages/{id}/budget/pdf` : rend la fiche budget (version validée) en PDF.
    pub async fn voyage_budget_pdf(&self, id: Uuid) -> Result<DocumentDto, ApiError> {
        self.appel(Methode::Post, &format!("/voyages/{id}/budget/pdf"), None)
            .await
    }

    /// `POST /voyages/{id}/circulaire/pdf` : rend la circulaire aux familles en PDF.
    pub async fn voyage_circulaire_pdf(&self, id: Uuid) -> Result<DocumentDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/voyages/{id}/circulaire/pdf"),
            None,
        )
        .await
    }

    /// `GET /voyages/{id}/documents` : documents émis que l'on a le droit de voir.
    pub async fn voyage_documents(&self, id: Uuid) -> Result<Vec<DocumentDto>, ApiError> {
        self.appel(Methode::Get, &format!("/voyages/{id}/documents"), None)
            .await
    }

    /// `GET /voyages/{id}/rappels` : rappels déjà constatés pour l'organisateur.
    pub async fn voyage_rappels(&self, id: Uuid) -> Result<Vec<RappelDto>, ApiError> {
        self.appel(Methode::Get, &format!("/voyages/{id}/rappels"), None)
            .await
    }
}

impl ApiClient {
    /// `GET /voyages/{id}/documents/{doc}` : octets d'un document émis. Le serveur contrôle le droit
    /// et l'empreinte à chaque lecture : il n'y a aucune adresse publique du magasin d'objets.
    pub async fn voyage_document(&self, sortie: Uuid, document: Uuid) -> Result<Vec<u8>, ApiError> {
        let (statut, octets) = crate::transport::envoyer_octets(
            &self.url(&format!("/voyages/{sortie}/documents/{document}")),
            self.auth(),
        )
        .await?;
        if (200..300).contains(&statut) {
            Ok(octets)
        } else {
            Err(ApiError::depuis_statut(
                statut,
                String::from_utf8_lossy(&octets).into_owned(),
            ))
        }
    }
}

impl ApiClient {
    /// `GET /voyages/{id}/participants.csv` : export de la liste (trace d'accès écrite par le serveur,
    /// R-SV27 ; cellules neutralisées contre l'injection de formule).
    pub async fn voyage_participants_csv(&self, id: Uuid) -> Result<Vec<u8>, ApiError> {
        let (statut, octets) = crate::transport::envoyer_octets(
            &self.url(&format!("/voyages/{id}/participants.csv")),
            self.auth(),
        )
        .await?;
        if (200..300).contains(&statut) {
            Ok(octets)
        } else {
            Err(ApiError::depuis_statut(
                statut,
                String::from_utf8_lossy(&octets).into_owned(),
            ))
        }
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
