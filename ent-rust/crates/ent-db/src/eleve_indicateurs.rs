//! Socle 2 (l'élève), lot E8 — pastilles (indicateurs) et synthèse de la fiche (§7.11, §14).
//!
//! * **Catalogue** : `domain_eleve::CATALOGUE_INDICATEURS` (du logiciel) ; la table
//!   `ent_catalogue_indicateur` lui est égale (test). Chaque domaine y déclare ses pastilles et
//!   **l'action qui les ouvre**.
//! * **Projection** (`ent_eleve_indicateur`) : les pastilles actuellement vraies d'un élève. Elle est
//!   maintenue par le **consommateur D0** `e8.indicateurs` de l'outbox : à chaque événement qui
//!   concerne une pastille (interne ← `eleve.regime_changed`, PAI ← `infirmerie.pai`, sans INE ←
//!   `eleve.identifiant_change`, confidentialité ← `eleve.confidentialite_changee`…), la pastille de
//!   CET élève est recalculée depuis les tables sources (fonction SQL unique) — idempotent, rejouable,
//!   sans dépendance à l'ordre des événements. Les pastilles qui dépendent de la date (échéance d'un
//!   PAI, majorité, mobilité) sont aussi recalculées chaque nuit.
//! * **Droits** : une pastille n'est jamais un droit. Le service la montre si l'acteur détient, **sur
//!   cet élève**, l'une des actions du catalogue ; un élève protégé est absent pour un tiers. Une
//!   tuile sans droit est masquée **avec sa raison** et ne dit rien de son contenu.
//! * **Contrat des tuiles** (`CONTRATS_TUILES`) : chaque domaine déclare son emplacement dans la
//!   synthèse (action qui l'ouvre, lien, personnel seul) ; la synthèse ne connaît aucun domaine en
//!   dur.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use domain_eleve::{
    contrat_tuile, indicateur_catalogue, indicateurs_de_l_evenement, ContratTuile, DomainError, Gravite,
    RaisonTuile, CATALOGUE_INDICATEURS, CONTRATS_TUILES, EVENEMENTS_ECOUTES,
};
use ent_auth::{Action, Role};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::droits_ctx::accorde;
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::{ContexteSysteme, TenantTx};

/// Nom stable du consommateur (clé d'idempotence : ne jamais le renommer sans migration).
pub const CONSOMMATEUR_INDICATEURS: &str = "e8.indicateurs";

pub fn consommateurs_indicateurs() -> Vec<Arc<dyn Consommateur>> {
    vec![Arc::new(Indicateurs)]
}

struct Indicateurs;

/// Élèves concernés par un événement : celui de l'événement, plus les deux fiches d'une fusion.
fn eleves_de(evt: &EvenementDistribue) -> Vec<Uuid> {
    let e = evt.event();
    let mut v: Vec<Uuid> = e.eleve_id.into_iter().collect();
    for cle in ["absorbe_id", "conserve_id"] {
        if let Some(id) = e.payload.get(cle).and_then(|x| x.as_str()).and_then(|s| s.parse::<Uuid>().ok()) {
            v.push(id);
        }
    }
    v.sort();
    v.dedup();
    v
}

async fn traiter(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), String> {
    let etab = evt.etablissement_id();
    sqlx::query("SELECT set_config('app.etablissement_id', $1, true)")
        .bind(etab.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let codes: Vec<String> =
        indicateurs_de_l_evenement(&evt.event().type_name).iter().map(|d| d.code.to_string()).collect();
    if codes.is_empty() {
        return Ok(());
    }
    for eleve in eleves_de(evt) {
        sqlx::query("SELECT ent_eleve_indicateurs_recalculer($1, $2, $3, $4, NULL)")
            .bind(etab)
            .bind(eleve)
            .bind(&codes)
            .bind(evt.event().id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

impl Consommateur for Indicateurs {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_INDICATEURS
    }

    fn types(&self) -> &'static [&'static str] {
        EVENEMENTS_ECOUTES
    }

    fn traiter<'a>(&'a self, tx: &'a mut PgConnection, evt: &'a EvenementDistribue) -> FutureTraitement<'a> {
        Box::pin(traiter(tx, evt))
    }
}

/// Une pastille telle que l'écran la reçoit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PastilleVue {
    pub code: &'static str,
    pub domaine: &'static str,
    pub libelle: &'static str,
    pub icone: &'static str,
    pub gravite: Gravite,
    pub valeur_code: Option<String>,
    pub valeur_date: Option<NaiveDate>,
    pub du: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtatTuile {
    Visible,
    /// Masquée : la raison, jamais le contenu.
    Masquee(RaisonTuile),
}

#[derive(Debug, Clone)]
pub struct TuileVue {
    pub domaine: &'static str,
    pub libelle: &'static str,
    pub rang: u8,
    pub lien: String,
    pub etat: EtatTuile,
    /// Pastilles de ce domaine que l'acteur a le droit de voir (vide si la tuile est masquée).
    pub pastilles: Vec<PastilleVue>,
}

/// Synthèse d'une fiche : les pastilles visibles (alerte d'abord) et les tuiles par domaine.
#[derive(Debug, Clone)]
pub struct SyntheseEleve {
    pub eleve_id: Uuid,
    pub pastilles: Vec<PastilleVue>,
    pub tuiles: Vec<TuileVue>,
}

/// Pour un code d'action : détenu sur cet élève, détenu quelque part (portée).
#[derive(Debug, Clone, Copy, Default)]
struct DroitAction {
    sur_eleve: bool,
    ailleurs: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct LigneProjection {
    code: String,
    valeur_code: Option<String>,
    valeur_date: Option<NaiveDate>,
    du: NaiveDate,
}

/// Toutes les actions que le catalogue et les contrats désignent (sans doublon).
fn actions_du_contrat() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = CATALOGUE_INDICATEURS
        .iter()
        .flat_map(|d| d.actions_requises.iter().copied())
        .chain(CONTRATS_TUILES.iter().flat_map(|c| c.actions_requises.iter().copied()))
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

impl PgEnt {
    /// Droits de l'acteur, pour chaque action du contrat, sur CET élève.
    async fn droits_du_contrat(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        eleve: Uuid,
    ) -> Result<HashMap<&'static str, DroitAction>, EntDbError> {
        let droits = self.droits_effectifs(etab, acteur, role).await?;
        let mut out = HashMap::new();
        for code in actions_du_contrat() {
            let Some(action) = Action::parse(code) else {
                // Un code inconnu serait une pastille publique : fail-closed.
                tracing::error!(code, "action du contrat de tuiles inconnue du catalogue");
                out.insert(code, DroitAction::default());
                continue;
            };
            let ailleurs = droits.iter().any(|d| accorde(d.action, action));
            let sur_eleve = ailleurs && self.autoriser_eleve(etab, acteur, role, action, eleve).await?;
            out.insert(code, DroitAction { sur_eleve, ailleurs });
        }
        Ok(out)
    }

    /// Synthèse de la fiche pour un acteur (E9b la dessine). Réservée au personnel : un élève absent,
    /// hors de la portée de l'annuaire ou protégé pour ce tiers répond `NotFound`. La lecture de
    /// pastilles sensibles (santé, confidentialité) est journalisée.
    pub async fn synthese_eleve_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
    ) -> Result<SyntheseEleve, EntDbError> {
        let parsed = Role::parse(role).ok_or(EntDbError::Forbidden)?;
        if !parsed.is_staff() {
            return Err(EntDbError::Forbidden);
        }
        // L'accueil n'a pas l'annuaire mais lit « maintenant » : il atteint la synthèse (seule la
        // tuile « scolarité », donc la pastille `interne`, s'ouvre pour lui).
        if !self.autoriser_eleve(etab, acteur, role, Action::ElevesAnnuaire, eleve).await?
            && !self.autoriser_eleve(etab, acteur, role, Action::EdtMaintenantLire, eleve).await?
        {
            return Err(DomainError::NotFound.into());
        }
        self.exiger_eleve_visible(etab, acteur, role, eleve).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let existe: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_eleve WHERE etablissement_id = $1 AND id = $2 AND fusionne_dans IS NULL",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_optional(&mut *tx)
        .await?;
        if existe.is_none() {
            return Err(DomainError::NotFound.into());
        }
        let lignes: Vec<LigneProjection> = sqlx::query_as(
            "SELECT code, valeur_code, valeur_date, du FROM ent_eleve_indicateur
              WHERE etablissement_id = $1 AND eleve_id = $2 ORDER BY code",
        )
        .bind(etab)
        .bind(eleve)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        let droits = self.droits_du_contrat(etab, acteur, role, eleve).await?;
        let a_le_droit = |actions: &[&str]| actions.iter().any(|a| droits.get(a).is_some_and(|d| d.sur_eleve));
        // M6 : sans le module santé activé, aucune pastille ni tuile de santé.
        let sante_active = self.module_sante_actif(etab).await?;
        let jour = self.aujourdhui_etablissement(etab).await?;
        // Règle générale : une pastille n'est visible que si la tuile de son domaine l'est (un
        // droit sur l'action de la pastille ne suffit pas si la tuile du domaine reste fermée).
        let tuile_ouverte = |domaine: &str| {
            (domaine != "sante" || sante_active)
                && contrat_tuile(domaine).is_some_and(|c| a_le_droit(c.actions_requises))
        };
        let mut pastilles: Vec<PastilleVue> = lignes
            .into_iter()
            .filter_map(|l| {
                let def = indicateur_catalogue(&l.code)?;
                (a_le_droit(def.actions_requises) && tuile_ouverte(def.domaine)).then_some(PastilleVue {
                    code: def.code,
                    domaine: def.domaine,
                    libelle: def.libelle,
                    icone: def.icone,
                    gravite: def.gravite,
                    valeur_code: l.valeur_code,
                    // `majeur` ne publie que le fait : ni la date de majorité ni son début ne
                    // doivent laisser deviner la date de naissance exacte.
                    valeur_date: if def.code == "majeur" { None } else { l.valeur_date },
                    du: if def.code == "majeur" { jour } else { l.du },
                })
            })
            .collect();
        pastilles.sort_by(|a, b| (a.gravite, a.libelle, a.code).cmp(&(b.gravite, b.libelle, b.code)));
        let mut tuiles: Vec<TuileVue> = CONTRATS_TUILES
            .iter()
            .map(|c: &ContratTuile| {
                let etat = if c.domaine == "sante" && !sante_active {
                    EtatTuile::Masquee(RaisonTuile::DroitManquant)
                } else if a_le_droit(c.actions_requises) {
                    EtatTuile::Visible
                } else if c.actions_requises.iter().any(|a| droits.get(a).is_some_and(|d| d.ailleurs)) {
                    EtatTuile::Masquee(RaisonTuile::HorsPortee)
                } else if c.personnel_seul {
                    EtatTuile::Masquee(RaisonTuile::ReserveAuPersonnel)
                } else {
                    EtatTuile::Masquee(RaisonTuile::DroitManquant)
                };
                let visible = etat == EtatTuile::Visible;
                TuileVue {
                    domaine: c.domaine,
                    libelle: c.libelle,
                    rang: c.rang,
                    lien: c.lien.replace("{eleve}", &eleve.to_string()),
                    pastilles: if visible {
                        pastilles.iter().filter(|p| p.domaine == c.domaine).cloned().collect()
                    } else {
                        Vec::new()
                    },
                    etat,
                }
            })
            .collect();
        tuiles.sort_by_key(|t| t.rang);
        if pastilles.iter().any(|p| matches!(p.domaine, "sante" | "protection")) {
            self.tracer_consultation(etab, acteur, role, Some(eleve), "pastilles_sensibles", "Consultation de pastilles sensibles", false)
                .await?;
        }
        Ok(SyntheseEleve { eleve_id: eleve, pastilles, tuiles })
    }

    /// Les pastilles visibles d'un élève (sans les tuiles) : mêmes droits et mêmes refus que la
    /// synthèse.
    pub async fn pastilles_eleve_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        eleve: Uuid,
    ) -> Result<Vec<PastilleVue>, EntDbError> {
        Ok(self.synthese_eleve_pour_acteur(role, etab, acteur, eleve).await?.pastilles)
    }

    /// Élèves portant une pastille (liste de travail : « sans INE », « PAI en cours »…). Il faut
    /// l'une des actions de la pastille **sur tout l'établissement** ; les élèves protégés sont
    /// absents pour qui n'y a pas droit.
    pub async fn eleves_par_pastille_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        code: &str,
        limite: i64,
    ) -> Result<Vec<Uuid>, EntDbError> {
        let def = indicateur_catalogue(code).ok_or(EntDbError::SaisieInvalide("pastille inconnue"))?;
        if !self.a_une_action_etablissement(etab, acteur, role, def.actions_requises).await? {
            return Err(EntDbError::Forbidden);
        }
        // M6 : une liste de santé exige le module santé de l'établissement.
        if def.domaine == "sante" {
            self.exiger_module_sante(etab).await?;
        }
        let profil = self.profil_champs(etab, acteur, role).await?;
        // Les protégés sont écartés en SQL AVANT la limite (pas de place laissée par un absent).
        let l: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT i.eleve_id FROM ent_eleve_indicateur i
              WHERE i.etablissement_id = $1 AND i.code = $2
                AND ($4 OR NOT ent_eleve_confidentiel($1, i.eleve_id))
              ORDER BY i.du DESC, i.eleve_id LIMIT $3",
        )
        .bind(etab)
        .bind(code)
        .bind(limite.clamp(1, 5000))
        .bind(profil.ouvert_partout(domain_eleve::GroupeChamps::Sensible))
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await?;
        let ids: Vec<Uuid> = l.into_iter().map(|(id,)| id).collect();
        self.tracer_consultation(etab, acteur, role, None, &format!("liste_pastille:{code}"), "Consultation d'une liste par pastille", false)
            .await?;
        Ok(ids)
    }

    /// Nombre d'élèves par pastille que l'acteur peut voir à l'échelle de l'établissement (tuiles
    /// « qualité des données », ELV-27) ; les élèves protégés ne comptent pas pour qui n'y a pas droit.
    pub async fn compteurs_pastilles_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
    ) -> Result<Vec<(&'static str, i64)>, EntDbError> {
        let profil = self.profil_champs(etab, acteur, role).await?;
        let voit_proteges = profil.ouvert_partout(domain_eleve::GroupeChamps::Sensible);
        if !Role::parse(role).is_some_and(Role::is_staff) {
            return Err(EntDbError::Forbidden);
        }
        let mut out = Vec::new();
        for def in CATALOGUE_INDICATEURS {
            if !self.a_une_action_etablissement(etab, acteur, role, def.actions_requises).await? {
                continue;
            }
            let (n,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM ent_eleve_indicateur i
                  WHERE i.etablissement_id = $1 AND i.code = $2
                    AND ($3 OR NOT ent_eleve_confidentiel($1, i.eleve_id))",
            )
            .bind(etab)
            .bind(def.code)
            .bind(voit_proteges)
            .fetch_one(&mut *TenantTx::begin(self.pool(), etab).await?)
            .await?;
            out.push((def.code, n));
        }
        Ok(out)
    }

    /// L'acteur détient-il l'une de ces actions sur l'établissement entier ?
    async fn a_une_action_etablissement(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        actions: &[&str],
    ) -> Result<bool, EntDbError> {
        for code in actions {
            if let Some(a) = Action::parse(code) {
                if self.a_action_etablissement(etab, acteur, role, a).await? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Reconstruit la projection d'un établissement depuis les tables sources (reprise après un
    /// incident, ajout d'une pastille au catalogue). `codes` : ceux à reconstruire (tous si `None`) ;
    /// `jour` : jour de référence (jour de l'établissement si `None`). Renvoie le nombre de lignes
    /// insérées, modifiées ou supprimées.
    pub async fn reconstruire_indicateurs(
        &self,
        etab: Uuid,
        codes: Option<&[String]>,
        jour: Option<NaiveDate>,
    ) -> Result<i64, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let n: i64 = sqlx::query_scalar("SELECT ent_eleve_indicateurs_recalculer($1, NULL, $2, NULL, $3)")
            .bind(etab)
            .bind(codes)
            .bind(jour)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(n)
    }

    /// Tâche quotidienne : recalcule, pour tous les établissements, les pastilles qui dépendent de la
    /// date (échéance d'un PAI, majorité, mobilité, mesure à terme). Idempotente.
    pub async fn recalcul_nocturne_indicateurs(&self) -> Result<i64, EntDbError> {
        let codes: Vec<String> =
            CATALOGUE_INDICATEURS.iter().filter(|d| d.depend_du_temps).map(|d| d.code.to_string()).collect();
        let mut total = 0;
        for etab in crate::tenant_tx::etablissements_systeme(self.pool(), ContexteSysteme::TacheNocturneEleves).await? {
            total += self.reconstruire_indicateurs(etab, Some(&codes), None).await?;
        }
        Ok(total)
    }

    /// Contrat d'une tuile (pour les domaines qui veulent savoir où ils s'affichent).
    pub fn contrat_de_tuile(domaine: &str) -> Option<&'static ContratTuile> {
        contrat_tuile(domaine)
    }
}
