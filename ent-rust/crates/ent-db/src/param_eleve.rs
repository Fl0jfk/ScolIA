//! Socle 2 (l'élève), lot E5 — paramétrage des champs de l'élève par établissement.
//!
//! * ELV-26 : [`PgEnt::parametrage_eleve`] (lecture), [`PgEnt::parametrer_champ`],
//!   [`PgEnt::definir_regle_auto`], [`PgEnt::appliquer_profil`] (préréglages public, privé
//!   sous contrat, hors contrat…) ;
//! * garde des commandes directes : [`PgEnt::exiger_champs_modifiables`] — un champ verrouillé
//!   ou réglé « par proposition » / « interdite » refuse la modification directe (E2, E3).
//!
//! Sans ligne, un champ prend la valeur par défaut du catalogue (modification libre) : un
//! établissement existant ne change pas de comportement tant qu'il n'a rien réglé. Chaque
//! commande = UNE transaction (`TenantTx`) : réglage + événement `parametrage.eleve_modifie`
//! (noms de champs, jamais de valeur) + journal. Droit : `eleves.parametrer` (portée
//! établissement), contrôlé ici et non seulement par la route.

use std::collections::HashMap;

use domain_eleve::{
    champ_catalogue, valider_regle_auto, valider_reglage, verifier_modification_directe,
    ErreurProposition, ModificationLocale, ProfilEtablissement, ReglageChamp, SaisieFamille,
    SourceFoi, SourceProposition, CATALOGUE_CHAMPS_ELEVE,
};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

/// Un champ du catalogue avec son réglage effectif.
#[derive(Debug, Clone)]
pub struct ChampParametre {
    pub code: &'static str,
    pub libelle: &'static str,
    pub reglage: ReglageChamp,
    /// Une ligne existe pour ce champ (sinon : valeur par défaut du catalogue).
    pub regle: bool,
    /// Sources pour lesquelles une règle d'application automatique est active.
    pub regles_auto: Vec<String>,
}

/// Paramétrage complet des champs de l'élève d'un établissement.
#[derive(Debug, Clone)]
pub struct ParametrageEleve {
    /// Profil appliqué, `None` si jamais appliqué (valeurs par défaut du catalogue).
    pub profil: Option<String>,
    pub champs: Vec<ChampParametre>,
}

/// Modification partielle du réglage d'un champ.
#[derive(Debug, Clone, Default)]
pub struct ModifChamp {
    pub champ: String,
    pub visible: Option<bool>,
    pub obligatoire_a: Option<Vec<String>>,
    pub saisie_famille: Option<SaisieFamille>,
    pub source_qui_fait_foi: Option<SourceFoi>,
    pub modification_locale: Option<ModificationLocale>,
    pub verrouille: Option<bool>,
}

fn refus(e: ErreurProposition) -> EntDbError {
    match e {
        ErreurProposition::Invalide(m) => EntDbError::SaisieInvalide(m),
        ErreurProposition::Refus(m) => EntDbError::Refus(RefusMetier::Interdit, m),
        ErreurProposition::Conflit(m) => EntDbError::ConflitDossier(m),
    }
}

type LigneReglage = (String, bool, Vec<String>, String, String, String, bool);

fn lire_ligne(l: LigneReglage) -> Result<(String, ReglageChamp), EntDbError> {
    let (champ, visible, obligatoire_a, famille, foi, locale, verrouille) = l;
    let illisible = || EntDbError::SaisieInvalide("réglage de champ illisible");
    Ok((
        champ,
        ReglageChamp {
            visible,
            obligatoire_a,
            saisie_famille: SaisieFamille::parse(&famille).ok_or_else(illisible)?,
            source_qui_fait_foi: SourceFoi::parse(&foi).ok_or_else(illisible)?,
            modification_locale: ModificationLocale::parse(&locale).ok_or_else(illisible)?,
            verrouille,
        },
    ))
}

const COLONNES_REGLAGE: &str = "champ, visible, obligatoire_a, saisie_famille, source_qui_fait_foi, modification_locale, verrouille";

/// Réglages de l'établissement (lignes existantes seulement).
pub(crate) async fn reglages_tx(
    conn: &mut PgConnection,
    etab: Uuid,
) -> Result<HashMap<String, ReglageChamp>, EntDbError> {
    let lignes: Vec<LigneReglage> = sqlx::query_as(&format!(
        "SELECT {COLONNES_REGLAGE} FROM ent_param_eleve_champ WHERE etablissement_id = $1"
    ))
    .bind(etab)
    .fetch_all(&mut *conn)
    .await?;
    lignes.into_iter().map(lire_ligne).collect()
}

/// Réglage effectif d'un champ : la ligne de l'établissement, sinon le défaut du catalogue.
pub(crate) async fn reglage_champ_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    champ: &str,
) -> Result<ReglageChamp, EntDbError> {
    let l: Option<LigneReglage> = sqlx::query_as(&format!(
        "SELECT {COLONNES_REGLAGE} FROM ent_param_eleve_champ
         WHERE etablissement_id = $1 AND champ = $2"
    ))
    .bind(etab)
    .bind(champ)
    .fetch_optional(&mut *conn)
    .await?;
    match l {
        Some(l) => Ok(lire_ligne(l)?.1),
        None => Ok(ReglageChamp::defaut()),
    }
}

/// Règle automatique active pour (champ, source) ?
pub(crate) async fn regle_auto_active_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    champ: &str,
    source: SourceProposition,
) -> Result<bool, EntDbError> {
    let (actif,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_param_eleve_regle_auto
                         WHERE etablissement_id = $1 AND champ = $2 AND source = $3 AND actif)",
    )
    .bind(etab)
    .bind(champ)
    .bind(source.code())
    .fetch_one(&mut *conn)
    .await?;
    Ok(actif)
}

impl PgEnt {
    async fn exiger_parametrer(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        if self
            .a_action_etablissement(etab, acteur, role, Action::ElevesParametrer)
            .await?
        {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Garde des commandes DIRECTES (corrections, changements, usages, identifiants) : un champ
    /// verrouillé, ou dont la modification locale n'est pas libre, refuse l'écriture directe.
    /// À appeler dans la transaction de la commande, avant d'écrire. Les doublons sont ignorés.
    pub(crate) async fn exiger_champs_modifiables(
        conn: &mut PgConnection,
        etab: Uuid,
        champs: &[&str],
    ) -> Result<(), EntDbError> {
        let mut vus: Vec<&str> = Vec::new();
        for c in champs {
            if vus.contains(c) || champ_catalogue(c).is_none() {
                continue;
            }
            vus.push(c);
            let r = reglage_champ_tx(&mut *conn, etab, c).await?;
            verifier_modification_directe(&r).map_err(refus)?;
        }
        Ok(())
    }

    /// Paramétrage effectif (tous les champs du catalogue). `eleves.parametrer`.
    pub async fn parametrage_eleve(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
    ) -> Result<ParametrageEleve, EntDbError> {
        self.exiger_parametrer(etab, acteur, role).await?;
        let mut conn = TenantTx::begin(self.pool(), etab).await?;
        Self::lire_parametrage(&mut conn, etab).await
    }

    async fn lire_parametrage(
        conn: &mut PgConnection,
        etab: Uuid,
    ) -> Result<ParametrageEleve, EntDbError> {
        let reglages = reglages_tx(&mut *conn, etab).await?;
        let regles: Vec<(String, String)> = sqlx::query_as(
            "SELECT champ, source FROM ent_param_eleve_regle_auto
             WHERE etablissement_id = $1 AND actif ORDER BY champ, source",
        )
        .bind(etab)
        .fetch_all(&mut *conn)
        .await?;
        let profil: Option<(String,)> =
            sqlx::query_as("SELECT profil FROM ent_param_eleve_profil WHERE etablissement_id = $1")
                .bind(etab)
                .fetch_optional(&mut *conn)
                .await?;
        let champs = CATALOGUE_CHAMPS_ELEVE
            .iter()
            .map(|d| ChampParametre {
                code: d.code,
                libelle: d.libelle,
                regle: reglages.contains_key(d.code),
                reglage: reglages.get(d.code).cloned().unwrap_or_else(ReglageChamp::defaut),
                regles_auto: regles
                    .iter()
                    .filter(|(c, _)| c == d.code)
                    .map(|(_, s)| s.clone())
                    .collect(),
            })
            .collect();
        Ok(ParametrageEleve {
            profil: profil.map(|p| p.0),
            champs,
        })
    }

    /// Événement et journal d'un changement de paramétrage : NOMS de champs, jamais de valeur.
    async fn tracer_parametrage(
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        acteur: Uuid,
        action: &'static str,
        champs: &[&str],
        profil: Option<&str>,
    ) -> Result<(), EntDbError> {
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: types::PARAMETRAGE_ELEVE_MODIFIE.to_string(),
            aggregate: AggregateKind::ParamEtab,
            aggregate_id: etab,
            eleve_id: None,
            payload: json!({ "action": action, "champs": champs, "profil": profil }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        sqlx::query(
            "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                             payload, visible_famille, actor_user_id, created_at)
             VALUES ($1,$2,NULL,'parametrage',$3,$4,$5,false,$6, now())",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(format!("eleve_{action}"))
        .bind("Paramétrage des champs de l'élève modifié")
        .bind(json!({ "champs": champs, "profil": profil }))
        .bind(acteur)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Écrit (upsert) le réglage d'un champ après contrôle des bornes du logiciel.
    async fn ecrire_reglage(
        conn: &mut PgConnection,
        etab: Uuid,
        champ: &str,
        r: &ReglageChamp,
        acteur: Uuid,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "INSERT INTO ent_param_eleve_champ
                (etablissement_id, champ, visible, obligatoire_a, saisie_famille,
                 source_qui_fait_foi, modification_locale, verrouille, modifie_le, modifie_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8, now(), $9)
             ON CONFLICT (etablissement_id, champ) DO UPDATE SET
                visible = EXCLUDED.visible, obligatoire_a = EXCLUDED.obligatoire_a,
                saisie_famille = EXCLUDED.saisie_famille,
                source_qui_fait_foi = EXCLUDED.source_qui_fait_foi,
                modification_locale = EXCLUDED.modification_locale,
                verrouille = EXCLUDED.verrouille, modifie_le = now(), modifie_par = EXCLUDED.modifie_par",
        )
        .bind(etab)
        .bind(champ)
        .bind(r.visible)
        .bind(&r.obligatoire_a)
        .bind(r.saisie_famille.code())
        .bind(r.source_qui_fait_foi.code())
        .bind(r.modification_locale.code())
        .bind(r.verrouille)
        .bind(acteur)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// ELV-26 : règle un champ (modification partielle). Les bornes du logiciel s'appliquent
    /// (l'INE n'est jamais ouvert à la famille ; « interdite » exige une source externe…).
    /// Si une règle automatique devient incohérente avec la nouvelle source, elle est retirée.
    pub async fn parametrer_champ(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        m: ModifChamp,
    ) -> Result<ChampParametre, EntDbError> {
        self.exiger_parametrer(etab, acteur, role).await?;
        let def = champ_catalogue(&m.champ)
            .ok_or(EntDbError::SaisieInvalide("champ inconnu du catalogue"))?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Sérialise les réglages concurrents d'un même établissement.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('param_eleve:' || $1::text, 0))")
            .bind(etab)
            .execute(&mut *tx)
            .await?;
        let mut r = reglage_champ_tx(&mut tx, etab, def.code).await?;
        if let Some(v) = m.visible {
            r.visible = v;
        }
        if let Some(v) = m.obligatoire_a {
            r.obligatoire_a = v;
        }
        if let Some(v) = m.saisie_famille {
            r.saisie_famille = v;
        }
        if let Some(v) = m.source_qui_fait_foi {
            r.source_qui_fait_foi = v;
        }
        if let Some(v) = m.modification_locale {
            r.modification_locale = v;
        }
        if let Some(v) = m.verrouille {
            r.verrouille = v;
        }
        valider_reglage(def, &r).map_err(refus)?;
        Self::ecrire_reglage(&mut tx, etab, def.code, &r, acteur).await?;
        // Une règle automatique qui ne pourrait plus s'appliquer (la source qui fait foi n'est
        // plus externe) est retirée plutôt que laissée trompeuse.
        let sources: Vec<(String,)> = sqlx::query_as(
            "SELECT source FROM ent_param_eleve_regle_auto
             WHERE etablissement_id = $1 AND champ = $2 AND actif",
        )
        .bind(etab)
        .bind(def.code)
        .fetch_all(&mut *tx)
        .await?;
        for (s,) in sources {
            let incoherente = SourceProposition::parse(&s)
                .map(|s| valider_regle_auto(def, s, &r).is_err())
                .unwrap_or(true);
            if incoherente {
                sqlx::query(
                    "UPDATE ent_param_eleve_regle_auto SET actif = false, modifie_le = now(), modifie_par = $4
                     WHERE etablissement_id = $1 AND champ = $2 AND source = $3",
                )
                .bind(etab)
                .bind(def.code)
                .bind(&s)
                .bind(acteur)
                .execute(&mut *tx)
                .await?;
            }
        }
        Self::marquer_personnalise(&mut tx, etab, acteur).await?;
        Self::tracer_parametrage(&mut tx, etab, acteur, "champ_modifie", &[def.code], None).await?;
        let regles: Vec<(String,)> = sqlx::query_as(
            "SELECT source FROM ent_param_eleve_regle_auto
             WHERE etablissement_id = $1 AND champ = $2 AND actif ORDER BY source",
        )
        .bind(etab)
        .bind(def.code)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ChampParametre {
            code: def.code,
            libelle: def.libelle,
            reglage: r,
            regle: true,
            regles_auto: regles.into_iter().map(|(s,)| s).collect(),
        })
    }

    /// Un profil déjà appliqué devient « personnalisé » dès qu'un réglage est fait à la main.
    async fn marquer_personnalise(
        conn: &mut PgConnection,
        etab: Uuid,
        acteur: Uuid,
    ) -> Result<(), EntDbError> {
        sqlx::query(
            "UPDATE ent_param_eleve_profil SET profil = 'personnalise', applique_le = now(), applique_par = $2
             WHERE etablissement_id = $1 AND profil <> 'personnalise'",
        )
        .bind(etab)
        .bind(acteur)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// ELV-26 : active ou désactive une règle d'application automatique (champ × source).
    /// Une règle est toujours explicite ; elle n'est acceptée que si elle peut s'appliquer
    /// (flux machine, champ non sensible, autorité externe qui fait foi).
    pub async fn definir_regle_auto(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        champ: &str,
        source: SourceProposition,
        actif: bool,
    ) -> Result<(), EntDbError> {
        self.exiger_parametrer(etab, acteur, role).await?;
        let def = champ_catalogue(champ)
            .ok_or(EntDbError::SaisieInvalide("champ inconnu du catalogue"))?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        if actif {
            let r = reglage_champ_tx(&mut tx, etab, def.code).await?;
            valider_regle_auto(def, source, &r).map_err(refus)?;
        }
        sqlx::query(
            "INSERT INTO ent_param_eleve_regle_auto (etablissement_id, champ, source, actif, modifie_le, modifie_par)
             VALUES ($1,$2,$3,$4, now(), $5)
             ON CONFLICT (etablissement_id, champ, source)
             DO UPDATE SET actif = EXCLUDED.actif, modifie_le = now(), modifie_par = EXCLUDED.modifie_par",
        )
        .bind(etab)
        .bind(def.code)
        .bind(source.code())
        .bind(actif)
        .bind(acteur)
        .execute(&mut *tx)
        .await?;
        Self::marquer_personnalise(&mut tx, etab, acteur).await?;
        Self::tracer_parametrage(
            &mut tx,
            etab,
            acteur,
            if actif { "regle_auto_active" } else { "regle_auto_retiree" },
            &[def.code],
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// ELV-26 : applique un profil d'établissement (préréglage). Remplace TOUS les réglages de
    /// champs et les règles automatiques par ceux du profil — l'établissement ajuste ensuite
    /// champ par champ. Les propositions en attente ne sont pas touchées.
    pub async fn appliquer_profil(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        profil: ProfilEtablissement,
    ) -> Result<ParametrageEleve, EntDbError> {
        self.exiger_parametrer(etab, acteur, role).await?;
        if profil == ProfilEtablissement::Personnalise {
            return Err(EntDbError::SaisieInvalide(
                "« personnalisé » constate des réglages manuels, il ne s'applique pas",
            ));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('param_eleve:' || $1::text, 0))")
            .bind(etab)
            .execute(&mut *tx)
            .await?;
        let mut codes: Vec<&str> = Vec::new();
        for def in CATALOGUE_CHAMPS_ELEVE {
            let r = domain_eleve::reglage_profil(profil, def.code)
                .ok_or(EntDbError::SaisieInvalide("champ inconnu du profil"))?;
            valider_reglage(def, &r).map_err(refus)?;
            Self::ecrire_reglage(&mut tx, etab, def.code, &r, acteur).await?;
            codes.push(def.code);
        }
        sqlx::query("DELETE FROM ent_param_eleve_regle_auto WHERE etablissement_id = $1")
            .bind(etab)
            .execute(&mut *tx)
            .await?;
        for (champ, source) in domain_eleve::regles_auto_profil(profil) {
            sqlx::query(
                "INSERT INTO ent_param_eleve_regle_auto (etablissement_id, champ, source, actif, modifie_par)
                 VALUES ($1,$2,$3,true,$4)",
            )
            .bind(etab)
            .bind(champ)
            .bind(source.code())
            .bind(acteur)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO ent_param_eleve_profil (etablissement_id, profil, applique_le, applique_par)
             VALUES ($1,$2, now(), $3)
             ON CONFLICT (etablissement_id)
             DO UPDATE SET profil = EXCLUDED.profil, applique_le = now(), applique_par = EXCLUDED.applique_par",
        )
        .bind(etab)
        .bind(profil.code())
        .bind(acteur)
        .execute(&mut *tx)
        .await?;
        Self::tracer_parametrage(&mut tx, etab, acteur, "profil_applique", &codes, Some(profil.code()))
            .await?;
        let p = Self::lire_parametrage(&mut tx, etab).await?;
        tx.commit().await?;
        Ok(p)
    }
}
