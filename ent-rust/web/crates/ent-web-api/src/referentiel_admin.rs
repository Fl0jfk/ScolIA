//! Administration du référentiel (socle 2, lot S2-2) : routes `/referentiel/*` d'`ent-api`
//! (années et bascule, sites, niveaux/MEF, périodes, calendrier, grille, séances, groupes).
//! Les droits (`referentiel.lire` / `referentiel.gerer`) sont vérifiés par le serveur.

use chrono::NaiveDate;
use ent_types::referentiel_admin::{
    AmorcageNiveauxDto, AnneeStatutDto, CorrespondanceDto, CreneauGrilleDto, CreneauGrilleSaisieDto,
    GroupeAdminDto, JourCalendrierDto, MefDto, MefImportDto, NiveauDto, PeriodeDto,
    PeriodeSaisieDto, PersonnelDto, PreparationDto, RapportImportMefDto, RapportLotDto,
    RapportOuvertureDto, RapportReconductionDto, RapportSeancesDto, SeanceDto, SiteDto,
};
use serde_json::json;
use uuid::Uuid;

use crate::communication::encoder_requete;
use crate::{ApiClient, ApiError, Methode};

fn vide(v: serde_json::Value) -> Option<serde_json::Value> {
    Some(v)
}

fn corps<T: serde::Serialize>(valeur: &T) -> Result<serde_json::Value, ApiError> {
    serde_json::to_value(valeur).map_err(|e| ApiError::Decodage(e.to_string()))
}

impl ApiClient {
    // ───── Années et bascule ─────

    /// `GET /referentiel/annees-statut`.
    pub async fn annees_statut(&self) -> Result<Vec<AnneeStatutDto>, ApiError> {
        self.appel(Methode::Get, "/referentiel/annees-statut", None)
            .await
    }

    /// `POST /referentiel/annees` : crée une année (non chevauchante).
    pub async fn creer_annee(
        &self,
        libelle: &str,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<serde_json::Value, ApiError> {
        let c = json!({ "libelle": libelle, "date_debut": debut, "date_fin": fin, "courante": false });
        self.appel(Methode::Post, "/referentiel/annees", vide(c))
            .await
    }

    /// `POST /referentiel/annees/{id}/preparer-suivante`.
    pub async fn preparer_annee_suivante(
        &self,
        id: Uuid,
        avec_services: bool,
    ) -> Result<PreparationDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/referentiel/annees/{id}/preparer-suivante"),
            vide(json!({ "avec_services": avec_services })),
        )
        .await
    }

    /// `POST /referentiel/annees/{id}/reconduire`.
    pub async fn reconduire_structures(
        &self,
        source: Uuid,
        cible: Uuid,
        avec_services: bool,
    ) -> Result<RapportReconductionDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/referentiel/annees/{source}/reconduire"),
            vide(json!({ "cible_id": cible, "avec_services": avec_services })),
        )
        .await
    }

    /// `POST /referentiel/annees/{cible}/monter` : aperçu (`apercu = true`) ou application.
    pub async fn monter_eleves(
        &self,
        cible: Uuid,
        source: Uuid,
        correspondances: &[CorrespondanceDto],
        apercu: bool,
    ) -> Result<RapportLotDto, ApiError> {
        let c = json!({ "source_id": source, "correspondances": correspondances, "apercu": apercu });
        self.appel(
            Methode::Post,
            &format!("/referentiel/annees/{cible}/monter"),
            vide(c),
        )
        .await
    }

    /// `POST /referentiel/annees/{id}/ouvrir`.
    pub async fn ouvrir_annee(&self, id: Uuid) -> Result<RapportOuvertureDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/referentiel/annees/{id}/ouvrir"),
            vide(json!({})),
        )
        .await
    }

    /// `POST /referentiel/annees/{id}/cloturer` (`forcer` : passe outre une période de notes ouverte, tracé).
    pub async fn cloturer_annee(&self, id: Uuid, forcer: bool) -> Result<AnneeStatutDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/referentiel/annees/{id}/cloturer"),
            vide(json!({ "forcer": forcer })),
        )
        .await
    }

    // ───── Périodes ─────

    /// `GET /referentiel/annees/{id}/periodes`.
    pub async fn periodes_annee(&self, id: Uuid) -> Result<Vec<PeriodeDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/referentiel/annees/{id}/periodes"),
            None,
        )
        .await
    }

    /// `PUT /referentiel/annees/{id}/periodes` : remplace les périodes (couverture sans trou).
    pub async fn definir_periodes(
        &self,
        id: Uuid,
        periodes: &[PeriodeSaisieDto],
    ) -> Result<Vec<PeriodeDto>, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/referentiel/annees/{id}/periodes"),
            Some(corps(&periodes)?),
        )
        .await
    }

    /// `POST /referentiel/annees/{id}/seances` : (re)génère les séances datées.
    pub async fn generer_seances(
        &self,
        id: Uuid,
        du: Option<NaiveDate>,
        au: Option<NaiveDate>,
    ) -> Result<RapportSeancesDto, ApiError> {
        self.appel(
            Methode::Post,
            &format!("/referentiel/annees/{id}/seances"),
            vide(json!({ "du": du, "au": au })),
        )
        .await
    }

    // ───── Sites, niveaux, MEF ─────

    /// `GET /referentiel/sites`.
    pub async fn sites(&self) -> Result<Vec<SiteDto>, ApiError> {
        self.appel(Methode::Get, "/referentiel/sites", None).await
    }

    /// `POST /referentiel/sites`.
    pub async fn creer_site(
        &self,
        code: &str,
        libelle: &str,
        adresse: Option<&str>,
        uai: Option<&str>,
    ) -> Result<SiteDto, ApiError> {
        let c = json!({ "code": code, "libelle": libelle, "adresse": adresse, "uai": uai });
        self.appel(Methode::Post, "/referentiel/sites", vide(c))
            .await
    }

    /// `PUT /referentiel/sites/{id}` : libellé, adresse, activation.
    pub async fn maj_site(
        &self,
        id: Uuid,
        libelle: Option<&str>,
        adresse: Option<&str>,
        actif: Option<bool>,
    ) -> Result<SiteDto, ApiError> {
        let c = json!({ "libelle": libelle, "adresse": adresse, "actif": actif });
        self.appel(Methode::Put, &format!("/referentiel/sites/{id}"), vide(c))
            .await
    }

    /// `GET /referentiel/niveaux`.
    pub async fn niveaux(&self) -> Result<Vec<NiveauDto>, ApiError> {
        self.appel(Methode::Get, "/referentiel/niveaux", None).await
    }

    /// `POST /referentiel/niveaux/amorcer` : nomenclature nationale + rattachement des classes (idempotent).
    pub async fn amorcer_niveaux(&self) -> Result<AmorcageNiveauxDto, ApiError> {
        self.appel(
            Methode::Post,
            "/referentiel/niveaux/amorcer",
            vide(json!({})),
        )
        .await
    }

    /// `GET /referentiel/mef`.
    pub async fn mefs(&self) -> Result<Vec<MefDto>, ApiError> {
        self.appel(Methode::Get, "/referentiel/mef", None).await
    }

    /// `POST /referentiel/mef/import`.
    pub async fn importer_mefs(
        &self,
        source: &str,
        lignes: &[MefImportDto],
    ) -> Result<RapportImportMefDto, ApiError> {
        self.appel(
            Methode::Post,
            "/referentiel/mef/import",
            vide(json!({ "source": source, "lignes": lignes })),
        )
        .await
    }

    // ───── Classes et groupes ─────

    /// `PUT /referentiel/classes/{id}/modifier` (réponse vide).
    pub async fn modifier_classe(
        &self,
        id: Uuid,
        site_id: Option<Uuid>,
        niveau_id: Option<Uuid>,
        mef_id: Option<Uuid>,
        archiver: Option<bool>,
    ) -> Result<(), ApiError> {
        let c = json!({ "site_id": site_id, "niveau_id": niveau_id, "mef_id": mef_id, "archiver": archiver });
        let _: serde_json::Value = self
            .appel(
                Methode::Put,
                &format!("/referentiel/classes/{id}/modifier"),
                vide(c),
            )
            .await?;
        Ok(())
    }

    /// `PUT /referentiel/classes/{id}/professeur-principal` (réponse vide).
    pub async fn designer_professeur_principal(
        &self,
        classe: Uuid,
        prof_user_id: Uuid,
    ) -> Result<(), ApiError> {
        let _: serde_json::Value = self
            .appel(
                Methode::Put,
                &format!("/referentiel/classes/{classe}/professeur-principal"),
                vide(json!({ "prof_user_id": prof_user_id })),
            )
            .await?;
        Ok(())
    }

    /// `GET /referentiel/groupes-admin` : groupes et ateliers de l'année avec leurs responsables.
    pub async fn groupes_admin(
        &self,
        annee_id: Option<Uuid>,
    ) -> Result<Vec<GroupeAdminDto>, ApiError> {
        let chemin = match annee_id {
            Some(a) => format!("/referentiel/groupes-admin?annee_id={a}"),
            None => "/referentiel/groupes-admin".to_string(),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `POST /referentiel/groupes` : groupe d'une classe, ou atelier transverse si `classe_id` est nul.
    pub async fn creer_groupe(
        &self,
        code: &str,
        libelle: Option<&str>,
        classe_id: Option<Uuid>,
        annee_id: Option<Uuid>,
    ) -> Result<serde_json::Value, ApiError> {
        let c = json!({ "code": code, "libelle": libelle, "classe_id": classe_id, "annee_id": annee_id });
        self.appel(Methode::Post, "/referentiel/groupes", vide(c))
            .await
    }

    /// `PUT /referentiel/groupes/{id}/responsables` (droit `roles.gerer`).
    pub async fn definir_responsables_groupe(
        &self,
        groupe: Uuid,
        responsables: &[Uuid],
    ) -> Result<Vec<Uuid>, ApiError> {
        self.appel(
            Methode::Put,
            &format!("/referentiel/groupes/{groupe}/responsables"),
            vide(json!({ "responsables": responsables })),
        )
        .await
    }

    /// `GET /personnel/search?q=` : personnel de l'établissement (choix d'un responsable).
    pub async fn chercher_personnel(&self, q: &str) -> Result<Vec<PersonnelDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/personnel/search?q={}", encoder_requete(q.trim())),
            None,
        )
        .await
    }

    // ───── Calendrier ─────

    /// `GET /referentiel/calendrier?du=&au=`.
    pub async fn calendrier(
        &self,
        du: NaiveDate,
        au: NaiveDate,
    ) -> Result<Vec<JourCalendrierDto>, ApiError> {
        self.appel(
            Methode::Get,
            &format!("/referentiel/calendrier?du={du}&au={au}"),
            None,
        )
        .await
    }

    /// `POST /referentiel/calendrier/jours` : neutralise une plage (férié, fermeture, journée banalisée).
    pub async fn ajouter_jours_fermes(
        &self,
        du: NaiveDate,
        au: NaiveDate,
        kind: &str,
        libelle: &str,
        site_id: Option<Uuid>,
    ) -> Result<i64, ApiError> {
        let c = json!({ "du": du, "au": au, "kind": kind, "libelle": libelle, "site_id": site_id });
        let r: serde_json::Value = self
            .appel(Methode::Post, "/referentiel/calendrier/jours", vide(c))
            .await?;
        Ok(r.get("ajoutes").and_then(|v| v.as_i64()).unwrap_or(0))
    }

    /// `DELETE /referentiel/calendrier/jours?jour=` (réponse vide).
    pub async fn retirer_jour_ferme(
        &self,
        jour: NaiveDate,
        site_id: Option<Uuid>,
    ) -> Result<(), ApiError> {
        let chemin = match site_id {
            Some(s) => format!("/referentiel/calendrier/jours?jour={jour}&site_id={s}"),
            None => format!("/referentiel/calendrier/jours?jour={jour}"),
        };
        let _: serde_json::Value = self.appel(Methode::Delete, &chemin, None).await?;
        Ok(())
    }

    /// `POST /referentiel/calendrier/vacances`.
    pub async fn creer_vacances(
        &self,
        libelle: &str,
        kind: &str,
        debut: NaiveDate,
        fin: NaiveDate,
    ) -> Result<(), ApiError> {
        let c = json!({ "libelle": libelle, "kind": kind, "debut": debut, "fin": fin });
        let _: serde_json::Value = self
            .appel(Methode::Post, "/referentiel/calendrier/vacances", vide(c))
            .await?;
        Ok(())
    }

    // ───── Grille horaire et séances ─────

    /// `GET /referentiel/grille`.
    pub async fn grille(&self, site_id: Option<Uuid>) -> Result<Vec<CreneauGrilleDto>, ApiError> {
        let chemin = match site_id {
            Some(s) => format!("/referentiel/grille?site_id={s}"),
            None => "/referentiel/grille".to_string(),
        };
        self.appel(Methode::Get, &chemin, None).await
    }

    /// `PUT /referentiel/grille` : remplace la grille.
    pub async fn definir_grille(
        &self,
        site_id: Option<Uuid>,
        creneaux: &[CreneauGrilleSaisieDto],
    ) -> Result<Vec<CreneauGrilleDto>, ApiError> {
        let chemin = match site_id {
            Some(s) => format!("/referentiel/grille?site_id={s}"),
            None => "/referentiel/grille".to_string(),
        };
        self.appel(Methode::Put, &chemin, Some(corps(&creneaux)?))
            .await
    }

    /// `POST /referentiel/grille/amorcer` : grille type M1…M4 / S1…S4.
    pub async fn amorcer_grille(&self) -> Result<Vec<CreneauGrilleDto>, ApiError> {
        self.appel(
            Methode::Post,
            "/referentiel/grille/amorcer",
            vide(json!({})),
        )
        .await
    }

    /// `GET /referentiel/seances?du=&au=`.
    pub async fn seances_datees(
        &self,
        du: NaiveDate,
        au: NaiveDate,
        classe_id: Option<Uuid>,
    ) -> Result<Vec<SeanceDto>, ApiError> {
        let mut chemin = format!("/referentiel/seances?du={du}&au={au}");
        if let Some(c) = classe_id {
            chemin.push_str(&format!("&classe_id={c}"));
        }
        self.appel(Methode::Get, &chemin, None).await
    }
}
