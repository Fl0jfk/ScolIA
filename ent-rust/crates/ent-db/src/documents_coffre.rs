//! Socle 7, lot D4 : **le coffre** — coffres du responsable, de l'élève et de l'agent, accusés de première ouverture,
//! export, notifications par événements, bascule à la majorité de l'élève, retrait sur suspension de lien
//! (`docs/socles/07_DOCUMENTS.md` §12, §14.6 DCT-83 à DCT-92, §19.2, Q-D12, Q-D13).
//!
//! Règles du lot, toutes appliquées ici (jamais dans le navigateur) :
//! * **le droit se relit à chaque lecture, à la date du jour** : une mise à disposition ne vaut jamais droit. Un parent
//!   ne voit un document de l'élève que si son lien porte l'autorité parentale, n'est pas suspendu et est actif ; il ne
//!   voit plus, après la majorité de l'élève, que ce qui relève du payeur (domaine financier) ; un élève **mineur** ne voit
//!   que le domaine « scolarité » jusqu'à C2 (jamais la santé, la justice, la discipline, le financier) ;
//! * **aucun document C4** ; **aucun document annulé**, remplacé (seule la version courante) ni retiré ;
//! * **lien suspendu → plus rien de nouveau, et le passé est retiré** (Q-D12) : à l'événement `lien.suspendu` les mises à
//!   disposition du coffre de la personne, pour cet élève, sont retirées (motif « lien suspendu ») ; la direction peut en
//!   **rendre** une explicitement (`documents.classer`) si le lien est de nouveau valable ; une réconciliation rattrape les
//!   suspensions posées sans événement ;
//! * **majorité** (`eleve.majeur`, socle 2) : les mises à disposition des parents sur les documents de l'élève sont
//!   retirées (hors payeur), l'élève reçoit les siennes sur un exemplaire **sans adresse** (jamais celle d'un parent) ;
//!   idempotent ; l'élève est prévenu ;
//! * **notification sans contenu, jamais de pièce jointe** (Q-D13) : « un document vous attend », rien d'autre — ni
//!   nature, ni titre, ni nom d'élève ; le message renvoie au coffre, connexion exigée. Le canal ne sait d'ailleurs rien
//!   porter d'autre que du texte ;
//! * **accusé de première ouverture** : posé une seule fois (même sous deux ouvertures simultanées), visible du
//!   personnel qui lit le registre et le domaine du document, jamais de la famille ; ce n'est pas une preuve de
//!   réception au sens d'un recommandé ;
//! * **export** : archive ZIP (PDF + `index.csv`), tâche **synchrone** bornée (500 documents, 200 Mio) ; chaque document
//!   exporté laisse une consultation « téléchargement » (journal des accès de la famille, socle 27).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use ent_auth::Action;
use ent_events::{types, AggregateKind, MetierEventDraft};
use serde_json::json;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::documents::*;
use crate::documents_natures::{est_majeur, titulaires_destinataires};
use crate::documents_repo::{journaliser, publier, texte, DocumentLu};
use crate::documents_zip::EcrivainZip;
use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::outbox::{Consommateur, EvenementDistribue, FutureTraitement, Publier};
use crate::ref_commun::{db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

pub const EVT_MISE_A_DISPOSITION: &str = "documents.mis_a_disposition";
pub const EVT_MISE_A_DISPOSITION_RETIREE: &str = "documents.mise_a_disposition_retiree";
pub const EVT_COFFRE_BASCULE: &str = "documents.coffre_bascule";
/// Nom stable du consommateur (clé d'idempotence de l'outbox) : ne jamais le renommer.
pub const CONSOMMATEUR_COFFRE: &str = "d4.documents_coffre";

/// Bornes de l'export (tâche synchrone ; le passage en tâche de fond viendra avec les écrans, lot D12).
pub const EXPORT_MAX_DOCUMENTS: usize = 500;
pub const EXPORT_MAX_OCTETS: usize = 200 * 1024 * 1024;

const MOTIF_LIEN_SUSPENDU: &str = "lien suspendu";
const MOTIF_MAJORITE: &str = "majorité de l'élève";

/// Les natures qui demandent un accusé de première ouverture (convocations, notifications ; §12.1). Une nature de
/// brique (socle 13…) y entre par son code : tout code qui contient `convocation` ou `notification`.
pub fn accuse_demande(nature_code: &str) -> bool {
    nature_code.contains("convocation") || nature_code.contains("notification")
}

fn refus(nature: RefusMetier, msg: &'static str) -> EntDbError {
    EntDbError::Refus(nature, msg)
}

// ───────────────────────── Types publiés ─────────────────────────

/// Filtres d'affichage du coffre (par enfant, année, famille de documents ; « pour moi »).
#[derive(Debug, Clone, Default)]
pub struct FiltreCoffre {
    pub eleve_id: Option<Uuid>,
    /// Les documents qui ne parlent d'aucun élève (factures dont on est payeur, documents « pour moi »).
    pub pour_moi: bool,
    pub annee: Option<i32>,
    /// Préfixe du code de nature (`inscriptions.`, `eleve.`…).
    pub famille: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LigneCoffre {
    pub mise_a_disposition_id: Uuid,
    pub document_id: Uuid,
    pub nature_code: String,
    pub titre: String,
    pub numero_affiche: Option<String>,
    pub eleve_id: Option<Uuid>,
    pub sensibilite: String,
    pub depuis: DateTime<Utc>,
    pub premiere_ouverture_le: Option<DateTime<Utc>>,
    pub numero_version: i32,
}

#[derive(Debug, Clone)]
pub struct AccuseVu {
    pub mise_a_disposition_id: Uuid,
    /// `personne`, `eleve` ou `agent`.
    pub titulaire: &'static str,
    pub destinataire: String,
    pub canal: String,
    pub depuis: DateTime<Utc>,
    pub premiere_ouverture_le: Option<DateTime<Utc>>,
    pub retiree_le: Option<DateTime<Utc>>,
    pub retrait_motif: Option<String>,
    /// La nature demande l'accusé (convocation, notification).
    pub accuse_demande: bool,
}

#[derive(Debug, Clone)]
pub struct NonOuvert {
    pub document_id: Uuid,
    pub mise_a_disposition_id: Uuid,
    pub nature_code: String,
    pub depuis: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ExportCoffre {
    pub octets: Vec<u8>,
    pub nom_fichier: String,
    pub nb_documents: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinataireMad {
    Personne(Uuid),
    Eleve(Uuid),
    Agent(Uuid),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BasculeFaite {
    pub retirees: usize,
    pub ouvertes: usize,
    /// Documents que l'élève ne peut pas recevoir faute d'exemplaire sans adresse : à réémettre à son nom.
    pub sans_exemplaire_neutre: usize,
}

// ───────────────────────── Droits du coffre (fonctions pures et lectures) ─────────────────────────

/// Qui est la personne qui ouvre le coffre : ses fiches de personne, ses élèves (« soi »), ses fiches d'agent.
#[derive(Debug, Clone, Default)]
pub struct TitulairesCoffre {
    pub personnes: Vec<Uuid>,
    /// Les fiches de personne qui représentent l'élève lui-même (élève majeur).
    pub personnes_eleves: HashSet<Uuid>,
    pub eleves: Vec<Uuid>,
    pub agents: Vec<Uuid>,
}

/// Une mise à disposition candidate, avec ce qu'il faut pour juger du droit.
#[derive(Debug, Clone, sqlx::FromRow)]
struct Candidate {
    mad_id: Uuid,
    document_id: Uuid,
    version_id: Uuid,
    nature_code: String,
    titre: String,
    sensibilite: String,
    domaine: String,
    eleve_id: Option<Uuid>,
    serie_code: Option<String>,
    numero_annee: Option<i32>,
    numero: Option<i32>,
    depuis: DateTime<Utc>,
    premiere_ouverture_le: Option<DateTime<Utc>>,
    mad_personne: Option<Uuid>,
    mad_eleve: Option<Uuid>,
    mad_agent: Option<Uuid>,
    rang: i32,
    numero_version: i32,
    cle_objet: String,
    empreinte_sha256: String,
    annee: i32,
}

const SELECT_COFFRE: &str = "SELECT m.id AS mad_id, d.id AS document_id, v.id AS version_id, d.nature_code, d.titre,
        d.sensibilite, d.domaine, d.eleve_id, d.serie_code, d.numero_annee, d.numero, m.depuis,
        m.premiere_ouverture_le, m.personne_id AS mad_personne, m.eleve_id AS mad_eleve, m.agent_id AS mad_agent,
        ex.rang, v.numero_version, ex.cle_objet, ex.empreinte_sha256, EXTRACT(YEAR FROM m.depuis)::int AS annee
   FROM ent_doc_mise_a_disposition m
   JOIN ent_doc_document d ON d.etablissement_id = m.etablissement_id AND d.id = m.document_id
   JOIN ent_doc_exemplaire ex ON ex.etablissement_id = m.etablissement_id AND ex.id = m.exemplaire_id
   JOIN ent_doc_version v ON v.etablissement_id = ex.etablissement_id AND v.id = ex.version_id
  WHERE m.etablissement_id = $1 AND m.canal = 'coffre' AND m.retiree_le IS NULL
    AND (m.jusqu_au IS NULL OR m.jusqu_au > now())
    AND d.etat = 'emis' AND d.sensibilite <> 'C4' AND v.numero_version = d.version_courante
    AND (m.personne_id = ANY($2) OR m.eleve_id = ANY($3) OR m.agent_id = ANY($4))";

/// Ce que le droit du jour sait : liens actifs, majorités.
#[derive(Debug, Default)]
struct ContexteCoffre {
    titulaires: TitulairesCoffre,
    /// (personne, élève) : lien portant l'autorité parentale, non suspendu, actif au jour.
    liens: HashSet<(Uuid, Uuid)>,
    majeurs: HashSet<Uuid>,
}

impl ContexteCoffre {
    /// Le jugement de droit, **pur** : voir les règles en tête de module.
    fn autorise(&self, c: &Candidate) -> bool {
        let (Some(sens), Some(dom)) = (Sensibilite::parse(&c.sensibilite), Domaine::parse(&c.domaine)) else {
            return false;
        };
        if sens == Sensibilite::C4 {
            return false;
        }
        let t = &self.titulaires;
        // Fiche d'agent : « Mes documents ». Aucun lien à vérifier.
        if c.mad_agent.is_some_and(|g| t.agents.contains(&g)) {
            return true;
        }
        // L'élève lui-même : par son compte (colonne `eleve_id`) ou par sa fiche de personne.
        let eleve_vise = c
            .mad_eleve
            .or_else(|| c.mad_personne.filter(|p| t.personnes_eleves.contains(p)).and(c.eleve_id));
        if let Some(e) = eleve_vise {
            // Mineur : seulement sa scolarité, jusqu'à C2 ; majeur : ce qui lui a été adressé.
            return t.eleves.contains(&e)
                && c.eleve_id.is_none_or(|d| d == e)
                && (self.majeurs.contains(&e) || (sens <= Sensibilite::C2 && dom == Domaine::Scolarite));
        }
        // Une personne (parent, payeur, tiers) : sa propre fiche seulement.
        let Some(p) = c.mad_personne.filter(|p| t.personnes.contains(p)) else {
            return false;
        };
        match c.eleve_id {
            // Document qui parle d'un élève : le lien du jour doit porter l'autorité parentale ; passé la majorité,
            // les parents ne gardent que ce qui relève du payeur.
            Some(e) => {
                self.liens.contains(&(p, e)) && (!self.majeurs.contains(&e) || dom == Domaine::Financier)
            }
            // Document « pour moi » : factures dont on est payeur, documents qui concernent la personne.
            None => true,
        }
    }
}

pub(crate) async fn resoudre_titulaires(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
) -> Result<TitulairesCoffre, EntDbError> {
    let mut t = TitulairesCoffre::default();
    let personnes: Vec<(Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT id, eleve_id FROM ent_personne WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
    )
    .bind(etab)
    .bind(user)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (p, e) in personnes {
        t.personnes.push(p);
        if let Some(e) = e {
            t.personnes_eleves.insert(p);
            t.eleves.push(e);
        }
    }
    let comptes: Vec<(Uuid,)> =
        sqlx::query_as("SELECT eleve_id FROM ent_eleve_compte WHERE etablissement_id = $1 AND user_id = $2")
            .bind(etab)
            .bind(user)
            .fetch_all(&mut *conn)
            .await
            .map_err(db)?;
    for (e,) in comptes {
        if !t.eleves.contains(&e) {
            t.eleves.push(e);
        }
    }
    let agents: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT ag.id FROM ent_agent ag
           JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
          WHERE ag.etablissement_id = $1 AND p.user_id = $2",
    )
    .bind(etab)
    .bind(user)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    t.agents = agents.into_iter().map(|x| x.0).collect();
    Ok(t)
}

pub(crate) async fn jour_etab(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
    let (j,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1)")
        .bind(etab)
        .fetch_one(conn)
        .await
        .map_err(db)?;
    Ok(j)
}

/// Élèves parmi `eleves` qui sont majeurs au jour donné (date de naissance exacte requise).
pub(crate) async fn majeurs_parmi(
    conn: &mut PgConnection,
    etab: Uuid,
    eleves: &[Uuid],
    jour: NaiveDate,
) -> Result<HashSet<Uuid>, EntDbError> {
    if eleves.is_empty() {
        return Ok(HashSet::new());
    }
    let lignes: Vec<(Uuid, Option<NaiveDate>)> = sqlx::query_as(
        "SELECT id, date_naissance FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)
            AND coalesce(date_naissance_precision, 'jour') = 'jour' AND coalesce(date_naissance_statut, 'declaree') <> 'contestee'",
    )
    .bind(etab)
    .bind(eleves)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(lignes
        .into_iter()
        .filter_map(|(id, n)| n.filter(|n| est_majeur(*n, jour)).map(|_| id))
        .collect())
}

pub(crate) async fn liens_autorite(
    conn: &mut PgConnection,
    etab: Uuid,
    personnes: &[Uuid],
    jour: NaiveDate,
) -> Result<HashSet<(Uuid, Uuid)>, EntDbError> {
    if personnes.is_empty() {
        return Ok(HashSet::new());
    }
    let l: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT personne_id, eleve_id FROM ent_eleve_lien
          WHERE etablissement_id = $1 AND personne_id = ANY($2)
            AND autorite_parentale AND suspendu_le IS NULL
            AND (date_debut IS NULL OR date_debut <= $3) AND (date_fin IS NULL OR date_fin >= $3)",
    )
    .bind(etab)
    .bind(personnes)
    .bind(jour)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(l.into_iter().collect())
}

/// Charge les candidates du coffre de `user` et le contexte de droit du jour. `mad` restreint à une mise à disposition.
async fn charger(
    conn: &mut PgConnection,
    etab: Uuid,
    user: Uuid,
    mad: Option<Uuid>,
) -> Result<(ContexteCoffre, Vec<Candidate>), EntDbError> {
    let jour = jour_etab(&mut *conn, etab).await?;
    let t = resoudre_titulaires(&mut *conn, etab, user).await?;
    if t.personnes.is_empty() && t.eleves.is_empty() && t.agents.is_empty() {
        return Ok((ContexteCoffre::default(), Vec::new()));
    }
    let sql = if mad.is_some() {
        format!("{SELECT_COFFRE} AND m.id = $5")
    } else {
        format!("{SELECT_COFFRE} ORDER BY m.depuis DESC, m.id")
    };
    let mut q = sqlx::query_as::<_, Candidate>(&sql)
        .bind(etab)
        .bind(&t.personnes)
        .bind(&t.eleves)
        .bind(&t.agents);
    if let Some(m) = mad {
        q = q.bind(m);
    }
    let cands = q.fetch_all(&mut *conn).await.map_err(db)?;
    let eleves: Vec<Uuid> = cands
        .iter()
        .filter_map(|c| c.eleve_id)
        .chain(t.eleves.iter().copied())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let liens = liens_autorite(&mut *conn, etab, &t.personnes, jour).await?;
    let majeurs = majeurs_parmi(&mut *conn, etab, &eleves, jour).await?;
    Ok((ContexteCoffre { titulaires: t, liens, majeurs }, cands))
}

fn numero_affiche(c: &Candidate) -> Option<String> {
    match (c.numero_annee, c.serie_code.as_deref(), c.numero) {
        (Some(a), Some(s), Some(n)) => Some(format!("{a}-{s}-{n:06}")),
        _ => None,
    }
}

fn filtre_ok(f: &FiltreCoffre, c: &Candidate) -> bool {
    f.eleve_id.is_none_or(|e| c.eleve_id == Some(e))
        && (!f.pour_moi || c.eleve_id.is_none())
        && f.annee.is_none_or(|a| c.annee == a)
        && f.famille.as_deref().is_none_or(|p| c.nature_code.starts_with(p))
}

fn ligne(c: &Candidate) -> LigneCoffre {
    LigneCoffre {
        mise_a_disposition_id: c.mad_id,
        document_id: c.document_id,
        nature_code: c.nature_code.clone(),
        titre: c.titre.clone(),
        numero_affiche: numero_affiche(c),
        eleve_id: c.eleve_id,
        sensibilite: c.sensibilite.clone(),
        depuis: c.depuis,
        premiere_ouverture_le: c.premiere_ouverture_le,
        numero_version: c.numero_version,
    }
}

/// Cellule CSV sûre : séparateur `;`, guillemets doublés, et neutralisation des formules (`=`, `+`, `-`, `@`).
fn cellule_csv(v: &str) -> String {
    let mut s = v.replace(['\r', '\n'], " ");
    if s.starts_with(['=', '+', '-', '@', '\t']) {
        s.insert(0, '\'');
    }
    if s.contains([';', '"']) {
        s = format!("\"{}\"", s.replace('"', "\"\""));
    }
    s
}

fn nom_fichier_sur(c: &Candidate) -> String {
    let base = numero_affiche(c).unwrap_or_else(|| c.document_id.simple().to_string());
    let nature: String = c
        .nature_code
        .chars()
        .map(|x| if x.is_ascii_alphanumeric() { x } else { '_' })
        .collect();
    format!("documents/{}_{}_{}.pdf", base, nature, &c.mad_id.simple().to_string()[..8])
}

// ───────────────────────── Service ─────────────────────────

impl PgEnt {
    /// Le coffre de la personne connectée (DCT-83) : ce qu'elle a le droit de voir **aujourd'hui**, le plus récent
    /// d'abord. Responsable : par enfant et « pour moi » ; élève : sa scolarité (mineur) ou tout ce qui lui est adressé
    /// (majeur) ; agent : « Mes documents ».
    pub async fn coffre_for_actor(
        &self,
        a: &Acteur<'_>,
        filtre: &FiltreCoffre,
    ) -> Result<Vec<LigneCoffre>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let (ctx, cands) = charger(&mut tx, a.etablissement_id, a.user_id, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(cands
            .iter()
            .filter(|c| ctx.autorise(c) && filtre_ok(filtre, c))
            .map(ligne)
            .collect())
    }

    /// Ouvre un document du coffre (DCT-84, DCT-86) : droit relu, lecture tracée (C2 et plus), **accusé de première
    /// ouverture posé une seule fois** (`documents.ouvert`), octets relus avec contrôle d'empreinte. Une mise à
    /// disposition qui n'est pas la sienne, retirée, d'un document annulé ou que le droit du jour n'ouvre plus répond
    /// « introuvable » (aucun indice).
    pub async fn ouvrir_du_coffre_for_actor(
        &self,
        a: &Acteur<'_>,
        mise_a_disposition_id: Uuid,
    ) -> Result<DocumentLu, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let (ctx, cands) = charger(&mut tx, a.etablissement_id, a.user_id, Some(mise_a_disposition_id)).await?;
        let c = cands.into_iter().find(|c| ctx.autorise(c)).ok_or_else(introuvable)?;
        if Sensibilite::parse(&c.sensibilite).is_some_and(Sensibilite::trace_la_lecture) {
            tracer(&mut tx, a, &c, "lecture").await?;
        }
        let mut evenement = None;
        if c.premiere_ouverture_le.is_none() {
            let r = sqlx::query(
                "UPDATE ent_doc_mise_a_disposition SET premiere_ouverture_le = now()
                  WHERE etablissement_id = $1 AND id = $2 AND premiere_ouverture_le IS NULL",
            )
            .bind(a.etablissement_id)
            .bind(c.mad_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            if r.rows_affected() == 1 {
                evenement = Some(
                    publier(
                        &mut tx,
                        a,
                        "documents.ouvert",
                        c.document_id,
                        c.eleve_id,
                        json!({ "nature": c.nature_code, "sensibilite": c.sensibilite, "rang": c.rang }),
                    )
                    .await?,
                );
            }
        }
        tx.commit().await.map_err(db)?;
        if let Some(ev) = evenement {
            self.diffuser_apres_commit(&[ev]);
        }
        let octets = self
            .lire_objet_empreinte(a.etablissement_id, &c.cle_objet, Some(&c.empreinte_sha256))
            .await?;
        Ok(DocumentLu {
            document_id: c.document_id,
            nature_code: c.nature_code,
            etat: "emis".into(),
            numero_version: c.numero_version,
            version_courante: c.numero_version,
            empreinte_sha256: c.empreinte_sha256,
            exemplaire_rang: Some(c.rang),
            octets,
        })
    }

    /// Les accusés d'un document (DCT-91) : qui l'a reçu, qui l'a ouvert, quand, qui l'a vu retirer. Droit : registre
    /// des documents **et** lecture du domaine du document sur son sujet ; C4 à personne.
    pub async fn accuses_for_actor(
        &self,
        a: &Acteur<'_>,
        document_id: Uuid,
    ) -> Result<Vec<AccuseVu>, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsRegistreLire).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let d: Option<(String, String, String, Option<Uuid>)> = sqlx::query_as(
            "SELECT nature_code, domaine, sensibilite, eleve_id FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (nature, domaine, sens, eleve) = d.ok_or_else(introuvable)?;
        if sens == "C4" {
            return Err(EntDbError::Forbidden);
        }
        let domaine = Domaine::parse(&domaine).ok_or(EntDbError::Forbidden)?;
        if !self.doc_peut_sur_sujet(a, domaine.action_lecture(), eleve).await? {
            return Err(EntDbError::Forbidden);
        }
        let lignes = sqlx::query(
            "SELECT m.id, m.canal, m.depuis, m.premiere_ouverture_le, m.retiree_le, m.retrait_motif,
                    CASE WHEN m.personne_id IS NOT NULL THEN 'personne' WHEN m.eleve_id IS NOT NULL THEN 'eleve' ELSE 'agent' END AS genre,
                    coalesce(
                        (SELECT btrim(coalesce(p.prenom || ' ', '') || p.nom) FROM ent_personne p
                          WHERE p.etablissement_id = m.etablissement_id AND p.id = m.personne_id),
                        (SELECT btrim(e.prenom || ' ' || e.nom) FROM ent_eleve e
                          WHERE e.etablissement_id = m.etablissement_id AND e.id = m.eleve_id),
                        (SELECT btrim(coalesce(p.prenom || ' ', '') || p.nom) FROM ent_agent ag
                           JOIN ent_personne p ON p.etablissement_id = ag.etablissement_id AND p.id = ag.personne_id
                          WHERE ag.etablissement_id = m.etablissement_id AND ag.id = m.agent_id),
                        '?') AS destinataire
               FROM ent_doc_mise_a_disposition m
              WHERE m.etablissement_id = $1 AND m.document_id = $2
              ORDER BY m.depuis, m.id",
        )
        .bind(a.etablissement_id)
        .bind(document_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let demande = accuse_demande(&nature);
        Ok(lignes
            .iter()
            .map(|r| AccuseVu {
                mise_a_disposition_id: r.get("id"),
                titulaire: match r.get::<String, _>("genre").as_str() {
                    "personne" => "personne",
                    "eleve" => "eleve",
                    _ => "agent",
                },
                destinataire: r.get("destinataire"),
                canal: r.get("canal"),
                depuis: r.get("depuis"),
                premiere_ouverture_le: r.get("premiere_ouverture_le"),
                retiree_le: r.get("retiree_le"),
                retrait_motif: r.get("retrait_motif"),
                accuse_demande: demande,
            })
            .collect())
    }

    /// Les mises à disposition d'une famille de natures **jamais ouvertes** depuis `avant` (DCT-91) : la relance est
    /// **proposée** au personnel, jamais envoyée ici. Au plus 200 lignes, filtrées par le droit du domaine.
    pub async fn non_ouverts_for_actor(
        &self,
        a: &Acteur<'_>,
        famille: &str,
        avant: DateTime<Utc>,
    ) -> Result<Vec<NonOuvert>, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsRegistreLire).await?;
        let famille = texte(famille, 2, 80)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let lignes: Vec<(Uuid, Uuid, String, String, Option<Uuid>, DateTime<Utc>)> = sqlx::query_as(
            "SELECT m.document_id, m.id, d.nature_code, d.domaine, d.eleve_id, m.depuis
               FROM ent_doc_mise_a_disposition m
               JOIN ent_doc_document d ON d.etablissement_id = m.etablissement_id AND d.id = m.document_id
              WHERE m.etablissement_id = $1 AND m.canal = 'coffre' AND m.retiree_le IS NULL
                AND m.premiere_ouverture_le IS NULL AND m.depuis <= $3
                AND d.etat = 'emis' AND d.sensibilite <> 'C4' AND left(d.nature_code, length($2)) = $2
              ORDER BY m.depuis LIMIT 200",
        )
        .bind(a.etablissement_id)
        .bind(&famille)
        .bind(avant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut droits: HashMap<(String, Option<Uuid>), bool> = HashMap::new();
        let mut sortie = Vec::new();
        for (document_id, mad, nature, domaine, eleve, depuis) in lignes {
            let cle = (domaine.clone(), eleve);
            let ok = match droits.get(&cle) {
                Some(v) => *v,
                None => {
                    let v = match Domaine::parse(&domaine) {
                        Some(d) => self.doc_peut_sur_sujet(a, d.action_lecture(), eleve).await?,
                        None => false,
                    };
                    droits.insert(cle, v);
                    v
                }
            };
            if ok {
                sortie.push(NonOuvert { document_id, mise_a_disposition_id: mad, nature_code: nature, depuis });
            }
        }
        Ok(sortie)
    }

    /// **Mettre à disposition** un document déjà émis (DCT-87) : `documents.emettre`, droit de lecture du domaine sur le
    /// sujet, **droits du destinataire à la date du jour**. L'exemplaire est celui du destinataire s'il en a un, sinon
    /// un exemplaire **sans adresse** (jamais celle d'un autre) ; faute de mieux, le document est à réémettre.
    pub async fn mettre_a_disposition_for_actor(
        &self,
        a: &Acteur<'_>,
        document_id: Uuid,
        dest: DestinataireMad,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsEmettre).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let doc = document_court(&mut tx, a.etablissement_id, document_id).await?;
        let domaine = Domaine::parse(&doc.domaine).ok_or(EntDbError::Forbidden)?;
        if doc.sensibilite == "C4" || !self.doc_peut_sur_sujet(a, domaine.action_lecture(), doc.eleve_id).await? {
            return Err(EntDbError::Forbidden);
        }
        if doc.etat != "emis" {
            return Err(refus(RefusMetier::Conflit, "seul un document émis se met à disposition"));
        }
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        destinataire_valable(&mut tx, a.etablissement_id, &doc, dest, jour).await?;
        let exemplaire = choisir_exemplaire(&mut tx, a.etablissement_id, document_id, dest).await?;
        let mad = inserer_mad(&mut tx, a, exemplaire.0, dest).await?;
        journaliser(
            &mut tx,
            a,
            Some(document_id),
            Some(doc.version_courante),
            "mis_a_disposition",
            (None, None),
            Some("coffre"),
            None,
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            EVT_MISE_A_DISPOSITION,
            document_id,
            doc.eleve_id,
            json!({ "nature": doc.nature_code, "canal": "coffre", "rang": exemplaire.1, "mise_a_disposition_id": mad }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(mad)
    }

    /// **Retirer** une mise à disposition (DCT-88, D40 erreur de destinataire) : `documents.classer`, motif obligatoire,
    /// journal, événement. Rend `true` si le destinataire l'avait déjà ouverte (l'ouverture antérieure reste visible).
    pub async fn retirer_mise_a_disposition_for_actor(
        &self,
        a: &Acteur<'_>,
        mise_a_disposition_id: Uuid,
        motif: &str,
    ) -> Result<bool, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsClasser).await?;
        let motif = texte(motif, 3, 500)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let m: Option<(Uuid, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<Uuid>, String)> = sqlx::query_as(
            "SELECT m.document_id, m.retiree_le, m.premiere_ouverture_le, d.eleve_id, d.nature_code
               FROM ent_doc_mise_a_disposition m
               JOIN ent_doc_document d ON d.etablissement_id = m.etablissement_id AND d.id = m.document_id
              WHERE m.etablissement_id = $1 AND m.id = $2 FOR UPDATE OF m",
        )
        .bind(a.etablissement_id)
        .bind(mise_a_disposition_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (document, retiree, ouverte, eleve, nature) = m.ok_or_else(introuvable)?;
        if retiree.is_some() {
            return Err(refus(RefusMetier::Conflit, "mise à disposition déjà retirée"));
        }
        sqlx::query(
            "UPDATE ent_doc_mise_a_disposition SET retiree_le = now(), retrait_motif = $3
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(mise_a_disposition_id)
        .bind(&motif)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            a,
            Some(document),
            None,
            "mise_a_disposition_retiree",
            (None, None),
            Some("coffre"),
            Some(&motif),
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            EVT_MISE_A_DISPOSITION_RETIREE,
            document,
            eleve,
            json!({ "nature": nature, "mise_a_disposition_id": mise_a_disposition_id, "ouverte": ouverte.is_some() }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ouverte.is_some())
    }

    /// **Rendre** explicitement une mise à disposition retirée (Q-D12 : « l'établissement peut en rendre une
    /// explicitement ») : `documents.classer`, motif obligatoire, **droit du destinataire relu aujourd'hui** (un lien
    /// toujours suspendu refuse). Une mise à disposition retirée ne se « réactive » pas (trigger 3703) : une **nouvelle**
    /// est créée sur une copie de l'exemplaire (même fichier, rang suivant).
    pub async fn rendre_mise_a_disposition_for_actor(
        &self,
        a: &Acteur<'_>,
        mise_a_disposition_id: Uuid,
        motif: &str,
    ) -> Result<Uuid, EntDbError> {
        self.exiger_action_documents(a, Action::DocumentsClasser).await?;
        let motif = texte(motif, 3, 500)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let m: Option<(Uuid, Uuid, Option<Uuid>, Option<Uuid>, Option<Uuid>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT document_id, exemplaire_id, personne_id, eleve_id, agent_id, retiree_le
               FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(mise_a_disposition_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (document_id, exemplaire_id, p, e, g, retiree) = m.ok_or_else(introuvable)?;
        if retiree.is_none() {
            return Err(refus(RefusMetier::Conflit, "cette mise à disposition n'est pas retirée"));
        }
        let dest = match (p, e, g) {
            (Some(p), _, _) => DestinataireMad::Personne(p),
            (_, Some(e), _) => DestinataireMad::Eleve(e),
            (_, _, Some(g)) => DestinataireMad::Agent(g),
            _ => return Err(introuvable()),
        };
        let doc = document_court(&mut tx, a.etablissement_id, document_id).await?;
        let domaine = Domaine::parse(&doc.domaine).ok_or(EntDbError::Forbidden)?;
        if doc.sensibilite == "C4" || !self.doc_peut_sur_sujet(a, domaine.action_lecture(), doc.eleve_id).await? {
            return Err(EntDbError::Forbidden);
        }
        if doc.etat != "emis" {
            return Err(refus(RefusMetier::Conflit, "seul un document émis se met à disposition"));
        }
        let jour = jour_etab(&mut tx, a.etablissement_id).await?;
        destinataire_valable(&mut tx, a.etablissement_id, &doc, dest, jour).await?;
        // L'exemplaire d'origine doit appartenir à la version courante ; sinon on repart du choix normal.
        let copie: Option<(Uuid, i32)> = sqlx::query_as(
            "INSERT INTO ent_doc_exemplaire
                 (id, etablissement_id, version_id, rang, destinataire_personne_id, destinataire_agent_id,
                  adresse_de_personne_id, cle_objet, empreinte_sha256)
             SELECT $3, ex.etablissement_id, ex.version_id,
                    (SELECT max(x.rang) + 1 FROM ent_doc_exemplaire x WHERE x.version_id = ex.version_id),
                    ex.destinataire_personne_id, ex.destinataire_agent_id, ex.adresse_de_personne_id,
                    ex.cle_objet, ex.empreinte_sha256
               FROM ent_doc_exemplaire ex
               JOIN ent_doc_version v ON v.etablissement_id = ex.etablissement_id AND v.id = ex.version_id
               JOIN ent_doc_document d ON d.etablissement_id = v.etablissement_id AND d.id = v.document_id
              WHERE ex.etablissement_id = $1 AND ex.id = $2 AND v.numero_version = d.version_courante
             RETURNING id, rang",
        )
        .bind(a.etablissement_id)
        .bind(exemplaire_id)
        .bind(Uuid::new_v4())
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (nouvel, rang) = match copie {
            Some(c) => c,
            None => choisir_exemplaire(&mut tx, a.etablissement_id, document_id, dest).await?,
        };
        let mad = inserer_mad(&mut tx, a, nouvel, dest).await?;
        journaliser(
            &mut tx,
            a,
            Some(document_id),
            Some(doc.version_courante),
            "mise_a_disposition_rendue",
            (None, None),
            Some("coffre"),
            Some(&motif),
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            EVT_MISE_A_DISPOSITION,
            document_id,
            doc.eleve_id,
            json!({ "nature": doc.nature_code, "canal": "coffre", "rang": rang, "mise_a_disposition_id": mad }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(mad)
    }

    /// **Exporter** son coffre (DCT-85, portabilité) : archive ZIP des PDF **qu'on a le droit de voir aujourd'hui** et
    /// `index.csv`. Chaque document laisse une consultation « téléchargement » (C2 et plus) ; l'export lui-même est
    /// journalisé. Borné (500 documents, 200 Mio) : au-delà, filtrer par enfant ou par année.
    pub async fn exporter_coffre_for_actor(
        &self,
        a: &Acteur<'_>,
        filtre: &FiltreCoffre,
    ) -> Result<ExportCoffre, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let (ctx, cands) = charger(&mut tx, a.etablissement_id, a.user_id, None).await?;
        let retenues: Vec<&Candidate> = cands.iter().filter(|c| ctx.autorise(c) && filtre_ok(filtre, c)).collect();
        if retenues.len() > EXPORT_MAX_DOCUMENTS {
            return Err(refus(
                RefusMetier::Conflit,
                "trop de documents pour un seul export : filtrez par enfant ou par année",
            ));
        }
        let eleves: Vec<Uuid> = retenues.iter().filter_map(|c| c.eleve_id).collect::<HashSet<_>>().into_iter().collect();
        let noms: HashMap<Uuid, String> = if eleves.is_empty() {
            HashMap::new()
        } else {
            sqlx::query_as::<_, (Uuid, String)>(
                "SELECT id, btrim(prenom || ' ' || nom) FROM ent_eleve WHERE etablissement_id = $1 AND id = ANY($2)",
            )
            .bind(a.etablissement_id)
            .bind(&eleves)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?
            .into_iter()
            .collect()
        };
        for c in &retenues {
            if Sensibilite::parse(&c.sensibilite).is_some_and(Sensibilite::trace_la_lecture) {
                tracer(&mut tx, a, c, "telechargement").await?;
            }
        }
        journaliser(
            &mut tx,
            a,
            None,
            None,
            "coffre_exporte",
            (None, None),
            Some(&format!("{} document(s)", retenues.len())),
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;

        let mut zip = EcrivainZip::nouveau();
        let mut index = String::from("\u{feff}fichier;numero;nature;titre;eleve;depuis;premiere_ouverture;empreinte_sha256\r\n");
        let mut total = 0usize;
        for c in &retenues {
            let octets = self
                .lire_objet_empreinte(a.etablissement_id, &c.cle_objet, Some(&c.empreinte_sha256))
                .await?;
            total += octets.len();
            if total > EXPORT_MAX_OCTETS {
                return Err(refus(RefusMetier::Conflit, "export trop volumineux : filtrez par enfant ou par année"));
            }
            let nom = nom_fichier_sur(c);
            zip.ajouter(&nom, &octets)
                .map_err(|_| refus(RefusMetier::Conflit, "export impossible"))?;
            index.push_str(&format!(
                "{};{};{};{};{};{};{};{}\r\n",
                cellule_csv(&nom),
                cellule_csv(&numero_affiche(c).unwrap_or_default()),
                cellule_csv(&c.nature_code),
                cellule_csv(&c.titre),
                cellule_csv(c.eleve_id.and_then(|e| noms.get(&e)).map(String::as_str).unwrap_or("")),
                c.depuis.format("%Y-%m-%d").to_string(),
                c.premiere_ouverture_le.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
                c.empreinte_sha256,
            ));
        }
        zip.ajouter("index.csv", index.as_bytes())
            .map_err(|_| refus(RefusMetier::Conflit, "export impossible"))?;
        let octets = zip.terminer();
        let sha256 = ent_object::sha256_hex(&octets);
        Ok(ExportCoffre {
            octets,
            nom_fichier: format!("coffre-scolia-{}.zip", Utc::now().format("%Y%m%d")),
            nb_documents: retenues.len(),
            sha256,
        })
    }

    /// Rattrape les suspensions de lien posées **sans événement** (deux chemins du socle 4 n'en publient pas) : retire
    /// du coffre les mises à disposition de personnes dont le lien avec l'élève du document est suspendu ou sans
    /// autorité. Contrôle de cohérence (DCT-128) ; rend le nombre de retraits. Le droit relu à chaque lecture protège
    /// déjà le passé : cette fonction ne fait que ranger.
    pub async fn reconcilier_liens_suspendus(&self, etab: Uuid) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let n = retirer_liens_invalides_tx(&mut tx, etab, None, None).await?;
        tx.commit().await.map_err(db)?;
        Ok(n)
    }

    /// Bascule du coffre pour **un** élève devenu majeur (DCT-92), hors événement (reprise manuelle, test).
    pub async fn basculer_coffre_majorite(&self, etab: Uuid, eleve_id: Uuid) -> Result<BasculeFaite, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (b, evs) = basculer_majorite_tx(&mut tx, etab, eleve_id, None).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evs);
        Ok(b)
    }
}

// ───────────────────────── Aides de transaction ─────────────────────────

pub(crate) fn est_introuvable(e: &EntDbError) -> bool {
    matches!(e, EntDbError::Domain(domain_eleve::DomainError::NotFound))
}

impl PgEnt {
    /// La mise à disposition de coffre **en cours** de la personne connectée sur ce document, s'il y en a une (la plus
    /// récente) : le droit n'est pas jugé ici, mais par [`PgEnt::ouvrir_du_coffre_for_actor`].
    pub(crate) async fn mise_a_disposition_du_titulaire(
        &self,
        a: &Acteur<'_>,
        document_id: Uuid,
    ) -> Result<Option<Uuid>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let t = resoudre_titulaires(&mut tx, a.etablissement_id, a.user_id).await?;
        let r: Option<(Uuid,)> = sqlx::query_as(
            "SELECT m.id FROM ent_doc_mise_a_disposition m
              WHERE m.etablissement_id = $1 AND m.document_id = $2 AND m.canal = 'coffre' AND m.retiree_le IS NULL
                AND (m.personne_id = ANY($3) OR m.eleve_id = ANY($4) OR m.agent_id = ANY($5))
              ORDER BY m.depuis DESC, m.id LIMIT 1",
        )
        .bind(a.etablissement_id)
        .bind(document_id)
        .bind(&t.personnes)
        .bind(&t.eleves)
        .bind(&t.agents)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(r.map(|x| x.0))
    }
}

async fn tracer(conn: &mut PgConnection, a: &Acteur<'_>, c: &Candidate, motif: &str) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_doc_consultation (id, etablissement_id, document_id, version_id, acteur_user_id, profil, canal, motif)
         VALUES ($1,$2,$3,$4,$5,$6,'web',$7)",
    )
    .bind(Uuid::new_v4())
    .bind(a.etablissement_id)
    .bind(c.document_id)
    .bind(c.version_id)
    .bind(a.user_id)
    .bind(a.role)
    .bind(motif)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct DocCourt {
    nature_code: String,
    sensibilite: String,
    domaine: String,
    etat: String,
    eleve_id: Option<Uuid>,
    personne_id: Option<Uuid>,
    agent_id: Option<Uuid>,
    version_courante: i32,
}

async fn document_court(conn: &mut PgConnection, etab: Uuid, id: Uuid) -> Result<DocCourt, EntDbError> {
    sqlx::query_as::<_, DocCourt>(
        "SELECT nature_code, sensibilite, domaine, etat, eleve_id, personne_id, agent_id, version_courante
           FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)
}

/// Le destinataire peut-il recevoir ce document **aujourd'hui** ? Même règle que la lecture, vue du côté de
/// l'attribution : titulaire à l'autorité parentale non suspendu (ou l'élève majeur) ; payeur seul après la majorité ;
/// élève mineur limité à sa scolarité jusqu'à C2 ; agent pour son propre dossier.
async fn destinataire_valable(
    conn: &mut PgConnection,
    etab: Uuid,
    doc: &DocCourt,
    dest: DestinataireMad,
    jour: NaiveDate,
) -> Result<(), EntDbError> {
    let sens = Sensibilite::parse(&doc.sensibilite).ok_or_else(invalide)?;
    let dom = Domaine::parse(&doc.domaine).ok_or_else(invalide)?;
    let non = |m: &'static str| Err(refus(RefusMetier::Conflit, m));
    match dest {
        DestinataireMad::Agent(g) => {
            if doc.agent_id != Some(g) {
                return non("ce document ne concerne pas cet agent");
            }
        }
        DestinataireMad::Eleve(e) => {
            if doc.eleve_id != Some(e) {
                return non("ce document ne concerne pas cet élève");
            }
            let majeur = majeurs_parmi(conn, etab, &[e], jour).await?.contains(&e);
            if !majeur && !(sens <= Sensibilite::C2 && dom == Domaine::Scolarite) {
                return non("un élève mineur ne reçoit pas ce document (nature ou sensibilité)");
            }
        }
        DestinataireMad::Personne(p) => match doc.eleve_id {
            None => {
                if doc.personne_id != Some(p) {
                    return non("ce document ne concerne pas cette personne");
                }
            }
            Some(e) => {
                let majeur = majeurs_parmi(conn, etab, &[e], jour).await?.contains(&e);
                let propre: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND id = $2 AND eleve_id = $3",
                )
                .bind(etab)
                .bind(p)
                .bind(e)
                .fetch_optional(&mut *conn)
                .await
                .map_err(db)?;
                if propre.is_some() {
                    if !majeur && !(sens <= Sensibilite::C2 && dom == Domaine::Scolarite) {
                        return non("un élève mineur ne reçoit pas ce document (nature ou sensibilité)");
                    }
                } else {
                    let titulaires = titulaires_destinataires(&mut *conn, etab, e, jour).await?;
                    if !titulaires.iter().any(|t| t.personne_id == Some(p)) {
                        return non("cette personne n'a pas l'autorité parentale active sur l'élève (lien absent ou suspendu)");
                    }
                    if majeur && dom != Domaine::Financier {
                        return non("l'élève est majeur : ses documents lui sont adressés, sauf ce qui relève du payeur");
                    }
                }
            }
        },
    }
    Ok(())
}

/// L'exemplaire à mettre à disposition : celui du destinataire s'il existe, sinon un exemplaire **sans adresse**.
async fn choisir_exemplaire(
    conn: &mut PgConnection,
    etab: Uuid,
    document_id: Uuid,
    dest: DestinataireMad,
) -> Result<(Uuid, i32), EntDbError> {
    let personne = match dest {
        DestinataireMad::Personne(p) => Some(p),
        _ => None,
    };
    let agent = match dest {
        DestinataireMad::Agent(g) => Some(g),
        _ => None,
    };
    let r: Option<(Uuid, i32)> = sqlx::query_as(
        "SELECT ex.id, ex.rang
           FROM ent_doc_exemplaire ex
           JOIN ent_doc_version v ON v.etablissement_id = ex.etablissement_id AND v.id = ex.version_id
           JOIN ent_doc_document d ON d.etablissement_id = v.etablissement_id AND d.id = v.document_id
          WHERE ex.etablissement_id = $1 AND d.id = $2 AND v.numero_version = d.version_courante
            AND ((ex.destinataire_personne_id IS NOT DISTINCT FROM $3 AND ex.destinataire_agent_id IS NOT DISTINCT FROM $4)
                 OR ex.adresse_de_personne_id IS NULL)
          ORDER BY (ex.destinataire_personne_id IS NOT DISTINCT FROM $3 AND ex.destinataire_agent_id IS NOT DISTINCT FROM $4) DESC, ex.rang
          LIMIT 1",
    )
    .bind(etab)
    .bind(document_id)
    .bind(personne)
    .bind(agent)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    r.ok_or_else(|| {
        refus(
            RefusMetier::Conflit,
            "aucun exemplaire ne peut être remis à ce destinataire sans révéler l'adresse d'un autre : réémettre le document à son nom",
        )
    })
}

async fn inserer_mad(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    exemplaire: Uuid,
    dest: DestinataireMad,
) -> Result<Uuid, EntDbError> {
    let (p, e, g) = match dest {
        DestinataireMad::Personne(p) => (Some(p), None, None),
        DestinataireMad::Eleve(e) => (None, Some(e), None),
        DestinataireMad::Agent(g) => (None, None, Some(g)),
    };
    let id = Uuid::new_v4();
    // `document_id` : lu depuis l'exemplaire pour que les deux colonnes ne divergent jamais.
    let r = sqlx::query(
        "INSERT INTO ent_doc_mise_a_disposition
             (id, etablissement_id, document_id, exemplaire_id, personne_id, eleve_id, agent_id, canal, cree_par)
         SELECT $1, ex.etablissement_id, v.document_id, ex.id, $3, $4, $5, 'coffre', $6
           FROM ent_doc_exemplaire ex
           JOIN ent_doc_version v ON v.etablissement_id = ex.etablissement_id AND v.id = ex.version_id
          WHERE ex.etablissement_id = $2 AND ex.id = $7",
    )
    .bind(id)
    .bind(a.etablissement_id)
    .bind(p)
    .bind(e)
    .bind(g)
    .bind(a.user_id)
    .bind(exemplaire)
    .execute(&mut *conn)
    .await;
    match r {
        Ok(x) if x.rows_affected() == 1 => Ok(id),
        Ok(_) => Err(introuvable()),
        Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23505") => Err(refus(
            RefusMetier::Conflit,
            "déjà mis à disposition de ce destinataire (retiré : la rendre explicitement)",
        )),
        Err(e) => Err(db(e)),
    }
}

/// Retire les mises à disposition de coffre dont le lien du jour n'est plus valable. `eleve`/`personne` restreignent.
async fn retirer_liens_invalides_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Option<Uuid>,
    personne: Option<Uuid>,
) -> Result<usize, EntDbError> {
    let jour = jour_etab(&mut *conn, etab).await?;
    let lignes: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "UPDATE ent_doc_mise_a_disposition m
            SET retiree_le = now(), retrait_motif = $5
           FROM ent_doc_document d
          WHERE m.etablissement_id = $1 AND d.etablissement_id = m.etablissement_id AND d.id = m.document_id
            AND m.canal = 'coffre' AND m.retiree_le IS NULL AND m.personne_id IS NOT NULL
            AND d.eleve_id IS NOT NULL
            AND ($2::uuid IS NULL OR d.eleve_id = $2) AND ($3::uuid IS NULL OR m.personne_id = $3)
            -- la fiche de l'élève lui-même n'est pas un lien
            AND NOT EXISTS (SELECT 1 FROM ent_personne pe WHERE pe.etablissement_id = m.etablissement_id
                               AND pe.id = m.personne_id AND pe.eleve_id = d.eleve_id)
            AND NOT EXISTS (SELECT 1 FROM ent_eleve_lien l
                             WHERE l.etablissement_id = m.etablissement_id AND l.personne_id = m.personne_id
                               AND l.eleve_id = d.eleve_id AND l.autorite_parentale AND l.suspendu_le IS NULL
                               AND (l.date_debut IS NULL OR l.date_debut <= $4) AND (l.date_fin IS NULL OR l.date_fin >= $4))
        RETURNING m.id, m.document_id",
    )
    .bind(etab)
    .bind(eleve)
    .bind(personne)
    .bind(jour)
    .bind(MOTIF_LIEN_SUSPENDU)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let systeme = Acteur { etablissement_id: etab, user_id: Uuid::nil(), role: "systeme" };
    for (_, doc) in &lignes {
        journaliser(
            &mut *conn,
            &systeme,
            Some(*doc),
            None,
            "mise_a_disposition_retiree",
            (None, None),
            Some("coffre"),
            Some(MOTIF_LIEN_SUSPENDU),
        )
        .await?;
    }
    Ok(lignes.len())
}

/// Bascule d'un élève devenu majeur : voir l'en-tête du module. Rend ce qui a été fait et les événements à diffuser.
async fn basculer_majorite_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    eleve: Uuid,
    causalite: Option<crate::outbox::Causalite>,
) -> Result<(BasculeFaite, Vec<ent_events::MetierEvent>), EntDbError> {
    let jour = jour_etab(&mut *conn, etab).await?;
    if !majeurs_parmi(&mut *conn, etab, &[eleve], jour).await?.contains(&eleve) {
        return Ok((BasculeFaite::default(), Vec::new()));
    }
    let mut fait = BasculeFaite::default();
    let systeme = Acteur { etablissement_id: etab, user_id: Uuid::nil(), role: "systeme" };
    // 1. Les parents ne reçoivent plus les documents de l'élève (le payeur garde ce qui relève de la facturation).
    let retires: Vec<(Uuid,)> = sqlx::query_as(
        "UPDATE ent_doc_mise_a_disposition m
            SET retiree_le = now(), retrait_motif = $3
           FROM ent_doc_document d
          WHERE m.etablissement_id = $1 AND d.etablissement_id = m.etablissement_id AND d.id = m.document_id
            AND d.eleve_id = $2 AND d.domaine <> 'financier'
            AND m.canal = 'coffre' AND m.retiree_le IS NULL AND m.personne_id IS NOT NULL
            AND NOT EXISTS (SELECT 1 FROM ent_personne pe WHERE pe.etablissement_id = m.etablissement_id
                               AND pe.id = m.personne_id AND pe.eleve_id = $2)
        RETURNING m.document_id",
    )
    .bind(etab)
    .bind(eleve)
    .bind(MOTIF_MAJORITE)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    fait.retirees = retires.len();
    // 2. L'élève reçoit ses documents (C2 au plus sans droit explicite), sur un exemplaire sans adresse.
    let documents: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT d.id, d.nature_code FROM ent_doc_document d
          WHERE d.etablissement_id = $1 AND d.eleve_id = $2 AND d.etat = 'emis'
            AND d.sensibilite IN ('C0','C1','C2') AND d.domaine <> 'financier'
            AND d.version_courante >= 1 AND d.origine = 'emis'
          ORDER BY d.cree_le, d.id",
    )
    .bind(etab)
    .bind(eleve)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    for (doc, _) in documents {
        let deja: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM ent_doc_mise_a_disposition WHERE etablissement_id = $1 AND document_id = $2
                AND eleve_id = $3 AND canal = 'coffre' AND retiree_le IS NULL",
        )
        .bind(etab)
        .bind(doc)
        .bind(eleve)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        if deja.is_some() {
            continue;
        }
        let ex: Option<(Uuid,)> = sqlx::query_as(
            "SELECT ex.id FROM ent_doc_exemplaire ex
               JOIN ent_doc_version v ON v.etablissement_id = ex.etablissement_id AND v.id = ex.version_id
               JOIN ent_doc_document d ON d.etablissement_id = v.etablissement_id AND d.id = v.document_id
              WHERE ex.etablissement_id = $1 AND d.id = $2 AND v.numero_version = d.version_courante
                AND ex.adresse_de_personne_id IS NULL
              ORDER BY ex.rang LIMIT 1",
        )
        .bind(etab)
        .bind(doc)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?;
        let Some((ex,)) = ex else {
            fait.sans_exemplaire_neutre += 1;
            continue;
        };
        let r = sqlx::query(
            "INSERT INTO ent_doc_mise_a_disposition
                 (id, etablissement_id, document_id, exemplaire_id, eleve_id, canal, cree_par)
             VALUES ($1,$2,$3,$4,$5,'coffre',NULL)
             ON CONFLICT DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(doc)
        .bind(ex)
        .bind(eleve)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        fait.ouvertes += r.rows_affected() as usize;
    }
    let mut evs = Vec::new();
    if fait.retirees + fait.ouvertes + fait.sans_exemplaire_neutre > 0 {
        journaliser(
            &mut *conn,
            &systeme,
            None,
            None,
            "coffre_bascule_majorite",
            (None, None),
            Some(&eleve.to_string()),
            Some(&format!(
                "{} retiré(s), {} ouvert(s) à l'élève, {} à réémettre",
                fait.retirees, fait.ouvertes, fait.sans_exemplaire_neutre
            )),
        )
        .await?;
        let draft = MetierEventDraft {
            etablissement_id: etab,
            type_name: EVT_COFFRE_BASCULE.to_string(),
            aggregate: AggregateKind::Documents,
            aggregate_id: eleve,
            eleve_id: Some(eleve),
            payload: json!({
                "retirees": fait.retirees, "ouvertes": fait.ouvertes, "a_reemettre": fait.sans_exemplaire_neutre,
            }),
            actor_user_id: None,
        };
        let ev = match causalite {
            Some(c) => conn.publier_avec(draft, c).await,
            None => conn.publier(draft).await,
        }
        ?;
        evs.push(ev);
    }
    Ok((fait, evs))
}

// ───────────────────────── Consommateur (outbox, socle 8) ─────────────────────────

/// Réagit aux événements du coffre : notifie (`documents.mis_a_disposition`), bascule à la majorité (`eleve.majeur`),
/// retire sur suspension (`lien.suspendu`), prévient l'élève (`documents.coffre_bascule`). Rejouable : chaque effet est
/// idempotent (retrait une fois, mise à disposition unique, notification unique par événement).
pub struct ConsommateurCoffre;

const TYPES_COFFRE: &[&str] = &[
    EVT_MISE_A_DISPOSITION,
    EVT_COFFRE_BASCULE,
    types::ELEVE_MAJEUR,
    types::LIEN_SUSPENDU,
];

impl Consommateur for ConsommateurCoffre {
    fn nom(&self) -> &'static str {
        CONSOMMATEUR_COFFRE
    }

    fn types(&self) -> &'static [&'static str] {
        TYPES_COFFRE
    }

    fn traiter<'a>(&'a self, tx: &'a mut PgConnection, evt: &'a EvenementDistribue) -> FutureTraitement<'a> {
        Box::pin(async move { traiter_coffre(tx, evt).await.map_err(|e| e.to_string()) })
    }
}

async fn traiter_coffre(tx: &mut PgConnection, evt: &EvenementDistribue) -> Result<(), EntDbError> {
    let etab = evt.etablissement_id();
    let e = evt.event();
    match e.type_name.as_str() {
        t if t == types::ELEVE_MAJEUR => {
            if let Some(eleve) = e.eleve_id {
                basculer_majorite_tx(tx, etab, eleve, Some(evt.causalite_suivante())).await?;
            }
        }
        t if t == types::LIEN_SUSPENDU => {
            let personne = e
                .payload
                .get("personne_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());
            retirer_liens_invalides_tx(tx, etab, e.eleve_id, personne).await?;
        }
        t if t == EVT_MISE_A_DISPOSITION => {
            let Some(mad) = e
                .payload
                .get("mise_a_disposition_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
            else {
                return Ok(());
            };
            // Ni nature, ni titre, ni nom d'élève, ni pièce jointe : « un document vous attend » (Q-D13).
            let user: Option<(Uuid,)> = sqlx::query_as(
                "SELECT coalesce(p.user_id, c.user_id, pa.user_id)
                   FROM ent_doc_mise_a_disposition m
                   LEFT JOIN ent_personne p ON p.etablissement_id = m.etablissement_id AND p.id = m.personne_id
                   LEFT JOIN ent_eleve_compte c ON c.etablissement_id = m.etablissement_id AND c.eleve_id = m.eleve_id
                   LEFT JOIN ent_agent ag ON ag.etablissement_id = m.etablissement_id AND ag.id = m.agent_id
                   LEFT JOIN ent_personne pa ON pa.etablissement_id = ag.etablissement_id AND pa.id = ag.personne_id
                  WHERE m.etablissement_id = $1 AND m.id = $2 AND m.canal = 'coffre' AND m.retiree_le IS NULL
                    AND coalesce(p.user_id, c.user_id, pa.user_id) IS NOT NULL",
            )
            .bind(etab)
            .bind(mad)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((user_id,)) = user {
                notifier(tx, evt, user_id, "document_disponible", "Un document vous attend", TEXTE_COFFRE).await?;
            }
        }
        t if t == EVT_COFFRE_BASCULE => {
            let Some(eleve) = e.eleve_id else { return Ok(()) };
            let user: Option<(Uuid,)> = sqlx::query_as(
                "SELECT coalesce(c.user_id, p.user_id)
                   FROM ent_eleve el
                   LEFT JOIN ent_eleve_compte c ON c.etablissement_id = el.etablissement_id AND c.eleve_id = el.id
                   LEFT JOIN ent_personne p ON p.etablissement_id = el.etablissement_id AND p.eleve_id = el.id
                  WHERE el.etablissement_id = $1 AND el.id = $2 AND coalesce(c.user_id, p.user_id) IS NOT NULL
                  LIMIT 1",
            )
            .bind(etab)
            .bind(eleve)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            if let Some((user_id,)) = user {
                notifier(
                    tx,
                    evt,
                    user_id,
                    "coffre_majorite",
                    "Votre coffre de documents est désormais le vôtre",
                    "Vous êtes majeur : vos documents vous sont adressés. Connectez-vous à ScolIA pour les consulter.",
                )
                .await?;
            }
        }
        _ => {}
    }
    Ok(())
}

const TEXTE_COFFRE: &str = "Connectez-vous à ScolIA pour le consulter dans votre coffre.";

async fn notifier(
    tx: &mut PgConnection,
    evt: &EvenementDistribue,
    user_id: Uuid,
    kind: &str,
    titre: &str,
    corps: &str,
) -> Result<(), EntDbError> {
    notifier_personne(
        tx,
        EnvoiAPlanifier {
            etablissement_id: evt.etablissement_id(),
            user_id,
            kind,
            titre,
            corps,
            eleve_id: None,
            notification_id: None,
            source_type: "evenement",
            source_id: evt.event().id,
            cle_dedup: format!("evt:{}:{}", evt.event().id, kind),
            droit_requis: None,
            // Trois documents arrivés le même jour : un seul message, dans le résumé de la personne.
            regroupable: true,
            canaux_voulus: &["email", "sms", "push"],
        },
        evt.event().id,
        Utc::now(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(sens: &str, dom: &str) -> Candidate {
        Candidate {
            mad_id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            version_id: Uuid::new_v4(),
            nature_code: "eleve.certificat_scolarite".into(),
            titre: "Certificat".into(),
            sensibilite: sens.into(),
            domaine: dom.into(),
            eleve_id: None,
            serie_code: Some("SCO".into()),
            numero_annee: Some(2026),
            numero: Some(7),
            depuis: Utc::now(),
            premiere_ouverture_le: None,
            mad_personne: None,
            mad_eleve: None,
            mad_agent: None,
            rang: 1,
            numero_version: 1,
            cle_objet: "x".into(),
            empreinte_sha256: "0".repeat(64),
            annee: 2026,
        }
    }

    fn ctx(personnes: &[Uuid], eleves: &[Uuid], liens: &[(Uuid, Uuid)], majeurs: &[Uuid]) -> ContexteCoffre {
        ContexteCoffre {
            titulaires: TitulairesCoffre {
                personnes: personnes.to_vec(),
                personnes_eleves: HashSet::new(),
                eleves: eleves.to_vec(),
                agents: vec![],
            },
            liens: liens.iter().copied().collect(),
            majeurs: majeurs.iter().copied().collect(),
        }
    }

    #[test]
    fn parent_avec_lien_actif_voit_parent_sans_lien_ne_voit_pas() {
        let (p, e) = (Uuid::new_v4(), Uuid::new_v4());
        let mut c = cand("C1", "scolarite");
        c.eleve_id = Some(e);
        c.mad_personne = Some(p);
        assert!(ctx(&[p], &[], &[(p, e)], &[]).autorise(&c));
        assert!(!ctx(&[p], &[], &[], &[]).autorise(&c), "lien suspendu, absent ou sans autorité");
        assert!(!ctx(&[Uuid::new_v4()], &[], &[(p, e)], &[]).autorise(&c), "la mise à disposition d'un autre");
    }

    #[test]
    fn apres_la_majorite_les_parents_gardent_le_payeur_seulement() {
        let (p, e) = (Uuid::new_v4(), Uuid::new_v4());
        let mut c = cand("C1", "scolarite");
        c.eleve_id = Some(e);
        c.mad_personne = Some(p);
        let x = ctx(&[p], &[], &[(p, e)], &[e]);
        assert!(!x.autorise(&c));
        c.domaine = "financier".into();
        assert!(x.autorise(&c));
    }

    #[test]
    fn eleve_mineur_limite_a_sa_scolarite_jusqu_a_c2() {
        let e = Uuid::new_v4();
        let mut c = cand("C1", "scolarite");
        c.eleve_id = Some(e);
        c.mad_eleve = Some(e);
        let mineur = ctx(&[], &[e], &[], &[]);
        assert!(mineur.autorise(&c));
        c.sensibilite = "C3".into();
        assert!(!mineur.autorise(&c));
        c.sensibilite = "C1".into();
        for d in ["sante", "justice", "financier", "disciplinaire", "social"] {
            c.domaine = d.into();
            assert!(!mineur.autorise(&c), "{d}");
        }
        c.domaine = "disciplinaire".into();
        assert!(ctx(&[], &[e], &[], &[e]).autorise(&c), "majeur : ce qui lui est adressé");
    }

    #[test]
    fn c4_jamais_et_document_d_un_autre_eleve_jamais() {
        let (e, autre) = (Uuid::new_v4(), Uuid::new_v4());
        let mut c = cand("C4", "scolarite");
        c.eleve_id = Some(e);
        c.mad_eleve = Some(e);
        assert!(!ctx(&[], &[e], &[], &[e]).autorise(&c));
        c.sensibilite = "C1".into();
        c.eleve_id = Some(autre);
        assert!(!ctx(&[], &[e], &[], &[e]).autorise(&c), "un document sur un autre élève");
    }

    #[test]
    fn document_pour_moi_sans_lien() {
        let p = Uuid::new_v4();
        let mut c = cand("C2", "financier");
        c.mad_personne = Some(p);
        assert!(ctx(&[p], &[], &[], &[]).autorise(&c));
        assert!(!ctx(&[], &[], &[], &[]).autorise(&c));
    }

    #[test]
    fn csv_neutralise_les_formules() {
        assert_eq!(cellule_csv("=1+1"), "'=1+1");
        assert_eq!(cellule_csv("@SUM(A1)"), "'@SUM(A1)");
        assert_eq!(cellule_csv("a;b"), "\"a;b\"");
        assert_eq!(cellule_csv("dit \"oui\""), "\"dit \"\"oui\"\"\"");
        assert_eq!(cellule_csv("ligne\nsuivante"), "ligne suivante");
    }

    #[test]
    fn accuse_demande_pour_convocations_et_notifications() {
        assert!(accuse_demande("discipline.convocation_conseil"));
        assert!(accuse_demande("inscriptions.notification_decision"));
        assert!(!accuse_demande("eleve.certificat_scolarite"));
    }

    #[test]
    fn nom_de_fichier_sur() {
        let c = cand("C1", "scolarite");
        let n = nom_fichier_sur(&c);
        assert!(n.starts_with("documents/2026-SCO-000007_eleve_certificat_scolarite_"), "{n}");
        assert!(n.ends_with(".pdf") && !n.contains(".."));
    }
}
