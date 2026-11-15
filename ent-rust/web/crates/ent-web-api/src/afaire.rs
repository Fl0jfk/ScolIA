//! À-faire unifié (socle 29, lots P2 à P4) : routes `/afaire*` d'`ent-api`.
//! Le serveur revérifie chaque tâche à la lecture et chaque droit à l'exécution ; ce client transporte.

use ent_types::afaire::{
    AFaire, ApercuAction, CorpsPrise, CorpsReattribution, ExecutionAction, FileAfaire, MesFiles,
    NiveauPrise, Orpheline, ResultatAction, Supervision,
};
use ent_types::recherche::{BlocageRecherche, CorpsDeblocageSoi, ReponseRecherche};
use uuid::Uuid;

use crate::{ApiClient, ApiError, Methode};

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    /// `GET /afaire` : l'à-faire de la personne connectée (jamais mis en cache).
    pub async fn a_faire(&self) -> Result<AFaire, ApiError> {
        self.appel(Methode::Get, "/afaire", None).await
    }

    /// `POST /afaire/{id}/apercu` : aperçu de la carte d'action (aucune écriture).
    pub async fn a_faire_apercu(&self, tache: Uuid) -> Result<ApercuAction, ApiError> {
        self.appel(Methode::Post, &format!("/afaire/{tache}/apercu"), None)
            .await
    }

    /// `POST /afaire/{id}/executer` : exécute la commande du propriétaire après confirmation.
    /// Une démarche déjà traitée répond `409` avec toujours le même texte.
    pub async fn a_faire_executer(
        &self,
        tache: Uuid,
        motif: Option<String>,
        decision: Option<String>,
    ) -> Result<ResultatAction, ApiError> {
        let c = corps(&ExecutionAction {
            confirme: true,
            motif,
            decision,
        })?;
        self.appel(Methode::Post, &format!("/afaire/{tache}/executer"), Some(c))
            .await
    }

    /// `GET /afaire/files` : « Mes files » (lot P4) : des nombres par file, jamais de contenu.
    pub async fn mes_files(&self) -> Result<MesFiles, ApiError> {
        self.appel(Methode::Get, "/afaire/files", None).await
    }

    /// `GET /afaire/files/{code}` : le contenu d'une file de l'index pour l'un de ses membres.
    pub async fn file_afaire(&self, code: &str) -> Result<FileAfaire, ApiError> {
        self.appel(Methode::Get, &format!("/afaire/files/{code}"), None)
            .await
    }

    /// `POST /afaire/{id}/prendre` : prendre en charge. `409` « déjà prise » si un collègue a été plus rapide.
    pub async fn a_faire_prendre(&self, tache: Uuid, niveau: NiveauPrise) -> Result<(), ApiError> {
        let c = corps(&CorpsPrise {
            niveau: Some(niveau),
        })?;
        self.appel(Methode::Post, &format!("/afaire/{tache}/prendre"), Some(c))
            .await
    }

    /// `POST /afaire/{id}/relacher` : relâcher sa prise.
    pub async fn a_faire_relacher(&self, tache: Uuid) -> Result<(), ApiError> {
        self.appel(Methode::Post, &format!("/afaire/{tache}/relacher"), None)
            .await
    }

    /// `POST /afaire/{id}/preparee` : remettre une tâche préparée à celui qui décide.
    pub async fn a_faire_preparee(&self, tache: Uuid) -> Result<(), ApiError> {
        self.appel(Methode::Post, &format!("/afaire/{tache}/preparee"), None)
            .await
    }

    /// `POST /afaire/{id}/reattribuer` : un responsable de service remet la tâche à un collègue de la file.
    pub async fn a_faire_reattribuer(
        &self,
        tache: Uuid,
        vers: Uuid,
        niveau: NiveauPrise,
    ) -> Result<(), ApiError> {
        let c = corps(&CorpsReattribution {
            vers_compte_id: vers,
            niveau: Some(niveau),
        })?;
        self.appel(
            Methode::Post,
            &format!("/afaire/{tache}/reattribuer"),
            Some(c),
        )
        .await
    }

    /// `GET /afaire/supervision` : nombres par file et par nature (direction), jamais un nom.
    pub async fn a_faire_supervision(&self) -> Result<Supervision, ApiError> {
        self.appel(Methode::Get, "/afaire/supervision", None).await
    }

    /// `GET /afaire/orphelines` : tâches sans destinataire servi (titre neutre seulement).
    pub async fn a_faire_orphelines(&self) -> Result<Vec<Orpheline>, ApiError> {
        self.appel(Methode::Get, "/afaire/orphelines", None).await
    }

    /// `POST /afaire/orphelines/{id}/sans-objet` : clore une orpheline après vérification du propriétaire.
    pub async fn a_faire_clore_orpheline(&self, tache: Uuid) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/afaire/orphelines/{tache}/sans-objet"),
            None,
        )
        .await
    }

    /// `GET /recherche?q=` (lot P5) : recherche globale côté serveur, 10 résultats par source, 40 au total.
    /// `429` quand la recherche est suspendue (texte fixe, aucun chiffre).
    pub async fn rechercher(&self, q: &str) -> Result<ReponseRecherche, ApiError> {
        let chemin = format!(
            "/recherche?q={}",
            crate::communication::encoder_requete(q.trim())
        );
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `GET /recherche/blocages` : qui est bloqué aujourd'hui (administrateur).
    pub async fn recherche_blocages(&self) -> Result<Vec<BlocageRecherche>, ApiError> {
        self.appel(Methode::Get, "/recherche/blocages", None).await
    }

    /// `POST /recherche/{compte}/debloquer` : lever un blocage (`recherche.debloquer`).
    pub async fn recherche_debloquer(&self, compte: Uuid) -> Result<(), ApiError> {
        self.appel(
            Methode::Post,
            &format!("/recherche/{compte}/debloquer"),
            None,
        )
        .await
    }

    /// `POST /recherche/debloquer-moi` : se débloquer soi-même, une fois par jour, motif de liste fermée.
    pub async fn recherche_debloquer_moi(&self, motif: &str) -> Result<(), ApiError> {
        let c = corps(&CorpsDeblocageSoi {
            motif: motif.to_string(),
        })?;
        self.appel(Methode::Post, "/recherche/debloquer-moi", Some(c))
            .await
    }
}
