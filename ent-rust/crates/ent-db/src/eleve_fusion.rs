//! Socle 2 (l'élève), lot E6 — fusion de deux fiches d'élèves et défusion à l'identique
//! (ELV-23, ELV-24, §7.10 de `docs/socles/02_ELEVE.md`).
//!
//! ## Principe
//!
//! L'élève **absorbé** n'est jamais supprimé : il reste, marqué `fusionne_dans`, et ce qui ne passe
//! pas reste sur lui. Chaque table qui porte `eleve_id` est déclarée dans le registre
//! [`domain_eleve::TABLES_ELEVE`] (un test de garde échoue si une table n'y figure pas) avec une
//! stratégie : *déplacer* (les lignes passent à l'élève conservé) ou *rester* (historique ou état
//! propre à la fiche : journaux, événements, versions d'identité, bulletins émis…).
//!
//! * **Aperçu** et **fusion** exécutent le MÊME code : l'aperçu l'exécute dans une transaction
//!   qu'il abandonne. Ce que l'aperçu annonce est donc exactement ce que la fusion fera.
//! * **Conflits** : si une ligne de l'absorbé heurte une contrainte d'unicité ou d'exclusion
//!   (même année d'inscription, même INE, scolarisations qui se chevauchent, deux comptes…), la
//!   ligne **de l'élève conservé l'emporte** et celle de l'absorbé reste sur l'absorbé. Chaque
//!   conflit est listé (table et clé, jamais le contenu) et doit être **acquitté** par l'appelant :
//!   la fusion est refusée tant qu'un conflit n'a pas été regardé. Pour garder la ligne de
//!   l'absorbé, on fusionne dans l'autre sens (l'aperçu des deux sens est possible).
//! * **Registre** : pour chaque ligne déplacée, la table, la clé et l'**empreinte** de la ligne
//!   après déplacement (`ent_eleve_fusion_ligne`). Un déplacement ne change que `eleve_id` : la
//!   défusion le remet donc **à l'identique**. Si une ligne déplacée a été **modifiée** depuis (son
//!   empreinte a changé) ou a disparu, la défusion est refusée et la liste est montrée.
//! * Délai d'annulation : 30 jours (Q-E16). Les données saisies depuis la fusion sur la fiche
//!   conservée y restent.
//! * Une fusion s'annule dans l'ordre inverse : si l'élève conservé a lui-même été absorbé depuis,
//!   ses lignes ont bougé et la défusion est refusée jusqu'à l'annulation de la fusion plus récente.
//!
//! ## Droits
//!
//! `eleves.fusionner` à la portée établissement (Admin, Direction). Un élève en confidentialité
//! renforcée exige en plus `eleves.confidentiel.gerer` : sans lui, l'élève « n'existe pas ». Pas de
//! décision sur un élève avec lequel l'agent est en conflit d'intérêts (R-E10).

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use domain_eleve::{
    domaine_fusion, domaine_protege, tables_a_deplacer, DomainError, GroupeChamps, StrategieFusion,
    TableEleve, DELAI_DEFUSION_JOURS, TABLES_ELEVE,
};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::eleve_champs::EleveVue;
use crate::eleve_repo::PgEleveRepo;
use crate::outbox::Publier;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Une ligne de l'absorbé qui ne peut pas passer à l'élève conservé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflitFusion {
    pub table: String,
    /// Clé de la ligne (colonnes de la clé primaire hors `eleve_id`, en texte).
    pub cle: Value,
    /// Domaine lisible (« Inscriptions », « Absences et retards »…).
    pub domaine: String,
}

impl ConflitFusion {
    fn cle_texte(&self) -> String {
        cle_texte(&self.table, &self.cle)
    }
}

/// Texte canonique d'une clé (indépendant de l'ordre des champs JSON).
fn cle_texte(table: &str, cle: &Value) -> String {
    let m: BTreeMap<String, String> = cle
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                .collect()
        })
        .unwrap_or_default();
    format!("{table}|{m:?}")
}

/// Ce qu'une table devient dans la fusion (ligne de l'aperçu).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneApercu {
    pub domaine: String,
    /// Nom technique : seulement pour les domaines non protégés.
    pub table: Option<String>,
    pub a_deplacer: usize,
    pub en_conflit: usize,
    /// Lignes qui restent sur l'absorbé (historiques).
    pub restent: usize,
}

/// Mise en garde de l'aperçu (codes stables, texte côté écran).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvertissementFusion {
    /// Au moins un des deux élèves est en confidentialité renforcée.
    EleveProtege,
    /// La paire a été déclarée distincte (jumeaux, homonymes) et l'identité n'a pas changé.
    DeclareeDistincte,
    /// Les deux fiches portent des INE actifs différents (le conservé garde le sien).
    IneDifferents,
    /// Deux comptes d'élève : celui de l'absorbé reste sur l'absorbé (à fermer à la main).
    DeuxComptes,
    /// L'élève absorbé reste scolarisé après la fusion (scolarisation en conflit).
    AbsorbeEncoreScolarise,
}

/// Aperçu complet d'une fusion (rien n'est écrit).
#[derive(Debug, Clone)]
pub struct ApercuFusion {
    pub absorbe: EleveVue,
    pub conserve: EleveVue,
    pub lignes: Vec<LigneApercu>,
    pub conflits: Vec<ConflitFusion>,
    /// Noms des champs d'identité qui diffèrent (seulement si l'acteur voit l'état civil).
    pub differences: Vec<&'static str>,
    pub avertissements: Vec<AvertissementFusion>,
    /// Faux si la fusion est impossible telle quelle (élève déjà absorbé…).
    pub possible: bool,
}

/// Demande de fusion (ELV-23).
#[derive(Debug, Clone)]
pub struct DemandeFusion {
    pub etablissement_id: Uuid,
    pub absorbe_id: Uuid,
    pub conserve_id: Uuid,
    /// `doublon_saisie`, `doublon_import`, `erreur_creation`, `autre` (texte libre obligatoire).
    pub motif_code: String,
    pub motif: Option<String>,
    /// Conflits regardés et acceptés : « la ligne du conservé l'emporte ».
    pub conflits_acquittes: Vec<ConflitFusion>,
    pub acteur: Uuid,
}

/// Une fusion enregistrée.
#[derive(Debug, Clone)]
pub struct FusionEleves {
    pub id: Uuid,
    pub absorbe_id: Uuid,
    pub conserve_id: Uuid,
    pub motif_code: String,
    pub motif: Option<String>,
    pub conflits: Vec<ConflitFusion>,
    pub resume: Vec<LigneApercu>,
    pub fusionne_par: Option<Uuid>,
    pub fusionne_le: DateTime<Utc>,
    pub annule_par: Option<Uuid>,
    pub annule_le: Option<DateTime<Utc>>,
    /// Dernier jour où l'annulation est possible.
    pub annulable_jusqu_au: DateTime<Utc>,
}

/// Résultat d'une demande de fusion.
#[derive(Debug, Clone)]
pub enum ResultatFusion {
    Faite(FusionEleves),
    /// Des conflits n'ont pas été acquittés : rien n'est écrit.
    ConflitsAResoudre(Vec<ConflitFusion>),
}

/// Une ligne déplacée qui n'est plus telle que la fusion l'a laissée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LigneModifiee {
    pub domaine: String,
    pub table: String,
    pub cle: Value,
    /// `modifiee` (contenu changé) ou `absente` (supprimée ou passée à un autre élève).
    pub raison: &'static str,
}

/// Résultat d'une demande de défusion.
#[derive(Debug, Clone)]
pub enum ResultatDefusion {
    Faite(FusionEleves),
    /// Des lignes déplacées ont été modifiées depuis : rien n'est restauré.
    LignesModifiees(Vec<LigneModifiee>),
}

// ───────────────────────── Aides SQL (registre statique, jamais de saisie) ─────────────────────────

fn guillemets(c: &str) -> String {
    format!("\"{c}\"")
}

/// `jsonb_build_object('id', t."id"::text, …)` : la clé d'une ligne, hors `eleve_id`.
fn objet_cle(t: &TableEleve) -> String {
    let cols: Vec<String> = t
        .cle_hors_eleve()
        .iter()
        .map(|c| format!("'{c}', t.{}::text", guillemets(c)))
        .collect();
    format!("jsonb_build_object({})", cols.join(", "))
}

/// `k("id" text, …)` : la définition de colonnes pour `jsonb_to_record(set)`.
fn definition_cle(t: &TableEleve) -> String {
    let cols: Vec<String> = t.cle_hors_eleve().iter().map(|c| format!("{} text", guillemets(c))).collect();
    format!("k({})", cols.join(", "))
}

/// `t."id"::text = k."id"` : la jointure d'une ligne avec sa clé.
fn jointure_cle(t: &TableEleve) -> String {
    t.cle_hors_eleve()
        .iter()
        .map(|c| format!("t.{0}::text = k.{0}", guillemets(c)))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn est_conflit_d_unicite(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if matches!(d.code().as_deref(), Some("23505") | Some("23P01")))
}

async fn point_de_reprise(tx: &mut TenantTx<'static>, nom: &str) -> Result<(), sqlx::Error> {
    sqlx::query(&format!("SAVEPOINT {nom}")).execute(&mut **tx).await.map(|_| ())
}
async fn revenir_au_point(tx: &mut TenantTx<'static>, nom: &str) -> Result<(), sqlx::Error> {
    sqlx::query(&format!("ROLLBACK TO SAVEPOINT {nom}")).execute(&mut **tx).await?;
    sqlx::query(&format!("RELEASE SAVEPOINT {nom}")).execute(&mut **tx).await.map(|_| ())
}
async fn liberer_point(tx: &mut TenantTx<'static>, nom: &str) -> Result<(), sqlx::Error> {
    sqlx::query(&format!("RELEASE SAVEPOINT {nom}")).execute(&mut **tx).await.map(|_| ())
}

/// Drapeaux locaux à la transaction : la garde des tables immuables accepte le repointage de
/// `eleve_id` (293), les ponts vers les anciennes tables de scolarité se taisent (le déplacement
/// des deux côtés est fait par la fusion elle-même).
async fn armer_drapeaux(tx: &mut TenantTx<'static>) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT set_config('ent.fusion_eleve', 'on', true), set_config('ent.inscription_pont', 'on', true)")
        .execute(&mut **tx)
        .await
        .map(|_| ())
}

/// Colonnes d'`ent_eleve` qui sont des projections de tables déplacées (statut dérivé, dates,
/// INE, badge) : sauvegardées avant la fusion, restaurées à la défusion.
const PROJECTIONS: &str = "status, statut, date_entree, date_sortie, motif_sortie, ine, numero_badge";

async fn lire_projections(tx: &mut TenantTx<'static>, etab: Uuid, eleve: Uuid) -> Result<Value, sqlx::Error> {
    sqlx::query_scalar(&format!(
        "SELECT to_jsonb(p) FROM (SELECT {PROJECTIONS} FROM ent_eleve WHERE etablissement_id = $1 AND id = $2) p"
    ))
    .bind(etab)
    .bind(eleve)
    .fetch_one(&mut **tx)
    .await
}

async fn ecrire_projections(
    tx: &mut TenantTx<'static>,
    etab: Uuid,
    eleve: Uuid,
    p: &Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "SELECT set_config('ent.scolarite_projection', 'on', true), set_config('ent.identifiant_projection', 'on', true)",
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE ent_eleve e SET status = p.status, statut = p.statut, date_entree = p.date_entree,
                date_sortie = p.date_sortie, motif_sortie = p.motif_sortie, ine = p.ine,
                numero_badge = p.numero_badge
           FROM jsonb_to_record($3::jsonb) AS p(status text, statut text, date_entree date, date_sortie date,
                                                motif_sortie text, ine text, numero_badge text)
          WHERE e.etablissement_id = $1 AND e.id = $2",
    )
    .bind(etab)
    .bind(eleve)
    .bind(p)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "SELECT set_config('ent.scolarite_projection', 'off', true), set_config('ent.identifiant_projection', 'off', true)",
    )
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Projections recalculées par la base (après un déplacement) pour un élève.
async fn recalculer_projections(tx: &mut TenantTx<'static>, etab: Uuid, eleve: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT ent_eleve_scolarite_projeter($1, $2)").bind(etab).bind(eleve).execute(&mut **tx).await?;
    for systeme in ["INE", "BADGE"] {
        sqlx::query("SELECT ent_eleve_identifiant_projeter($1, $2, $3)")
            .bind(etab)
            .bind(eleve)
            .bind(systeme)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// Empreintes (`md5` de l'image JSON de la ligne) des lignes `cles` de `table` portées par `eleve`.
async fn empreintes(
    tx: &mut TenantTx<'static>,
    t: &TableEleve,
    etab: Uuid,
    eleve: Uuid,
    cles: &[Value],
) -> Result<HashMap<String, String>, sqlx::Error> {
    if cles.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT {obj} AS cle, md5(to_jsonb(t.*)::text) AS empreinte
           FROM {table} t JOIN jsonb_to_recordset($3::jsonb) AS {def} ON {jointure}
          WHERE t.etablissement_id = $1 AND t.eleve_id = $2",
        obj = objet_cle(t),
        table = t.table,
        def = definition_cle(t),
        jointure = jointure_cle(t),
    );
    let lignes: Vec<(Value, String)> = sqlx::query_as(&sql)
        .bind(etab)
        .bind(eleve)
        .bind(Value::Array(cles.to_vec()))
        .fetch_all(&mut **tx)
        .await?;
    Ok(lignes.into_iter().map(|(c, e)| (cle_texte(t.table, &c), e)).collect())
}

// ───────────────────────── Plan : le déplacement lui-même ─────────────────────────

struct PlanTable {
    table: &'static TableEleve,
    deplacees: Vec<Value>,
    conflits: Vec<Value>,
    restent: usize,
}

struct Plan {
    tables: Vec<PlanTable>,
}

impl Plan {
    fn conflits(&self) -> Vec<ConflitFusion> {
        self.tables
            .iter()
            .flat_map(|p| {
                p.conflits.iter().map(|c| ConflitFusion {
                    table: p.table.table.to_string(),
                    cle: c.clone(),
                    domaine: domaine_fusion(p.table.table).to_string(),
                })
            })
            .collect()
    }

    /// Lignes de l'aperçu, regroupées par domaine ; les domaines protégés ne donnent qu'un total.
    fn lignes(&self) -> Vec<LigneApercu> {
        let mut par_domaine: BTreeMap<String, LigneApercu> = BTreeMap::new();
        for p in &self.tables {
            if p.deplacees.is_empty() && p.conflits.is_empty() && p.restent == 0 {
                continue;
            }
            let domaine = domaine_fusion(p.table.table).to_string();
            let protege = domaine_protege(p.table.table);
            let cle = if protege { domaine.clone() } else { format!("{domaine}|{}", p.table.table) };
            let l = par_domaine.entry(cle).or_insert_with(|| LigneApercu {
                domaine: domaine.clone(),
                table: (!protege).then(|| p.table.table.to_string()),
                a_deplacer: 0,
                en_conflit: 0,
                restent: 0,
            });
            l.a_deplacer += p.deplacees.len();
            l.en_conflit += p.conflits.len();
            l.restent += p.restent;
        }
        par_domaine.into_values().collect()
    }
}

async fn compter(tx: &mut TenantTx<'static>, t: &TableEleve, etab: Uuid, eleve: Uuid) -> Result<usize, sqlx::Error> {
    let n: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM {} t WHERE t.etablissement_id = $1 AND t.eleve_id = $2",
        t.table
    ))
    .bind(etab)
    .bind(eleve)
    .fetch_one(&mut **tx)
    .await?;
    Ok(usize::try_from(n).unwrap_or(0))
}

/// Déplace les lignes d'UNE table : d'un coup si aucune contrainte ne s'y oppose, sinon ligne par
/// ligne (chacune sous un point de reprise) pour isoler les conflits.
async fn deplacer_table(
    tx: &mut TenantTx<'static>,
    t: &'static TableEleve,
    etab: Uuid,
    absorbe: Uuid,
    conserve: Uuid,
) -> Result<(Vec<Value>, Vec<Value>), EntDbError> {
    armer_drapeaux(tx).await?;
    point_de_reprise(tx, "fusion_table").await?;
    let tout = format!(
        "UPDATE {table} AS t SET eleve_id = $3 WHERE t.etablissement_id = $1 AND t.eleve_id = $2 RETURNING {obj}",
        table = t.table,
        obj = objet_cle(t)
    );
    match sqlx::query_scalar::<_, Value>(&tout)
        .bind(etab)
        .bind(absorbe)
        .bind(conserve)
        .fetch_all(&mut **tx)
        .await
    {
        Ok(cles) => {
            liberer_point(tx, "fusion_table").await?;
            return Ok((cles, Vec::new()));
        }
        Err(e) if est_conflit_d_unicite(&e) => revenir_au_point(tx, "fusion_table").await?,
        Err(e) => return Err(e.into()),
    }
    // Conflit quelque part dans la table : ligne par ligne.
    let cles: Vec<Value> = sqlx::query_scalar(&format!(
        "SELECT {obj} FROM {table} t WHERE t.etablissement_id = $1 AND t.eleve_id = $2 ORDER BY 1::text",
        obj = objet_cle(t),
        table = t.table
    ))
    .bind(etab)
    .bind(absorbe)
    .fetch_all(&mut **tx)
    .await?;
    let une = format!(
        "UPDATE {table} AS t SET eleve_id = $3 FROM jsonb_to_record($4::jsonb) AS {def}
          WHERE t.etablissement_id = $1 AND t.eleve_id = $2 AND {jointure}",
        table = t.table,
        def = definition_cle(t),
        jointure = jointure_cle(t)
    );
    let (mut deplacees, mut conflits) = (Vec::new(), Vec::new());
    for cle in cles {
        point_de_reprise(tx, "fusion_ligne").await?;
        match sqlx::query(&une).bind(etab).bind(absorbe).bind(conserve).bind(&cle).execute(&mut **tx).await {
            Ok(_) => {
                liberer_point(tx, "fusion_ligne").await?;
                deplacees.push(cle);
            }
            Err(e) if est_conflit_d_unicite(&e) => {
                revenir_au_point(tx, "fusion_ligne").await?;
                conflits.push(cle);
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok((deplacees, conflits))
}

/// Exécute tous les déplacements dans la transaction de l'appelant (qui décide de valider ou
/// d'abandonner) et recalcule les projections des deux élèves.
async fn planifier(
    tx: &mut TenantTx<'static>,
    etab: Uuid,
    absorbe: Uuid,
    conserve: Uuid,
) -> Result<Plan, EntDbError> {
    let mut tables = Vec::new();
    for t in TABLES_ELEVE {
        if t.strategie == StrategieFusion::Reste {
            let restent = compter(tx, t, etab, absorbe).await?;
            tables.push(PlanTable { table: t, deplacees: Vec::new(), conflits: Vec::new(), restent });
        }
    }
    for t in tables_a_deplacer() {
        let (deplacees, conflits) = deplacer_table(tx, t, etab, absorbe, conserve).await?;
        tables.push(PlanTable { table: t, deplacees, conflits, restent: 0 });
    }
    recalculer_projections(tx, etab, absorbe).await?;
    recalculer_projections(tx, etab, conserve).await?;
    Ok(Plan { tables })
}

// ───────────────────────── Service ─────────────────────────

#[derive(sqlx::FromRow)]
struct LigneFusion {
    id: Uuid,
    absorbe_id: Uuid,
    conserve_id: Uuid,
    motif_code: String,
    motif: Option<String>,
    conflits_resolus: Value,
    resume: Value,
    fusionne_par: Option<Uuid>,
    fusionne_le: DateTime<Utc>,
    annule_par: Option<Uuid>,
    annule_le: Option<DateTime<Utc>>,
}

impl LigneFusion {
    fn en_fusion(self) -> FusionEleves {
        let conflits: Vec<ConflitFusion> = serde_json::from_value(self.conflits_resolus).unwrap_or_default();
        let resume: Vec<LigneApercu> = self
            .resume
            .get("lignes")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|l| LigneApercu {
                        domaine: l["domaine"].as_str().unwrap_or_default().to_string(),
                        table: l["table"].as_str().map(str::to_string),
                        a_deplacer: l["a_deplacer"].as_u64().unwrap_or(0) as usize,
                        en_conflit: l["en_conflit"].as_u64().unwrap_or(0) as usize,
                        restent: l["restent"].as_u64().unwrap_or(0) as usize,
                    })
                    .collect()
            })
            .unwrap_or_default();
        FusionEleves {
            id: self.id,
            absorbe_id: self.absorbe_id,
            conserve_id: self.conserve_id,
            motif_code: self.motif_code,
            motif: self.motif,
            conflits,
            resume,
            fusionne_par: self.fusionne_par,
            fusionne_le: self.fusionne_le,
            annule_par: self.annule_par,
            annule_le: self.annule_le,
            annulable_jusqu_au: self.fusionne_le + Duration::days(DELAI_DEFUSION_JOURS),
        }
    }
}

const COLONNES_FUSION: &str = "id, absorbe_id, conserve_id, motif_code, motif, conflits_resolus, resume,
    fusionne_par, fusionne_le, annule_par, annule_le";

#[derive(sqlx::FromRow)]
struct EtatEleve {
    id: Uuid,
    fusionne_dans: Option<Uuid>,
    protege: bool,
    ine: Option<String>,
    nom: String,
    prenom: String,
    nom_naissance: Option<String>,
    sexe: Option<String>,
    date_naissance: Option<chrono::NaiveDate>,
    prenoms: Option<Vec<String>>,
}

fn motif_valide(code: &str, texte: Option<&str>) -> Result<(), EntDbError> {
    if !["doublon_saisie", "doublon_import", "erreur_creation", "autre"].contains(&code) {
        return Err(EntDbError::SaisieInvalide("motif de fusion inconnu"));
    }
    match texte.map(str::trim) {
        Some(t) if t.chars().count() > 500 => Err(EntDbError::SaisieInvalide("motif trop long (500 caractères)")),
        None | Some("") if code == "autre" => Err(EntDbError::SaisieInvalide("motif libre obligatoire pour « autre »")),
        _ => Ok(()),
    }
}

impl PgEnt {
    async fn exiger_droit_fusion(&self, etab: Uuid, acteur: Uuid, role: &str) -> Result<(), EntDbError> {
        if self.a_action_etablissement(etab, acteur, role, Action::ElevesFusionner).await? {
            Ok(())
        } else {
            Err(EntDbError::Forbidden)
        }
    }

    /// Verrouille les deux élèves (ordre des identifiants : pas d'interblocage entre deux
    /// fusions croisées) et renvoie leur état. Un élève protégé sans G6 « n'existe pas ».
    async fn verrouiller_paire(
        &self,
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        absorbe: Uuid,
        conserve: Uuid,
    ) -> Result<(EtatEleve, EtatEleve), EntDbError> {
        if absorbe == conserve {
            return Err(EntDbError::SaisieInvalide("une fiche ne se fusionne pas avec elle-même"));
        }
        let lignes: Vec<EtatEleve> = sqlx::query_as(
            "SELECT e.id, e.fusionne_dans, ent_eleve_confidentiel(e.etablissement_id, e.id) AS protege,
                    e.ine, e.nom, e.prenom, e.nom_naissance, e.sexe, e.date_naissance, e.prenoms
               FROM ent_eleve e WHERE e.etablissement_id = $1 AND e.id IN ($2, $3) ORDER BY e.id FOR UPDATE",
        )
        .bind(etab)
        .bind(absorbe)
        .bind(conserve)
        .fetch_all(&mut **tx)
        .await?;
        if lignes.len() != 2 {
            return Err(DomainError::NotFound.into());
        }
        if lignes.iter().any(|l| l.protege) {
            let profil = self.profil_champs(etab, acteur, role).await?;
            if !profil.ouvert_partout(GroupeChamps::Sensible) {
                return Err(DomainError::NotFound.into());
            }
        }
        // R-E10 : pas de décision sur un élève avec lequel l'agent est en conflit d'intérêts.
        self.exiger_pas_de_conflit(etab, acteur, role, absorbe).await?;
        self.exiger_pas_de_conflit(etab, acteur, role, conserve).await?;
        let mut it = lignes.into_iter();
        let (x, y) = (it.next().expect("deux lignes"), it.next().expect("deux lignes"));
        Ok(if x.id == absorbe { (x, y) } else { (y, x) })
    }

    /// ELV-23 (aperçu) : ce que la fusion ferait, sans rien écrire.
    pub async fn apercu_fusion_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        absorbe_id: Uuid,
        conserve_id: Uuid,
    ) -> Result<ApercuFusion, EntDbError> {
        self.exiger_droit_fusion(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (absorbe, conserve) = self.verrouiller_paire(&mut tx, etab, acteur, role, absorbe_id, conserve_id).await?;
        let profil = self.profil_champs(etab, acteur, role).await?;
        let repo = PgEleveRepo::new(self.pool().clone());
        let eleves = repo.list_by_ids(etab, &[absorbe_id, conserve_id]).await?;
        let trouve = |id: Uuid| eleves.iter().find(|e| e.id == id).cloned().ok_or(DomainError::NotFound);
        let (ea, ec) = (trouve(absorbe_id)?, trouve(conserve_id)?);
        let possible = absorbe.fusionne_dans.is_none() && conserve.fusionne_dans.is_none();
        let mut avertissements = Vec::new();
        let mut differences: Vec<&'static str> = Vec::new();
        let (mut lignes, mut conflits) = (Vec::new(), Vec::new());
        if possible {
            let plan = planifier(&mut tx, etab, absorbe_id, conserve_id).await?;
            lignes = plan.lignes();
            conflits = plan.conflits();
            if conflits.iter().any(|c| c.table == "ent_eleve_compte") {
                avertissements.push(AvertissementFusion::DeuxComptes);
            }
            let statut_absorbe: Option<String> =
                sqlx::query_scalar("SELECT statut FROM ent_eleve WHERE etablissement_id = $1 AND id = $2")
                    .bind(etab)
                    .bind(absorbe_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if statut_absorbe.as_deref() == Some("scolarise") {
                avertissements.push(AvertissementFusion::AbsorbeEncoreScolarise);
            }
        }
        // L'aperçu s'abandonne : `tx` n'est jamais validée.
        drop(tx);
        if absorbe.protege || conserve.protege {
            avertissements.push(AvertissementFusion::EleveProtege);
        }
        if let (Some(a), Some(b)) = (&absorbe.ine, &conserve.ine) {
            if !a.eq_ignore_ascii_case(b) {
                avertissements.push(AvertissementFusion::IneDifferents);
            }
        }
        let (x, y) = domain_eleve::paire_ordonnee(absorbe_id, conserve_id);
        let distincte: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_eleve_non_doublon n
                              JOIN ent_eleve a ON a.etablissement_id = n.etablissement_id AND a.id = n.eleve_a
                              JOIN ent_eleve b ON b.etablissement_id = n.etablissement_id AND b.id = n.eleve_b
                             WHERE n.etablissement_id = $1 AND n.eleve_a = $2 AND n.eleve_b = $3
                               AND a.identite_version_id IS NOT DISTINCT FROM n.identite_a
                               AND b.identite_version_id IS NOT DISTINCT FROM n.identite_b)",
        )
        .bind(etab)
        .bind(x)
        .bind(y)
        .fetch_one(self.pool())
        .await?;
        if distincte {
            avertissements.push(AvertissementFusion::DeclareeDistincte);
        }
        if profil.ouvert_partout(GroupeChamps::EtatCivil) {
            if absorbe.nom != conserve.nom {
                differences.push("nom");
            }
            if absorbe.prenom != conserve.prenom {
                differences.push("prenom");
            }
            if absorbe.nom_naissance != conserve.nom_naissance {
                differences.push("nom_naissance");
            }
            if absorbe.prenoms != conserve.prenoms {
                differences.push("prenoms");
            }
            if absorbe.sexe != conserve.sexe {
                differences.push("sexe");
            }
            if absorbe.date_naissance != conserve.date_naissance {
                differences.push("date_naissance");
            }
        }
        Ok(ApercuFusion {
            absorbe: EleveVue::de(ea, profil.pour(absorbe_id)),
            conserve: EleveVue::de(ec, profil.pour(conserve_id)),
            lignes,
            conflits,
            differences,
            avertissements,
            possible,
        })
    }

    /// ELV-23 ★ : fusionne `absorbe` dans `conserve`. Refusée (rien n'est écrit) tant que des
    /// conflits n'ont pas été acquittés.
    pub async fn fusionner_eleves_pour_acteur(
        &self,
        role: &str,
        demande: DemandeFusion,
    ) -> Result<ResultatFusion, EntDbError> {
        let (etab, acteur) = (demande.etablissement_id, demande.acteur);
        self.exiger_droit_fusion(etab, acteur, role).await?;
        motif_valide(&demande.motif_code, demande.motif.as_deref())?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (absorbe, conserve) = self
            .verrouiller_paire(&mut tx, etab, acteur, role, demande.absorbe_id, demande.conserve_id)
            .await?;
        if absorbe.fusionne_dans.is_some() || conserve.fusionne_dans.is_some() {
            return Err(EntDbError::ConflitDossier("une des fiches est déjà fusionnée"));
        }
        let proj_absorbe = lire_projections(&mut tx, etab, absorbe.id).await?;
        let proj_conserve = lire_projections(&mut tx, etab, conserve.id).await?;
        let plan = planifier(&mut tx, etab, absorbe.id, conserve.id).await?;
        let conflits = plan.conflits();
        let acquittes: HashSet<String> = demande.conflits_acquittes.iter().map(ConflitFusion::cle_texte).collect();
        let non_acquittes: Vec<ConflitFusion> =
            conflits.iter().filter(|c| !acquittes.contains(&c.cle_texte())).cloned().collect();
        if !non_acquittes.is_empty() {
            drop(tx); // rien n'est écrit
            return Ok(ResultatFusion::ConflitsAResoudre(non_acquittes));
        }
        // L'élève absorbé est marqué ; les doublons qui le concernent sont mis à jour.
        let (x, y) = domain_eleve::paire_ordonnee(absorbe.id, conserve.id);
        sqlx::query("UPDATE ent_eleve SET fusionne_dans = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(absorbe.id)
            .bind(conserve.id)
            .execute(&mut *tx)
            .await?;
        let touches: Vec<(Uuid, String)> = sqlx::query_as(
            "UPDATE ent_eleve_doublon d
                SET statut = CASE WHEN d.eleve_a = $2 AND d.eleve_b = $3 THEN 'fusionne' ELSE 'obsolete' END,
                    maj_le = now()
              FROM (SELECT id, statut FROM ent_eleve_doublon
                     WHERE etablissement_id = $1 AND statut = 'a_examiner'
                       AND (eleve_a = $4 OR eleve_b = $4)) av
             WHERE d.id = av.id
         RETURNING d.id, av.statut",
        )
        .bind(etab)
        .bind(x)
        .bind(y)
        .bind(absorbe.id)
        .fetch_all(&mut *tx)
        .await?;
        // Empreintes des lignes déplacées, une fois TOUT le reste fait (projections, marquage).
        let mut lignes_registre: Vec<Value> = Vec::new();
        for p in &plan.tables {
            if p.deplacees.is_empty() {
                continue;
            }
            let emp = empreintes(&mut tx, p.table, etab, conserve.id, &p.deplacees).await?;
            for cle in &p.deplacees {
                let e = emp.get(&cle_texte(p.table.table, cle)).ok_or(EntDbError::ConflitDossier(
                    "une ligne déplacée n'a pas pu être relue",
                ))?;
                lignes_registre.push(json!({ "table_nom": p.table.table, "cle": cle, "empreinte": e }));
            }
        }
        let lignes_apercu = plan.lignes();
        let resume = json!({
            "lignes": lignes_apercu.iter().map(|l| json!({
                "domaine": l.domaine, "table": l.table, "a_deplacer": l.a_deplacer,
                "en_conflit": l.en_conflit, "restent": l.restent })).collect::<Vec<_>>(),
            "projections": { "absorbe": proj_absorbe, "conserve": proj_conserve },
            "nb_lignes": lignes_registre.len(),
        });
        let fusion_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_eleve_fusion (id, etablissement_id, absorbe_id, conserve_id, motif_code, motif,
                    conflits_resolus, resume, doublons_touches, fusionne_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(fusion_id)
        .bind(etab)
        .bind(absorbe.id)
        .bind(conserve.id)
        .bind(&demande.motif_code)
        .bind(demande.motif.as_deref().map(str::trim).filter(|m| !m.is_empty()))
        .bind(serde_json::to_value(&conflits).unwrap_or(Value::Null))
        .bind(&resume)
        .bind(json!(touches.iter().map(|(id, st)| json!({ "id": id, "statut": st })).collect::<Vec<_>>()))
        .bind(acteur)
        .execute(&mut *tx)
        .await?;
        if !lignes_registre.is_empty() {
            sqlx::query(
                "INSERT INTO ent_eleve_fusion_ligne (fusion_id, etablissement_id, table_nom, cle, empreinte)
                 SELECT $1, $2, x.table_nom, x.cle, x.empreinte
                   FROM jsonb_to_recordset($3::jsonb) AS x(table_nom text, cle jsonb, empreinte text)",
            )
            .bind(fusion_id)
            .bind(etab)
            .bind(Value::Array(lignes_registre))
            .execute(&mut *tx)
            .await?;
        }
        self.tracer_fusion(&mut tx, etab, fusion_id, absorbe.id, conserve.id, "fusionne", acteur, &demande.motif_code)
            .await?;
        let ligne: LigneFusion = sqlx::query_as(&format!(
            "SELECT {COLONNES_FUSION} FROM ent_eleve_fusion WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(fusion_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ResultatFusion::Faite(ligne.en_fusion()))
    }

    /// Événement (sans valeur d'identité : identifiants et motif codé) et journal d'audit, dans la
    /// transaction du fait. Le journal est écrit dans le domaine « eleve_sensible » (absent de la
    /// chronologie 360°) et jamais montré à la famille.
    #[allow(clippy::too_many_arguments)]
    async fn tracer_fusion(
        &self,
        tx: &mut TenantTx<'static>,
        etab: Uuid,
        fusion_id: Uuid,
        absorbe: Uuid,
        conserve: Uuid,
        quoi: &str,
        acteur: Uuid,
        motif_code: &str,
    ) -> Result<(), EntDbError> {
        let (type_name, action, libelle) = if quoi == "fusionne" {
            (types::ELEVE_FUSIONNE, "eleve_fusionne", "Fiche d'élève fusionnée")
        } else {
            (types::ELEVE_DEFUSIONNE, "eleve_defusionne", "Fusion de fiches d'élèves annulée")
        };
        tx.publier(MetierEventDraft {
            etablissement_id: etab,
            type_name: type_name.to_string(),
            aggregate: AggregateKind::Eleve,
            aggregate_id: conserve,
            eleve_id: Some(conserve),
            payload: json!({ "fusion_id": fusion_id, "absorbe_id": absorbe, "conserve_id": conserve }),
            actor_user_id: Some(acteur.to_string()),
        })
        .await?;
        for eleve in [absorbe, conserve] {
            sqlx::query(
                "INSERT INTO ent_journal_modif (id, etablissement_id, eleve_id, domaine, action, libelle,
                                                 payload, visible_famille, actor_user_id, created_at)
                 VALUES ($1,$2,$3,'eleve_sensible',$4,$5,$6,false,$7, now())",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(eleve)
            .bind(action)
            .bind(libelle)
            .bind(json!({ "fusion_id": fusion_id, "motif_code": motif_code }))
            .bind(acteur)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    /// ELV-24 ★ : annule une fusion (≤ 30 jours) en remettant chaque ligne déplacée à l'absorbé.
    /// Refusée si une ligne déplacée a été modifiée depuis, ou si l'élève conservé a été absorbé
    /// depuis (annuler d'abord la fusion plus récente).
    pub async fn defusionner_eleves_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        fusion_id: Uuid,
    ) -> Result<ResultatDefusion, EntDbError> {
        self.exiger_droit_fusion(etab, acteur, role).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let f: Option<LigneFusion> = sqlx::query_as(&format!(
            "SELECT {COLONNES_FUSION} FROM ent_eleve_fusion WHERE etablissement_id = $1 AND id = $2 FOR UPDATE"
        ))
        .bind(etab)
        .bind(fusion_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(f) = f else { return Err(DomainError::NotFound.into()) };
        let (absorbe_id, conserve_id) = (f.absorbe_id, f.conserve_id);
        let (absorbe, conserve) = self.verrouiller_paire(&mut tx, etab, acteur, role, absorbe_id, conserve_id).await?;
        if f.annule_le.is_some() {
            return Err(EntDbError::ConflitDossier("fusion déjà annulée"));
        }
        if Utc::now() - f.fusionne_le > Duration::days(DELAI_DEFUSION_JOURS) {
            return Err(EntDbError::ConflitDossier("délai d'annulation de la fusion dépassé"));
        }
        if conserve.fusionne_dans.is_some() {
            return Err(EntDbError::ConflitDossier(
                "l'élève conservé a été fusionné depuis : annuler d'abord la fusion plus récente",
            ));
        }
        if absorbe.fusionne_dans != Some(conserve_id) {
            return Err(EntDbError::ConflitDossier("l'état de la fiche absorbée ne correspond plus à la fusion"));
        }
        // Les lignes déplacées, table par table.
        let lignes: Vec<(String, Value, String)> = sqlx::query_as(
            "SELECT table_nom, cle, empreinte FROM ent_eleve_fusion_ligne WHERE fusion_id = $1 ORDER BY id",
        )
        .bind(fusion_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut par_table: BTreeMap<String, Vec<(Value, String)>> = BTreeMap::new();
        for (t, c, e) in lignes {
            par_table.entry(t).or_default().push((c, e));
        }
        // 1. Aucune ligne déplacée n'a été modifiée ou supprimée depuis.
        let mut modifiees: Vec<LigneModifiee> = Vec::new();
        for (nom, l) in &par_table {
            let Some(t) = domain_eleve::table_eleve(nom) else {
                return Err(EntDbError::ConflitDossier("table inconnue du registre de fusion"));
            };
            let cles: Vec<Value> = l.iter().map(|(c, _)| c.clone()).collect();
            let actuelles = empreintes(&mut tx, t, etab, conserve_id, &cles).await?;
            for (cle, attendue) in l {
                match actuelles.get(&cle_texte(nom, cle)) {
                    None => modifiees.push(LigneModifiee {
                        domaine: domaine_fusion(nom).to_string(),
                        table: nom.clone(),
                        cle: cle.clone(),
                        raison: "absente",
                    }),
                    Some(e) if e != attendue => modifiees.push(LigneModifiee {
                        domaine: domaine_fusion(nom).to_string(),
                        table: nom.clone(),
                        cle: cle.clone(),
                        raison: "modifiee",
                    }),
                    Some(_) => {}
                }
            }
        }
        if !modifiees.is_empty() {
            drop(tx);
            return Ok(ResultatDefusion::LignesModifiees(modifiees));
        }
        // 2. Chaque ligne retourne à l'absorbé (seul `eleve_id` change, donc « à l'identique »).
        for (nom, l) in &par_table {
            let t = domain_eleve::table_eleve(nom).expect("table vérifiée plus haut");
            armer_drapeaux(&mut tx).await?;
            let cles = Value::Array(l.iter().map(|(c, _)| c.clone()).collect());
            let sql = format!(
                "UPDATE {table} AS t SET eleve_id = $3 FROM jsonb_to_recordset($4::jsonb) AS {def}
                  WHERE t.etablissement_id = $1 AND t.eleve_id = $2 AND {jointure}",
                table = t.table,
                def = definition_cle(t),
                jointure = jointure_cle(t)
            );
            let r = sqlx::query(&sql)
                .bind(etab)
                .bind(conserve_id)
                .bind(absorbe_id)
                .bind(&cles)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    if est_conflit_d_unicite(&e) {
                        EntDbError::ConflitDossier("la défusion heurte une ligne créée depuis la fusion")
                    } else {
                        e.into()
                    }
                })?;
            if r.rows_affected() as usize != l.len() {
                return Err(EntDbError::ConflitDossier("des lignes déplacées n'ont pas pu être restituées"));
            }
        }
        // 3. Projections d'`ent_eleve` telles qu'avant la fusion ; marquage levé ; doublons rétablis.
        let proj = f.resume.get("projections").cloned().unwrap_or(Value::Null);
        for (id, cle) in [(absorbe_id, "absorbe"), (conserve_id, "conserve")] {
            if let Some(p) = proj.get(cle).filter(|p| p.is_object()) {
                ecrire_projections(&mut tx, etab, id, p).await?;
            } else {
                recalculer_projections(&mut tx, etab, id).await?;
            }
        }
        sqlx::query("UPDATE ent_eleve SET fusionne_dans = NULL WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(absorbe_id)
            .execute(&mut *tx)
            .await?;
        let touches: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT (x->>'id')::uuid, x->>'statut'
               FROM ent_eleve_fusion fu, jsonb_array_elements(fu.doublons_touches) x
              WHERE fu.id = $1",
        )
        .bind(fusion_id)
        .fetch_all(&mut *tx)
        .await?;
        for (id, statut) in touches {
            sqlx::query("UPDATE ent_eleve_doublon SET statut = $3, maj_le = now() WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .bind(statut)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("UPDATE ent_eleve_fusion SET annule_le = now(), annule_par = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(fusion_id)
            .bind(acteur)
            .execute(&mut *tx)
            .await?;
        self.tracer_fusion(&mut tx, etab, fusion_id, absorbe_id, conserve_id, "defusionne", acteur, &f.motif_code)
            .await?;
        let ligne: LigneFusion = sqlx::query_as(&format!(
            "SELECT {COLONNES_FUSION} FROM ent_eleve_fusion WHERE etablissement_id = $1 AND id = $2"
        ))
        .bind(etab)
        .bind(fusion_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ResultatDefusion::Faite(ligne.en_fusion()))
    }

    /// Fusions récentes de l'établissement (les plus récentes d'abord), annulées comprises.
    pub async fn fusions_pour_acteur(
        &self,
        role: &str,
        etab: Uuid,
        acteur: Uuid,
        limite: i64,
    ) -> Result<Vec<FusionEleves>, EntDbError> {
        self.exiger_droit_fusion(etab, acteur, role).await?;
        let profil = self.profil_champs(etab, acteur, role).await?;
        let voit_proteges = profil.ouvert_partout(GroupeChamps::Sensible);
        let lignes: Vec<LigneFusion> = sqlx::query_as(&format!(
            "SELECT {cols} FROM ent_eleve_fusion f
              WHERE f.etablissement_id = $1
                AND ($2 OR NOT (ent_eleve_confidentiel(f.etablissement_id, f.absorbe_id)
                                OR ent_eleve_confidentiel(f.etablissement_id, f.conserve_id)))
              ORDER BY f.fusionne_le DESC, f.id LIMIT $3",
            cols = COLONNES_FUSION
                .split(',')
                .map(|c| format!("f.{}", c.trim()))
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .bind(etab)
        .bind(voit_proteges)
        .bind(limite.clamp(1, 200))
        .fetch_all(self.pool())
        .await?;
        Ok(lignes.into_iter().map(LigneFusion::en_fusion).collect())
    }

    /// Redirection d'une fiche absorbée : l'identifiant de la fiche qui a pris sa place (en
    /// suivant la chaîne), ou l'identifiant lui-même. Toute lecture par identifiant passe par ici.
    pub async fn eleve_effectif(&self, etab: Uuid, eleve_id: Uuid) -> Result<Uuid, EntDbError> {
        let r: Option<Uuid> = sqlx::query_scalar(
            "WITH RECURSIVE chaine(id, suivant, n) AS (
                SELECT id, fusionne_dans, 0 FROM ent_eleve WHERE etablissement_id = $1 AND id = $2
              UNION ALL
                SELECT e.id, e.fusionne_dans, c.n + 1 FROM chaine c
                  JOIN ent_eleve e ON e.etablissement_id = $1 AND e.id = c.suivant WHERE c.n < 20)
             SELECT id FROM chaine WHERE suivant IS NULL ORDER BY n DESC LIMIT 1",
        )
        .bind(etab)
        .bind(eleve_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(r.unwrap_or(eleve_id))
    }
}
