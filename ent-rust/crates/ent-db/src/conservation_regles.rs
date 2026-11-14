//! **Règles de conservation par établissement** (socle 27, lot G1, `27_RGPD_ARCHIVES.md` §8.2, §14.2).
//!
//! Le catalogue ([`crate::conservation`]) est une **proposition** de ScolIA ; ce module porte ce que
//! l'établissement en fait :
//!
//! * chargement du catalogue en règles « proposées » (version 1), jamais « validées » (RGP-130) ;
//! * ajustement des durées dans leurs bornes légales connues, avec motif (RGP-132) ;
//! * **validation** par le rôle désigné (DPO en général ; droit `conservation.regles.valider`,
//!   paramétrable D-017) — RGP-133 ;
//! * **DPO facultatif** : un établissement sans DPO nommé voit ses règles « en attente de DPO ». Cela ne
//!   bloque **rien** d'autre que la purge réelle : [`PgEnt::purge_autorisee`] ne répond « oui » que pour
//!   une règle validée (R-RG1) ;
//! * politique générée et « pourquoi garde-t-on ? » (RGP-137, RGP-138) ;
//! * tableau des sanctions à effacer (R511-13 IV, RG-D06), en lecture : aucune suppression ici.
//!
//! Aucune purge n'est exécutée par ce module. Toute transition est journalisée dans la **même
//! transaction** (R-RG40), dans `cons_regle_journal` (ajout seul). Cloisonnement : tout passe par
//! `TenantTx` (RLS d'établissement) ; aucun accès hors établissement.

use crate::conservation::{
    self, bornes, categorie, categorie_de_nom, visa_exige, Categorie, Fiabilite, Profil, SortFinal,
    CATALOGUE, VERSION_CATALOGUE,
};
use crate::discipline_vs::date_effacement;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Months, NaiveDate, Utc};
use domain_eleve::DomainError;
use ent_auth::Action;
use serde::Serialize;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

fn db(e: sqlx::Error) -> EntDbError {
    EntDbError::Sqlx(e)
}

fn invalide(m: &'static str) -> EntDbError {
    EntDbError::SaisieInvalide(m)
}

/// État d'une règle tel qu'on l'affiche partout où une durée apparaît (R-RG1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatAffichage {
    /// Validée : un sort final peut être **proposé** dans un lot (il n'est jamais automatique).
    Validee,
    /// Proposée, et **aucun DPO n'est nommé** : « en attente de DPO ».
    EnAttenteDpo,
    /// Proposée, un DPO est nommé : à valider.
    AValider,
}

/// Une règle vue par l'établissement.
#[derive(Clone, Debug, Serialize)]
pub struct RegleVue {
    pub categorie: String,
    pub libelle: String,
    pub fait_generateur: String,
    /// Version de la règle en vigueur (validée) ou, à défaut, de la proposition ouverte.
    pub version: i32,
    pub etat: EtatAffichage,
    pub utilite_mois: Option<i32>,
    pub intermediaire_mois: Option<i32>,
    pub sort_final: String,
    pub visa_exige: bool,
    pub source: String,
    /// `verifie`, `non_verifie` ou `recommande` : la marque [V], [NV] ou [R] du catalogue.
    pub fiabilite: String,
    pub motif_ajustement: Option<String>,
    pub valide_le: Option<DateTime<Utc>>,
    /// Une proposition plus récente attend validation alors qu'une règle validée est en vigueur.
    pub proposition_en_cours: bool,
}

/// La politique de conservation générée pour l'établissement.
#[derive(Clone, Debug, Serialize)]
pub struct Politique {
    pub version_catalogue: String,
    pub profil: String,
    pub dpo_nomme: bool,
    /// Catégories pour lesquelles aucune règle n'a été chargée (à charger avant tout).
    pub non_chargees: Vec<String>,
    pub regles: Vec<RegleVue>,
    pub nb_validees: usize,
    /// Texte de synthèse : jamais « conforme » (R-RG9) ; il décrit les choix de l'établissement.
    pub synthese: String,
}

/// Réponse à « pourquoi garde-t-on cette donnée ? » (RGP-138, au niveau de la table ou de la catégorie).
#[derive(Clone, Debug, Serialize)]
pub struct Explication {
    pub cible: String,
    pub categorie: String,
    pub libelle: String,
    pub fait_generateur: String,
    pub utilite_mois: Option<i32>,
    pub intermediaire_mois: Option<i32>,
    pub sort_final: String,
    pub source: String,
    pub etat: EtatAffichage,
    /// Phrase prête à afficher.
    pub texte: String,
}

/// Désignation du DPO, telle qu'elle est lue.
#[derive(Clone, Debug, Serialize)]
pub struct Gouvernance {
    pub profil: String,
    pub dpo_nomme: bool,
    pub dpo_nom: Option<String>,
    pub dpo_contact: Option<String>,
    pub dpo_externe: Option<bool>,
}

/// Une sanction dont l'effacement est échu ou proche (R511-13 IV). Aucun texte libre.
#[derive(Clone, Debug, Serialize)]
pub struct SanctionAEffacer {
    pub sanction_id: Uuid,
    pub eleve_id: Uuid,
    pub categorie: String,
    pub prononcee_le: NaiveDate,
    pub echeance: NaiveDate,
    /// `echue` (à effacer maintenant, par l'exécuteur du socle 13) ou `a_venir` (dans l'année).
    pub etat: String,
}

fn fiabilite_code(f: Fiabilite) -> &'static str {
    match f {
        Fiabilite::Verifie => "verifie",
        Fiabilite::NonVerifie => "non_verifie",
        Fiabilite::Recommande => "recommande",
    }
}

struct LigneRegle {
    id: Uuid,
    categorie_code: String,
    version: i32,
    utilite_mois: Option<i32>,
    intermediaire_mois: Option<i32>,
    sort_final: String,
    visa_exige: bool,
    motif_ajustement: Option<String>,
    etat: String,
    valide_le: Option<DateTime<Utc>>,
}

impl LigneRegle {
    fn depuis(r: &sqlx::postgres::PgRow) -> Self {
        Self {
            id: r.get("id"),
            categorie_code: r.get("categorie_code"),
            version: r.get("version"),
            utilite_mois: r.get("utilite_mois"),
            intermediaire_mois: r.get("intermediaire_mois"),
            sort_final: r.get("sort_final"),
            visa_exige: r.get("visa_exige"),
            motif_ajustement: r.get("motif_ajustement"),
            etat: r.get("etat"),
            valide_le: r.get("valide_le"),
        }
    }
}

const COLONNES_REGLE: &str =
    "id, categorie_code, version, utilite_mois, intermediaire_mois, sort_final, \
     visa_exige, motif_ajustement, etat, valide_le";

/// Profil et DPO de l'établissement (profil par défaut tant que rien n'est posé).
async fn lire_gouvernance(
    c: &mut PgConnection,
    etab: Uuid,
) -> Result<(Profil, Gouvernance), EntDbError> {
    let row = sqlx::query(
        "SELECT profil, dpo_nom, dpo_contact, dpo_externe FROM rgp_gouvernance WHERE etablissement_id = $1",
    )
    .bind(etab)
    .fetch_optional(&mut *c)
    .await
    .map_err(db)?;
    let (profil_txt, nom, contact, externe): (
        String,
        Option<String>,
        Option<String>,
        Option<bool>,
    ) = match row {
        Some(r) => (
            r.get("profil"),
            r.get("dpo_nom"),
            r.get("dpo_contact"),
            r.get("dpo_externe"),
        ),
        None => ("prive_sous_contrat".to_string(), None, None, None),
    };
    let profil = Profil::parse(&profil_txt).unwrap_or(Profil::PriveSousContrat);
    Ok((
        profil,
        Gouvernance {
            profil: profil_txt,
            dpo_nomme: nom.is_some(),
            dpo_nom: nom,
            dpo_contact: contact,
            dpo_externe: externe,
        },
    ))
}

async fn journaliser(
    c: &mut PgConnection,
    etab: Uuid,
    regle_id: Uuid,
    categorie_code: &str,
    evenement: &str,
    par: Uuid,
    role: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO cons_regle_journal (id, etablissement_id, regle_id, categorie_code, evenement, par_compte_id, par_role)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(regle_id)
    .bind(categorie_code)
    .bind(evenement)
    .bind(par)
    .bind(role)
    .execute(&mut *c)
    .await
    .map_err(db)?;
    Ok(())
}

fn vue(
    c: &Categorie,
    valide: Option<&LigneRegle>,
    ouverte: Option<&LigneRegle>,
    dpo_nomme: bool,
) -> RegleVue {
    // La règle affichée est la validée en vigueur ; à défaut, la proposition ouverte.
    let retenue = valide.or(ouverte);
    let etat = if valide.is_some() {
        EtatAffichage::Validee
    } else if dpo_nomme {
        EtatAffichage::AValider
    } else {
        EtatAffichage::EnAttenteDpo
    };
    RegleVue {
        categorie: c.code.to_string(),
        libelle: c.libelle.to_string(),
        fait_generateur: c.fait_generateur.to_string(),
        version: retenue.map(|r| r.version).unwrap_or(0),
        etat,
        utilite_mois: retenue
            .map(|r| r.utilite_mois)
            .unwrap_or(c.utilite_mois.map(i32::from)),
        intermediaire_mois: retenue
            .map(|r| r.intermediaire_mois)
            .unwrap_or(c.intermediaire_mois.map(i32::from)),
        sort_final: retenue
            .map(|r| r.sort_final.clone())
            .unwrap_or_else(|| c.sort.code().to_string()),
        visa_exige: retenue.map(|r| r.visa_exige).unwrap_or(c.visa),
        source: c.source.to_string(),
        fiabilite: fiabilite_code(c.fiabilite).to_string(),
        motif_ajustement: retenue.and_then(|r| r.motif_ajustement.clone()),
        valide_le: valide.and_then(|r| r.valide_le),
        proposition_en_cours: valide.is_some() && ouverte.is_some(),
    }
}

async fn regles_de(c: &mut PgConnection, etab: Uuid) -> Result<Vec<LigneRegle>, EntDbError> {
    let rows = sqlx::query(&format!(
        "SELECT {COLONNES_REGLE} FROM cons_regle WHERE etablissement_id = $1 AND etat IN ('validee', 'proposee')
          ORDER BY categorie_code, version"
    ))
    .bind(etab)
    .fetch_all(&mut *c)
    .await
    .map_err(db)?;
    Ok(rows.iter().map(LigneRegle::depuis).collect())
}

fn construire_politique(profil: &Gouvernance, lignes: &[LigneRegle]) -> Politique {
    let mut regles = Vec::new();
    let mut non_chargees = Vec::new();
    for c in CATALOGUE {
        let valide = lignes
            .iter()
            .find(|l| l.categorie_code == c.code && l.etat == "validee");
        let ouverte = lignes
            .iter()
            .find(|l| l.categorie_code == c.code && l.etat == "proposee");
        if valide.is_none() && ouverte.is_none() {
            non_chargees.push(c.code.to_string());
        }
        regles.push(vue(c, valide, ouverte, profil.dpo_nomme));
    }
    let nb_validees = regles
        .iter()
        .filter(|r| r.etat == EtatAffichage::Validee)
        .count();
    let total = regles.len();
    let synthese = if profil.dpo_nomme {
        format!(
            "{nb_validees} catégorie(s) sur {total} validée(s). Tant qu'une catégorie n'est pas validée, \
             aucune donnée de cette catégorie n'est détruite : elle est conservée."
        )
    } else {
        format!(
            "Aucun DPO n'est nommé : {} catégorie(s) sur {total} en attente de DPO ({nb_validees} validée(s)). \
             Rien n'est bloqué dans le logiciel ; seule la destruction réelle de données attend la validation.",
            total - nb_validees
        )
    };
    Politique {
        version_catalogue: VERSION_CATALOGUE.to_string(),
        profil: profil.profil.clone(),
        dpo_nomme: profil.dpo_nomme,
        non_chargees,
        regles,
        nb_validees,
        synthese,
    }
}

fn texte_duree(m: Option<i32>) -> String {
    match m {
        None => "non fixée".to_string(),
        Some(0) => "aucune".to_string(),
        Some(n) if n % 12 == 0 => format!("{} an(s)", n / 12),
        Some(n) => format!("{n} mois"),
    }
}

impl PgEnt {
    /// Politique de conservation générée (RGP-137) : toutes les catégories, avec leur état.
    /// Droit `conformite.lire`.
    pub async fn politique_conservation(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Politique, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        self.politique_conservation_tx(&mut tx, etab).await
    }

    /// Politique lue dans la transaction de l'appelant, **sans contrôle de droit** (l'appelant l'a fait).
    pub(crate) async fn politique_conservation_tx(
        &self,
        c: &mut PgConnection,
        etab: Uuid,
    ) -> Result<Politique, EntDbError> {
        let (_, gouv) = lire_gouvernance(c, etab).await?;
        let lignes = regles_de(c, etab).await?;
        Ok(construire_politique(&gouv, &lignes))
    }

    /// Charge le catalogue du profil en règles **proposées** (RGP-130). Idempotent : une catégorie déjà
    /// chargée n'est pas touchée. Droit `conservation.regles.proposer`. Renvoie le nombre de règles créées.
    pub async fn charger_catalogue_conservation(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<usize, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConservationReglesProposer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (profil, _) = lire_gouvernance(&mut tx, etab).await?;
        let mut crees = 0;
        for c in CATALOGUE {
            let existe: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM cons_regle WHERE etablissement_id = $1 AND categorie_code = $2 LIMIT 1",
            )
            .bind(etab)
            .bind(c.code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if existe.is_some() {
                continue;
            }
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO cons_regle (id, etablissement_id, categorie_code, version, utilite_mois, intermediaire_mois,
                    sort_final, visa_exige, version_catalogue, etat, propose_par_compte_id)
                 VALUES ($1, $2, $3, 1, $4, $5, $6, $7, $8, 'proposee', $9)",
            )
            .bind(id)
            .bind(etab)
            .bind(c.code)
            .bind(c.utilite_mois.map(i32::from))
            .bind(c.intermediaire_mois.map(i32::from))
            .bind(c.sort.code())
            .bind(visa_exige(c, profil))
            .bind(VERSION_CATALOGUE)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            journaliser(&mut tx, etab, id, c.code, "proposee", acteur, role).await?;
            crees += 1;
        }
        tx.commit().await?;
        Ok(crees)
    }

    /// Ajuste les durées d'une catégorie dans leurs bornes, avec un motif (RGP-132). Crée une nouvelle
    /// version « proposée » ; la règle validée en vigueur reste appliquée jusqu'à validation (§8.2 point 4).
    /// Droit `conservation.regles.proposer`.
    #[allow(clippy::too_many_arguments)]
    pub async fn ajuster_regle_conservation(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        code: &str,
        utilite_mois: Option<u16>,
        intermediaire_mois: Option<u16>,
        critere_tri: Option<&str>,
        motif: &str,
    ) -> Result<RegleVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConservationReglesProposer)
            .await?;
        let cat = categorie(code).ok_or(invalide("catégorie inconnue"))?;
        let motif = motif.trim();
        if motif.chars().count() < 3
            || motif.chars().count() > 500
            || motif.chars().any(|c| c.is_control() && c != '\n')
        {
            return Err(invalide(
                "motif d'ajustement obligatoire (3 à 500 caractères)",
            ));
        }
        let critere = match critere_tri.map(str::trim) {
            Some("") | None => None,
            Some(t)
                if t.chars().count() <= 500 && !t.chars().any(|c| c.is_control() && c != '\n') =>
            {
                Some(t.to_string())
            }
            Some(_) => return Err(invalide("critère de tri invalide")),
        };
        if critere.is_some() && cat.sort != SortFinal::Tri {
            return Err(invalide(
                "un critère de tri ne s'applique qu'aux catégories triées",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (profil, gouv) = lire_gouvernance(&mut tx, etab).await?;
        let b = bornes(code, profil);
        for (v, min, max) in [
            (utilite_mois, b.utilite_min, b.utilite_max),
            (intermediaire_mois, b.intermediaire_min, b.intermediaire_max),
        ] {
            if let Some(v) = v {
                if v > conservation::DUREE_MAX_MOIS {
                    return Err(invalide("durée trop longue"));
                }
                if min.is_some_and(|m| v < m) {
                    return Err(invalide("durée inférieure au minimum légal connu"));
                }
                if max.is_some_and(|m| v > m) {
                    return Err(invalide("durée supérieure au maximum légal connu"));
                }
            }
        }
        // Un ajustement part de la règle en vigueur (validée), à défaut de la proposition ouverte.
        let lignes = regles_de(&mut tx, etab).await?;
        let valide = lignes
            .iter()
            .find(|l| l.categorie_code == code && l.etat == "validee");
        let ouverte = lignes
            .iter()
            .find(|l| l.categorie_code == code && l.etat == "proposee");
        let base = valide
            .or(ouverte)
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let prochaine: i32 = sqlx::query_scalar(
            "SELECT COALESCE(max(version), 0) + 1 FROM cons_regle WHERE etablissement_id = $1 AND categorie_code = $2",
        )
        .bind(etab)
        .bind(code)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if let Some(o) = ouverte {
            sqlx::query("UPDATE cons_regle SET etat = 'remplacee', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(o.id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            journaliser(&mut tx, etab, o.id, code, "remplacee", acteur, role).await?;
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO cons_regle (id, etablissement_id, categorie_code, version, utilite_mois, intermediaire_mois,
                sort_final, critere_tri, visa_exige, motif_ajustement, version_catalogue, etat, propose_par_compte_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'proposee', $12)",
        )
        .bind(id)
        .bind(etab)
        .bind(code)
        .bind(prochaine)
        .bind(utilite_mois.map(i32::from).or(base.utilite_mois))
        .bind(intermediaire_mois.map(i32::from).or(base.intermediaire_mois))
        .bind(&base.sort_final)
        .bind(critere)
        .bind(visa_exige(cat, profil))
        .bind(motif)
        .bind(VERSION_CATALOGUE)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(&mut tx, etab, id, code, "ajustee", acteur, role).await?;
        let lignes = regles_de(&mut tx, etab).await?;
        let v = vue(
            cat,
            lignes
                .iter()
                .find(|l| l.categorie_code == code && l.etat == "validee"),
            lignes
                .iter()
                .find(|l| l.categorie_code == code && l.etat == "proposee"),
            gouv.dpo_nomme,
        );
        tx.commit().await?;
        Ok(v)
    }

    /// Valide la proposition ouverte d'une catégorie (RGP-133). Droit `conservation.regles.valider`
    /// (rôle paramétrable : le DPO en général). La validation remplace la règle validée précédente, dans
    /// la même transaction. Refusée s'il n'y a pas de proposition ouverte.
    pub async fn valider_regle_conservation(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        code: &str,
    ) -> Result<RegleVue, EntDbError> {
        let mut v = self
            .valider_regles_conservation(etab, acteur, role, &[code])
            .await?;
        v.pop().ok_or(EntDbError::Domain(DomainError::NotFound))
    }

    /// Valide plusieurs catégories d'un coup (validation « en bloc »). Tout ou rien : si une catégorie n'a
    /// pas de proposition ouverte, rien n'est validé.
    pub async fn valider_regles_conservation(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        codes: &[&str],
    ) -> Result<Vec<RegleVue>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConservationReglesValider)
            .await?;
        if codes.is_empty() || codes.len() > CATALOGUE.len() {
            return Err(invalide("liste de catégories invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (_, gouv) = lire_gouvernance(&mut tx, etab).await?;
        let mut sortie = Vec::new();
        for code in codes {
            let cat = categorie(code).ok_or(invalide("catégorie inconnue"))?;
            let lignes = regles_de(&mut tx, etab).await?;
            let ouverte = lignes
                .iter()
                .find(|l| l.categorie_code == *code && l.etat == "proposee")
                .ok_or(EntDbError::ConflitDossier(
                    "aucune proposition à valider pour cette catégorie",
                ))?;
            // L'ancienne règle validée est remplacée AVANT la nouvelle : l'index n'admet qu'une validée.
            if let Some(ancienne) = lignes
                .iter()
                .find(|l| l.categorie_code == *code && l.etat == "validee")
            {
                sqlx::query("UPDATE cons_regle SET etat = 'remplacee', modifie_le = now() WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(ancienne.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                journaliser(&mut tx, etab, ancienne.id, code, "remplacee", acteur, role).await?;
            }
            sqlx::query(
                "UPDATE cons_regle SET etat = 'validee', valide_par_compte_id = $3, valide_par_role = $4,
                        valide_le = now(), modifie_le = now()
                  WHERE etablissement_id = $1 AND id = $2",
            )
            .bind(etab)
            .bind(ouverte.id)
            .bind(acteur)
            .bind(role)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            journaliser(&mut tx, etab, ouverte.id, code, "validee", acteur, role).await?;
            let lignes = regles_de(&mut tx, etab).await?;
            sortie.push(vue(
                cat,
                lignes
                    .iter()
                    .find(|l| l.categorie_code == *code && l.etat == "validee"),
                lignes
                    .iter()
                    .find(|l| l.categorie_code == *code && l.etat == "proposee"),
                gouv.dpo_nomme,
            ));
        }
        tx.commit().await?;
        Ok(sortie)
    }

    /// **Garde de la purge réelle** (R-RG1) : un sort final n'est permis que pour une catégorie dont une règle
    /// est **validée** dans cet établissement. Toute autre réponse est « non » : on conserve. Appelée par les
    /// lots de conservation (G6 et suivants) avant toute proposition de lot. Appel système : aucun droit
    /// d'utilisateur, aucune donnée de personne.
    pub async fn purge_autorisee(
        &self,
        etab: Uuid,
        categorie_code: &str,
    ) -> Result<bool, EntDbError> {
        if categorie(categorie_code).is_none() {
            return Ok(false);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM cons_regle WHERE etablissement_id = $1 AND categorie_code = $2 AND etat = 'validee'",
        )
        .bind(etab)
        .bind(categorie_code)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        Ok(n > 0)
    }

    /// « Pourquoi garde-t-on cette donnée ? » (RGP-138) pour une table du registre ou une catégorie.
    /// Droit `conformite.lire`.
    pub async fn pourquoi_garde_t_on(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        cible: &str,
    ) -> Result<Explication, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteLire)
            .await?;
        let code = match categorie(cible) {
            Some(c) => c.code,
            None => categorie_de_nom(cible).ok_or(EntDbError::Domain(DomainError::NotFound))?,
        };
        let cat = categorie(code).ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (_, gouv) = lire_gouvernance(&mut tx, etab).await?;
        let lignes = regles_de(&mut tx, etab).await?;
        let v = vue(
            cat,
            lignes
                .iter()
                .find(|l| l.categorie_code == code && l.etat == "validee"),
            lignes
                .iter()
                .find(|l| l.categorie_code == code && l.etat == "proposee"),
            gouv.dpo_nomme,
        );
        let fin = match v.etat {
            EtatAffichage::Validee => format!("Règle validée : {}.", sort_en_clair(&v.sort_final)),
            EtatAffichage::AValider => {
                "Règle non validée : rien n'est détruit tant que le DPO ne l'a pas validée."
                    .to_string()
            }
            EtatAffichage::EnAttenteDpo => {
                "Règle en attente de DPO : rien n'est détruit tant qu'elle n'est pas validée."
                    .to_string()
            }
        };
        let texte = format!(
            "« {} » : durée d'utilité courante {}, conservation intermédiaire {}, à compter de : {}. Source : {}. {fin}",
            v.libelle,
            texte_duree(v.utilite_mois),
            texte_duree(v.intermediaire_mois),
            v.fait_generateur,
            v.source,
        );
        Ok(Explication {
            cible: cible.to_string(),
            categorie: v.categorie,
            libelle: v.libelle,
            fait_generateur: v.fait_generateur,
            utilite_mois: v.utilite_mois,
            intermediaire_mois: v.intermediaire_mois,
            sort_final: v.sort_final,
            source: v.source,
            etat: v.etat,
            texte,
        })
    }

    /// Désigne le DPO de l'établissement (RGP-100). Droit `conformite.gerer`. Le DPO est **facultatif** :
    /// tant qu'il n'est pas nommé, les règles restent « en attente de DPO ».
    pub async fn designer_dpo(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        nom: &str,
        contact: &str,
        externe: bool,
    ) -> Result<Gouvernance, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        let nom = nom.trim();
        let contact = contact.trim();
        if nom.is_empty() || nom.chars().count() > 200 || nom.chars().any(char::is_control) {
            return Err(invalide("nom du DPO invalide"));
        }
        if contact.chars().count() < 3
            || contact.chars().count() > 254
            || contact.chars().any(char::is_control)
        {
            return Err(invalide("contact du DPO invalide"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO rgp_gouvernance (etablissement_id, dpo_nom, dpo_contact, dpo_externe, dpo_designe_le, dpo_designe_par_compte_id)
             VALUES ($1, $2, $3, $4, now(), $5)
             ON CONFLICT (etablissement_id) DO UPDATE
                SET dpo_nom = EXCLUDED.dpo_nom, dpo_contact = EXCLUDED.dpo_contact, dpo_externe = EXCLUDED.dpo_externe,
                    dpo_personne_id = NULL, dpo_designe_le = now(),
                    dpo_designe_par_compte_id = EXCLUDED.dpo_designe_par_compte_id, modifie_le = now()",
        )
        .bind(etab)
        .bind(nom)
        .bind(contact)
        .bind(externe)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            etab,
            Uuid::new_v4(),
            "rgpd.gouvernance",
            "dpo_designe",
            acteur,
            role,
        )
        .await?;
        let (_, g) = lire_gouvernance(&mut tx, etab).await?;
        tx.commit().await?;
        Ok(g)
    }

    /// Retire le DPO (départ, fin de mission). Les règles validées le restent ; les propositions
    /// repassent « en attente de DPO ». Droit `conformite.gerer`.
    pub async fn retirer_dpo(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Gouvernance, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteGerer)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "UPDATE rgp_gouvernance SET dpo_nom = NULL, dpo_contact = NULL, dpo_externe = NULL, dpo_personne_id = NULL,
                    dpo_designe_le = NULL, dpo_designe_par_compte_id = NULL, modifie_le = now()
              WHERE etablissement_id = $1",
        )
        .bind(etab)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            etab,
            Uuid::new_v4(),
            "rgpd.gouvernance",
            "dpo_retire",
            acteur,
            role,
        )
        .await?;
        let (_, g) = lire_gouvernance(&mut tx, etab).await?;
        tx.commit().await?;
        Ok(g)
    }

    /// Gouvernance (profil, DPO) de l'établissement. Droit `conformite.lire`.
    pub async fn gouvernance(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Gouvernance, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConformiteLire)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        Ok(lire_gouvernance(&mut tx, etab).await?.1)
    }

    /// **Tableau des sanctions à effacer** (R511-13 IV, RG-D06) : sanctions non effacées dont l'échéance est
    /// échue ou tombe dans les 12 mois, selon la règle d'effacement de leur catégorie. Lecture seule : l'effacement
    /// reste celui du socle 13. Aucun texte libre ni nom : élève par identifiant. Droit `discipline.gerer`, et
    /// **trace d'accès bloquante** (R-RG8) : sans trace, rien n'est servi.
    pub async fn sanctions_a_effacer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        au_jour: NaiveDate,
    ) -> Result<Vec<SanctionAEffacer>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DisciplineGerer)
            .await?;
        self.tracer_acces_sensible(
            etab,
            acteur,
            role,
            None,
            "consultation",
            "sanctions_a_effacer",
            "Consultation du tableau des sanctions à effacer",
        )
        .await?;
        let horizon = au_jour
            .checked_add_months(Months::new(12))
            .unwrap_or(au_jour);
        let candidates: Vec<(Uuid, Uuid, NaiveDate, String)> = {
            let mut tx = TenantTx::begin(self.pool(), etab).await?;
            sqlx::query_as(
                "SELECT s.id, s.eleve_id, s.jour, t.categorie
                   FROM ent_sanction s
                   JOIN ent_sanction_type t ON t.id = s.type_id AND t.etablissement_id = s.etablissement_id
                  WHERE s.etablissement_id = $1 AND s.effacee_at IS NULL AND t.categorie IS NOT NULL
                  ORDER BY s.jour, s.id",
            )
            .bind(etab)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
        };
        let mut regles = std::collections::HashMap::new();
        let mut sortie = Vec::new();
        for (id, eleve_id, jour, categorie) in candidates {
            let mode = match regles.get(&categorie) {
                Some(m) => *m,
                None => {
                    let m = self.regle_effacement(etab, &categorie).await?;
                    regles.insert(categorie.clone(), m);
                    m
                }
            };
            let Some(due) = date_effacement(jour, mode) else {
                continue;
            };
            if due > horizon {
                continue;
            }
            sortie.push(SanctionAEffacer {
                sanction_id: id,
                eleve_id,
                categorie,
                prononcee_le: jour,
                echeance: due,
                etat: if au_jour >= due { "echue" } else { "a_venir" }.to_string(),
            });
        }
        Ok(sortie)
    }
}

fn sort_en_clair(code: &str) -> &'static str {
    match code {
        "destruction" => {
            "destruction à l'issue de la conservation, après un lot relu et validé par une personne"
        }
        "tri" => "tri par le service d'archives à l'issue de la conservation",
        "conservation_definitive" => "conservation définitive ou versement aux archives",
        "effacement_dossier" => "effacement du dossier selon la règle du socle discipline",
        "suit_objet" => "suit le sort de l'objet auquel la ligne se rattache",
        "anonymisation_compte" => "anonymisation du compte après avis préalable",
        "suit_contrat" => "suit la fin du contrat",
        _ => "sort fixé par un autre socle",
    }
}
