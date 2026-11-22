//! Socle 3, lot I4 — **accords des responsables** (`docs/socles/03_INSCRIPTIONS.md` §4.4, §8.4, INS-45 à INS-57).
//!
//! * Une **règle de campagne** (`accord_responsables`) dit qui doit consentir : `un_seul` (réinscription,
//!   public) ou `tous_les_titulaires` (admission dans le privé). Le logiciel ne qualifie pas l'acte : il applique
//!   la règle.
//! * Un **titulaire** est une déclaration de la démarche dont l'autorité parentale est « oui ». Un parent sans
//!   autorité (« non », ou « inconnue » tant que l'établissement n'a pas tranché) **ne décide pas** : sa
//!   réponse est refusée côté serveur, même avec un lien valide. L'autorité exclusive déclarée ne vaut que si une
//!   décision de justice **acceptée** est au dossier (sinon un parent pourrait écarter l'autre en la déclarant).
//! * **Réponses** : en ligne **sans compte** (lien à usage unique dérivé d'un secret serveur + code à six chiffres
//!   envoyé à la même adresse, dix minutes, cinq essais), depuis un compte (session vérifiée, lien d'autorité
//!   actif quand l'élève existe), ou sur **papier** (scanné, pièce acceptée, saisi par le secrétariat avec son
//!   nom). Rien n'est stocké en clair : ni jeton, ni code.
//! * **Désaccord** : un refus d'un titulaire (en ligne, ou déclaré par le secrétariat pour un parent qui écrit ou
//!   appelle) **bloque** le dossier (`bloque_desaccord`) : aucune décision, confirmation ni validation. Message
//!   **neutre** aux titulaires (il ne dit pas qui a refusé). La levée se fait **avec une pièce** (accord écrit du
//!   parent, ou décision de justice) par la direction.
//! * **« Ne pas solliciter »** (danger, ordonnance de protection) : aucune demande, relance ni copie ; l'accord
//!   se **dispense** par décision motivée de la direction. Sans cela, le dossier attend.
//! * **Historique** : chaque changement laisse une ligne d'ajout seul (qui, comment, quand, quelle pièce).
//!
//! **Signature** : si un accord devait être signé (contrat, mandat), la brique de signature du socle 7 (lot D6)
//! s'y branchera par le port [`PortSignatureAccord`] ; ce lot ne la recode pas.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use ent_auth::Action;
use ent_events::{types, AggregateKind};
use ent_types::inscriptions_pieces::{
    AccordDto, AccordPapierBody, AccordPublicDto, AccordsDossierDto, DemanderAccordsBody, DesaccordBody,
    DispenseAccordBody, LeverDesaccordBody,
};
use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::inscriptions::{
    commit, conflit, dossier_tx, introuvable, invalide, publier, suivi_tx, transitionner_tx, DossierRow,
};
use crate::inscriptions_etats::{etat_effectif, Cause};
use crate::ref_commun::db;
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Durée de validité d'une demande d'accord (jours).
pub const ACCORD_VALIDITE_JOURS: i32 = 15;
/// Envois de code par demande.
pub const CODES_PAR_DEMANDE_MAX: i16 = 5;
/// Durée de vie d'un code (minutes) et nombre d'essais.
pub const CODE_VALIDITE_MIN: i32 = 10;
pub const CODE_ESSAIS_MAX: i16 = 5;

/// Port de signature (lot D6 du socle 7, en cours dans une autre session) : un accord qui doit être **signé**
/// (contrat de scolarisation, mandat) passe par le circuit de signature, jamais par ce module. Aucune
/// implémentation ici : le jour où D6 est fusionné, l'adaptateur appelle la brique et rend la preuve.
pub trait PortSignatureAccord {
    /// Demande la signature de l'accord `accord_id` aux signataires (déclarations de la démarche).
    fn demander_signature(&self, etab: Uuid, accord_id: Uuid, declarations: &[Uuid]) -> Result<(), String>;
}

/// Objet de l'accord attendu pour la nature d'un dossier (`None` : aucun accord à recueillir).
pub fn objet_de_la_nature(nature: &str) -> Option<&'static str> {
    match nature {
        "admission" | "en_cours_annee" => Some("admission"),
        "reinscription" => Some("reinscription"),
        _ => None,
    }
}

// ───────────────────────────── Jeton et code dérivés ─────────────────────────────

/// Jeton du lien : dérivé (HMAC) de l'identifiant de la demande sous le secret serveur ; il se recalcule au
/// moment d'écrire le message, il n'est jamais stocké.
pub(crate) fn jeton_accord(demande_id: Uuid) -> Result<String, String> {
    let sb = ent_auth::SecretBox::from_env().map_err(|e| e.to_string())?;
    sb.hmac_hex("accord-v1", demande_id.as_bytes()).map_err(|e| e.to_string())
}

/// Code à six chiffres de l'envoi numéro `compteur` : dérivé aussi, valable tant que le compteur ne change pas.
pub(crate) fn code_accord(demande_id: Uuid, compteur: i16) -> Result<String, String> {
    let sb = ent_auth::SecretBox::from_env().map_err(|e| e.to_string())?;
    let mut m = demande_id.as_bytes().to_vec();
    m.extend_from_slice(&compteur.to_be_bytes());
    let h = sb.hmac_hex("accord-code-v1", &m).map_err(|e| e.to_string())?;
    let n = u32::from_str_radix(&h[..8], 16).map_err(|e| e.to_string())?;
    Ok(format!("{:06}", n % 1_000_000))
}

/// Comparaison sans fuite de durée : on compare les empreintes de ce qui est présenté et de ce qui est attendu.
fn egaux(a: &str, b: &str) -> bool {
    ent_object::sha256_hex(a.as_bytes()) == ent_object::sha256_hex(b.as_bytes())
}

// ───────────────────────────── Titulaires ─────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct Titulaire {
    pub id: Uuid,
    pub ordre: i16,
    pub nom: String,
    pub email: Option<String>,
    pub autorite: String,
    pub exclusive: bool,
    pub ne_pas_solliciter: bool,
    pub est_deposant: bool,
}

async fn declarations_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
) -> Result<Vec<Titulaire>, EntDbError> {
    let rows: Vec<(Uuid, i16, String, Option<String>, String, bool, bool, bool)> = sqlx::query_as(
        "SELECT id, ordre, nom, email, autorite_parentale, autorite_exclusive_declaree, ne_pas_solliciter, est_deposant
           FROM ent_inscription_declaration_responsable
          WHERE etablissement_id = $1 AND demarche_id = $2
            AND (cardinality(pour_enfants) = 0 OR $3 = ANY (pour_enfants))
          ORDER BY ordre",
    )
    .bind(etab)
    .bind(d.demarche_id)
    .bind(d.id)
    .fetch_all(conn)
    .await
    .map_err(db)?;
    Ok(rows
        .into_iter()
        .map(|(id, ordre, nom, email, autorite, exclusive, nps, dep)| Titulaire {
            id,
            ordre,
            nom,
            email,
            autorite,
            exclusive,
            ne_pas_solliciter: nps,
            est_deposant: dep,
        })
        .collect())
}

/// Une décision de justice **acceptée** est-elle au dossier ? (Condition de l'autorité exclusive.)
async fn decision_de_justice_acceptee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
) -> Result<bool, EntDbError> {
    let (ok,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_inscription_piece
                         WHERE etablissement_id = $1 AND dossier_id = $2 AND code = 'decision_justice'
                           AND applicable AND statut = 'acceptee')",
    )
    .bind(etab)
    .bind(dossier_id)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(ok)
}

/// Les titulaires dont l'accord compte : autorité « oui » ; si une autorité exclusive est déclarée **et**
/// appuyée par une décision de justice acceptée, elle seule compte.
async fn titulaires_requis_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    toutes: &[Titulaire],
) -> Result<Vec<Titulaire>, EntDbError> {
    let mut requis: Vec<Titulaire> = toutes.iter().filter(|t| t.autorite == "oui").cloned().collect();
    if requis.iter().any(|t| t.exclusive) && decision_de_justice_acceptee_tx(conn, etab, d.id).await? {
        requis.retain(|t| t.exclusive);
    }
    Ok(requis)
}

/// Marque « à ne pas solliciter » : sur la déclaration, ou sur la fiche (réserve du socle 4) de la même adresse.
async fn protege_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    t: &Titulaire,
) -> Result<bool, EntDbError> {
    if t.ne_pas_solliciter {
        return Ok(true);
    }
    let Some(email) = t.email.as_deref() else {
        return Ok(false);
    };
    let (p,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_personne_reserve nps
                          JOIN ent_personne p ON p.id = nps.personne_id AND p.etablissement_id = nps.etablissement_id
                         WHERE nps.etablissement_id = $1 AND nps.ne_pas_solliciter AND lower(p.email) = lower($2))",
    )
    .bind(etab)
    .bind(email)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(p)
}

type LigneAccord = (Uuid, Uuid, String, String, Option<String>, Option<DateTime<Utc>>, Option<Uuid>);

async fn accords_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
    objet: &str,
) -> Result<Vec<LigneAccord>, EntDbError> {
    sqlx::query_as(
        "SELECT id, declaration_id, objet, reponse, moyen, repondu_le, piece_id FROM ent_inscription_accord
          WHERE etablissement_id = $1 AND dossier_id = $2 AND objet = $3",
    )
    .bind(etab)
    .bind(dossier_id)
    .bind(objet)
    .fetch_all(conn)
    .await
    .map_err(db)
}

async fn regle_campagne_tx(conn: &mut PgConnection, etab: Uuid, campagne: Uuid) -> Result<String, EntDbError> {
    sqlx::query_scalar("SELECT accord_responsables FROM ent_inscription_campagne WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(campagne)
        .fetch_one(conn)
        .await
        .map_err(db)
}

/// La règle est-elle satisfaite pour l'objet ? Aucun refus non levé ; `un_seul` : au moins un titulaire a
/// donné son accord ; `tous_les_titulaires` : chaque titulaire requis a donné son accord ou en est dispensé.
pub(crate) fn complet(regle: &str, requis: &[Titulaire], accords: &[LigneAccord]) -> bool {
    let reponse = |decl: Uuid| accords.iter().find(|a| a.1 == decl).map(|a| a.3.as_str());
    if requis.iter().any(|t| reponse(t.id) == Some("refuse")) {
        return false;
    }
    if requis.is_empty() {
        return false;
    }
    if regle == "tous_les_titulaires" {
        requis.iter().all(|t| matches!(reponse(t.id), Some("donne" | "dispense")))
    } else {
        requis.iter().any(|t| reponse(t.id) == Some("donne"))
    }
}

/// Garde de la décision, de la confirmation et de la validation : le dossier n'est pas bloqué et ses accords
/// sont complets. Un objet sans accord (affectés) ne se garde pas.
pub(crate) async fn exiger_accords_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
) -> Result<(), EntDbError> {
    if d.etat == "bloque_desaccord" {
        return Err(conflit(
            "dossier bloqué : désaccord des responsables (levée avec pièce par la direction)",
        ));
    }
    let Some(objet) = objet_de_la_nature(&d.nature) else {
        return Ok(());
    };
    let regle = regle_campagne_tx(&mut *conn, etab, d.campagne_id).await?;
    let toutes = declarations_tx(&mut *conn, etab, d).await?;
    let requis = titulaires_requis_tx(&mut *conn, etab, d, &toutes).await?;
    let accords = accords_tx(&mut *conn, etab, d.id, objet).await?;
    // Réinscription : la confirmation de la famille vaut accord sous la règle `un_seul` (elle repose sur le lien
    // du socle 4) ; on n'exige alors que l'absence de refus. Dossier dont aucun accord n'a jamais été ouvert
    // (écrit avant le lot I4) sous `un_seul` : de même.
    if regle == "un_seul"
        && (d.nature == "reinscription" || accords.is_empty())
        && !requis.iter().any(|t| accords.iter().any(|a| a.1 == t.id && a.3 == "refuse"))
    {
        return Ok(());
    }
    if complet(&regle, &requis, &accords) {
        Ok(())
    } else if regle == "tous_les_titulaires" {
        Err(conflit("accord de tous les titulaires de l'autorité parentale attendu"))
    } else {
        Err(conflit("accord d'un titulaire de l'autorité parentale attendu"))
    }
}

// ───────────────────────────── Écritures de base ─────────────────────────────

/// Une réponse à écrire : sens, moyen, pièce, motif, auteur (identifiant d'agent et type d'acteur).
struct Reponse<'a> {
    vers: &'a str,
    moyen: &'a str,
    piece_id: Option<Uuid>,
    motif: Option<&'a str>,
    acteur: (Option<Uuid>, &'a str),
}

impl<'a> Reponse<'a> {
    fn simple(vers: &'a str, moyen: &'a str, acteur: (Option<Uuid>, &'a str)) -> Self {
        Self { vers, moyen, piece_id: None, motif: None, acteur }
    }
}

/// Écrit la réponse d'un accord et sa trace d'ajout seul.
async fn ecrire_reponse_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    accord_id: Uuid,
    r: &Reponse<'_>,
) -> Result<(), EntDbError> {
    let avant: String =
        sqlx::query_scalar("SELECT reponse FROM ent_inscription_accord WHERE etablissement_id = $1 AND id = $2 FOR UPDATE")
            .bind(etab)
            .bind(accord_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db)?;
    sqlx::query(
        "UPDATE ent_inscription_accord
            SET reponse = $3, moyen = $4, piece_id = $5, motif = $6, repondu_le = now(), saisi_par = $7
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(accord_id)
    .bind(r.vers)
    .bind(r.moyen)
    .bind(r.piece_id)
    .bind(r.motif)
    .bind(r.acteur.0)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO ent_inscription_accord_trace
            (id, etablissement_id, accord_id, de_reponse, vers_reponse, moyen, acteur_user_id, acteur_type, piece_id, motif)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(accord_id)
    .bind(&avant)
    .bind(r.vers)
    .bind(r.moyen)
    .bind(r.acteur.0)
    .bind(r.acteur.1)
    .bind(r.piece_id)
    .bind(r.motif)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    // Une demande en cours est close dès qu'une réponse est enregistrée, par quelque moyen que ce soit.
    sqlx::query(
        "UPDATE ent_inscription_accord_demande SET etat = 'repondue', close_le = now()
          WHERE etablissement_id = $1 AND accord_id = $2 AND etat = 'envoyee'",
    )
    .bind(etab)
    .bind(accord_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Crée (si besoin) les lignes d'accord « attendu » d'un dossier pour les titulaires requis.
async fn assurer_accords_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    objet: &str,
    toutes: &[Titulaire],
) -> Result<(), EntDbError> {
    for t in toutes.iter().filter(|t| t.autorite == "oui") {
        sqlx::query(
            "INSERT INTO ent_inscription_accord (id, etablissement_id, dossier_id, declaration_id, objet)
             VALUES ($1,$2,$3,$4,$5) ON CONFLICT (dossier_id, declaration_id, objet) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(d.id)
        .bind(t.id)
        .bind(objet)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

async fn nom_etablissement_tx(conn: &mut PgConnection, etab: Uuid) -> Result<String, EntDbError> {
    let n: Option<String> = sqlx::query_scalar("SELECT nom FROM ent_etablissement WHERE id = $1")
        .bind(etab)
        .fetch_optional(conn)
        .await
        .map_err(db)?;
    Ok(n.unwrap_or_else(|| "l'établissement".into()))
}

/// Ouvre une demande d'accord (la précédente, si elle existe, est annulée) et met le message en file.
/// Jamais vers une personne à ne pas solliciter, jamais vers une adresse absente.
async fn creer_demande_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    accord_id: Uuid,
    t: &Titulaire,
    actor: Option<Uuid>,
) -> Result<Option<Uuid>, EntDbError> {
    let Some(email) = t.email.as_deref().filter(|e| !e.trim().is_empty()) else {
        return Ok(None);
    };
    if protege_tx(&mut *conn, etab, t).await? {
        return Ok(None);
    }
    sqlx::query(
        "UPDATE ent_inscription_accord_demande SET etat = 'annulee', close_le = now()
          WHERE etablissement_id = $1 AND accord_id = $2 AND etat = 'envoyee'",
    )
    .bind(etab)
    .bind(accord_id)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let demande = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_inscription_accord_demande (id, etablissement_id, accord_id, expire_le, cree_par)
         VALUES ($1,$2,$3, now() + make_interval(days => $4), $5)",
    )
    .bind(demande)
    .bind(etab)
    .bind(accord_id)
    .bind(ACCORD_VALIDITE_JOURS)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    let nom_etab = nom_etablissement_tx(&mut *conn, etab).await?;
    let corps = format!(
        "Bonjour,\n\n{nom_etab} a reçu une demande d'inscription pour {}. Votre accord est nécessaire.\n\n\
         Ouvrez ce lien (il ne sert qu'une fois, pendant {ACCORD_VALIDITE_JOURS} jours) : {{LIEN}}\n\n\
         Aucun compte n'est nécessaire. Si vous n'êtes pas concerné(e), ignorez ce message.",
        d.eleve_prenom
    );
    sqlx::query(
        "INSERT INTO ent_inscription_envoi
            (id, etablissement_id, dossier_id, cle, kind, destinataire, sujet, corps, accord_demande_id)
         VALUES ($1,$2,$3,$4,'information',$5,$6,$7,$8)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(d.id)
    .bind(format!("accord:{demande}"))
    .bind(email)
    .bind("Votre accord pour une inscription")
    .bind(corps)
    .bind(demande)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    publier(
        &mut *conn,
        etab,
        types::INSCRIPTION_ACCORD_DEMANDE,
        AggregateKind::Admission,
        d.id,
        d.eleve_id,
        json!({ "dossier_id": d.id, "objet": "admission" }),
        actor,
    )
    .await?;
    Ok(Some(demande))
}

/// Message **neutre** aux titulaires quand le dossier est bloqué : il ne dit pas qui a refusé.
async fn informer_desaccord_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    toutes: &[Titulaire],
) -> Result<(), EntDbError> {
    let nom_etab = nom_etablissement_tx(&mut *conn, etab).await?;
    for t in toutes.iter().filter(|t| t.autorite == "oui") {
        let Some(email) = t.email.as_deref().filter(|e| !e.trim().is_empty()) else {
            continue;
        };
        if protege_tx(&mut *conn, etab, t).await? {
            continue;
        }
        sqlx::query(
            "INSERT INTO ent_inscription_envoi
                (id, etablissement_id, dossier_id, cle, kind, destinataire, sujet, corps)
             VALUES ($1,$2,$3,$4,'information',$5,$6,$7) ON CONFLICT (etablissement_id, cle) DO NOTHING",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(d.id)
        .bind(format!("desaccord:{}:{}:{}", d.id, t.id, Utc::now().timestamp()))
        .bind(email)
        .bind("Votre dossier d'inscription est en attente")
        .bind(format!(
            "Bonjour,\n\nLe dossier d'inscription de {} est suspendu : {nom_etab} ne peut pas trancher entre des \
             avis différents des titulaires de l'autorité parentale. Seule une décision commune ou une décision du \
             juge aux affaires familiales permet de poursuivre. Vous pouvez contacter l'établissement.",
            d.eleve_prenom
        ))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

/// Pose le blocage pour désaccord (idempotent) : état, suivi, événement. Les demandes en cours sont closes.
async fn bloquer_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    texte: &str,
    actor: Option<Uuid>,
) -> Result<(), EntDbError> {
    if d.etat == "bloque_desaccord" {
        return Ok(());
    }
    transitionner_tx(&mut *conn, etab, d, "bloque_desaccord", Cause::Blocage, Some(texte), actor).await?;
    publier(
        &mut *conn,
        etab,
        types::INSCRIPTION_BLOQUE_DESACCORD,
        AggregateKind::Admission,
        d.id,
        d.eleve_id,
        json!({ "dossier_id": d.id, "etat": "bloque_desaccord" }),
        actor,
    )
    .await?;
    Ok(())
}

// ───────────────────────────── Initialisation ─────────────────────────────

/// À appeler quand un dossier d'admission est déposé : crée les accords attendus ; le **déposant** donne le sien
/// par le dépôt lui-même (`depot_conjoint`) ; sous la règle `tous_les_titulaires`, une demande part à chacun des
/// autres titulaires qui a une adresse (les autres : accord papier, saisi par le secrétariat).
pub(crate) async fn initialiser_accords_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
) -> Result<usize, EntDbError> {
    let d = dossier_tx(&mut *conn, etab, dossier_id).await?;
    let Some(objet) = objet_de_la_nature(&d.nature) else {
        return Ok(0);
    };
    let regle = regle_campagne_tx(&mut *conn, etab, d.campagne_id).await?;
    let toutes = declarations_tx(&mut *conn, etab, &d).await?;
    assurer_accords_tx(&mut *conn, etab, &d, objet, &toutes).await?;
    let accords = accords_tx(&mut *conn, etab, d.id, objet).await?;
    let mut demandes = 0;
    for t in toutes.iter().filter(|t| t.autorite == "oui") {
        let Some(a) = accords.iter().find(|a| a.1 == t.id) else {
            continue;
        };
        if a.3 != "attendu" {
            continue;
        }
        if t.est_deposant {
            ecrire_reponse_tx(&mut *conn, etab, a.0, &Reponse::simple("donne", "depot_conjoint", (None, "responsable"))).await?;
        } else if regle == "tous_les_titulaires"
            && creer_demande_tx(&mut *conn, etab, &d, a.0, t, None).await?.is_some()
        {
            demandes += 1;
        }
    }
    Ok(demandes)
}

// ───────────────────────────── Lecture ─────────────────────────────

pub(crate) async fn accords_dossier_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
) -> Result<AccordsDossierDto, EntDbError> {
    let objet = objet_de_la_nature(&d.nature).unwrap_or("admission");
    let regle = regle_campagne_tx(&mut *conn, etab, d.campagne_id).await?;
    let toutes = declarations_tx(&mut *conn, etab, d).await?;
    let requis = titulaires_requis_tx(&mut *conn, etab, d, &toutes).await?;
    let accords = accords_tx(&mut *conn, etab, d.id, objet).await?;
    let en_cours: HashSet<Uuid> = sqlx::query_scalar(
        "SELECT accord_id FROM ent_inscription_accord_demande
          WHERE etablissement_id = $1 AND etat = 'envoyee' AND expire_le > now()",
    )
    .bind(etab)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?
    .into_iter()
    .collect();
    let dtos = accords
        .iter()
        .filter_map(|a| {
            let t = toutes.iter().find(|t| t.id == a.1)?;
            Some(AccordDto {
                id: a.0,
                declaration_id: a.1,
                rang: t.ordre,
                nom: t.nom.clone(),
                objet: a.2.clone(),
                reponse: a.3.clone(),
                moyen: a.4.clone(),
                repondu_le: a.5,
                demande_en_cours: en_cours.contains(&a.0),
                a_piece: a.6.is_some(),
            })
        })
        .collect();
    Ok(AccordsDossierDto {
        objet: objet.into(),
        regle: regle.clone(),
        complet: complet(&regle, &requis, &accords),
        accords: dtos,
        bloque_desaccord: d.etat == "bloque_desaccord",
    })
}

/// Trouve l'accord d'un responsable par son rang dans la démarche du dossier.
async fn accord_du_rang_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    d: &DossierRow,
    rang: i16,
) -> Result<(Uuid, Titulaire), EntDbError> {
    let objet = objet_de_la_nature(&d.nature).ok_or_else(|| conflit("ce dossier ne demande pas d'accord"))?;
    let toutes = declarations_tx(&mut *conn, etab, d).await?;
    let t = toutes.iter().find(|t| t.ordre == rang).cloned().ok_or_else(introuvable)?;
    assurer_accords_tx(&mut *conn, etab, d, objet, &toutes).await?;
    let accords = accords_tx(&mut *conn, etab, d.id, objet).await?;
    let a = accords.iter().find(|a| a.1 == t.id).ok_or_else(|| {
        conflit("ce responsable n'a pas l'autorité parentale : il ne donne pas d'accord")
    })?;
    Ok((a.0, t))
}

impl PgEnt {
    /// Accords d'un dossier : personnel qui instruit.
    pub async fn accords_dossier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
    ) -> Result<AccordsDossierDto, EntDbError> {
        self.refuser_si_deporte(etab, actor, dossier_id).await?;
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        let r = accords_dossier_tx(&mut tx, etab, &d).await?;
        tx.rollback().await.map_err(db)?;
        Ok(r)
    }

    /// (Re)demande leur accord aux titulaires qui n'ont pas répondu (ou aux rangs indiqués). Une relance
    /// remplace la demande en cours ; un responsable à ne pas solliciter n'en reçoit jamais. Rend le nombre de
    /// demandes envoyées.
    pub async fn demander_accords_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
        b: &DemanderAccordsBody,
    ) -> Result<usize, EntDbError> {
        self.refuser_si_deporte(etab, actor, dossier_id).await?;
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        let eff = etat_effectif(&d.etat, d.etat_avant_blocage.as_deref());
        if crate::inscriptions_etats::est_terminal(&d.nature, eff) || eff == "brouillon" {
            return Err(conflit("dossier clos ou non déposé : aucune demande d'accord"));
        }
        let objet = objet_de_la_nature(&d.nature).ok_or_else(|| conflit("ce dossier ne demande pas d'accord"))?;
        let toutes = declarations_tx(&mut tx, etab, &d).await?;
        let requis = titulaires_requis_tx(&mut tx, etab, &d, &toutes).await?;
        assurer_accords_tx(&mut tx, etab, &d, objet, &toutes).await?;
        let accords = accords_tx(&mut tx, etab, d.id, objet).await?;
        let mut n = 0;
        for t in &requis {
            if !b.rangs.is_empty() && !b.rangs.contains(&t.ordre) {
                continue;
            }
            let Some(a) = accords.iter().find(|a| a.1 == t.id) else {
                continue;
            };
            if matches!(a.3.as_str(), "donne" | "dispense") {
                continue;
            }
            if creer_demande_tx(&mut tx, etab, &d, a.0, t, Some(actor)).await?.is_some() {
                n += 1;
            }
        }
        if n > 0 {
            suivi_tx(
                &mut tx,
                etab,
                dossier_id,
                "relance",
                None,
                Some("Demande d'accord envoyée aux responsables"),
                false,
                Some(actor),
            )
            .await?;
        }
        commit(tx).await?;
        Ok(n)
    }

    /// Accord (ou refus) reçu **sur papier** : le secrétariat saisit la réponse avec son nom ; un accord s'appuie
    /// sur la pièce `accord_responsable` du dossier, **acceptée**. Un refus bloque le dossier.
    pub async fn saisir_accord_papier_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
        b: &AccordPapierBody,
    ) -> Result<AccordsDossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        if !matches!(b.reponse.as_str(), "donne" | "refuse") {
            return Err(invalide("réponse : donne ou refuse"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        refuser_si_clos(&d)?;
        let (accord_id, _) = accord_du_rang_tx(&mut tx, etab, &d, b.rang).await?;
        let piece = match (&b.piece_code, b.reponse.as_str()) {
            (Some(code), _) => Some(piece_acceptee_tx(&mut tx, etab, dossier_id, code, "accord_responsable").await?),
            (None, "donne") => return Err(invalide("un accord papier s'appuie sur la pièce scannée (accord_responsable)")),
            (None, _) => None,
        };
        ecrire_reponse_tx(
            &mut tx,
            etab,
            accord_id,
            &Reponse { piece_id: piece, ..Reponse::simple(&b.reponse, "papier", (Some(actor), "secretariat")) },
        )
        .await?;
        self.apres_reponse_tx(&mut tx, etab, &d, &b.reponse, "papier", Some(actor)).await?;
        let d2 = dossier_tx(&mut tx, etab, dossier_id).await?;
        let sortie = accords_dossier_tx(&mut tx, etab, &d2).await?;
        commit(tx).await?;
        Ok(sortie)
    }

    /// Un titulaire fait connaître son **désaccord** (courrier, appel, visite à l'accueil), qu'il soit ou non le
    /// déposant : le dossier est bloqué, les deux parents reçoivent le même message neutre.
    pub async fn declarer_desaccord_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
        b: &DesaccordBody,
    ) -> Result<AccordsDossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsGerer).await?;
        let motif = b.motif.trim();
        if !(3..=1000).contains(&motif.chars().count()) {
            return Err(invalide("motif obligatoire (3 à 1000 caractères)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        refuser_si_clos(&d)?;
        let (accord_id, _) = accord_du_rang_tx(&mut tx, etab, &d, b.rang).await?;
        ecrire_reponse_tx(
            &mut tx,
            etab,
            accord_id,
            &Reponse { motif: Some(motif), ..Reponse::simple("refuse", "papier", (Some(actor), "secretariat")) },
        )
        .await?;
        self.apres_reponse_tx(&mut tx, etab, &d, "refuse", "papier", Some(actor)).await?;
        let d2 = dossier_tx(&mut tx, etab, dossier_id).await?;
        let sortie = accords_dossier_tx(&mut tx, etab, &d2).await?;
        commit(tx).await?;
        Ok(sortie)
    }

    /// **Dispense** l'accord d'un responsable (danger, ordonnance de protection, décision de justice) :
    /// décision motivée de la direction ; la pièce, quand elle existe, est jointe.
    pub async fn dispenser_accord_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
        b: &DispenseAccordBody,
    ) -> Result<AccordsDossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider).await?;
        let motif = b.motif.trim();
        if !(3..=1000).contains(&motif.chars().count()) {
            return Err(invalide("motif obligatoire (3 à 1000 caractères)"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        refuser_si_clos(&d)?;
        let (accord_id, _) = accord_du_rang_tx(&mut tx, etab, &d, b.rang).await?;
        let piece = match b.piece_code.as_deref() {
            Some(c) => Some(piece_acceptee_tx(&mut tx, etab, dossier_id, c, "").await?),
            None => None,
        };
        ecrire_reponse_tx(
            &mut tx,
            etab,
            accord_id,
            &Reponse {
                piece_id: piece,
                motif: Some(motif),
                ..Reponse::simple("dispense", "decision_direction", (Some(actor), "direction"))
            },
        )
        .await?;
        suivi_tx(&mut tx, etab, dossier_id, "commentaire", None, Some("Accord d'un responsable dispensé (décision de la direction)"), false, Some(actor)).await?;
        publier(
            &mut tx,
            etab,
            types::INSCRIPTION_ACCORD_REPONDU,
            AggregateKind::Admission,
            dossier_id,
            d.eleve_id,
            json!({ "dossier_id": dossier_id, "objet": objet_de_la_nature(&d.nature), "reponse": "dispense" }),
            Some(actor),
        )
        .await?;
        let sortie = accords_dossier_tx(&mut tx, etab, &d).await?;
        commit(tx).await?;
        Ok(sortie)
    }

    /// **Lève** le désaccord d'un responsable, avec pièce : son accord écrit (`accord_ecrit`, pièce
    /// `accord_responsable`) ou une décision de justice (`decision_justice`, qui dispense son accord). La pièce
    /// doit être **acceptée** au dossier. Le dossier ne se débloque que quand plus aucun refus n'est en cours.
    pub async fn lever_desaccord_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        dossier_id: Uuid,
        b: &LeverDesaccordBody,
    ) -> Result<AccordsDossierDto, EntDbError> {
        self.ins_exiger(etab, actor, role, Action::InscriptionsDecider).await?;
        let motif = b.motif.trim();
        if !(3..=1000).contains(&motif.chars().count()) {
            return Err(invalide("motif obligatoire (3 à 1000 caractères)"));
        }
        let (type_attendu, vers) = match b.issue.as_str() {
            "accord_ecrit" => ("accord_responsable", "donne"),
            "decision_justice" => ("decision_justice", "dispense"),
            _ => return Err(invalide("issue : accord_ecrit ou decision_justice")),
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        if d.etat != "bloque_desaccord" {
            return Err(conflit("le dossier n'est pas bloqué"));
        }
        let (accord_id, _) = accord_du_rang_tx(&mut tx, etab, &d, b.rang).await?;
        let reponse: String = sqlx::query_scalar(
            "SELECT reponse FROM ent_inscription_accord WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(accord_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if reponse != "refuse" {
            return Err(conflit("ce responsable n'a pas déclaré de désaccord"));
        }
        let piece = piece_acceptee_tx(&mut tx, etab, dossier_id, &b.piece_code, type_attendu).await?;
        // La pièce ne lève qu'un désaccord : jamais réutilisée pour un autre refus du même dossier.
        let (deja,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_inscription_accord WHERE etablissement_id = $1 AND dossier_id = $2 AND piece_id = $3 AND id <> $4",
        )
        .bind(etab)
        .bind(dossier_id)
        .bind(piece)
        .bind(accord_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja > 0 {
            return Err(conflit("cette pièce a déjà levé un autre désaccord"));
        }
        let moyen = if vers == "donne" { "papier" } else { "decision_direction" };
        ecrire_reponse_tx(
            &mut tx,
            etab,
            accord_id,
            &Reponse { piece_id: Some(piece), motif: Some(motif), ..Reponse::simple(vers, moyen, (Some(actor), "direction")) },
        )
        .await?;
        let (refus_restants,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_inscription_accord WHERE etablissement_id = $1 AND dossier_id = $2 AND reponse = 'refuse'",
        )
        .bind(etab)
        .bind(dossier_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if refus_restants == 0 {
            let vers_etat = d
                .etat_avant_blocage
                .clone()
                .ok_or_else(|| conflit("état d'avant le blocage inconnu"))?;
            transitionner_tx(&mut tx, etab, &d, &vers_etat, Cause::Blocage, Some(motif), Some(actor)).await?;
            publier(
                &mut tx,
                etab,
                types::INSCRIPTION_BLOCAGE_LEVE,
                AggregateKind::Admission,
                dossier_id,
                d.eleve_id,
                json!({ "dossier_id": dossier_id, "etat": vers_etat }),
                Some(actor),
            )
            .await?;
        }
        let d2 = dossier_tx(&mut tx, etab, dossier_id).await?;
        let sortie = accords_dossier_tx(&mut tx, etab, &d2).await?;
        commit(tx).await?;
        Ok(sortie)
    }

    /// Suites d'une réponse enregistrée : suivi, événement, et blocage + message neutre en cas de refus.
    async fn apres_reponse_tx(
        &self,
        tx: &mut TenantTx<'_>,
        etab: Uuid,
        d: &DossierRow,
        reponse: &str,
        moyen: &str,
        actor: Option<Uuid>,
    ) -> Result<(), EntDbError> {
        let objet = objet_de_la_nature(&d.nature);
        publier(
            tx,
            etab,
            types::INSCRIPTION_ACCORD_REPONDU,
            AggregateKind::Admission,
            d.id,
            d.eleve_id,
            json!({ "dossier_id": d.id, "objet": objet, "reponse": reponse, "moyen": moyen }),
            actor,
        )
        .await?;
        if reponse == "refuse" {
            // Les demandes encore ouvertes des autres responsables n'ont plus lieu d'être : le dossier est suspendu.
            sqlx::query(
                "UPDATE ent_inscription_accord_demande SET etat = 'annulee', close_le = now()
                  WHERE etablissement_id = $1 AND etat = 'envoyee'
                    AND accord_id IN (SELECT id FROM ent_inscription_accord WHERE etablissement_id = $1 AND dossier_id = $2)",
            )
            .bind(etab)
            .bind(d.id)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
            bloquer_tx(
                tx,
                etab,
                d,
                "Désaccord des responsables : seule une décision commune ou du juge aux affaires familiales permet de poursuivre",
                actor,
            )
            .await?;
            let toutes = declarations_tx(tx, etab, d).await?;
            informer_desaccord_tx(tx, etab, d, &toutes).await?;
        } else {
            suivi_tx(tx, etab, d.id, "commentaire", None, Some("Accord d'un responsable enregistré"), false, actor)
                .await?;
        }
        Ok(())
    }

    // ───────────── Écran public sans compte ─────────────

    /// Établissement d'une demande d'accord, avant de connaître le tenant (jeton du lien reçu par e-mail).
    pub async fn etablissement_de_accord(&self, demande_id: Uuid) -> Result<Option<Uuid>, EntDbError> {
        let r: Option<(Uuid,)> =
            sqlx::query_as("SELECT etablissement_id FROM ent_inscription_accord_demande WHERE id = $1")
                .bind(demande_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?;
        Ok(r.map(|r| r.0))
    }

    /// Ce qu'un responsable voit en ouvrant son lien : l'établissement, le prénom de l'enfant, l'objet, l'état.
    /// Jeton faux ou demande inconnue : la même réponse « lien invalide ou expiré » (aucun indice).
    pub async fn accord_public(
        &self,
        etab: Uuid,
        demande_id: Uuid,
        jeton: &str,
    ) -> Result<AccordPublicDto, EntDbError> {
        verifier_jeton(demande_id, jeton)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (etat, expire, dossier_id, reponse): (String, DateTime<Utc>, Uuid, String) = sqlx::query_as(
            "SELECT dm.etat, dm.expire_le, a.dossier_id, a.reponse
               FROM ent_inscription_accord_demande dm
               JOIN ent_inscription_accord a ON a.etablissement_id = dm.etablissement_id AND a.id = dm.accord_id
              WHERE dm.etablissement_id = $1 AND dm.id = $2",
        )
        .bind(etab)
        .bind(demande_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(lien_invalide)?;
                let prenom: String = sqlx::query_scalar(
            "SELECT eleve_prenom FROM ent_inscription_dossier WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(dossier_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let nom = nom_etablissement_tx(&mut tx, etab).await?;
        tx.rollback().await.map_err(db)?;
        let etat = if etat == "envoyee" && expire <= Utc::now() { "expiree".to_string() } else { etat };
        Ok(AccordPublicDto {
            etablissement: nom,
            enfant_prenom: prenom,
            objet: "admission".into(),
            reponse: (etat == "repondue").then_some(reponse),
            etat,
            expire_le: expire,
        })
    }

    /// Envoie le code de vérification **à l'adresse de la déclaration** (jamais à une adresse fournie par
    /// l'appelant). Cinq envois au plus par demande ; la réponse est toujours la même.
    pub async fn envoyer_code_accord(&self, etab: Uuid, demande_id: Uuid, jeton: &str) -> Result<(), EntDbError> {
        verifier_jeton(demande_id, jeton)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ligne: Option<(String, DateTime<Utc>, i16, Uuid, Option<String>, Uuid)> = sqlx::query_as(
            "SELECT dm.etat, dm.expire_le, dm.code_compteur, a.dossier_id, x.email, a.id
               FROM ent_inscription_accord_demande dm
               JOIN ent_inscription_accord a ON a.etablissement_id = dm.etablissement_id AND a.id = dm.accord_id
               JOIN ent_inscription_declaration_responsable x ON x.id = a.declaration_id
              WHERE dm.etablissement_id = $1 AND dm.id = $2 FOR UPDATE OF dm",
        )
        .bind(etab)
        .bind(demande_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((etat, expire, compteur, dossier_id, email, _)) = ligne else {
            return Err(lien_invalide());
        };
        if etat != "envoyee" || expire <= Utc::now() {
            return Err(lien_invalide());
        }
        if compteur >= CODES_PAR_DEMANDE_MAX {
            return Err(conflit("trop de codes demandés : contactez l'établissement"));
        }
        let Some(email) = email.filter(|e| !e.trim().is_empty()) else {
            return Err(lien_invalide());
        };
        let nouveau = compteur + 1;
        sqlx::query(
            "UPDATE ent_inscription_accord_demande
                SET code_compteur = $3, code_essais = 0, code_expire_le = now() + make_interval(mins => $4)
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande_id)
        .bind(nouveau)
        .bind(CODE_VALIDITE_MIN)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_inscription_envoi
                (id, etablissement_id, dossier_id, cle, kind, destinataire, sujet, corps, accord_demande_id)
             VALUES ($1,$2,$3,$4,'information',$5,$6,$7,$8)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(dossier_id)
        .bind(format!("accord-code:{demande_id}:{nouveau}"))
        .bind(email)
        .bind("Votre code de vérification")
        .bind(format!(
            "Votre code de vérification : {{CODE}}\n\nIl est valable {CODE_VALIDITE_MIN} minutes. Ne le communiquez à personne."
        ))
        .bind(demande_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        commit(tx).await?;
        Ok(())
    }

    /// Réponse d'un responsable sur son lien (jeton + code), **sans compte**. Seul un titulaire de l'autorité
    /// parentale répond : l'autorité est relue au moment de la réponse. Un refus bloque le dossier.
    pub async fn repondre_accord(
        &self,
        etab: Uuid,
        demande_id: Uuid,
        jeton: &str,
        code: &str,
        reponse: &str,
    ) -> Result<(), EntDbError> {
        verifier_jeton(demande_id, jeton)?;
        if !matches!(reponse, "donne" | "refuse") {
            return Err(invalide("réponse : donne ou refuse"));
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ligne: Option<(String, DateTime<Utc>, i16, Option<DateTime<Utc>>, i16, Uuid, Uuid, Uuid)> = sqlx::query_as(
            "SELECT dm.etat, dm.expire_le, dm.code_compteur, dm.code_expire_le, dm.code_essais,
                    dm.accord_id, a.dossier_id, a.declaration_id
               FROM ent_inscription_accord_demande dm
               JOIN ent_inscription_accord a ON a.etablissement_id = dm.etablissement_id AND a.id = dm.accord_id
              WHERE dm.etablissement_id = $1 AND dm.id = $2 FOR UPDATE OF dm",
        )
        .bind(etab)
        .bind(demande_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((etat, expire, compteur, code_expire, essais, accord_id, dossier_id, decl_id)) = ligne else {
            return Err(lien_invalide());
        };
        if etat != "envoyee" || expire <= Utc::now() {
            return Err(lien_invalide());
        }
        // Code : présent, non expiré, essais restants ; chaque essai raté est compté et conservé (la base
        // refuse un sixième).
        let code_ok = compteur > 0
            && code_expire.is_some_and(|e| e > Utc::now())
            && essais < CODE_ESSAIS_MAX
            && code_accord(demande_id, compteur).is_ok_and(|attendu| egaux(&attendu, code.trim()));
        if !code_ok {
            if compteur > 0 && essais < CODE_ESSAIS_MAX {
                sqlx::query(
                    "UPDATE ent_inscription_accord_demande SET code_essais = code_essais + 1
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(etab)
                .bind(demande_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                commit(tx).await?;
            }
            return Err(invalide("code incorrect ou expiré"));
        }
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        if crate::inscriptions_etats::est_terminal(&d.nature, etat_effectif(&d.etat, d.etat_avant_blocage.as_deref())) {
            return Err(lien_invalide());
        }
        // L'autorité parentale est relue maintenant : un parent sans autorité ne décide pas.
        let autorite: String = sqlx::query_scalar(
            "SELECT autorite_parentale FROM ent_inscription_declaration_responsable WHERE id = $1 AND etablissement_id = $2",
        )
        .bind(decl_id)
        .bind(etab)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if autorite != "oui" {
            return Err(conflit(
                "seul un titulaire de l'autorité parentale répond à cette demande : contactez l'établissement",
            ));
        }
        ecrire_reponse_tx(&mut tx, etab, accord_id, &Reponse::simple(reponse, "en_ligne_code", (None, "responsable"))).await?;
        self.apres_reponse_tx(&mut tx, etab, &d, reponse, "en_ligne_code", None).await?;
        commit(tx).await?;
        Ok(())
    }

    /// Réponse d'un responsable **depuis son compte** (session vérifiée) : l'adresse vérifiée doit être celle de
    /// la déclaration, et, quand l'élève existe (réinscription), le lien d'autorité parentale doit être actif.
    pub async fn repondre_accord_compte(
        &self,
        etab: Uuid,
        jeton_session: &str,
        dossier_id: Uuid,
        reponse: &str,
    ) -> Result<(), EntDbError> {
        if !matches!(reponse, "donne" | "refuse") {
            return Err(invalide("réponse : donne ou refuse"));
        }
        let v = self.resoudre_jeton_preinscription(etab, jeton_session).await?;
        if !v.par_session {
            return Err(EntDbError::Forbidden);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let d = dossier_tx(&mut tx, etab, dossier_id).await?;
        refuser_si_clos(&d)?;
        let toutes = declarations_tx(&mut tx, etab, &d).await?;
        let Some(t) = toutes
            .iter()
            .find(|t| t.email.as_deref().is_some_and(|e| e.trim().eq_ignore_ascii_case(v.email.trim())))
            .cloned()
        else {
            // Aucun indice sur l'existence du dossier.
            return Err(introuvable());
        };
        if t.autorite != "oui" {
            return Err(conflit(
                "seul un titulaire de l'autorité parentale répond à cette demande : contactez l'établissement",
            ));
        }
        if let (Some(eleve), Some(user)) = (d.eleve_id, v.user_id) {
            // Élève connu (réinscription) : le lien d'autorité du socle 4 fait foi.
            let (ok,): (bool,) = sqlx::query_as(
                "SELECT EXISTS (SELECT 1 FROM ent_eleve_lien_actif l
                                 JOIN ent_personne p ON p.etablissement_id = l.etablissement_id AND p.id = l.personne_id
                                WHERE l.etablissement_id = $1 AND l.eleve_id = $2 AND p.user_id = $3 AND l.autorite_parentale)",
            )
            .bind(etab)
            .bind(eleve)
            .bind(user)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !ok {
                return Err(EntDbError::Forbidden);
            }
        }
        let (accord_id, _) = accord_du_rang_tx(&mut tx, etab, &d, t.ordre).await?;
        ecrire_reponse_tx(&mut tx, etab, accord_id, &Reponse::simple(reponse, "en_ligne_compte", (v.user_id, "responsable"))).await?;
        self.apres_reponse_tx(&mut tx, etab, &d, reponse, "en_ligne_compte", v.user_id).await?;
        commit(tx).await?;
        Ok(())
    }
}

fn lien_invalide() -> EntDbError {
    invalide("lien invalide ou expiré")
}

fn verifier_jeton(demande_id: Uuid, jeton: &str) -> Result<(), EntDbError> {
    if jeton.trim().is_empty() || jeton.len() > 256 {
        return Err(lien_invalide());
    }
    match jeton_accord(demande_id) {
        Ok(attendu) if egaux(&attendu, jeton.trim()) => Ok(()),
        _ => Err(lien_invalide()),
    }
}

fn refuser_si_clos(d: &DossierRow) -> Result<(), EntDbError> {
    let eff = etat_effectif(&d.etat, d.etat_avant_blocage.as_deref());
    if crate::inscriptions_etats::est_terminal(&d.nature, eff) || eff == "brouillon" {
        return Err(conflit("dossier clos ou non déposé"));
    }
    Ok(())
}

/// La pièce `code` du dossier est **acceptée** (et du type attendu quand `attendu` n'est pas vide) : rend son
/// identifiant. Sans pièce acceptée, rien ne prouve la réponse.
async fn piece_acceptee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    dossier_id: Uuid,
    code: &str,
    attendu: &str,
) -> Result<Uuid, EntDbError> {
    if !attendu.is_empty() && code != attendu {
        return Err(invalide("cette pièce ne convient pas : le type attendu est différent"));
    }
    let r: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, statut FROM ent_inscription_piece
          WHERE etablissement_id = $1 AND dossier_id = $2 AND code = $3 AND applicable",
    )
    .bind(etab)
    .bind(dossier_id)
    .bind(code)
    .fetch_optional(conn)
    .await
    .map_err(db)?;
    match r {
        None => Err(conflit("pièce inconnue pour ce dossier : ajoutez-la aux pièces attendues puis déposez-la")),
        Some((id, statut)) if statut == "acceptee" => Ok(id),
        Some(_) => Err(conflit("la pièce doit d'abord être déposée puis acceptée")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titulaire(n: u8, autorite: &str, exclusive: bool) -> Titulaire {
        Titulaire {
            id: Uuid::from_u128(u128::from(n)),
            ordre: i16::from(n),
            nom: format!("T{n}"),
            email: None,
            autorite: autorite.into(),
            exclusive,
            ne_pas_solliciter: false,
            est_deposant: n == 1,
        }
    }

    fn accord(n: u8, reponse: &str) -> LigneAccord {
        (Uuid::from_u128(100 + u128::from(n)), Uuid::from_u128(u128::from(n)), "admission".into(), reponse.into(), None, None, None)
    }

    #[test]
    fn regle_tous_les_titulaires() {
        let requis = [titulaire(1, "oui", false), titulaire(2, "oui", false)];
        assert!(!complet("tous_les_titulaires", &requis, &[accord(1, "donne"), accord(2, "attendu")]));
        assert!(complet("tous_les_titulaires", &requis, &[accord(1, "donne"), accord(2, "donne")]));
        // Une dispense de la direction vaut accord ; un refus non levé bloque toujours.
        assert!(complet("tous_les_titulaires", &requis, &[accord(1, "donne"), accord(2, "dispense")]));
        assert!(!complet("tous_les_titulaires", &requis, &[accord(1, "donne"), accord(2, "refuse")]));
    }

    #[test]
    fn regle_un_seul() {
        let requis = [titulaire(1, "oui", false), titulaire(2, "oui", false)];
        assert!(complet("un_seul", &requis, &[accord(1, "donne"), accord(2, "attendu")]));
        assert!(!complet("un_seul", &requis, &[accord(1, "attendu"), accord(2, "attendu")]));
        // Même sous `un_seul`, un désaccord déclaré par l'autre titulaire bloque (art. 372-2).
        assert!(!complet("un_seul", &requis, &[accord(1, "donne"), accord(2, "refuse")]));
        // Une dispense seule ne vaut pas accord sous `un_seul`.
        assert!(!complet("un_seul", &requis, &[accord(1, "dispense"), accord(2, "attendu")]));
    }

    #[test]
    fn aucun_titulaire_aucun_accord() {
        assert!(!complet("un_seul", &[], &[]));
        assert!(!complet("tous_les_titulaires", &[], &[]));
    }

    #[test]
    fn objets_par_nature() {
        assert_eq!(objet_de_la_nature("admission"), Some("admission"));
        assert_eq!(objet_de_la_nature("en_cours_annee"), Some("admission"));
        assert_eq!(objet_de_la_nature("reinscription"), Some("reinscription"));
        assert_eq!(objet_de_la_nature("affectes"), None);
    }

    #[test]
    fn comparaison_des_jetons() {
        assert!(egaux("abc", "abc"));
        assert!(!egaux("abc", "abd"));
        assert!(!egaux("", "abc"));
    }
}
