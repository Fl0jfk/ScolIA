//! Formulaires, autorisations et consentements (socle 2, lot S2-8, D24 :
//! FRM-01→05).
//!
//! **Modèle.** Un formulaire est un `consentement`, une `autorisation` (objets
//! stables, ex. `droit_image`, versionnés) ou un `sondage`. Il est ciblé
//! (établissement, niveaux, classes, groupes). Chaque réponse est une
//! **décision datée, immuable et signée** (`accepte`, `refuse`, `retire`) avec
//! la qualité du signataire et l'empreinte du texte signé ; l'historique n'est
//! jamais écrasé.
//!
//! **Qui signe** (FO0, R-FC2, R-FC5, R-FC13). Pour un élève qui ne décide pas seul : un titulaire de
//! l'autorité parentale dont le lien est **communicant** (actif, non suspendu, hors interdiction de
//! communication : `ent_eleve_lien_communicant`) ; pour une **autorisation**, qui a en plus le droit
//! `autoriser_sorties` du lien (FD-10). Pour un élève qui décide seul (`ent_eleve_decide_seul` : majeur à date
//! certaine, émancipé ; **pas** le majeur sous tutelle ni celui dont la naissance est contestée) : l'élève, avec
//! son propre compte, et lui seul ; ses parents ne sont plus sollicités (R-FC12, FD-04).
//!
//! **Décision effective** ([`etat_effectif`]) : la **dernière** décision de chaque signataire encore qualifié,
//! lue selon la **règle d'accord** de l'objet ([`Regle`], R-FC8, FD-17) :
//! * `un` : un accord valide suffit ; un refus connu **sans** accord ⇒ `refuse` ; un refus **et** un accord ⇒
//!   `desaccord` ;
//! * `tous` (droit à l'image, voix, mention nominative, enregistrement visio) : chaque titulaire attendu doit
//!   accepter ; le silence de l'un ⇒ `en_attente` ; un refus ⇒ `refuse`.
//!
//! **Versions** (FD-01, R-FC19). Chaque publication d'une version `n+1` est qualifiée `forme`, `fond` ou
//! `restriction`. Un accord donné sur la version `v` reste valable tant qu'aucune version de **fond** n'a été
//! publiée après `v` ; sinon l'état est `a_renouveler`. Les **refus** restent protecteurs d'une version à l'autre.
//!
//! **Ce que voit une famille** (R-FC16, R-FC58, FD-03) : sa propre décision, jamais l'état global ni la décision
//! de l'autre titulaire. Les verdicts sont lus par les personnels (résultats, trombinoscope), pas par les familles.
//!
//! **Sans réponse** (R-FC18) : jamais un droit. Seul `accepte` donne un droit ([`donne_droit`]).
//!
//! **Application.** Le refus (ou le désaccord) du `droit_image` masque la photo de l'élève dans le trombinoscope
//! et à la lecture de la photo ([`OBJET_DROIT_IMAGE`]). L'absence de décision ne masque rien : la photo sert aux
//! usages administratifs (DOS-11, mission, réservé aux personnels) ; les usages visibles des familles ou du public
//! relèvent du port `verifier` (FO1).
//!
//! **Santé** (R-FC23, FRM-261, D-034) : aucun objet de santé ni de donnée sensible n'est accepté tant que
//! l'hébergeur HDS n'est pas en place ([`SANTE_ACTIVEE`]).

use crate::campagnes::{
    eleves_de_audience, responsables_de, sha256_hex, types_com, valider_audience,
};
use crate::notifications::{db, notifier_personne, EnvoiAPlanifier};
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use chrono::{DateTime, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeSet, HashMap, HashSet};
use uuid::Uuid;
use crate::tenant_tx::TenantTx;

/// Objet du consentement dont dépend l'affichage des photos.
pub const OBJET_DROIT_IMAGE: &str = "droit_image";

fn introuvable() -> EntDbError {
    EntDbError::Domain(domain_eleve::DomainError::NotFound)
}

// ---------------------------------------------------------------------------
// Règle de décision effective (pure)
// ---------------------------------------------------------------------------

/// Module santé (socle 34) : désactivé tant que l'hébergeur HDS n'est pas en place (D-034, R-FC23).
pub const SANTE_ACTIVEE: bool = false;

/// Règle d'accord d'un objet (socle 18 §7.1). La règle ne se choisit jamais à la baisse (R-FC6, FO1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regle {
    /// Un accord d'un titulaire suffit ; un refus connu est un désaccord.
    Un,
    /// Chaque titulaire attendu doit accorder.
    Tous,
}

/// Règle par défaut d'un objet du catalogue (§8.1) : `tous` pour tout ce qui touche à l'image, à la voix et à la
/// mention nominative (C. civ. 372-1, choix prudent tant qu'un juriste n'a pas tranché, Q-FC1) ; `un` sinon.
pub fn regle_par_defaut(objet: Option<&str>) -> Regle {
    match objet {
        Some(
            "droit_image" | "voix" | "mention_nominative" | "enregistrement_visio" | "voyage_etranger",
        ) => Regle::Tous,
        _ => Regle::Un,
    }
}

/// Dernière décision d'un signataire qualifié.
#[derive(Debug, Clone, Copy)]
pub struct DecisionRetenue<'a> {
    pub qualite: &'a str,
    pub decision: &'a str,
    pub version: i32,
}

/// Seul un accord effectif donne un droit : `aucune`, `en_attente`, `a_renouveler`, `desaccord` et `refuse`
/// n'en donnent aucun (R-FC18, R-FC3).
pub fn donne_droit(etat: &str) -> bool {
    etat == "accepte"
}

/// État effectif d'un élève pour un objet, à partir de la **dernière** décision de chaque signataire qualifié.
///
/// * `decide_seul` : l'élève décide (seules ses décisions comptent) ;
/// * `attendus` : nombre de titulaires qui doivent accorder (règle `tous`) ;
/// * `seuil` : plus petite version dont un accord reste valable (version de fond la plus récente, 1 sinon).
pub fn etat_effectif(
    dernieres: &[DecisionRetenue<'_>],
    decide_seul: bool,
    regle: Regle,
    attendus: usize,
    seuil: i32,
) -> &'static str {
    let attendue = if decide_seul { "eleve_majeur" } else { "responsable" };
    let pertinentes: Vec<&DecisionRetenue<'_>> = dernieres.iter().filter(|d| d.qualite == attendue).collect();
    if pertinentes.is_empty() {
        return "aucune";
    }
    let refus = pertinentes.iter().any(|d| matches!(d.decision, "refuse" | "retire"));
    let valides = pertinentes.iter().filter(|d| d.decision == "accepte" && d.version >= seuil).count();
    let perimees = pertinentes.iter().filter(|d| d.decision == "accepte" && d.version < seuil).count();
    if refus {
        return if regle == Regle::Un && !decide_seul && valides > 0 { "desaccord" } else { "refuse" };
    }
    if decide_seul || regle == Regle::Un {
        return if valides > 0 {
            "accepte"
        } else if perimees > 0 {
            "a_renouveler"
        } else {
            "aucune"
        };
    }
    let attendus = attendus.max(1);
    if valides >= attendus {
        "accepte"
    } else if perimees > 0 && valides + perimees >= attendus {
        "a_renouveler"
    } else {
        "en_attente"
    }
}

/// Objet de santé ou de donnée sensible (RGPD art. 9) : refusé (R-FC22, R-FC23, FRM-261).
pub fn objet_sensible(objet: &str) -> bool {
    const RACINES: [&str; 10] = [
        "sante", "medic", "allerg", "handicap", "psycho", "religion", "opinion", "ethni", "syndical",
        "orientation_sex",
    ];
    RACINES.iter().any(|r| objet.contains(r))
}

/// Forme d'un code d'objet (`^[a-z][a-z0-9_]{1,39}$`), contrôlée aussi à la lecture (FD-19).
pub fn objet_valide(o: &str) -> bool {
    o.len() >= 2
        && o.len() <= 40
        && o.starts_with(|c: char| c.is_ascii_lowercase())
        && o.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Formulaire {
    pub id: Uuid,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub type_formulaire: String,
    pub objet: Option<String>,
    pub version: i32,
    pub titre: String,
    pub texte: String,
    pub statut: String,
    pub audience: String,
    pub cible_ids: Vec<Uuid>,
    pub cree_par: Uuid,
    pub publie_le: Option<DateTime<Utc>>,
    /// Qualification de la version (`forme`, `fond`, `restriction`) ; `None` pour une première version.
    pub changement: Option<String>,
    pub publie_par: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

const COLONNES_FORMULAIRE: &str = "id, type, objet, version, titre, texte, statut, audience, \
    cible_ids, cree_par, publie_le, changement, publie_par, created_at";

#[derive(Debug, Clone, Deserialize)]
pub struct NouveauFormulaire {
    #[serde(rename = "type")]
    pub type_formulaire: String,
    pub objet: Option<String>,
    pub titre: String,
    pub texte: String,
    pub audience: String,
    #[serde(default)]
    pub cible_ids: Vec<Uuid>,
}

/// Ce qu'une famille voit : un formulaire à traiter pour un de ses enfants.
#[derive(Debug, Clone, Serialize)]
pub struct FormulaireFamille {
    pub formulaire_id: Uuid,
    #[serde(rename = "type")]
    pub type_formulaire: String,
    pub objet: Option<String>,
    pub version: i32,
    pub titre: String,
    pub texte: String,
    pub eleve_id: Uuid,
    pub eleve_prenom: String,
    /// Ma dernière décision (`accepte`, `refuse`, `retire`), si j'en ai donné une.
    pub ma_decision: Option<String>,
    /// **Toujours `None`** : une famille ne voit jamais l'état global, qui révélerait la décision de l'autre
    /// titulaire (R-FC16, R-FC58, FD-03). Le champ reste pour la compatibilité du contrat d'API.
    pub etat: Option<String>,
    /// Vrai si aucune décision de ma part ne couvre la version en cours (jamais répondu, ou accord à
    /// renouveler après une version de fond).
    pub a_repondre: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EtatConsentement {
    pub eleve_id: Uuid,
    pub objet: String,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsentementEleve {
    pub eleve_id: Uuid,
    pub nom: String,
    pub prenom: String,
    pub decision: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResultatsFormulaire {
    pub cibles: usize,
    pub accepte: usize,
    pub refuse: usize,
    pub sans_reponse: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReponseFormulaire {
    pub eleve_id: Uuid,
    /// `accepte`, `refuse` ou `retire`.
    pub decision: String,
    pub commentaire: Option<String>,
}

// ---------------------------------------------------------------------------
// Lecture des décisions effectives
// ---------------------------------------------------------------------------

/// Nature d'un objet et plus petite version dont un accord reste valable (FD-01) : la dernière version de **fond**
/// publiée ou archivée, sinon 1. Un sondage (sans objet) n'a ni l'un ni l'autre.
async fn nature_et_seuil(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    objet: Option<&str>,
) -> Result<(String, i32), EntDbError> {
    let Some(objet) = objet else {
        return Ok(("sondage".to_string(), 1));
    };
    let row: Option<(String, Option<i32>)> = sqlx::query_as(
        "SELECT type,
                max(version) FILTER (WHERE changement = 'fond' AND statut IN ('publie', 'archive'))
           FROM ent_formulaire
          WHERE etablissement_id = $1 AND objet = $2
          GROUP BY type LIMIT 1",
    )
    .bind(etab)
    .bind(objet)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    Ok(row
        .map(|(t, fond)| (t, fond.unwrap_or(1)))
        .unwrap_or_else(|| ("consentement".to_string(), 1)))
}

/// Élèves, parmi `eleves`, qui décident eux-mêmes (majeur à date certaine, émancipé ; ni tutelle ni date
/// contestée : fonction canonique `ent_eleve_decide_seul`, FD-05).
async fn eleves_decidant_seuls(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
) -> Result<HashSet<Uuid>, EntDbError> {
    if eleves.is_empty() {
        return Ok(HashSet::new());
    }
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_eleve
          WHERE etablissement_id = $1 AND id = ANY($2) AND ent_eleve_decide_seul($1, id)",
    )
    .bind(etab)
    .bind(eleves)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// État effectif de chaque élève demandé (clé absente = `aucune`).
async fn etats_effectifs(
    conn: &mut sqlx::PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
    objet: Option<&str>,
    formulaire_id: Option<Uuid>,
) -> Result<HashMap<Uuid, &'static str>, EntDbError> {
    if eleves.is_empty() {
        return Ok(HashMap::new());
    }
    let (nature, seuil) = nature_et_seuil(&mut *conn, etab, objet).await?;
    let exige_sorties = nature == "autorisation";
    let regle = regle_par_defaut(objet);
    // Dernière décision de chaque signataire, encore qualifié au moment de la lecture : lien communicant
    // (interdiction de communication, suspension, fin de lien : plus décideur, FD-02), droit de sortie pour une
    // autorisation (FD-10).
    let rows: Vec<(Uuid, String, String, i32)> = sqlx::query_as(
        "WITH derniere AS (
            SELECT DISTINCT ON (r.eleve_id, r.signataire_user_id)
                   r.eleve_id, r.signataire_user_id, r.qualite, r.decision, r.version
              FROM ent_formulaire_reponse r
             WHERE r.etablissement_id = $1 AND r.eleve_id = ANY($2)
               AND ($3::text IS NULL OR r.objet = $3)
               AND ($4::uuid IS NULL OR r.formulaire_id = $4)
             ORDER BY r.eleve_id, r.signataire_user_id, r.decide_le DESC, r.id DESC)
         SELECT d.eleve_id, d.qualite, d.decision, d.version
           FROM derniere d
          WHERE (d.qualite = 'responsable' AND EXISTS (
                    SELECT 1 FROM ent_eleve_lien_communicant k
                     WHERE k.etablissement_id = $1 AND k.eleve_id = d.eleve_id
                       AND k.user_id = d.signataire_user_id AND k.autorite_parentale
                       AND ($5 = false OR k.peut_autoriser_sorties)))
             OR (d.qualite = 'eleve_majeur' AND EXISTS (
                    SELECT 1 FROM ent_eleve_compte c
                     WHERE c.etablissement_id = $1 AND c.eleve_id = d.eleve_id
                       AND c.user_id = d.signataire_user_id))",
    )
    .bind(etab)
    .bind(eleves)
    .bind(objet)
    .bind(formulaire_id)
    .bind(exige_sorties)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    // Titulaires attendus (règle `tous`) : les mêmes conditions, avec ou sans compte (le silence d'un titulaire
    // sans compte garde l'accord en attente).
    let attendus: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT k.eleve_id, count(DISTINCT COALESCE(k.personne_id, k.user_id))
           FROM ent_eleve_lien_communicant k
          WHERE k.etablissement_id = $1 AND k.eleve_id = ANY($2) AND k.autorite_parentale
            AND ($3 = false OR k.peut_autoriser_sorties)
          GROUP BY k.eleve_id",
    )
    .bind(etab)
    .bind(eleves)
    .bind(exige_sorties)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let attendus: HashMap<Uuid, usize> = attendus.into_iter().map(|(e, n)| (e, n as usize)).collect();
    let seuls = eleves_decidant_seuls(&mut *conn, etab, eleves).await?;
    let mut par_eleve: HashMap<Uuid, Vec<(String, String, i32)>> = HashMap::new();
    for (e, q, d, v) in rows {
        par_eleve.entry(e).or_default().push((q, d, v));
    }
    Ok(eleves
        .iter()
        .map(|e| {
            let d: Vec<DecisionRetenue<'_>> = par_eleve
                .get(e)
                .map(|v| {
                    v.iter()
                        .map(|(q, d, ver)| DecisionRetenue { qualite: q.as_str(), decision: d.as_str(), version: *ver })
                        .collect()
                })
                .unwrap_or_default();
            (*e, etat_effectif(&d, seuls.contains(e), regle, attendus.get(e).copied().unwrap_or(0), seuil))
        })
        .collect())
}

/// Enfant (ou élève lui-même) pour lequel une personne peut décider.
#[derive(Debug, Clone)]
struct Enfant {
    id: Uuid,
    prenom: String,
    /// `responsable` ou `eleve_majeur`.
    qualite: &'static str,
    /// Le lien porte le droit `autoriser_sorties` (toujours vrai pour l'élève lui-même).
    peut_sorties: bool,
}

impl Enfant {
    /// Une autorisation ne se signe qu'avec le droit de sortie du lien (FD-10, CK-08).
    fn peut_repondre_a(&self, type_formulaire: &str) -> bool {
        type_formulaire != "autorisation" || self.peut_sorties
    }
}

impl PgEnt {
    /// Élèves, parmi `eleves`, qui ont **refusé** l'objet, ou dont les titulaires sont en **désaccord**
    /// (application des consentements : photo masquée, etc.).
    pub(crate) async fn eleves_ayant_refuse(
        &self,
        etab: Uuid,
        eleves: &[Uuid],
        objet: &str,
    ) -> Result<HashSet<Uuid>, EntDbError> {
        let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        Ok(etats_effectifs(&mut conn, etab, eleves, Some(objet), None)
            .await?
            .into_iter()
            .filter(|(_, e)| matches!(*e, "refuse" | "desaccord"))
            .map(|(id, _)| id)
            .collect())
    }

    async fn exiger_formulaires(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::FormulairesGerer).await
    }

    // ----- Gestion (direction, CPE, administratif) ---------------------------

    pub async fn creer_formulaire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        n: NouveauFormulaire,
    ) -> Result<Formulaire, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        if !["consentement", "autorisation", "sondage"].contains(&n.type_formulaire.as_str()) {
            return Err(EntDbError::SaisieInvalide("type de formulaire inconnu"));
        }
        let titre = n.titre.trim().to_string();
        let texte = n.texte.trim().to_string();
        if titre.is_empty() || titre.chars().count() > 200 {
            return Err(EntDbError::SaisieInvalide("titre invalide"));
        }
        if texte.is_empty() || texte.chars().count() > 10_000 {
            return Err(EntDbError::SaisieInvalide("texte invalide"));
        }
        let objet = match (n.type_formulaire.as_str(), n.objet.as_deref().map(str::trim)) {
            ("sondage", None) | ("sondage", Some("")) => None,
            ("sondage", Some(_)) => {
                return Err(EntDbError::SaisieInvalide("un sondage n'a pas d'objet"))
            }
            (_, Some(o)) if objet_valide(o) => Some(o.to_string()),
            _ => {
                return Err(EntDbError::SaisieInvalide(
                    "objet requis (minuscules, chiffres et _ ; ex. droit_image)",
                ))
            }
        };
        if let Some(o) = &objet {
            if !SANTE_ACTIVEE && objet_sensible(o) {
                return Err(EntDbError::SaisieInvalide(
                    "donnée de santé ou sensible : interdite tant que l'hébergeur HDS n'est pas en place",
                ));
            }
        }
        let mut cibles = n.cible_ids.clone();
        cibles.sort();
        cibles.dedup();
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        valider_audience(&mut tx, etab, &n.audience, &cibles).await?;
        if let Some(o) = &objet {
            // Les versions d'un même objet se créent en série.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 11))")
                .bind(format!("{etab}:{o}"))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        // Un objet a une seule nature, jamais changée (FD-15, R-FC21) : une « autorisation » d'objet
        // `droit_image` ne devient pas la v2 du consentement.
        if let Some(o) = &objet {
            let existante: Option<(String,)> = sqlx::query_as(
                "SELECT type FROM ent_formulaire WHERE etablissement_id = $1 AND objet = $2 LIMIT 1",
            )
            .bind(etab)
            .bind(o)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if existante.is_some_and(|(t,)| t != n.type_formulaire) {
                return Err(EntDbError::SaisieInvalide(
                    "cet objet existe déjà avec une autre nature (consentement, autorisation)",
                ));
            }
        }
        let (version,): (i32,) = sqlx::query_as(
            "SELECT COALESCE(max(version), 0) + 1 FROM ent_formulaire
              WHERE etablissement_id = $1 AND objet IS NOT DISTINCT FROM $2 AND $2 IS NOT NULL",
        )
        .bind(etab)
        .bind(&objet)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_formulaire
                (id, etablissement_id, type, objet, version, titre, texte, audience, cible_ids, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(&n.type_formulaire)
        .bind(&objet)
        .bind(version)
        .bind(&titre)
        .bind(&texte)
        .bind(&n.audience)
        .bind(&cibles)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        self.formulaire(etab, id).await
    }

    async fn formulaire(&self, etab: Uuid, id: Uuid) -> Result<Formulaire, EntDbError> {
        sqlx::query_as(&format!(
            "SELECT {COLONNES_FORMULAIRE} FROM ent_formulaire WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(id)
        .fetch_optional(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)
    }

    pub async fn formulaires_gestion(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
    ) -> Result<Vec<Formulaire>, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        sqlx::query_as(&format!(
            "SELECT {COLONNES_FORMULAIRE} FROM ent_formulaire
              WHERE etablissement_id = $1 ORDER BY created_at DESC LIMIT 200"
        ))
        .bind(etab)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)
    }

    /// Publie sans qualifier le changement : **par prudence**, une version qui en remplace une autre est
    /// publiée comme version de **fond** (les familles sont sollicitées de nouveau, R-FC19). Pour une
    /// correction de forme ou une restriction, voir [`PgEnt::publier_formulaire_qualifie`].
    pub async fn publier_formulaire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Formulaire, EntDbError> {
        self.publier_formulaire_qualifie(etab, acteur, role, id, None).await
    }

    /// Publie une version (FRM-103) et prévient les décideurs concernés.
    ///
    /// `changement` qualifie une version qui en remplace une autre : `forme` (les accords restent valables,
    /// personne n'est sollicité), `fond` (les accords passent `a_renouveler`, les refus restent) ou
    /// `restriction` (accords valables pour les usages restants). Absent : `fond`. Pour une première version
    /// il doit être absent. Une version de numéro inférieur à une version déjà publiée est refusée (FD-14).
    pub async fn publier_formulaire_qualifie(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
        changement: Option<&str>,
    ) -> Result<Formulaire, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        if changement.is_some_and(|c| !["forme", "fond", "restriction"].contains(&c)) {
            return Err(EntDbError::SaisieInvalide("changement : forme, fond ou restriction"));
        }
        let f = self.formulaire(etab, id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        if let Some(o) = &f.objet {
            // Les publications d'un même objet se sérialisent (même verrou que la création des versions).
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 11))")
                .bind(format!("{etab}:{o}"))
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        let mut changement_retenu: Option<&str> = None;
        if let Some(o) = &f.objet {
            let (plus_recente, nb): (Option<i32>, i64) = sqlx::query_as(
                "SELECT max(version), count(*) FROM ent_formulaire
                  WHERE etablissement_id = $1 AND objet = $2 AND id <> $3
                    AND statut IN ('publie', 'archive')",
            )
            .bind(etab)
            .bind(o)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if plus_recente.is_some_and(|v| v > f.version) {
                return Err(EntDbError::ConflitDossier(
                    "une version plus récente de cet objet est déjà publiée (numéro inférieur refusé)",
                ));
            }
            if nb > 0 {
                changement_retenu = Some(changement.unwrap_or("fond"));
            } else if changement.is_some() {
                return Err(EntDbError::SaisieInvalide(
                    "une première version n'a pas de qualification de changement",
                ));
            }
        } else if changement.is_some() {
            return Err(EntDbError::SaisieInvalide("un sondage n'a pas de versions"));
        }
        let publie = sqlx::query(
            "UPDATE ent_formulaire
                SET statut = 'publie', publie_le = now(), publie_par = $3, changement = $4
              WHERE etablissement_id = $1 AND id = $2 AND statut = 'brouillon'",
        )
        .bind(etab)
        .bind(id)
        .bind(acteur)
        .bind(changement_retenu)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if publie.rows_affected() == 0 {
            return Err(EntDbError::ConflitDossier("le formulaire n'est pas un brouillon"));
        }
        if let Some(o) = &f.objet {
            sqlx::query(
                "UPDATE ent_formulaire SET statut = 'archive', archive_le = now(), archive_par = $4
                  WHERE etablissement_id = $1 AND objet = $2 AND id <> $3 AND statut = 'publie'",
            )
            .bind(etab)
            .bind(o)
            .bind(id)
            .bind(acteur)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let evt = tx
            .publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: types_com::FORMULAIRE_PUBLIE.to_string(),
                aggregate: AggregateKind::Annonce,
                aggregate_id: id,
                eleve_id: None,
                payload: json!({
                    "formulaire_id": id, "type": f.type_formulaire, "objet": f.objet,
                    "version": f.version, "audience": f.audience, "changement": changement_retenu,
                }),
                actor_user_id: Some(acteur.to_string()),
            })
            .await?;
        // Forme ou restriction : les accords restent valables, personne n'est sollicité.
        if matches!(changement_retenu, Some("forme" | "restriction")) {
            tx.commit().await.map_err(db)?;
            return self.formulaire(etab, id).await;
        }
        let eleves: Vec<Uuid> = eleves_de_audience(
            &mut tx,
            etab,
            &f.audience,
            &f.cible_ids,
            domain_eleve::horloge::jour_paris(),
        )
        .await?
        .into_iter()
        .collect();
        // Seuls les élèves dont aucune décision ne couvre la version reçoivent une demande (un refus protège
        // déjà ; un accord valable n'est pas redemandé).
        let (objet_lu, form_lu) = match &f.objet {
            Some(o) => (Some(o.as_str()), None),
            None => (None, Some(id)),
        };
        let etats = etats_effectifs(&mut tx, etab, &eleves, objet_lu, form_lu).await?;
        let a_solliciter: Vec<Uuid> = eleves
            .iter()
            .copied()
            .filter(|e| matches!(etats.get(e).copied().unwrap_or("aucune"), "aucune" | "a_renouveler" | "en_attente"))
            .collect();
        let seuls = eleves_decidant_seuls(&mut tx, etab, &a_solliciter).await?;
        let mineurs: Vec<Uuid> = a_solliciter.iter().copied().filter(|e| !seuls.contains(e)).collect();
        let majeurs: Vec<Uuid> = a_solliciter.iter().copied().filter(|e| seuls.contains(e)).collect();
        let mut responsables = responsables_de(&mut tx, etab, &mineurs).await?;
        if f.type_formulaire == "autorisation" {
            // Une autorisation ne se demande qu'aux titulaires qui ont le droit `autoriser_sorties` (FD-10).
            let ayants: Vec<(Uuid,)> = sqlx::query_as(
                "SELECT DISTINCT k.user_id FROM ent_eleve_lien_communicant k
                  WHERE k.etablissement_id = $1 AND k.eleve_id = ANY($2)
                    AND k.autorite_parentale AND k.peut_autoriser_sorties AND k.user_id IS NOT NULL",
            )
            .bind(etab)
            .bind(&mineurs)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let ayants: HashSet<Uuid> = ayants.into_iter().map(|(u,)| u).collect();
            responsables.retain(|r| ayants.contains(&r.user_id));
        }
        // Qui a déjà une décision valable pour cette version n'est pas sollicité de nouveau.
        let seuil = nature_et_seuil(&mut tx, etab, f.objet.as_deref()).await?.1;
        let deja: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT DISTINCT r.signataire_user_id, r.eleve_id FROM ent_formulaire_reponse r
              WHERE r.etablissement_id = $1 AND r.eleve_id = ANY($2)
                AND ($3::text IS NULL OR r.objet = $3) AND ($4::uuid IS NULL OR r.formulaire_id = $4)
                AND r.version >= $5",
        )
        .bind(etab)
        .bind(&mineurs)
        .bind(objet_lu)
        .bind(form_lu)
        .bind(seuil)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let deja: HashSet<Uuid> = deja.into_iter().map(|(u, _)| u).collect();
        responsables.retain(|r| !deja.contains(&r.user_id));
        // Titre neutre : le titre d'un formulaire peut être sensible (R-FC37, FD-08) et ne figure ni dans
        // l'e-mail, ni dans le push, ni dans le SMS (type `formulaire` neutre dans `notifications.rs`).
        let titre = "Un formulaire vous attend";
        let corps = "Un formulaire vous attend dans l'ENT (rubrique Formulaires et consentements).";
        let maintenant = Utc::now();
        for r in responsables {
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: r.user_id,
                    kind: "formulaire",
                    titre,
                    corps,
                    eleve_id: Some(r.eleve_id),
                    notification_id: None,
                    source_type: "systeme",
                    source_id: id,
                    cle_dedup: format!("form:{id}"),
                    droit_requis: Some("lien"),
                    regroupable: true,
                    canaux_voulus: &["email", "push"],
                },
                evt.id,
                maintenant,
            )
            .await?;
        }
        // Élève qui décide seul : prévenu directement, ses parents ne le sont pas (FD-04, FRM-174).
        let comptes: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT c.user_id, c.eleve_id FROM ent_eleve_compte c
              WHERE c.etablissement_id = $1 AND c.eleve_id = ANY($2)",
        )
        .bind(etab)
        .bind(&majeurs)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        for (user_id, eleve_id) in comptes {
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id,
                    kind: "formulaire",
                    titre,
                    corps,
                    eleve_id: Some(eleve_id),
                    notification_id: None,
                    source_type: "systeme",
                    source_id: id,
                    cle_dedup: format!("form:{id}"),
                    droit_requis: None,
                    regroupable: true,
                    canaux_voulus: &["email", "push"],
                },
                evt.id,
                maintenant,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        self.formulaire(etab, id).await
    }

    pub async fn archiver_formulaire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<Formulaire, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "UPDATE ent_formulaire SET statut = 'archive', archive_le = now(), archive_par = $3
              WHERE etablissement_id = $1 AND id = $2 AND statut <> 'archive'",
        )
        .bind(etab)
        .bind(id)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        self.formulaire(etab, id).await
    }

    /// Résultats d'un formulaire (FRM-05) : comptages, jamais d'identité.
    pub async fn resultats_formulaire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        id: Uuid,
    ) -> Result<ResultatsFormulaire, EntDbError> {
        self.exiger_formulaires(etab, acteur, role).await?;
        let f = self.formulaire(etab, id).await?;
        let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cibles: Vec<Uuid> = eleves_de_audience(
            &mut conn,
            etab,
            &f.audience,
            &f.cible_ids,
            domain_eleve::horloge::jour_paris(),
        )
        .await?
        .into_iter()
        .collect();
        let (objet, form) = match &f.objet {
            Some(o) => (Some(o.as_str()), None),
            None => (None, Some(id)),
        };
        let etats = etats_effectifs(&mut conn, etab, &cibles, objet, form).await?;
        // Comptages seulement, jamais d'identité. `refuse` compte aussi les désaccords ; `sans_reponse` tout
        // ce qui ne donne pas de droit sans être un refus (aucune réponse, attente d'un autre titulaire,
        // accord à renouveler).
        let compte = |f: &dyn Fn(&str) -> bool| cibles.iter().filter(|c| f(etats.get(c).copied().unwrap_or("aucune"))).count();
        Ok(ResultatsFormulaire {
            cibles: cibles.len(),
            accepte: compte(&|e| e == "accepte"),
            refuse: compte(&|e| matches!(e, "refuse" | "desaccord")),
            sans_reponse: compte(&|e| matches!(e, "aucune" | "en_attente" | "a_renouveler")),
        })
    }

    // ----- Lecture du personnel (FRM-04) -------------------------------------

    /// Qui consent pour un objet donné, dans une classe : lecture du seul objet
    /// demandé, pour les élèves de la classe.
    pub async fn consentements_classe_for_actor(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        objet: &str,
        classe_id: Uuid,
    ) -> Result<Vec<ConsentementEleve>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::ConsentementsLire)
            .await?;
        if !objet_valide(objet) {
            return Err(EntDbError::SaisieInvalide("objet invalide (minuscules, chiffres et _)"));
        }
        let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let eleves: Vec<Uuid> = eleves_de_audience(
            &mut conn,
            etab,
            "classe",
            &[classe_id],
            domain_eleve::horloge::jour_paris(),
        )
        .await?
        .into_iter()
        .collect();
        let etats = etats_effectifs(&mut conn, etab, &eleves, Some(objet), None).await?;
        let noms: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, nom, prenom FROM ent_eleve
              WHERE etablissement_id = $1 AND id = ANY($2) ORDER BY nom, prenom",
        )
        .bind(etab)
        .bind(&eleves)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        Ok(noms
            .into_iter()
            .map(|(id, nom, prenom)| ConsentementEleve {
                eleve_id: id,
                nom,
                prenom,
                decision: etats.get(&id).copied().unwrap_or("aucune").to_string(),
            })
            .collect())
    }

    // ----- Famille / élève majeur ---------------------------------------------

    /// Enfants pour lesquels `user_id` peut décider : (élève, prénom, qualité, droit de sortie).
    ///
    /// * titulaire : lien **communicant** (FD-02), autorité parentale, élève qui ne décide pas seul ;
    /// * élève : son compte, s'il décide seul (majeur à date certaine, émancipé : FD-04, FD-05).
    async fn signables(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Enfant>, EntDbError> {
        let rows: Vec<(Uuid, String, String, bool)> = sqlx::query_as(
            "SELECT e.id, e.prenom, 'responsable' AS qualite, bool_or(l.peut_autoriser_sorties)
               FROM ent_eleve_lien_communicant l
               JOIN ent_eleve e ON e.id = l.eleve_id AND e.etablissement_id = l.etablissement_id
              WHERE l.etablissement_id = $1 AND l.user_id = $2 AND l.autorite_parentale
                AND NOT ent_eleve_decide_seul($1, e.id)
              GROUP BY e.id, e.prenom
             UNION ALL
             SELECT e.id, e.prenom, 'eleve_majeur', true
               FROM ent_eleve_compte c
               JOIN ent_eleve e ON e.id = c.eleve_id AND e.etablissement_id = c.etablissement_id
              WHERE c.etablissement_id = $1 AND c.user_id = $2
                AND ent_eleve_decide_seul($1, e.id)",
        )
        .bind(etab)
        .bind(user_id)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .map(|(id, prenom, q, peut_sorties)| Enfant {
                id,
                prenom,
                qualite: if q == "eleve_majeur" { "eleve_majeur" } else { "responsable" },
                peut_sorties,
            })
            .collect())
    }

    /// Formulaires publiés à traiter (ou déjà traités) pour mes enfants. Ne montre que **ma** décision (R-FC16).
    pub async fn formulaires_famille(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<FormulaireFamille>, EntDbError> {
        let enfants = self.signables(etab, user_id).await?;
        if enfants.is_empty() {
            return Ok(Vec::new());
        }
        let formulaires: Vec<Formulaire> = sqlx::query_as(&format!(
            "SELECT {COLONNES_FORMULAIRE} FROM ent_formulaire
              WHERE etablissement_id = $1 AND statut = 'publie'
              ORDER BY publie_le DESC LIMIT 100"
        ))
        .bind(etab)
        .fetch_all(&mut *TenantTx::begin(self.pool(), etab).await?)
        .await
        .map_err(db)?;
        let ids: Vec<Uuid> = enfants.iter().map(|e| e.id).collect();
        let mut conn = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        // Mes dernières décisions par (élève, objet ou formulaire), avec leur version.
        let miennes: Vec<(Uuid, Option<String>, Uuid, String, i32)> = sqlx::query_as(
            "SELECT DISTINCT ON (eleve_id, COALESCE(objet, formulaire_id::text))
                    eleve_id, objet, formulaire_id, decision, version
               FROM ent_formulaire_reponse
              WHERE etablissement_id = $1 AND signataire_user_id = $2 AND eleve_id = ANY($3)
              ORDER BY eleve_id, COALESCE(objet, formulaire_id::text), decide_le DESC, id DESC",
        )
        .bind(etab)
        .bind(user_id)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let mut sortie = Vec::new();
        for f in formulaires {
            let cibles = eleves_de_audience(
                &mut conn,
                etab,
                &f.audience,
                &f.cible_ids,
                domain_eleve::horloge::jour_paris(),
            )
            .await?;
            let seuil = nature_et_seuil(&mut conn, etab, f.objet.as_deref()).await?.1;
            for enfant in enfants.iter().filter(|e| cibles.contains(&e.id) && e.peut_repondre_a(&f.type_formulaire)) {
                let ma = miennes.iter().find(|m| {
                    m.0 == enfant.id
                        && match &f.objet {
                            Some(o) => m.1.as_deref() == Some(o.as_str()),
                            None => m.2 == f.id,
                        }
                });
                // Un accord donné avant une version de fond doit être renouvelé ; un refus reste protecteur.
                let a_repondre = match ma {
                    None => true,
                    Some(m) => m.3 == "accepte" && m.4 < seuil,
                };
                sortie.push(FormulaireFamille {
                    formulaire_id: f.id,
                    type_formulaire: f.type_formulaire.clone(),
                    objet: f.objet.clone(),
                    version: f.version,
                    titre: f.titre.clone(),
                    texte: f.texte.clone(),
                    eleve_id: enfant.id,
                    eleve_prenom: enfant.prenom.clone(),
                    a_repondre,
                    ma_decision: ma.map(|m| m.3.clone()),
                    etat: None,
                });
            }
        }
        Ok(sortie)
    }

    /// Enregistre ma décision (signature) pour un enfant. Idempotent : répéter la
    /// même décision pour la même version n'écrit rien. `retire` suppose une acceptation antérieure.
    /// Retourne **ma** décision, jamais l'état global (R-FC58).
    pub async fn repondre_formulaire(
        &self,
        etab: Uuid,
        user_id: Uuid,
        formulaire_id: Uuid,
        r: ReponseFormulaire,
    ) -> Result<EtatConsentement, EntDbError> {
        if !["accepte", "refuse", "retire"].contains(&r.decision.as_str()) {
            return Err(EntDbError::SaisieInvalide("décision inconnue"));
        }
        let commentaire = r
            .commentaire
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_string);
        if commentaire.as_deref().is_some_and(|c| c.chars().count() > 1000) {
            return Err(EntDbError::SaisieInvalide("commentaire trop long"));
        }
        let f = self.formulaire(etab, formulaire_id).await?;
        if f.statut != "publie" {
            return Err(EntDbError::ConflitDossier("ce formulaire n'est plus ouvert aux réponses"));
        }
        // Qualité du signataire (indiscernable d'un inconnu s'il n'a pas le droit : interdiction de
        // communication, lien sans le droit de sortie pour une autorisation, majeur qui décide seul).
        let enfants = self.signables(etab, user_id).await?;
        let Some(enfant) = enfants
            .iter()
            .find(|e| e.id == r.eleve_id && e.peut_repondre_a(&f.type_formulaire))
        else {
            return Err(introuvable());
        };
        let qualite = enfant.qualite;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let cibles = eleves_de_audience(
            &mut tx,
            etab,
            &f.audience,
            &f.cible_ids,
            domain_eleve::horloge::jour_paris(),
        )
        .await?;
        if !cibles.contains(&r.eleve_id) {
            return Err(introuvable());
        }
        // Les réponses d'un même signataire pour un même sujet se sérialisent.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 12))")
            .bind(format!("{etab}:{user_id}:{}", r.eleve_id))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let derniere: Option<(String, i32)> = sqlx::query_as(
            "SELECT decision, version FROM ent_formulaire_reponse
              WHERE etablissement_id = $1 AND signataire_user_id = $2 AND eleve_id = $3
                AND ($4::text IS NULL OR objet = $4)
                AND ($5::uuid IS NULL OR formulaire_id = $5)
              ORDER BY decide_le DESC, id DESC LIMIT 1",
        )
        .bind(etab)
        .bind(user_id)
        .bind(r.eleve_id)
        .bind(&f.objet)
        .bind(if f.objet.is_none() { Some(formulaire_id) } else { None })
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if r.decision == "retire" {
            if f.type_formulaire == "sondage" {
                return Err(EntDbError::SaisieInvalide("un sondage ne se retire pas"));
            }
            if derniere.as_ref().map(|d| d.0.as_str()) != Some("accepte") {
                return Err(EntDbError::ConflitDossier("aucun consentement à retirer"));
            }
        }
        // Même décision sur la même version : rien à écrire. Une décision identique sur une **autre** version
        // est une nouvelle décision (accord renouvelé après une version de fond, FD-01).
        let identique = derniere.as_ref().is_some_and(|d| d.0 == r.decision && d.1 == f.version);
        if !identique {
            let id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO ent_formulaire_reponse
                    (id, etablissement_id, formulaire_id, objet, version, eleve_id,
                     signataire_user_id, qualite, decision, commentaire, empreinte)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
            )
            .bind(id)
            .bind(etab)
            .bind(formulaire_id)
            .bind(&f.objet)
            .bind(f.version)
            .bind(r.eleve_id)
            .bind(user_id)
            .bind(qualite)
            .bind(&r.decision)
            .bind(&commentaire)
            .bind(sha256_hex(&format!(
                "{}|{}|{}|{}",
                f.objet.as_deref().unwrap_or(""),
                f.version,
                f.titre,
                f.texte
            )))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let type_evt = match (f.type_formulaire.as_str(), r.decision.as_str()) {
                ("sondage", _) => types_com::FORMULAIRE_REPONDU,
                (_, "accepte") => types_com::CONSENTEMENT_DONNE,
                (_, "refuse") => types_com::CONSENTEMENT_REFUSE,
                _ => types_com::CONSENTEMENT_RETIRE,
            };
            tx.publier(MetierEventDraft {
                etablissement_id: etab,
                type_name: type_evt.to_string(),
                aggregate: AggregateKind::Annonce,
                aggregate_id: formulaire_id,
                eleve_id: Some(r.eleve_id),
                // Identifiants seulement : ni commentaire ni texte libre.
                payload: json!({
                    "formulaire_id": formulaire_id, "objet": f.objet, "version": f.version,
                    "decision_id": id,
                }),
                actor_user_id: Some(user_id.to_string()),
            })
            .await?;
        }
        tx.commit().await.map_err(db)?;
        Ok(EtatConsentement {
            eleve_id: r.eleve_id,
            objet: f.objet.clone().unwrap_or_default(),
            decision: r.decision,
        })
    }

    /// Photos masquées par un refus du droit à l'image, parmi ces élèves.
    pub async fn eleves_photo_masquee(
        &self,
        etab: Uuid,
        eleves: &[Uuid],
    ) -> Result<BTreeSet<Uuid>, EntDbError> {
        Ok(self
            .eleves_ayant_refuse(etab, eleves, OBJET_DROIT_IMAGE)
            .await?
            .into_iter()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d<'a>(qualite: &'a str, decision: &'a str, version: i32) -> DecisionRetenue<'a> {
        DecisionRetenue { qualite, decision, version }
    }

    #[test]
    fn regle_un_refus_seul_ou_desaccord() {
        // Refus sans accord : refus ; refus et accord : désaccord connu (R-FC7), la photo reste masquée.
        assert_eq!(etat_effectif(&[d("responsable", "refuse", 1)], false, Regle::Un, 2, 1), "refuse");
        let mixte = [d("responsable", "accepte", 1), d("responsable", "refuse", 1)];
        assert_eq!(etat_effectif(&mixte, false, Regle::Un, 2, 1), "desaccord");
        let retire = [d("responsable", "accepte", 1), d("responsable", "retire", 1)];
        assert_eq!(etat_effectif(&retire, false, Regle::Un, 2, 1), "desaccord");
        assert!(!donne_droit("desaccord"));
    }

    #[test]
    fn acceptation_et_absence() {
        assert_eq!(etat_effectif(&[d("responsable", "accepte", 1)], false, Regle::Un, 2, 1), "accepte");
        assert_eq!(etat_effectif(&[], false, Regle::Un, 2, 1), "aucune");
        assert!(!donne_droit("aucune"), "sans réponse : jamais un droit (R-FC18)");
    }

    #[test]
    fn regle_tous_exige_chaque_titulaire() {
        // FD-17 : un seul accord ne suffit plus pour l'image.
        let un = [d("responsable", "accepte", 1)];
        assert_eq!(etat_effectif(&un, false, Regle::Tous, 2, 1), "en_attente");
        let deux = [d("responsable", "accepte", 1), d("responsable", "accepte", 1)];
        assert_eq!(etat_effectif(&deux, false, Regle::Tous, 2, 1), "accepte");
        // Un titulaire unique : son accord suffit.
        assert_eq!(etat_effectif(&un, false, Regle::Tous, 1, 1), "accepte");
        let refus = [d("responsable", "accepte", 1), d("responsable", "refuse", 1)];
        assert_eq!(etat_effectif(&refus, false, Regle::Tous, 2, 1), "refuse");
        assert!(!donne_droit("en_attente"));
    }

    #[test]
    fn version_de_fond_rend_a_renouveler_mais_pas_les_refus() {
        // FD-01 : accord sur la v1, version de fond v2 (seuil 2) ⇒ à renouveler.
        let v1 = [d("responsable", "accepte", 1)];
        assert_eq!(etat_effectif(&v1, false, Regle::Un, 1, 2), "a_renouveler");
        let v2 = [d("responsable", "accepte", 2)];
        assert_eq!(etat_effectif(&v2, false, Regle::Un, 1, 2), "accepte");
        // Le refus de la v1 reste protecteur.
        assert_eq!(etat_effectif(&[d("responsable", "refuse", 1)], false, Regle::Un, 1, 2), "refuse");
        // `tous` : un titulaire à jour, l'autre périmé ⇒ à renouveler ; un seul répondant ⇒ en attente.
        let mixte = [d("responsable", "accepte", 2), d("responsable", "accepte", 1)];
        assert_eq!(etat_effectif(&mixte, false, Regle::Tous, 2, 2), "a_renouveler");
    }

    #[test]
    fn majeur_seul_son_avis_compte() {
        // Un parent refuse mais l'élève décide seul : seule la décision de l'élève compte.
        let m = [d("responsable", "refuse", 1), d("eleve_majeur", "accepte", 1)];
        assert_eq!(etat_effectif(&m, true, Regle::Tous, 2, 1), "accepte");
        // Mineur : la décision de l'élève (ancienne qualité) ne compte pas.
        assert_eq!(etat_effectif(&[d("eleve_majeur", "accepte", 1)], false, Regle::Un, 1, 1), "aucune");
        assert_eq!(
            etat_effectif(&[d("responsable", "accepte", 1)], true, Regle::Un, 1, 1),
            "aucune",
            "parents sans effet sur un élève qui décide seul (R-FC12)"
        );
    }

    #[test]
    fn regles_par_defaut_et_objets_sensibles() {
        assert_eq!(regle_par_defaut(Some("droit_image")), Regle::Tous);
        assert_eq!(regle_par_defaut(Some("sortie_musee")), Regle::Un);
        assert_eq!(regle_par_defaut(None), Regle::Un);
        assert!(objet_sensible("sante_allergies") && objet_sensible("fiche_medicale") && objet_sensible("religion_cantine"));
        assert!(!objet_sensible("droit_image") && !objet_sensible("orientation_scolaire"));
        assert!(objet_valide("droit_image") && !objet_valide("Droit Image!") && !objet_valide("d"));
    }
}
