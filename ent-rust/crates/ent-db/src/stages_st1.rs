//! Socle 39 (stages), lot ST1 (a) : dispositifs, paramètres de l'établissement, organismes
//! (état, adresse, sites, contacts) et périodes (`docs/socles/39_STAGES.md` §9, §10, §19.1 à
//! §19.3).
//!
//! * **Droits D-017** : `stages.parametrer` (direction), `stages.organismes.gerer` (carnet),
//!   `stages.periodes.gerer` (périodes) ; lecture des dispositifs, des périodes et des paramètres
//!   avec `stages.lire` à l'établissement ou l'une des actions de gestion. Les **contacts** (des
//!   personnes extérieures) ne sont lisibles qu'avec `stages.organismes.gerer` : ni le professeur
//!   ni le CPE ne les voient (corrige DN-09, détail d'organisme ouvert à tout compte).
//! * **Une transaction** (`TenantTx`, RLS) par commande : le fait, la version, et l'événement
//!   `stages.*` (identifiants, codes et dates seulement, jamais un nom ni une coordonnée).
//! * **Verrou optimiste** : toute modification porte la `version` lue ; sinon refus `ConflitDeVersion`.

use chrono::NaiveDate;
use domain_eleve::{
    periode_stage, valider_contact, valider_etat_organisme, valider_organisme, valider_parametres,
    valider_periode, valider_publics, valider_site, ContactSaisie, DomainError, ModifierOrganisme,
    ModifierParametres, PeriodeSaisie, PublicPeriode, SiteSaisie, StageErreur,
};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use ent_types::stages::*;
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::stage_repo::{ecriture, refus_stage};
use crate::tenant_tx::TenantTx;

type R<T> = Result<T, EntDbError>;

const EV_PARAMETRES: &str = "stages.parametres_modifies";
const EV_ORGANISME_MODIFIE: &str = "stages.organisme_modifie";
const EV_ORGANISME_ETAT: &str = "stages.organisme_etat";
const EV_SITE: &str = "stages.site_modifie";
const EV_CONTACT: &str = "stages.contact_modifie";
const EV_PERIODE_PLANIFIEE: &str = "stages.periode_planifiee";
const EV_PERIODE_MODIFIEE: &str = "stages.periode_modifiee";
const EV_PERIODE_PUBLIEE: &str = "stages.periode_publiee";
const EV_PERIODE_CLOSE: &str = "stages.periode_close";

fn introuvable() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

async fn tenant(pool: &sqlx::PgPool, etab: Uuid) -> R<TenantTx<'static>> {
    TenantTx::begin(pool, etab)
        .await
        .map_err(|e| EntDbError::Domain(crate::pg_err::map_sqlx("stage", e)))
}

async fn fin(tx: TenantTx<'static>) -> R<()> {
    tx.commit()
        .await
        .map_err(|e| EntDbError::Domain(crate::pg_err::map_sqlx("stage", e)))
}

async fn personne_de(conn: &mut PgConnection, etab: Uuid, user: Uuid) -> R<Option<Uuid>> {
    let r: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 LIMIT 1",
    )
    .bind(etab)
    .bind(user)
    .fetch_optional(conn)
    .await
    .map_err(ecriture)?;
    Ok(r.map(|(i,)| i))
}

async fn evenement(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    aggregat: Uuid,
    acteur: Uuid,
    payload: serde_json::Value,
) -> R<()> {
    conn.publier(MetierEventDraft {
        etablissement_id: etab,
        type_name: type_name.to_string(),
        aggregate: AggregateKind::Stage,
        aggregate_id: aggregat,
        eleve_id: None,
        payload,
        actor_user_id: Some(acteur.to_string()),
    })
    .await?;
    Ok(())
}

/// Mise à jour refusée : la ligne n'existe pas (404) ou sa version a changé (conflit).
fn echec_maj(existe: bool) -> EntDbError {
    if existe {
        refus_stage(StageErreur::ConflitDeVersion)
    } else {
        introuvable()
    }
}

impl PgEnt {
    async fn exiger_stage(&self, etab: Uuid, acteur: Uuid, role: &str, action: Action) -> R<()> {
        if self.a_action_etablissement(etab, acteur, role, action).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Lecture « établissement » : `stages.lire` à l'établissement ou une action de gestion des stages.
    async fn peut_lire_stages_etablissement(&self, etab: Uuid, acteur: Uuid, role: &str) -> R<bool> {
        for a in [
            Action::StagesLire,
            Action::StagesParametrer,
            Action::StagesPeriodesGerer,
            Action::StagesOrganismesGerer,
        ] {
            if self.a_action_etablissement(etab, acteur, role, a).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    // ───────────────────────── Dispositifs et paramètres (STG-100 à STG-109) ─────────────────────────

    /// Dispositifs fournis par ScolIA et leur activation dans l'établissement. Un dispositif sans
    /// ligne n'est **pas** actif : la direction active explicitement ceux qui la concernent.
    pub async fn dispositifs_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> R<Vec<DispositifStageDto>> {
        if !self.peut_lire_stages_etablissement(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = tenant(self.pool(), etab).await?;
        let lignes: Vec<(String, String, bool, bool, bool, bool)> = sqlx::query_as(
            "SELECT r.code, r.libelle, r.observation_seule, r.regime_at, r.allocation,
                    COALESCE(d.actif, false)
               FROM ref_stage_dispositif r
               LEFT JOIN ent_stage_dispositif_etab d
                      ON d.etablissement_id = $1 AND d.dispositif_code = r.code
              ORDER BY r.ordre, r.code",
        )
        .bind(etab)
        .fetch_all(&mut *tx)
        .await
        .map_err(ecriture)?;
        Ok(lignes
            .into_iter()
            .map(|(code, libelle, observation_seule, regime_at, allocation, actif)| DispositifStageDto {
                code,
                libelle,
                observation_seule,
                regime_at,
                allocation,
                actif,
            })
            .collect())
    }

    /// STG-100 : active ou désactive un dispositif (`stages.parametrer`). Désactiver n'annule rien :
    /// les stages et les périodes existants gardent leur dispositif.
    pub async fn activer_dispositif_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        code: &str,
        actif: bool,
    ) -> R<()> {
        self.exiger_stage(etab, acteur, role, Action::StagesParametrer).await?;
        let mut tx = tenant(self.pool(), etab).await?;
        let existe: Option<(String,)> =
            sqlx::query_as("SELECT code FROM ref_stage_dispositif WHERE code = $1")
                .bind(code)
                .fetch_optional(&mut *tx)
                .await
                .map_err(ecriture)?;
        if existe.is_none() {
            return Err(refus_stage(StageErreur::DispositifInconnu));
        }
        let personne = personne_de(&mut tx, etab, acteur).await?;
        sqlx::query(
            "INSERT INTO ent_stage_dispositif_etab
                 (etablissement_id, dispositif_code, actif, modifie_par_personne_id)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (etablissement_id, dispositif_code)
             DO UPDATE SET actif = EXCLUDED.actif, modifie_le = now(),
                           modifie_par_personne_id = EXCLUDED.modifie_par_personne_id",
        )
        .bind(etab)
        .bind(code)
        .bind(actif)
        .bind(personne)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        evenement(
            &mut tx,
            etab,
            EV_PARAMETRES,
            etab,
            acteur,
            json!({ "dispositif": code, "actif": actif }),
        )
        .await?;
        fin(tx).await
    }

    /// Paramètres de l'établissement (valeurs par défaut tant que rien n'est enregistré).
    pub async fn parametres_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> R<ParametresStageDto> {
        if !self.peut_lire_stages_etablissement(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = tenant(self.pool(), etab).await?;
        lire_parametres(&mut tx, etab).await
    }

    /// STG-101 à STG-109 : modifie les paramètres. Première écriture : sans `version` ; ensuite la
    /// version lue est exigée.
    pub async fn modifier_parametres_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        corps: ModifierParametresStageBody,
    ) -> R<ParametresStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesParametrer).await?;
        let cmd = valider_parametres(ModifierParametres {
            assurance: corps
                .assurance
                .map(|a| a.map(|a| (a.assureur_nom, a.contrat))),
            deliberation: corps.deliberation.map(|d| d.map(|d| (d.reference, d.date))),
            max_stagiaires_referent: corps.max_stagiaires_referent,
            max_stagiaires_organisme: corps.max_stagiaires_organisme,
            relance_ouverture_j: corps.relance_ouverture_j,
            relance_sans_lieu_j: corps.relance_sans_lieu_j,
            conservation_contacts_annees: corps.conservation_contacts_annees,
            etablissement_signe_en_dernier: corps.etablissement_signe_en_dernier,
            accord_titulaires: corps.accord_titulaires,
            stage_famille: corps.stage_famille,
        })
        .map_err(refus_stage)?;
        let mut tx = tenant(self.pool(), etab).await?;
        let personne = personne_de(&mut tx, etab, acteur).await?;
        let actuel: Option<(i32,)> = sqlx::query_as(
            "SELECT version FROM ent_stage_parametre WHERE etablissement_id = $1 FOR UPDATE",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ecriture)?;
        let creation = actuel.is_none();
        match (actuel, corps.version) {
            (None, _) => {
                sqlx::query(
                    "INSERT INTO ent_stage_parametre (etablissement_id, modifie_par_personne_id)
                     VALUES ($1, $2) ON CONFLICT DO NOTHING",
                )
                .bind(etab)
                .bind(personne)
                .execute(&mut *tx)
                .await
                .map_err(ecriture)?;
            }
            (Some((v,)), Some(lue)) if v == lue => {}
            (Some(_), _) => return Err(refus_stage(StageErreur::ConflitDeVersion)),
        }
        // Un champ non fourni garde sa valeur : chaque colonne est `COALESCE(nouvelle, ancienne)`,
        // sauf les facultatives, pilotées par un drapeau « fourni » (effacement possible).
        let (assureur, contrat) = match &cmd.assurance {
            Some(Some((a, c))) => (Some(a.clone()), Some(c.clone())),
            _ => (None, None),
        };
        let (delib_ref, delib_le) = match &cmd.deliberation {
            Some(Some((r, d))) => (Some(r.clone()), Some(*d)),
            _ => (None, None),
        };
        sqlx::query(
            "UPDATE ent_stage_parametre SET
                assureur_nom = CASE WHEN $2 THEN $3 ELSE assureur_nom END,
                assurance_contrat = CASE WHEN $2 THEN $4 ELSE assurance_contrat END,
                deliberation_ref = CASE WHEN $5 THEN $6 ELSE deliberation_ref END,
                deliberation_le = CASE WHEN $5 THEN $7 ELSE deliberation_le END,
                max_stagiaires_referent = CASE WHEN $8 THEN $9 ELSE max_stagiaires_referent END,
                max_stagiaires_organisme = CASE WHEN $10 THEN $11 ELSE max_stagiaires_organisme END,
                relance_ouverture_j = COALESCE($12, relance_ouverture_j),
                relance_sans_lieu_j = COALESCE($13, relance_sans_lieu_j),
                conservation_contacts_annees = COALESCE($14, conservation_contacts_annees),
                etablissement_signe_en_dernier = COALESCE($15, etablissement_signe_en_dernier),
                accord_titulaires = COALESCE($16, accord_titulaires),
                stage_famille = COALESCE($17, stage_famille),
                modifie_par_personne_id = $18, modifie_le = now(),
                version = version + CASE WHEN $19 THEN 0 ELSE 1 END
              WHERE etablissement_id = $1",
        )
        .bind(etab)
        .bind(cmd.assurance.is_some())
        .bind(assureur)
        .bind(contrat)
        .bind(cmd.deliberation.is_some())
        .bind(delib_ref)
        .bind(delib_le)
        .bind(cmd.max_stagiaires_referent.is_some())
        .bind(cmd.max_stagiaires_referent.flatten())
        .bind(cmd.max_stagiaires_organisme.is_some())
        .bind(cmd.max_stagiaires_organisme.flatten())
        .bind(cmd.relance_ouverture_j)
        .bind(cmd.relance_sans_lieu_j)
        .bind(cmd.conservation_contacts_annees)
        .bind(cmd.etablissement_signe_en_dernier)
        .bind(cmd.accord_titulaires)
        .bind(cmd.stage_famille)
        .bind(personne)
        .bind(creation)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let relu = lire_parametres(&mut tx, etab).await?;
        evenement(
            &mut tx,
            etab,
            EV_PARAMETRES,
            etab,
            acteur,
            json!({ "version": relu.version }),
        )
        .await?;
        fin(tx).await?;
        Ok(relu)
    }

    // ───────────────────────── Organismes (STG-110, STG-114, STG-115, STG-117) ─────────────────────────

    /// Un organisme du carnet. `stages.organismes.gerer`, ou `stages.lire` à l'établissement
    /// (sans le texte libre du motif, réservé à la gestion du carnet).
    pub async fn organisme_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> R<OrganismeStageDto> {
        let gere = self.a_action_etablissement(etab, acteur, role, Action::StagesOrganismesGerer).await?;
        if !gere && !self.a_action_etablissement(etab, acteur, role, Action::StagesLire).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = tenant(self.pool(), etab).await?;
        let mut o = lire_organisme(&mut tx, etab, id).await?.ok_or_else(introuvable)?;
        if !gere {
            o.etat_motif_texte = None;
        }
        Ok(o)
    }

    /// STG-110 : type, enseigne, adresse structurée. `stages.organismes.gerer`, version exigée.
    pub async fn modifier_organisme_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        corps: ModifierOrganismeBody,
    ) -> R<OrganismeStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesOrganismesGerer).await?;
        let m = valider_organisme(ModifierOrganisme {
            type_organisme: corps.type_organisme,
            enseigne: corps.enseigne,
            adresse: corps.adresse,
            code_postal: corps.code_postal,
            commune: corps.commune,
            pays: corps.pays,
        })
        .map_err(refus_stage)?;
        let mut tx = tenant(self.pool(), etab).await?;
        let r = sqlx::query(
            "UPDATE ent_entreprise SET
                type = COALESCE($4, type),
                enseigne = CASE WHEN $5 THEN $6 ELSE enseigne END,
                adresse = COALESCE($7, adresse),
                code_postal = CASE WHEN $8 THEN $9 ELSE code_postal END,
                commune = CASE WHEN $10 THEN $11 ELSE commune END,
                pays = COALESCE($12, pays),
                version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(corps.version)
        .bind(m.type_organisme)
        .bind(m.enseigne.is_some())
        .bind(m.enseigne.clone().flatten())
        .bind(m.adresse)
        .bind(m.code_postal.is_some())
        .bind(m.code_postal.clone().flatten())
        .bind(m.commune.is_some())
        .bind(m.commune.clone().flatten())
        .bind(m.pays)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        if r.rows_affected() == 0 {
            return Err(echec_maj(organisme_existe(&mut tx, etab, id).await?));
        }
        evenement(&mut tx, etab, EV_ORGANISME_MODIFIE, id, acteur, json!({ "organisme_id": id })).await?;
        let o = lire_organisme(&mut tx, etab, id).await?.ok_or_else(introuvable)?;
        fin(tx).await?;
        Ok(o)
    }

    /// STG-117 : change l'état d'un organisme (motif codé, jamais une personne nommée).
    /// `stages.organismes.gerer` ; « interdit » : `stages.parametrer` (direction).
    pub async fn changer_etat_organisme_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        corps: ChangerEtatOrganismeBody,
    ) -> R<OrganismeStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesOrganismesGerer).await?;
        let direction = self.a_action_etablissement(etab, acteur, role, Action::StagesParametrer).await?;
        let (etat, motif, texte) = valider_etat_organisme(
            &corps.etat,
            corps.motif_code.as_deref(),
            corps.motif_texte.as_deref(),
            direction,
        )
        .map_err(refus_stage)?;
        let mut tx = tenant(self.pool(), etab).await?;
        let personne = personne_de(&mut tx, etab, acteur).await?;
        let r = sqlx::query(
            "UPDATE ent_entreprise SET etat = $4, etat_motif_code = $5, etat_motif_texte = $6,
                    etat_decide_par_personne_id = $7, etat_decide_le = now(),
                    version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(corps.version)
        .bind(&etat)
        .bind(&motif)
        .bind(&texte)
        .bind(personne)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        if r.rows_affected() == 0 {
            return Err(echec_maj(organisme_existe(&mut tx, etab, id).await?));
        }
        // Le texte libre ne quitte jamais la table : l'événement ne porte que l'état et le code.
        evenement(
            &mut tx,
            etab,
            EV_ORGANISME_ETAT,
            id,
            acteur,
            json!({ "organisme_id": id, "etat": etat, "motif": motif }),
        )
        .await?;
        let o = lire_organisme(&mut tx, etab, id).await?.ok_or_else(introuvable)?;
        fin(tx).await?;
        Ok(o)
    }

    /// Sites d'un organisme.
    pub async fn sites_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        organisme: Uuid,
    ) -> R<Vec<SiteStageDto>> {
        if !self.peut_lire_stages_etablissement(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = tenant(self.pool(), etab).await?;
        let lignes = sqlx::query_as::<_, SiteRow>(&format!(
            "SELECT {COLS_SITE} FROM ent_entreprise_site
              WHERE etablissement_id = $1 AND entreprise_id = $2 ORDER BY libelle"
        ))
        .bind(etab)
        .bind(organisme)
        .fetch_all(&mut *tx)
        .await
        .map_err(ecriture)?;
        Ok(lignes.into_iter().map(SiteRow::dto).collect())
    }

    /// STG-114 : ajoute un site. `stages.organismes.gerer`.
    pub async fn creer_site_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        organisme: Uuid,
        corps: SiteStageBody,
    ) -> R<SiteStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesOrganismesGerer).await?;
        let s = site_valide(&corps)?;
        let mut tx = tenant(self.pool(), etab).await?;
        if !organisme_existe(&mut tx, etab, organisme).await? {
            return Err(introuvable());
        }
        let personne = personne_de(&mut tx, etab, acteur).await?;
        let id = Uuid::new_v4();
        let ligne = sqlx::query_as::<_, SiteRow>(&format!(
            "INSERT INTO ent_entreprise_site
                (id, etablissement_id, entreprise_id, libelle, adresse_ligne1, adresse_ligne2,
                 code_postal, commune, pays, cree_par_personne_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING {COLS_SITE}"
        ))
        .bind(id)
        .bind(etab)
        .bind(organisme)
        .bind(&s.libelle)
        .bind(&s.adresse_ligne1)
        .bind(&s.adresse_ligne2)
        .bind(&s.code_postal)
        .bind(&s.commune)
        .bind(&s.pays)
        .bind(personne)
        .fetch_one(&mut *tx)
        .await
        .map_err(ecriture)?;
        evenement(&mut tx, etab, EV_SITE, id, acteur, json!({ "organisme_id": organisme, "site_id": id })).await?;
        fin(tx).await?;
        Ok(ligne.dto())
    }

    /// Modifie un site (version exigée).
    pub async fn modifier_site_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        corps: SiteStageBody,
    ) -> R<SiteStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesOrganismesGerer).await?;
        let version = corps.version.ok_or_else(|| refus_stage(StageErreur::ConflitDeVersion))?;
        let s = site_valide(&corps)?;
        let mut tx = tenant(self.pool(), etab).await?;
        let ligne = sqlx::query_as::<_, SiteRow>(&format!(
            "UPDATE ent_entreprise_site SET libelle = $4, adresse_ligne1 = $5, adresse_ligne2 = $6,
                    code_postal = $7, commune = $8, pays = $9, version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND version = $3 RETURNING {COLS_SITE}"
        ))
        .bind(etab)
        .bind(id)
        .bind(version)
        .bind(&s.libelle)
        .bind(&s.adresse_ligne1)
        .bind(&s.adresse_ligne2)
        .bind(&s.code_postal)
        .bind(&s.commune)
        .bind(&s.pays)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ecriture)?;
        let Some(ligne) = ligne else {
            let existe: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_entreprise_site WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ecriture)?;
            return Err(echec_maj(existe.is_some()));
        };
        evenement(&mut tx, etab, EV_SITE, id, acteur, json!({ "organisme_id": ligne.entreprise_id, "site_id": id })).await?;
        fin(tx).await?;
        Ok(ligne.dto())
    }

    /// Contacts d'un organisme (personnes extérieures) : `stages.organismes.gerer` seulement.
    pub async fn contacts_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        organisme: Uuid,
    ) -> R<Vec<ContactStageDto>> {
        self.exiger_stage(etab, acteur, role, Action::StagesOrganismesGerer).await?;
        let mut tx = tenant(self.pool(), etab).await?;
        let lignes = sqlx::query_as::<_, ContactRow>(&format!(
            "SELECT {COLS_CONTACT} FROM ent_entreprise_contact
              WHERE etablissement_id = $1 AND entreprise_id = $2 AND actif ORDER BY nom, prenom"
        ))
        .bind(etab)
        .bind(organisme)
        .fetch_all(&mut *tx)
        .await
        .map_err(ecriture)?;
        Ok(lignes.into_iter().map(ContactRow::dto).collect())
    }

    /// STG-115 : ajoute un contact (tuteur, signataire, RH).
    pub async fn creer_contact_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        organisme: Uuid,
        corps: ContactStageBody,
    ) -> R<ContactStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesOrganismesGerer).await?;
        let c = contact_valide(&corps)?;
        let mut tx = tenant(self.pool(), etab).await?;
        if !organisme_existe(&mut tx, etab, organisme).await? {
            return Err(introuvable());
        }
        let personne = personne_de(&mut tx, etab, acteur).await?;
        let id = Uuid::new_v4();
        let ligne = sqlx::query_as::<_, ContactRow>(&format!(
            "INSERT INTO ent_entreprise_contact
                (id, etablissement_id, entreprise_id, nom, prenom, fonction, email, telephone,
                 est_tuteur, est_signataire, est_rh, cree_par_personne_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING {COLS_CONTACT}"
        ))
        .bind(id)
        .bind(etab)
        .bind(organisme)
        .bind(&c.nom)
        .bind(&c.prenom)
        .bind(&c.fonction)
        .bind(&c.email)
        .bind(&c.telephone)
        .bind(c.est_tuteur)
        .bind(c.est_signataire)
        .bind(c.est_rh)
        .bind(personne)
        .fetch_one(&mut *tx)
        .await
        .map_err(ecriture)?;
        // Identifiants seulement : ni nom ni coordonnée dans l'événement.
        evenement(&mut tx, etab, EV_CONTACT, id, acteur, json!({ "organisme_id": organisme, "contact_id": id })).await?;
        fin(tx).await?;
        Ok(ligne.dto())
    }

    /// Modifie un contact (version exigée).
    pub async fn modifier_contact_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        corps: ContactStageBody,
    ) -> R<ContactStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesOrganismesGerer).await?;
        let version = corps.version.ok_or_else(|| refus_stage(StageErreur::ConflitDeVersion))?;
        let c = contact_valide(&corps)?;
        let mut tx = tenant(self.pool(), etab).await?;
        let ligne = sqlx::query_as::<_, ContactRow>(&format!(
            "UPDATE ent_entreprise_contact SET nom = $4, prenom = $5, fonction = $6, email = $7,
                    telephone = $8, est_tuteur = $9, est_signataire = $10, est_rh = $11,
                    version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND version = $3 AND actif
          RETURNING {COLS_CONTACT}"
        ))
        .bind(etab)
        .bind(id)
        .bind(version)
        .bind(&c.nom)
        .bind(&c.prenom)
        .bind(&c.fonction)
        .bind(&c.email)
        .bind(&c.telephone)
        .bind(c.est_tuteur)
        .bind(c.est_signataire)
        .bind(c.est_rh)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ecriture)?;
        let Some(ligne) = ligne else {
            let existe: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_entreprise_contact WHERE etablissement_id = $1 AND id = $2 AND actif",
            )
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ecriture)?;
            return Err(echec_maj(existe.is_some()));
        };
        evenement(&mut tx, etab, EV_CONTACT, id, acteur, json!({ "organisme_id": ligne.entreprise_id, "contact_id": id })).await?;
        fin(tx).await?;
        Ok(ligne.dto())
    }

    /// Désactive un contact (jamais de suppression : changement d'état daté, E6).
    pub async fn desactiver_contact_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        version: i32,
    ) -> R<()> {
        self.exiger_stage(etab, acteur, role, Action::StagesOrganismesGerer).await?;
        let mut tx = tenant(self.pool(), etab).await?;
        let r = sqlx::query(
            "UPDATE ent_entreprise_contact SET actif = false, desactive_le = now(),
                    version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2 AND version = $3 AND actif",
        )
        .bind(etab)
        .bind(id)
        .bind(version)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        if r.rows_affected() == 0 {
            let existe: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_entreprise_contact WHERE etablissement_id = $1 AND id = $2 AND actif",
            )
            .bind(etab)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ecriture)?;
            return Err(echec_maj(existe.is_some()));
        }
        evenement(&mut tx, etab, EV_CONTACT, id, acteur, json!({ "contact_id": id, "actif": false })).await?;
        fin(tx).await
    }

    // ───────────────────────── Périodes (STG-120 à STG-124) ─────────────────────────

    /// Périodes de l'établissement (filtre année et état).
    pub async fn periodes_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        filtre: &FiltrePeriodesStage,
    ) -> R<Vec<PeriodeStageDto>> {
        if !self.peut_lire_stages_etablissement(etab, acteur, role).await? {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = tenant(self.pool(), etab).await?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_stage_periode
              WHERE etablissement_id = $1
                AND ($2::uuid IS NULL OR annee_id = $2)
                AND ($3::text IS NULL OR etat = $3)
              ORDER BY du, libelle",
        )
        .bind(etab)
        .bind(filtre.annee_id)
        .bind(filtre.etat.as_deref())
        .fetch_all(&mut *tx)
        .await
        .map_err(ecriture)?;
        let mut out = Vec::with_capacity(ids.len());
        for (id,) in ids {
            if let Some(p) = lire_periode(&mut tx, etab, id).await? {
                out.push(p);
            }
        }
        Ok(out)
    }

    /// STG-120 / STG-123 : planifie une période (collective, ou individuelle avec motif) dans
    /// l'année du socle 5, pour un dispositif **activé**. `stages.periodes.gerer`.
    pub async fn creer_periode_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        corps: CreerPeriodeStageBody,
    ) -> R<PeriodeStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesPeriodesGerer).await?;
        let saisie = valider_periode(PeriodeSaisie {
            libelle: corps.libelle,
            du: corps.du,
            au: corps.au,
            individuelle: corps.individuelle,
            motif: corps.motif,
        })
        .map_err(refus_stage)?;
        let publics = publics_valides(corps.individuelle, &corps.publics)?;
        let mut tx = tenant(self.pool(), etab).await?;
        let actif: Option<(bool,)> = sqlx::query_as(
            "SELECT COALESCE((SELECT actif FROM ent_stage_dispositif_etab
                               WHERE etablissement_id = $1 AND dispositif_code = r.code), false)
               FROM ref_stage_dispositif r WHERE r.code = $2",
        )
        .bind(etab)
        .bind(&corps.dispositif_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ecriture)?;
        match actif {
            None => return Err(refus_stage(StageErreur::DispositifInconnu)),
            Some((false,)) => return Err(refus_stage(StageErreur::DispositifInactif)),
            Some((true,)) => {}
        }
        verifier_dans_annee(&mut tx, etab, corps.annee_id, saisie.du, saisie.au).await?;
        let personne = personne_de(&mut tx, etab, acteur).await?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_stage_periode
                (id, etablissement_id, annee_id, dispositif_code, libelle, du, au, type, motif,
                 cree_par_personne_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(corps.annee_id)
        .bind(&corps.dispositif_code)
        .bind(&saisie.libelle)
        .bind(saisie.du)
        .bind(saisie.au)
        .bind(if corps.individuelle { "individuelle" } else { "collective" })
        .bind(&saisie.motif)
        .bind(personne)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        remplacer_publics(&mut tx, etab, id, &publics).await?;
        evenement(
            &mut tx,
            etab,
            EV_PERIODE_PLANIFIEE,
            id,
            acteur,
            json!({
                "periode_id": id,
                "dispositif": corps.dispositif_code,
                "du": saisie.du.to_string(),
                "au": saisie.au.to_string(),
                "individuelle": corps.individuelle,
            }),
        )
        .await?;
        let p = lire_periode(&mut tx, etab, id).await?.ok_or_else(introuvable)?;
        fin(tx).await?;
        Ok(p)
    }

    /// STG-122 : modifie une période non close (libellé, dates, motif, publics remplacés).
    pub async fn modifier_periode_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        corps: ModifierPeriodeStageBody,
    ) -> R<PeriodeStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesPeriodesGerer).await?;
        let mut tx = tenant(self.pool(), etab).await?;
        let actuelle: Option<(String, String, Uuid, i32)> = sqlx::query_as(
            "SELECT etat, type, annee_id, version FROM ent_stage_periode
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ecriture)?;
        let Some((etat, type_periode, annee_id, version)) = actuelle else {
            return Err(introuvable());
        };
        if version != corps.version {
            return Err(refus_stage(StageErreur::ConflitDeVersion));
        }
        if etat == periode_stage::CLOSE {
            return Err(refus_stage(StageErreur::PeriodeDansLeTemps));
        }
        let individuelle = type_periode == "individuelle";
        let saisie = valider_periode(PeriodeSaisie {
            libelle: corps.libelle,
            du: corps.du,
            au: corps.au,
            individuelle,
            motif: corps.motif,
        })
        .map_err(refus_stage)?;
        let publics = publics_valides(individuelle, &corps.publics)?;
        if etat == periode_stage::PUBLIEE && publics.is_empty() {
            return Err(refus_stage(StageErreur::PeriodeSansPublic));
        }
        verifier_dans_annee(&mut tx, etab, annee_id, saisie.du, saisie.au).await?;
        sqlx::query(
            "UPDATE ent_stage_periode SET libelle = $3, du = $4, au = $5, motif = $6,
                    version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(&saisie.libelle)
        .bind(saisie.du)
        .bind(saisie.au)
        .bind(&saisie.motif)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        remplacer_publics(&mut tx, etab, id, &publics).await?;
        evenement(
            &mut tx,
            etab,
            EV_PERIODE_MODIFIEE,
            id,
            acteur,
            json!({ "periode_id": id, "du": saisie.du.to_string(), "au": saisie.au.to_string() }),
        )
        .await?;
        let p = lire_periode(&mut tx, etab, id).await?.ok_or_else(introuvable)?;
        fin(tx).await?;
        Ok(p)
    }

    /// STG-121 : publie la période (visible des familles et des élèves). Au moins un public.
    pub async fn publier_periode_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        version: i32,
    ) -> R<PeriodeStageDto> {
        self.changer_etat_periode(etab, acteur, role, id, version, periode_stage::PUBLIEE).await
    }

    /// STG-124 : ferme la période.
    pub async fn fermer_periode_stage_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        version: i32,
    ) -> R<PeriodeStageDto> {
        self.changer_etat_periode(etab, acteur, role, id, version, periode_stage::CLOSE).await
    }

    async fn changer_etat_periode(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        version: i32,
        vers: &str,
    ) -> R<PeriodeStageDto> {
        self.exiger_stage(etab, acteur, role, Action::StagesPeriodesGerer).await?;
        let mut tx = tenant(self.pool(), etab).await?;
        let actuelle: Option<(String, i32)> = sqlx::query_as(
            "SELECT etat, version FROM ent_stage_periode
              WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ecriture)?;
        let Some((etat, v)) = actuelle else {
            return Err(introuvable());
        };
        if v != version {
            return Err(refus_stage(StageErreur::ConflitDeVersion));
        }
        if !periode_stage::transition_permise(&etat, vers) {
            return Err(refus_stage(StageErreur::EtatPeriodeInvalide));
        }
        if vers == periode_stage::PUBLIEE {
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_stage_periode_public WHERE etablissement_id = $1 AND periode_id = $2",
            )
            .bind(etab)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(ecriture)?;
            if n == 0 {
                return Err(refus_stage(StageErreur::PeriodeSansPublic));
            }
        }
        sqlx::query(
            "UPDATE ent_stage_periode SET etat = $3,
                    publiee_le = CASE WHEN $3 = 'publiee' THEN now() ELSE publiee_le END,
                    close_le = CASE WHEN $3 = 'close' THEN now() ELSE close_le END,
                    version = version + 1, modifie_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .bind(vers)
        .execute(&mut *tx)
        .await
        .map_err(ecriture)?;
        let type_ev = if vers == periode_stage::PUBLIEE { EV_PERIODE_PUBLIEE } else { EV_PERIODE_CLOSE };
        evenement(&mut tx, etab, type_ev, id, acteur, json!({ "periode_id": id, "de": etat, "vers": vers })).await?;
        let p = lire_periode(&mut tx, etab, id).await?.ok_or_else(introuvable)?;
        fin(tx).await?;
        Ok(p)
    }
}

// ───────────────────────── Aides de lecture et de validation ─────────────────────────

fn site_valide(c: &SiteStageBody) -> R<SiteSaisie> {
    valider_site(SiteSaisie {
        libelle: c.libelle.clone(),
        adresse_ligne1: c.adresse_ligne1.clone(),
        adresse_ligne2: c.adresse_ligne2.clone(),
        code_postal: c.code_postal.clone(),
        commune: c.commune.clone(),
        pays: c.pays.clone().unwrap_or_default(),
    })
    .map_err(refus_stage)
}

fn contact_valide(c: &ContactStageBody) -> R<ContactSaisie> {
    valider_contact(ContactSaisie {
        nom: c.nom.clone(),
        prenom: c.prenom.clone(),
        fonction: c.fonction.clone(),
        email: c.email.clone(),
        telephone: c.telephone.clone(),
        est_tuteur: c.est_tuteur,
        est_signataire: c.est_signataire,
        est_rh: c.est_rh,
    })
    .map_err(refus_stage)
}

fn publics_valides(individuelle: bool, publics: &[PublicPeriodeDto]) -> R<Vec<PublicPeriode>> {
    let mut v = Vec::with_capacity(publics.len());
    for p in publics {
        v.push(match (p.classe_id, p.groupe_id, p.eleve_id) {
            (Some(c), None, None) => PublicPeriode::Classe(c),
            (None, Some(g), None) => PublicPeriode::Groupe(g),
            (None, None, Some(e)) => PublicPeriode::Eleve(e),
            _ => return Err(refus_stage(StageErreur::PeriodePublicInvalide)),
        });
    }
    valider_publics(individuelle, &v).map_err(refus_stage)
}

/// La période tient dans l'année scolaire du socle 5 (même établissement).
async fn verifier_dans_annee(
    conn: &mut PgConnection,
    etab: Uuid,
    annee: Uuid,
    du: NaiveDate,
    au: NaiveDate,
) -> R<()> {
    let borne: Option<(NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT date_debut, date_fin FROM ent_annee_scolaire WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(annee)
    .fetch_optional(conn)
    .await
    .map_err(ecriture)?;
    match borne {
        None => Err(introuvable()),
        Some((d, f)) if du >= d && au <= f => Ok(()),
        Some(_) => Err(refus_stage(StageErreur::PeriodeHorsAnnee)),
    }
}

async fn remplacer_publics(
    conn: &mut PgConnection,
    etab: Uuid,
    periode: Uuid,
    publics: &[PublicPeriode],
) -> R<()> {
    sqlx::query("DELETE FROM ent_stage_periode_public WHERE etablissement_id = $1 AND periode_id = $2")
        .bind(etab)
        .bind(periode)
        .execute(&mut *conn)
        .await
        .map_err(ecriture)?;
    for p in publics {
        let (classe, groupe, eleve) = match p {
            PublicPeriode::Classe(i) => (Some(*i), None, None),
            PublicPeriode::Groupe(i) => (None, Some(*i), None),
            PublicPeriode::Eleve(i) => (None, None, Some(*i)),
        };
        // Une classe, un groupe ou un élève d'un autre établissement échoue sur la clé composée.
        sqlx::query(
            "INSERT INTO ent_stage_periode_public (id, etablissement_id, periode_id, classe_id, groupe_id, eleve_id)
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(periode)
        .bind(classe)
        .bind(groupe)
        .bind(eleve)
        .execute(&mut *conn)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(d) if d.code().as_deref() == Some("23503") => {
                refus_stage(StageErreur::PeriodePublicInvalide)
            }
            _ => ecriture(e),
        })?;
    }
    Ok(())
}

async fn lire_periode(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> R<Option<PeriodeStageDto>> {
    let ligne: Option<(Uuid, Uuid, String, String, NaiveDate, NaiveDate, String, Option<String>, String, i32)> =
        sqlx::query_as(
            "SELECT id, annee_id, dispositif_code, libelle, du, au, type, motif, etat, version
               FROM ent_stage_periode WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(ecriture)?;
    let Some((id, annee_id, dispositif_code, libelle, du, au, type_periode, motif, etat, version)) = ligne
    else {
        return Ok(None);
    };
    let publics: Vec<(Option<Uuid>, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
        "SELECT classe_id, groupe_id, eleve_id FROM ent_stage_periode_public
          WHERE etablissement_id = $1 AND periode_id = $2 ORDER BY cree_le, id",
    )
    .bind(etab)
    .bind(id)
    .fetch_all(conn)
    .await
    .map_err(ecriture)?;
    Ok(Some(PeriodeStageDto {
        id,
        annee_id,
        dispositif_code,
        libelle,
        du,
        au,
        type_periode,
        motif,
        etat,
        publics: publics
            .into_iter()
            .map(|(classe_id, groupe_id, eleve_id)| PublicPeriodeDto { classe_id, groupe_id, eleve_id })
            .collect(),
        version,
    }))
}

async fn lire_parametres(conn: &mut PgConnection, etab: Uuid) -> R<ParametresStageDto> {
    #[derive(sqlx::FromRow)]
    struct P {
        assureur_nom: Option<String>,
        assurance_contrat: Option<String>,
        deliberation_ref: Option<String>,
        deliberation_le: Option<NaiveDate>,
        max_stagiaires_referent: Option<i16>,
        max_stagiaires_organisme: Option<i16>,
        relance_ouverture_j: i16,
        relance_sans_lieu_j: i16,
        alerte_veille_heure: chrono::NaiveTime,
        conservation_contacts_annees: i16,
        etablissement_signe_en_dernier: bool,
        accord_titulaires: String,
        stage_famille: String,
        version: i32,
    }
    let p: Option<P> = sqlx::query_as(
        "SELECT assureur_nom, assurance_contrat, deliberation_ref, deliberation_le,
                max_stagiaires_referent, max_stagiaires_organisme, relance_ouverture_j,
                relance_sans_lieu_j, alerte_veille_heure, conservation_contacts_annees,
                etablissement_signe_en_dernier, accord_titulaires, stage_famille, version
           FROM ent_stage_parametre WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(conn)
    .await
    .map_err(ecriture)?;
    Ok(match p {
        Some(p) => ParametresStageDto {
            assurance_renseignee: p.assureur_nom.is_some() && p.assurance_contrat.is_some(),
            assureur_nom: p.assureur_nom,
            assurance_contrat: p.assurance_contrat,
            deliberation_ref: p.deliberation_ref,
            deliberation_le: p.deliberation_le,
            max_stagiaires_referent: p.max_stagiaires_referent,
            max_stagiaires_organisme: p.max_stagiaires_organisme,
            relance_ouverture_j: p.relance_ouverture_j,
            relance_sans_lieu_j: p.relance_sans_lieu_j,
            alerte_veille_heure: p.alerte_veille_heure,
            conservation_contacts_annees: p.conservation_contacts_annees,
            etablissement_signe_en_dernier: p.etablissement_signe_en_dernier,
            accord_titulaires: p.accord_titulaires,
            stage_famille: p.stage_famille,
            version: p.version,
        },
        // Valeurs par défaut du lot (S-3901 à S-3914) ; `version` 0 : rien d'enregistré.
        None => ParametresStageDto {
            assureur_nom: None,
            assurance_contrat: None,
            deliberation_ref: None,
            deliberation_le: None,
            max_stagiaires_referent: None,
            max_stagiaires_organisme: None,
            relance_ouverture_j: 60,
            relance_sans_lieu_j: 30,
            alerte_veille_heure: chrono::NaiveTime::from_hms_opt(16, 0, 0).expect("16 h"),
            conservation_contacts_annees: 3,
            etablissement_signe_en_dernier: true,
            accord_titulaires: "un".into(),
            stage_famille: "autorise_avec_alerte".into(),
            assurance_renseignee: false,
            version: 0,
        },
    })
}

async fn organisme_existe(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> R<bool> {
    let r: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM ent_entreprise WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(id)
            .fetch_optional(conn)
            .await
            .map_err(ecriture)?;
    Ok(r.is_some())
}

async fn lire_organisme(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> R<Option<OrganismeStageDto>> {
    #[derive(sqlx::FromRow)]
    struct O {
        id: Uuid,
        raison_sociale: String,
        siret: Option<String>,
        adresse: String,
        #[sqlx(rename = "type")]
        type_organisme: String,
        enseigne: Option<String>,
        code_postal: Option<String>,
        commune: Option<String>,
        pays: String,
        etat: String,
        etat_motif_code: Option<String>,
        etat_motif_texte: Option<String>,
        version: i32,
    }
    let o: Option<O> = sqlx::query_as(
        "SELECT id, raison_sociale, siret, adresse, type, enseigne, code_postal, commune, pays,
                etat, etat_motif_code, etat_motif_texte, version
           FROM ent_entreprise WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(ecriture)?;
    Ok(o.map(|o| OrganismeStageDto {
        id: o.id,
        raison_sociale: o.raison_sociale,
        siret: o.siret,
        adresse: o.adresse,
        type_organisme: o.type_organisme,
        enseigne: o.enseigne,
        code_postal: o.code_postal,
        commune: o.commune,
        pays: o.pays,
        etat: o.etat,
        etat_motif_code: o.etat_motif_code,
        etat_motif_texte: o.etat_motif_texte,
        version: o.version,
    }))
}

const COLS_SITE: &str = "id, entreprise_id, libelle, adresse_ligne1, adresse_ligne2, code_postal, commune, pays, actif, version";

#[derive(sqlx::FromRow)]
struct SiteRow {
    id: Uuid,
    entreprise_id: Uuid,
    libelle: String,
    adresse_ligne1: String,
    adresse_ligne2: Option<String>,
    code_postal: String,
    commune: String,
    pays: String,
    actif: bool,
    version: i32,
}

impl SiteRow {
    fn dto(self) -> SiteStageDto {
        SiteStageDto {
            id: self.id,
            entreprise_id: self.entreprise_id,
            libelle: self.libelle,
            adresse_ligne1: self.adresse_ligne1,
            adresse_ligne2: self.adresse_ligne2,
            code_postal: self.code_postal,
            commune: self.commune,
            pays: self.pays,
            actif: self.actif,
            version: self.version,
        }
    }
}

const COLS_CONTACT: &str = "id, entreprise_id, nom, prenom, fonction, email, telephone, est_tuteur, est_signataire, est_rh, informe_le, actif, version";

#[derive(sqlx::FromRow)]
struct ContactRow {
    id: Uuid,
    entreprise_id: Uuid,
    nom: String,
    prenom: String,
    fonction: Option<String>,
    email: Option<String>,
    telephone: Option<String>,
    est_tuteur: bool,
    est_signataire: bool,
    est_rh: bool,
    informe_le: Option<NaiveDate>,
    actif: bool,
    version: i32,
}

impl ContactRow {
    fn dto(self) -> ContactStageDto {
        ContactStageDto {
            id: self.id,
            entreprise_id: self.entreprise_id,
            nom: self.nom,
            prenom: self.prenom,
            fonction: self.fonction,
            email: self.email,
            telephone: self.telephone,
            est_tuteur: self.est_tuteur,
            est_signataire: self.est_signataire,
            est_rh: self.est_rh,
            informe_le: self.informe_le,
            actif: self.actif,
            version: self.version,
        }
    }
}

