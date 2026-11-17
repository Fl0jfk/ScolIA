//! Socle 7, lot D6 : **service de la brique de signature**, partie 2 — les quatre modes (passkey, code SMS à usage
//! unique, guichet, papier), le signataire externe (lien + code), la finalisation (PDF signé, dossier de preuve,
//! cachet), l'horodatage RFC 3161, le téléchargement des pièces, la vérification interne des signatures.
//!
//! * **passkey** : défi WebAuthn tiré par le serveur (`webauthn-rs` 0.5 n'accepte pas de défi imposé par l'appelant,
//!   D-062 interdit d'ajouter `webauthn-rs-core`) ; le **lien** défi ↔ empreinte ↔ intention ↔ signataire ↔ nonce est
//!   écrit au serveur puis **scellé dans la chaîne** ; vérification utilisateur exigée ; défi à usage unique (brûlé
//!   avant la vérification : un rejeu ne passe jamais) ;
//! * **code SMS** : six chiffres, haché en base, 10 minutes, 5 essais, un envoi au plus trois fois par heure ;
//! * **guichet** : agent identifié (`documents.signatures.constater`), identité vue (type, jamais copiée), confirmation
//!   par code envoyé au téléphone du dossier (niveau G) ; sans confirmation la signature reste simple (G0) ;
//! * **papier** : dépôt du scan (PDF désarmé ou image), puis **contrôle humain** (`documents.signatures.gerer`) ;
//! * un signataire ne signe qu'**une fois**, sur **la version du circuit**, tant que le circuit est ouvert.

use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Duration, Utc};
use ent_auth::{Action, PublicKeyCredential};
use ent_events::MetierEvent;
use serde_json::json;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::champs_chiffres::ChampsSensibles;
use crate::documents::*;
use crate::documents_repo::{journaliser, publier, texte, Moteur};
use crate::documents_signature::*;
use crate::documents_signature_repo::*;
use crate::passkey_repo::{CounterOutcome, PgPasskeyRepo};
use crate::ref_commun::{db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;

/// Durée de vie d'un défi passkey et d'un code (secondes / minutes).
const DEFI_PASSKEY_MIN: i64 = 5;
const CODE_MIN: i64 = 10;
const CODE_ESSAIS_MAX: i16 = 5;
const CODES_PAR_HEURE: i64 = 3;
/// Plafond **cumulé** d'échecs de code par signataire (tous codes confondus), puis blocage et alerte (M2).
const CODE_ECHECS_CUMULES_MAX: i64 = 10;
const PAPIER_MAX_OCTETS: usize = 15 * 1024 * 1024;
const TRACE_MAX_OCTETS: usize = 300 * 1024;

pub const EVT_SIGNE_SANS_CONFIRMATION: &str = "documents.signe_sans_confirmation";
pub const EVT_HORODATAGE_ECHEC: &str = "documents.horodatage_echec";
pub const EVT_SIGNATURE_BLOQUEE: &str = "documents.signature_bloquee";

pub type FutureEnvoi<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Port d'envoi du code à usage unique (OVH SMS, D-004, branché par le socle 8 : tant qu'aucun adaptateur n'est fourni,
/// le mode « code SMS » est seulement prévu).
pub trait EnvoiCodeSignature: Send + Sync {
    fn envoyer<'a>(&'a self, telephone: &'a str, code: &'a str) -> FutureEnvoi<'a>;
}

/// Ce que la signature emprunte aux autres briques : cachet (via le moteur) et service d'horodatage.
#[derive(Clone, Copy)]
pub struct ContexteSignature<'a> {
    pub moteur: Moteur<'a>,
    pub horodatage: &'a dyn FournisseurHorodatage,
}

#[derive(Debug, Clone)]
pub struct DefiPasskey {
    pub defi_id: Uuid,
    /// `RequestChallengeResponse` WebAuthn, à passer à `navigator.credentials.get`.
    pub options: serde_json::Value,
    pub intention: String,
    pub empreinte: String,
}

#[derive(Debug, Clone)]
pub struct DefiCode {
    pub defi_id: Uuid,
    pub canal_masque: String,
    pub intention: String,
    pub empreinte: String,
}

#[derive(Debug, Clone)]
pub struct SignatureFaite {
    pub signature_id: Uuid,
    pub niveau: Niveau,
    pub circuit_complet: bool,
    /// Avertissement du mode (mode faible, tablette sans confirmation).
    pub avertissement: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConstatGuichet {
    pub signataire_id: Uuid,
    /// `carte_identite`, `passeport`, `titre_sejour`, `permis`, `carte_professionnelle`, `autre`.
    pub identite_vue: String,
    /// Tracé de la signature sur la tablette (PNG, 300 Ko au plus), gardé comme objet scellé.
    pub trace: Option<Vec<u8>>,
    /// Confirmation par le code reçu sur le téléphone du dossier : (défi, code saisi).
    pub confirmation: Option<(Uuid, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionPapier {
    Valider,
    Refuser(String),
}

struct Prepare {
    circuit: LigneCircuit,
    role: String,
    libelle_role: String,
    nature: &'static NatureSignature,
    empreinte: String,
    avertissement: Option<&'static str>,
    eleve_id: Option<Uuid>,
    personne_id: Option<Uuid>,
}

#[derive(Default)]
struct Preuves {
    defi_id: Option<Uuid>,
    lien: Option<String>,
    credential_id: Option<String>,
    client_data: Option<String>,
    authenticator_data: Option<String>,
    assertion_signature: Option<String>,
    canal_masque: Option<String>,
    constate_par: Option<Uuid>,
    identite_vue: Option<String>,
    confirmation: Option<&'static str>,
    trace_sha: Option<String>,
    trace_cle: Option<String>,
    papier_cle: Option<String>,
    papier_sha: Option<String>,
    formule: Option<String>,
    compte: Option<Uuid>,
}

fn mode_refus(e: ErreurMode) -> EntDbError {
    match e {
        ErreurMode::Bloque(m) => refus(RefusMetier::Conflit, m),
        _ => refus(RefusMetier::Conflit, "ce mode de signature n'est pas proposé pour ce signataire"),
    }
}

/// Contrôles communs à toute signature : signataire invité, circuit ouvert et dans les délais, mode autorisé.
async fn preparer(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    signataire_id: Uuid,
    mode: ModeSignature,
    etat_attendu: &str,
) -> Result<Prepare, EntDbError> {
    let s = sqlx::query(
        "SELECT circuit_id, role, libelle_role, etat, personne_id, eleve_id FROM ent_doc_signataire
          WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
    )
    .bind(a.etablissement_id)
    .bind(signataire_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let etat: String = s.get("etat");
    let circuit_id: Uuid = s.get("circuit_id");
    let circuit = charger_circuit(conn, a.etablissement_id, circuit_id, true).await?;
    if circuit.etat != "ouvert" || etat != etat_attendu {
        return Err(refus(RefusMetier::Conflit, "ce document n'attend plus cette signature"));
    }
    let jour = jour_etab(conn, a.etablissement_id).await?;
    if circuit.echeance.is_some_and(|e| e < jour) {
        return Err(refus(RefusMetier::Conflit, "l'échéance de signature est dépassée"));
    }
    let nature = nature_signature(&circuit.nature_code)
        .ok_or_else(|| refus(RefusMetier::Invalide, "nature sans circuit de signature"))?;
    let (autorise,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_doc_signataire_mode
                         WHERE etablissement_id = $1 AND signataire_id = $2 AND mode = $3)",
    )
    .bind(a.etablissement_id)
    .bind(signataire_id)
    .bind(mode.code())
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if !autorise {
        return Err(refus(RefusMetier::Conflit, "ce mode de signature n'est pas proposé à ce signataire"));
    }
    // Le mode doit rester admissible aujourd'hui (réglage modifié depuis le lancement : un mode devenu bloqué l'est).
    let reglage = reglage_menu(conn, a.etablissement_id, nature.code).await?;
    let m = menu(nature, reglage.as_ref());
    let avertissement = match evaluer_mode(nature, &m, mode) {
        Ok(av) => av,
        Err(ErreurMode::HorsMenu) => menu(nature, None)
            .iter()
            .find(|c| c.mode == mode)
            .and_then(|c| c.signale),
        Err(e) => return Err(mode_refus(e)),
    };
    let (empreinte,): (String,) = sqlx::query_as(
        "SELECT empreinte_sha256 FROM ent_doc_version WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(a.etablissement_id)
    .bind(circuit.version_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    let (eleve_sujet,): (Option<Uuid>,) = sqlx::query_as(
        "SELECT eleve_id FROM ent_doc_document WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(a.etablissement_id)
    .bind(circuit.document_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    Ok(Prepare {
        circuit,
        role: s.get("role"),
        libelle_role: s.get("libelle_role"),
        nature,
        empreinte,
        avertissement,
        eleve_id: eleve_sujet,
        personne_id: s.get("personne_id"),
    })
}

/// Le signataire est-il bien la personne connectée, et son droit tient-il toujours aujourd'hui ?
async fn exiger_le_signataire(
    pg: &PgEnt,
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    signataire_id: Uuid,
    p: &Prepare,
) -> Result<Option<String>, EntDbError> {
    if compte_du_signataire(conn, a.etablissement_id, signataire_id).await? != Some(a.user_id) {
        return Err(interdit());
    }
    let jour = jour_etab(conn, a.etablissement_id).await?;
    match p.role.as_str() {
        "titulaire" => {
            let (pers, el) = (p.personne_id, p.eleve_id);
            match (pers, el) {
                (Some(pe), Some(el)) if titulaire_en_droit(conn, a.etablissement_id, pe, el, jour).await? => {}
                _ => return Err(refus(RefusMetier::Conflit, "votre lien avec l'élève ne vous permet plus de signer")),
            }
            Ok(None)
        }
        "etablissement" => {
            let pouvoir = pg.peut_signer_compte(a.etablissement_id, a.user_id, &p.circuit.nature_code).await?;
            if !pouvoir.autorise {
                return Err(interdit());
            }
            Ok(pouvoir.formule)
        }
        _ => Ok(None),
    }
}

/// Écrit la signature (chaînée), met à jour le signataire, invite le suivant, finalise si le circuit est complet.
async fn ecrire_signature(
    pg: &PgEnt,
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    ctx: &ContexteSignature<'_>,
    signataire_id: Uuid,
    p: &Prepare,
    mode: ModeSignature,
    niveau: Niveau,
    pr: Preuves,
) -> Result<(Uuid, bool, Vec<MetierEvent>), EntDbError> {
    let etab = a.etablissement_id;
    let circuit_id = p.circuit.id;
    let intention = intention_texte(p.nature.libelle, &p.libelle_role);
    let isha = sha256_hex(intention.as_bytes());
    let horodatage = tronquer_micros(Utc::now());
    let dernier: Option<(i32, String)> = sqlx::query_as(
        "SELECT rang_chaine, entree_sha256 FROM ent_doc_signature
          WHERE etablissement_id = $1 AND circuit_id = $2 ORDER BY rang_chaine DESC LIMIT 1",
    )
    .bind(etab)
    .bind(circuit_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let (rang, precedent) = match dernier {
        Some((r, h)) => (r + 1, h),
        None => (1, tete_initiale(circuit_id)),
    };
    let entree = EntreeChaine {
        circuit_id,
        signataire_id,
        version_id: p.circuit.version_id,
        rang,
        empreinte_signee: p.empreinte.clone(),
        methode: mode.code().to_string(),
        niveau: niveau.code().to_string(),
        intention_sha256: isha.clone(),
        horodatage,
        lien_sha256: pr.lien.clone(),
        precedent: precedent.clone(),
    };
    let sceau = sceller_entree(&entree);
    let signature_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO ent_doc_signature
            (id, etablissement_id, circuit_id, signataire_id, version_id, rang_chaine, empreinte_signee, methode, niveau,
             intention_texte, intention_sha256, horodatage_serveur, defi_id, lien_sha256, passkey_credential_id,
             client_data_json, authenticator_data_b64, assertion_signature_b64, code_canal_masque, constate_par_user_id,
             identite_vue, confirmation, trace_sha256, trace_cle_objet, papier_cle_objet, papier_sha256,
             formule_delegation, compte_user_id, precedent_sha256, entree_sha256)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30)",
    )
    .bind(signature_id)
    .bind(etab)
    .bind(circuit_id)
    .bind(signataire_id)
    .bind(p.circuit.version_id)
    .bind(rang)
    .bind(&p.empreinte)
    .bind(mode.code())
    .bind(niveau.code())
    .bind(&intention)
    .bind(&isha)
    .bind(horodatage)
    .bind(pr.defi_id)
    .bind(&pr.lien)
    .bind(&pr.credential_id)
    .bind(&pr.client_data)
    .bind(&pr.authenticator_data)
    .bind(&pr.assertion_signature)
    .bind(&pr.canal_masque)
    .bind(pr.constate_par)
    .bind(&pr.identite_vue)
    .bind(pr.confirmation)
    .bind(&pr.trace_sha)
    .bind(&pr.trace_cle)
    .bind(&pr.papier_cle)
    .bind(&pr.papier_sha)
    .bind(&pr.formule)
    .bind(pr.compte)
    .bind(&precedent)
    .bind(&sceau)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query("UPDATE ent_doc_signataire SET etat = 'signe' WHERE etablissement_id = $1 AND id = $2")
        .bind(etab)
        .bind(signataire_id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    // « Un parent suffit » : les autres titulaires ne bloquent plus.
    if p.role == "titulaire" && p.circuit.regle == "un" {
        sqlx::query(
            "UPDATE ent_doc_signataire SET requis = false
              WHERE etablissement_id = $1 AND circuit_id = $2 AND role = 'titulaire' AND id <> $3 AND requis",
        )
        .bind(etab)
        .bind(circuit_id)
        .bind(signataire_id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    journaliser(
        conn,
        a,
        Some(p.circuit.document_id),
        None,
        "signature_ajoutee",
        (Some("invite"), Some("signe")),
        Some(&signature_id.to_string()),
        None,
    )
    .await?;
    let mut evts = vec![
        publier(
            conn,
            a,
            EVT_SIGNE,
            p.circuit.document_id,
            p.eleve_id,
            json!({ "circuit_id": circuit_id, "signataire_id": signataire_id, "nature": p.circuit.nature_code,
                    "niveau": niveau.code(), "methode": mode.code() }),
        )
        .await?,
    ];
    // Le circuit est-il complet ?
    let complet = circuit_complet(conn, etab, circuit_id, &p.circuit.regle).await?;
    if complet {
        evts.push(pg.finaliser_circuit(conn, a, ctx, &p.circuit).await?);
    } else {
        let circuit = charger_circuit(conn, etab, circuit_id, false).await?;
        for sid in inviter_suivants(conn, a, &circuit).await? {
            evts.push(
                publier(
                    conn,
                    a,
                    EVT_SIGNATURE_DEMANDEE,
                    p.circuit.document_id,
                    p.eleve_id,
                    json!({ "circuit_id": circuit_id, "signataire_id": sid, "nature": p.circuit.nature_code }),
                )
                .await?,
            );
        }
    }
    Ok((signature_id, complet, evts))
}

async fn circuit_complet(
    conn: &mut PgConnection,
    etab: Uuid,
    circuit_id: Uuid,
    regle: &str,
) -> Result<bool, EntDbError> {
    let jour = jour_etab(conn, etab).await?;
    // M1 : un élève mineur au circuit exige au moins un titulaire **signé**, quelle que soit la règle.
    let naissances: Vec<(Option<chrono::NaiveDate>,)> = sqlx::query_as(
        "SELECT e.date_naissance FROM ent_doc_signataire s
           JOIN ent_eleve e ON e.etablissement_id = s.etablissement_id AND e.id = s.eleve_id
          WHERE s.etablissement_id = $1 AND s.circuit_id = $2 AND s.role = 'eleve' AND s.etat <> 'remplace'",
    )
    .bind(etab)
    .bind(circuit_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let mineur = naissances
        .iter()
        .any(|(n,)| !n.is_some_and(|n| crate::documents_natures::est_majeur(n, jour)));
    let r = sqlx::query(
        "SELECT count(*) FILTER (WHERE s.role <> 'titulaire' AND s.requis AND s.etat NOT IN ('signe', 'retire', 'remplace')) AS autres,
                count(*) FILTER (WHERE s.role = 'titulaire' AND s.etat IN ('attendu', 'invite', 'papier_depose', 'signe')) AS tit_actifs,
                -- G0 (tablette sans confirmation) ne satisfait jamais seul la règle des titulaires (E3).
                count(*) FILTER (WHERE s.role = 'titulaire' AND s.etat = 'signe' AND g.niveau <> 'G0') AS tit_signes,
                count(*) FILTER (WHERE s.role = 'titulaire' AND s.etat <> 'remplace') AS tit_total,
                count(*) FILTER (WHERE s.etat = 'signe') AS signes
           FROM ent_doc_signataire s
           LEFT JOIN ent_doc_signature g ON g.etablissement_id = s.etablissement_id AND g.signataire_id = s.id
          WHERE s.etablissement_id = $1 AND s.circuit_id = $2",
    )
    .bind(etab)
    .bind(circuit_id)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    let autres: i64 = r.get("autres");
    let actifs: i64 = r.get("tit_actifs");
    let signes_t: i64 = r.get("tit_signes");
    let total_t: i64 = r.get("tit_total");
    let signes: i64 = r.get("signes");
    let tit_ok = if total_t == 0 {
        // E2 : un circuit sans titulaire (élève majeur seul, agents, externes) n'attend aucun titulaire.
        !mineur
    } else {
        match regle {
            "tous" => actifs > 0 && signes_t == actifs,
            "un" => signes_t >= 1,
            _ => !mineur || signes_t >= 1,
        }
    };
    Ok(signes >= 1 && autres == 0 && tit_ok)
}

/// Comparaison en temps constant de deux empreintes hexadécimales (F1).
fn egal_en_temps_constant(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn credential_en_texte(c: &PublicKeyCredential) -> Result<(String, String, String, String), EntDbError> {
    let v = serde_json::to_value(c).map_err(|_| invalide())?;
    let s = |p: &str| v.pointer(p).and_then(|x| x.as_str()).map(str::to_string);
    match (
        s("/id"),
        s("/response/clientDataJSON"),
        s("/response/authenticatorData"),
        s("/response/signature"),
    ) {
        (Some(i), Some(c), Some(d), Some(g)) => Ok((i, c, d, g)),
        _ => Err(invalide()),
    }
}

impl PgEnt {
    /// E4 : droit `action` sur le sujet du document auquel le signataire appartient.
    async fn exiger_sur_signataire(&self, a: &Acteur<'_>, action: Action, signataire_id: Uuid) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let eleve = eleve_du_signataire(&mut tx, a.etablissement_id, signataire_id).await;
        tx.commit().await.map_err(db)?;
        self.exiger_sur_sujet(a, action, eleve?).await
    }

    // ─────────────── Passkey (S2) ───────────────

    /// Démarre une signature par passkey : défi WebAuthn tiré par le serveur et **lié** à l'empreinte de la version, à
    /// l'intention affichée, au signataire et à un nonce. À usage unique, cinq minutes.
    pub async fn demarrer_signature_passkey_for_actor(
        &self,
        a: &Acteur<'_>,
        signataire_id: Uuid,
    ) -> Result<DefiPasskey, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let p = preparer(&mut tx, a, signataire_id, ModeSignature::Passkey, "invite").await?;
        exiger_le_signataire(self, &mut tx, a, signataire_id, &p).await?;
        let passkeys = PgPasskeyRepo::new(self.pool().clone()).list_passkeys(a.user_id).await.map_err(|e| {
            tracing::error!(error = %e, "signature : lecture des passkeys");
            EntDbError::Domain(domain_eleve::DomainError::Infrastructure)
        })?;
        if passkeys.is_empty() {
            return Err(refus(RefusMetier::Conflit, "aucune passkey enregistrée : choisissez le code par SMS"));
        }
        let (rcr, etat) = self.passkey_engine()?.start_authentication(&passkeys)?;
        let options = serde_json::to_value(&rcr).map_err(|_| invalide())?;
        let defi_webauthn = options
            .pointer("/publicKey/challenge")
            .and_then(|c| c.as_str())
            .ok_or_else(invalide)?
            .to_string();
        let etat_json = serde_json::to_string(&etat).map_err(|_| invalide())?;
        let intention = intention_texte(p.nature.libelle, &p.libelle_role);
        let isha = sha256_hex(intention.as_bytes());
        let nonce = nonce_aleatoire();
        let lien = lien_defi(&defi_webauthn, &p.empreinte, signataire_id, &isha, &nonce);
        let defi_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_doc_defi
                (id, etablissement_id, circuit_id, signataire_id, version_id, methode, empreinte_signee, intention_texte,
                 nonce, defi_webauthn, etat_webauthn, lien_sha256, demande_par, expire_le)
             VALUES ($1,$2,$3,$4,$5,'passkey',$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(defi_id)
        .bind(a.etablissement_id)
        .bind(p.circuit.id)
        .bind(signataire_id)
        .bind(p.circuit.version_id)
        .bind(&p.empreinte)
        .bind(&intention)
        .bind(&nonce)
        .bind(&defi_webauthn)
        .bind(&etat_json)
        .bind(&lien)
        .bind(a.user_id)
        .bind(Utc::now() + Duration::minutes(DEFI_PASSKEY_MIN))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(DefiPasskey { defi_id, options, intention, empreinte: p.empreinte })
    }

    /// Termine la signature par passkey : assertion vérifiée (vérification utilisateur exigée), compteur appliqué,
    /// signature chaînée. Le défi est **brûlé avant** toute vérification : un rejeu échoue toujours.
    pub async fn signer_passkey_for_actor(
        &self,
        a: &Acteur<'_>,
        ctx: &ContexteSignature<'_>,
        defi_id: Uuid,
        credential: &PublicKeyCredential,
    ) -> Result<SignatureFaite, EntDbError> {
        // 1. Brûler le défi (transaction propre, validée quoi qu'il arrive ensuite).
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let d = sqlx::query(
            "SELECT signataire_id, circuit_id, version_id, etat_webauthn, lien_sha256, demande_par, consomme_le,
                    (expire_le > now()) AS vivant
               FROM ent_doc_defi WHERE etablissement_id = $1 AND id = $2 AND methode = 'passkey' FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(defi_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let signataire_id: Uuid = d.get("signataire_id");
        let demande_par: Uuid = d.get("demande_par");
        let consomme: Option<DateTime<Utc>> = d.get("consomme_le");
        let vivant: bool = d.get("vivant");
        if demande_par != a.user_id {
            return Err(interdit());
        }
        if consomme.is_some() || !vivant {
            return Err(refus(RefusMetier::Conflit, "défi expiré ou déjà utilisé : recommencez"));
        }
        sqlx::query("UPDATE ent_doc_defi SET consomme_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(defi_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let etat_txt: String = d.get("etat_webauthn");
        let lien: Option<String> = d.get("lien_sha256");
        let version_defi: Uuid = d.get("version_id");
        tx.commit().await.map_err(db)?;

        // 2. Vérifier l'assertion (hors transaction) et appliquer le compteur.
        let etat: ent_auth::PasskeyAuthentication =
            serde_json::from_str(&etat_txt).map_err(|_| EntDbError::Passkey(ent_auth::PasskeyError::BadState))?;
        let resultat = self.passkey_engine()?.finish_authentication(credential, &etat)?;
        match PgPasskeyRepo::new(self.pool().clone()).apply_authentication(&resultat).await.map_err(|_| {
            EntDbError::Domain(domain_eleve::DomainError::Infrastructure)
        })? {
            CounterOutcome::Regressed { .. } => return Err(EntDbError::Passkey(ent_auth::PasskeyError::PossibleCompromise)),
            CounterOutcome::Unknown => return Err(EntDbError::Passkey(ent_auth::PasskeyError::AuthFinish)),
            CounterOutcome::Updated | CounterOutcome::Unchanged => {}
        }
        let (cid, client, auth, sig) = credential_en_texte(credential)?;

        // 3. Écrire la signature.
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let p = preparer(&mut tx, a, signataire_id, ModeSignature::Passkey, "invite").await?;
        if p.circuit.version_id != version_defi {
            return Err(refus(RefusMetier::Conflit, "le document a changé depuis le défi : recommencez"));
        }
        exiger_le_signataire(self, &mut tx, a, signataire_id, &p).await?;
        let pr = Preuves {
            defi_id: Some(defi_id),
            lien,
            credential_id: Some(cid),
            client_data: Some(client),
            authenticator_data: Some(auth),
            assertion_signature: Some(sig),
            compte: Some(a.user_id),
            ..Default::default()
        };
        let (sid, complet, evts) =
            ecrire_signature(self, &mut tx, a, ctx, signataire_id, &p, ModeSignature::Passkey, Niveau::S2, pr).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evts);
        if complet {
            let _ = self.horodater_circuit(a.etablissement_id, p.circuit.id, ctx.horodatage).await;
        }
        Ok(SignatureFaite {
            signature_id: sid,
            niveau: Niveau::S2,
            circuit_complet: complet,
            avertissement: p.avertissement.map(str::to_string),
        })
    }

    // ─────────────── Code à usage unique (S1) ───────────────

    async fn creer_defi_code(
        &self,
        a: &Acteur<'_>,
        port: &dyn EnvoiCodeSignature,
        signataire_id: Uuid,
        mode: ModeSignature,
        guichet: bool,
    ) -> Result<DefiCode, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let p = preparer(&mut tx, a, signataire_id, mode, "invite").await?;
        if !guichet {
            exiger_le_signataire(self, &mut tx, a, signataire_id, &p).await?;
        }
        let (recents,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_doc_defi
              WHERE etablissement_id = $1 AND signataire_id = $2 AND methode = 'code_sms' AND cree_le > now() - interval '1 hour'",
        )
        .bind(a.etablissement_id)
        .bind(signataire_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if recents >= CODES_PAR_HEURE {
            return Err(refus(RefusMetier::Conflit, "trop de codes demandés : réessayez plus tard"));
        }
        let telephone = telephone_du_signataire(&mut tx, a.etablissement_id, signataire_id).await?;
        let code = generer_code_numerique();
        let defi_id = Uuid::new_v4();
        let intention = intention_texte(p.nature.libelle, &p.libelle_role);
        let canal = masquer_telephone(&telephone);
        sqlx::query(
            "INSERT INTO ent_doc_defi
                (id, etablissement_id, circuit_id, signataire_id, version_id, methode, empreinte_signee, intention_texte,
                 nonce, code_sha256, canal_masque, agent_user_id, demande_par, expire_le)
             VALUES ($1,$2,$3,$4,$5,'code_sms',$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(defi_id)
        .bind(a.etablissement_id)
        .bind(p.circuit.id)
        .bind(signataire_id)
        .bind(p.circuit.version_id)
        .bind(&p.empreinte)
        .bind(&intention)
        .bind(nonce_aleatoire())
        .bind(empreinte_code(defi_id, &code))
        .bind(&canal)
        .bind(guichet.then_some(a.user_id))
        .bind(a.user_id)
        .bind(Utc::now() + Duration::minutes(CODE_MIN))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let texte_sms = format!("ScolIA : votre code de signature est {code}. Ne le communiquez à personne.");
        port.envoyer(&telephone, &texte_sms).await.map_err(|_| {
            refus(RefusMetier::Conflit, "envoi du code impossible pour le moment")
        })?;
        Ok(DefiCode { defi_id, canal_masque: canal, intention, empreinte: p.empreinte })
    }

    /// Envoie un code à usage unique au téléphone enregistré du signataire (mode « code SMS »).
    pub async fn demander_code_signature_for_actor(
        &self,
        a: &Acteur<'_>,
        port: &dyn EnvoiCodeSignature,
        signataire_id: Uuid,
    ) -> Result<DefiCode, EntDbError> {
        self.creer_defi_code(a, port, signataire_id, ModeSignature::CodeSms, false).await
    }

    /// Vérifie un code dans la transaction courante : compte l'essai (même en cas d'échec : le compteur est validé),
    /// brûle le défi au succès. `Ok(None)` = code incorrect (essai compté, à valider par l'appelant).
    async fn verifier_code_tx(
        conn: &mut PgConnection,
        a: &Acteur<'_>,
        defi_id: Uuid,
        signataire_id: Uuid,
        code: &str,
        guichet_agent: Option<Uuid>,
        alertes: &mut Vec<MetierEvent>,
    ) -> Result<Option<(String, Option<String>)>, EntDbError> {
        let d = sqlx::query(
            "SELECT signataire_id, code_sha256, canal_masque, consomme_le, tentatives, agent_user_id, demande_par,
                    (expire_le > now()) AS vivant, lien_sha256
               FROM ent_doc_defi WHERE etablissement_id = $1 AND id = $2 AND methode = 'code_sms' FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(defi_id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(db)?
        .ok_or_else(introuvable)?;
        let sid: Uuid = d.get("signataire_id");
        let consomme: Option<DateTime<Utc>> = d.get("consomme_le");
        let (cumul,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(sum(tentatives), 0)::bigint FROM ent_doc_defi
              WHERE etablissement_id = $1 AND signataire_id = $2 AND methode = 'code_sms'",
        )
        .bind(a.etablissement_id)
        .bind(signataire_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        if cumul >= CODE_ECHECS_CUMULES_MAX {
            return Err(refus(
                RefusMetier::Conflit,
                "trop d'échecs de code pour ce signataire : signature bloquée, l'émetteur a été prévenu",
            ));
        }
        let tentatives: i16 = d.get("tentatives");
        let vivant: bool = d.get("vivant");
        let agent: Option<Uuid> = d.get("agent_user_id");
        let demande_par: Uuid = d.get("demande_par");
        if sid != signataire_id || agent != guichet_agent || demande_par != a.user_id {
            return Err(interdit());
        }
        if consomme.is_some() || !vivant || tentatives >= CODE_ESSAIS_MAX {
            return Err(refus(RefusMetier::Conflit, "code expiré, déjà utilisé ou trop d'essais : demandez-en un nouveau"));
        }
        let attendu: String = d.get("code_sha256");
        let canal: String = d.get("canal_masque");
        if !egal_en_temps_constant(&attendu, &empreinte_code(defi_id, code.trim())) {
            sqlx::query("UPDATE ent_doc_defi SET tentatives = tentatives + 1 WHERE etablissement_id = $1 AND id = $2")
                .bind(a.etablissement_id)
                .bind(defi_id)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            if cumul + 1 >= CODE_ECHECS_CUMULES_MAX {
                // Seuil atteint : alerte à l'émetteur (événement sans contenu) et trace au journal.
                let (doc, eleve): (Uuid, Option<Uuid>) = sqlx::query_as(
                    "SELECT c.document_id, d.eleve_id FROM ent_doc_circuit c
                       JOIN ent_doc_document d ON d.etablissement_id = c.etablissement_id AND d.id = c.document_id
                      WHERE c.etablissement_id = $1 AND c.id = (SELECT circuit_id FROM ent_doc_defi WHERE etablissement_id = $1 AND id = $2)",
                )
                .bind(a.etablissement_id)
                .bind(defi_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(db)?;
                journaliser(conn, a, Some(doc), None, "signature_bloquee", (None, None), Some(&signataire_id.to_string()), Some("trop d'échecs de code"))
                    .await?;
                alertes.push(
                    publier(
                        conn,
                        a,
                        EVT_SIGNATURE_BLOQUEE,
                        doc,
                        eleve,
                        json!({ "signataire_id": signataire_id, "raison": "echecs_de_code" }),
                    )
                    .await?,
                );
            }
            return Ok(None);
        }
        sqlx::query("UPDATE ent_doc_defi SET consomme_le = now() WHERE etablissement_id = $1 AND id = $2")
            .bind(a.etablissement_id)
            .bind(defi_id)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        Ok(Some((canal, d.get("lien_sha256"))))
    }

    /// Termine la signature par code à usage unique (niveau S1, accord simple).
    pub async fn signer_code_for_actor(
        &self,
        a: &Acteur<'_>,
        ctx: &ContexteSignature<'_>,
        defi_id: Uuid,
        signataire_id: Uuid,
        code: &str,
    ) -> Result<SignatureFaite, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let p = preparer(&mut tx, a, signataire_id, ModeSignature::CodeSms, "invite").await?;
        exiger_le_signataire(self, &mut tx, a, signataire_id, &p).await?;
        let mut alertes = Vec::new();
        let Some((canal, lien)) = Self::verifier_code_tx(&mut tx, a, defi_id, signataire_id, code, None, &mut alertes).await? else {
            // L'essai raté est compté et VALIDÉ, puis on refuse.
            tx.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&alertes);
            return Err(refus(RefusMetier::Conflit, "code incorrect"));
        };
        let pr = Preuves {
            defi_id: Some(defi_id),
            lien,
            canal_masque: Some(canal),
            compte: Some(a.user_id),
            ..Default::default()
        };
        let (sid, complet, evts) =
            ecrire_signature(self, &mut tx, a, ctx, signataire_id, &p, ModeSignature::CodeSms, Niveau::S1, pr).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evts);
        if complet {
            let _ = self.horodater_circuit(a.etablissement_id, p.circuit.id, ctx.horodatage).await;
        }
        Ok(SignatureFaite {
            signature_id: sid,
            niveau: Niveau::S1,
            circuit_complet: complet,
            avertissement: p.avertissement.map(str::to_string),
        })
    }

    // ─────────────── Signataire externe (lien + code) ───────────────

    /// Signataire externe : le **jeton du lien** (reçu une seule fois, jamais stocké) identifie la ligne ; le code part au
    /// mobile saisi par l'établissement. Rend (signataire, défi) ; l'externe n'a pas de compte : aucune session.
    pub async fn externe_demander_code(
        &self,
        etab: Uuid,
        jeton: &str,
        port: &dyn EnvoiCodeSignature,
    ) -> Result<(Uuid, DefiCode), EntDbError> {
        let (sid, circuit) = self.externe_resoudre(etab, jeton).await?;
        let _ = circuit;
        let a = Acteur { etablissement_id: etab, user_id: Uuid::nil(), role: "externe" };
        let defi = self.creer_defi_code_externe(&a, port, sid).await?;
        Ok((sid, defi))
    }

    async fn externe_resoudre(&self, etab: Uuid, jeton: &str) -> Result<(Uuid, Uuid), EntDbError> {
        if jeton.len() != 64 || !jeton.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(introuvable());
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let r: Option<(Uuid, Uuid, bool)> = sqlx::query_as(
            "SELECT id, circuit_id, (lien_expire_le > now()) FROM ent_doc_signataire
              WHERE etablissement_id = $1 AND lien_jeton_sha256 = $2 AND role = 'externe'",
        )
        .bind(etab)
        .bind(sha256_hex(jeton.as_bytes()))
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        match r {
            Some((s, c, true)) => Ok((s, c)),
            // Lien inconnu ou expiré : même réponse.
            _ => Err(introuvable()),
        }
    }

    async fn creer_defi_code_externe(
        &self,
        a: &Acteur<'_>,
        port: &dyn EnvoiCodeSignature,
        signataire_id: Uuid,
    ) -> Result<DefiCode, EntDbError> {
        // Même mécanique que le code d'un titulaire ; le « demandeur » est le signataire lui-même (identifiant nul).
        self.creer_defi_code(a, port, signataire_id, ModeSignature::CodeSms, true).await
    }

    /// Termine la signature d'un externe par lien + code (S1).
    pub async fn externe_signer_code(
        &self,
        etab: Uuid,
        ctx: &ContexteSignature<'_>,
        jeton: &str,
        defi_id: Uuid,
        code: &str,
    ) -> Result<SignatureFaite, EntDbError> {
        let (sid, _) = self.externe_resoudre(etab, jeton).await?;
        let a = Acteur { etablissement_id: etab, user_id: Uuid::nil(), role: "externe" };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let p = preparer(&mut tx, &a, sid, ModeSignature::CodeSms, "invite").await?;
        let mut alertes = Vec::new();
        let Some((canal, lien)) = Self::verifier_code_tx(&mut tx, &a, defi_id, sid, code, Some(a.user_id), &mut alertes).await? else {
            tx.commit().await.map_err(db)?;
            self.diffuser_apres_commit(&alertes);
            return Err(refus(RefusMetier::Conflit, "code incorrect"));
        };
        let pr = Preuves { defi_id: Some(defi_id), lien, canal_masque: Some(canal), ..Default::default() };
        let (signature_id, complet, evts) =
            ecrire_signature(self, &mut tx, &a, ctx, sid, &p, ModeSignature::CodeSms, Niveau::S1, pr).await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evts);
        if complet {
            let _ = self.horodater_circuit(etab, p.circuit.id, ctx.horodatage).await;
        }
        Ok(SignatureFaite {
            signature_id,
            niveau: Niveau::S1,
            circuit_complet: complet,
            avertissement: p.avertissement.map(str::to_string),
        })
    }

    // ─────────────── Guichet (G) ───────────────

    /// L'agent déclenche l'envoi du code de confirmation au téléphone **du dossier** de la personne présente.
    pub async fn demarrer_confirmation_guichet_for_actor(
        &self,
        a: &Acteur<'_>,
        port: &dyn EnvoiCodeSignature,
        signataire_id: Uuid,
    ) -> Result<DefiCode, EntDbError> {
        self.exiger_sur_signataire(a, Action::DocumentsSignaturesConstater, signataire_id).await?;
        self.creer_defi_code(a, port, signataire_id, ModeSignature::Guichet, true).await
    }

    /// **Constate** une signature au guichet (DCT-69) : agent identifié (`documents.signatures.constater`), identité
    /// **vue** (type seulement), tracé sur tablette (scellé), confirmation par code (niveau G) ou sans (G0 : simple,
    /// affichée comme telle). L'agent ne constate jamais sa propre signature.
    pub async fn constater_guichet_for_actor(
        &self,
        a: &Acteur<'_>,
        ctx: &ContexteSignature<'_>,
        c: ConstatGuichet,
    ) -> Result<SignatureFaite, EntDbError> {
        self.exiger_sur_signataire(a, Action::DocumentsSignaturesConstater, c.signataire_id).await?;
        let identite = c.identite_vue.trim().to_string();
        if !["carte_identite", "passeport", "titre_sejour", "permis", "carte_professionnelle", "autre"]
            .contains(&identite.as_str())
        {
            return Err(refus(RefusMetier::Invalide, "type de pièce d'identité vue inconnu"));
        }
        if c.trace.as_ref().is_some_and(|t| t.len() > TRACE_MAX_OCTETS || !t.starts_with(b"\x89PNG\r\n\x1a\n")) {
            return Err(refus(RefusMetier::Invalide, "tracé : PNG de 300 Ko au plus"));
        }
        let mut trace_cle = None;
        let mut trace_sha = None;
        if let Some(t) = &c.trace {
            let cle = ent_object::ObjectStore::key_for(a.etablissement_id, "signatures", Uuid::new_v4(), "png");
            let info = self.objet_put(&cle, t).await?;
            trace_sha = Some(info.sha256.clone());
            trace_cle = Some((cle, info));
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let p = preparer(&mut tx, a, c.signataire_id, ModeSignature::Guichet, "invite").await?;
        if !matches!(p.role.as_str(), "titulaire" | "eleve" | "externe") {
            return Err(refus(RefusMetier::Conflit, "le guichet ne sert que les familles, les élèves et les externes"));
        }
        if compte_du_signataire(&mut tx, a.etablissement_id, c.signataire_id).await? == Some(a.user_id) {
            return Err(refus(RefusMetier::Conflit, "un agent ne constate pas sa propre signature"));
        }
        let (niveau, confirmation, defi, lien) = match &c.confirmation {
            Some((defi_id, code)) => {
                let mut alertes = Vec::new();
                let Some((_, lien)) =
                    Self::verifier_code_tx(&mut tx, a, *defi_id, c.signataire_id, code, Some(a.user_id), &mut alertes).await?
                else {
                    tx.commit().await.map_err(db)?;
                    self.diffuser_apres_commit(&alertes);
                    return Err(refus(RefusMetier::Conflit, "code de confirmation incorrect"));
                };
                (Niveau::G, "code", Some(*defi_id), lien)
            }
            None => {
                // E3 : la tablette sans confirmation n'est admise que si la nature le déclare et que le SMS n'y est ni
                // bloqué ni faible ; sinon un agent seul ne signe jamais à la place de la personne.
                if !g0_autorise(p.nature) {
                    return Err(refus(
                        RefusMetier::Conflit,
                        "la confirmation par code est obligatoire au guichet pour cette nature de document",
                    ));
                }
                (Niveau::G0, "aucune", None, None)
            }
        };
        if let Some((cle, info)) = &trace_cle {
            crate::objets::enregistrer_objet(&mut *tx, a.etablissement_id, info, Some("image/png")).await?;
            let _ = cle;
        }
        let pr = Preuves {
            defi_id: defi,
            lien,
            constate_par: Some(a.user_id),
            identite_vue: Some(identite),
            confirmation: Some(confirmation),
            trace_sha,
            trace_cle: trace_cle.map(|(k, _)| k),
            ..Default::default()
        };
        let (sid, complet, evts) = ecrire_signature(
            self,
            &mut tx,
            a,
            ctx,
            c.signataire_id,
            &p,
            ModeSignature::Guichet,
            niveau,
            pr,
        )
        .await?;
        let mut evts = evts;
        if niveau == Niveau::G0 {
            // Événement distinct : le personnel voit les signatures simples sans confirmation.
            evts.push(
                publier(
                    &mut tx,
                    a,
                    EVT_SIGNE_SANS_CONFIRMATION,
                    p.circuit.document_id,
                    p.eleve_id,
                    json!({ "circuit_id": p.circuit.id, "signataire_id": c.signataire_id, "nature": p.circuit.nature_code }),
                )
                .await?,
            );
            journaliser(
                &mut tx,
                a,
                Some(p.circuit.document_id),
                None,
                "signature_sans_confirmation",
                (None, None),
                Some(&sid.to_string()),
                None,
            )
            .await?;
        }
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evts);
        if complet {
            let _ = self.horodater_circuit(a.etablissement_id, p.circuit.id, ctx.horodatage).await;
        }
        let avertissement = match niveau {
            Niveau::G0 => Some("tablette sans confirmation : signature simple".to_string()),
            _ => p.avertissement.map(str::to_string),
        };
        Ok(SignatureFaite { signature_id: sid, niveau, circuit_complet: complet, avertissement })
    }

    // ─────────────── Papier (P) ───────────────

    /// Dépose le scan d'un papier signé (DCT-70) : par l'établissement (`gerer` ou `constater`). PDF désarmé ou image
    /// JPEG/PNG convertie en PDF ; rien n'est servi à la famille. La signature reste « déposée, à valider ».
    pub async fn deposer_papier_for_actor(
        &self,
        a: &Acteur<'_>,
        signataire_id: Uuid,
        octets: &[u8],
    ) -> Result<(), EntDbError> {
        if self.exiger_sur_signataire(a, Action::DocumentsSignaturesGerer, signataire_id).await.is_err() {
            self.exiger_sur_signataire(a, Action::DocumentsSignaturesConstater, signataire_id).await?;
        }
        if octets.is_empty() || octets.len() > PAPIER_MAX_OCTETS {
            return Err(refus(RefusMetier::Invalide, "scan vide ou trop volumineux (15 Mio au plus)"));
        }
        let pdf = if octets.starts_with(b"%PDF-") {
            ent_pdf::piece::desarmer_pdf(octets)
                .map_err(|_| refus(RefusMetier::Invalide, "PDF refusé : enregistrez-le sans mot de passe ou prenez une photo"))?
                .octets
        } else {
            ent_pdf::piece::images_en_pdf(&[octets])
                .map_err(|_| refus(RefusMetier::Invalide, "fichier refusé : PDF, JPEG ou PNG attendu"))?
                .pdf
        };
        let cle = ent_object::ObjectStore::key_for(a.etablissement_id, "signatures", Uuid::new_v4(), "pdf");
        let info = self.objet_put(&cle, &pdf).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let p = preparer(&mut tx, a, signataire_id, ModeSignature::Papier, "invite").await?;
        crate::objets::enregistrer_objet(&mut *tx, a.etablissement_id, &info, Some("application/pdf")).await?;
        sqlx::query(
            "UPDATE ent_doc_signataire SET etat = 'papier_depose', papier_cle_objet = $3, papier_depose_le = now()
              WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(signataire_id)
        .bind(&cle)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            a,
            Some(p.circuit.document_id),
            None,
            "papier_depose",
            (Some("invite"), Some("papier_depose")),
            Some(&signataire_id.to_string()),
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Lit le scan déposé pour le contrôle (personnel qui gère les signatures seulement ; consultation journalisée).
    pub async fn lire_papier_for_actor(&self, a: &Acteur<'_>, signataire_id: Uuid) -> Result<Vec<u8>, EntDbError> {
        self.exiger_sur_signataire(a, Action::DocumentsSignaturesGerer, signataire_id).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let r: Option<(Option<String>, Uuid)> = sqlx::query_as(
            "SELECT s.papier_cle_objet, c.document_id FROM ent_doc_signataire s
               JOIN ent_doc_circuit c ON c.etablissement_id = s.etablissement_id AND c.id = s.circuit_id
              WHERE s.etablissement_id = $1 AND s.id = $2",
        )
        .bind(a.etablissement_id)
        .bind(signataire_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (cle, doc) = r.ok_or_else(introuvable)?;
        let cle = cle.ok_or_else(introuvable)?;
        journaliser(&mut tx, a, Some(doc), None, "papier_consulte", (None, None), Some(&signataire_id.to_string()), None).await?;
        tx.commit().await.map_err(db)?;
        self.objet_get(&cle).await
    }

    /// **Contrôle humain** du papier (DCT-71) : « signature présente, pages complètes, version identique ». Valider crée
    /// la signature de niveau P ; refuser remet le signataire en attente (motif au journal).
    pub async fn controler_papier_for_actor(
        &self,
        a: &Acteur<'_>,
        ctx: &ContexteSignature<'_>,
        signataire_id: Uuid,
        decision: DecisionPapier,
    ) -> Result<Option<SignatureFaite>, EntDbError> {
        self.exiger_sur_signataire(a, Action::DocumentsSignaturesGerer, signataire_id).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let p = preparer(&mut tx, a, signataire_id, ModeSignature::Papier, "papier_depose").await?;
        if compte_du_signataire(&mut tx, a.etablissement_id, signataire_id).await? == Some(a.user_id) {
            return Err(refus(RefusMetier::Conflit, "on ne contrôle pas son propre papier"));
        }
        let (cle,): (Option<String>,) = sqlx::query_as(
            "SELECT papier_cle_objet FROM ent_doc_signataire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(signataire_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let cle = cle.ok_or_else(introuvable)?;
        match decision {
            DecisionPapier::Refuser(motif) => {
                let motif = texte(&motif, 3, 500)?;
                sqlx::query(
                    "UPDATE ent_doc_signataire SET etat = 'invite', papier_cle_objet = NULL, papier_depose_le = NULL
                      WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(signataire_id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                journaliser(
                    &mut tx,
                    a,
                    Some(p.circuit.document_id),
                    None,
                    "papier_refuse",
                    (Some("papier_depose"), Some("invite")),
                    Some(&signataire_id.to_string()),
                    Some(&motif),
                )
                .await?;
                tx.commit().await.map_err(db)?;
                Ok(None)
            }
            DecisionPapier::Valider => {
                let (sha,): (String,) =
                    sqlx::query_as("SELECT sha256 FROM ent_objet WHERE etablissement_id = $1 AND cle_objet = $2")
                        .bind(a.etablissement_id)
                        .bind(&cle)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(db)?;
                let pr = Preuves {
                    constate_par: Some(a.user_id),
                    papier_cle: Some(cle),
                    papier_sha: Some(sha),
                    ..Default::default()
                };
                let (sid, complet, evts) = ecrire_signature(
                    self,
                    &mut tx,
                    a,
                    ctx,
                    signataire_id,
                    &p,
                    ModeSignature::Papier,
                    Niveau::P,
                    pr,
                )
                .await?;
                tx.commit().await.map_err(db)?;
                self.diffuser_apres_commit(&evts);
                if complet {
                    let _ = self.horodater_circuit(a.etablissement_id, p.circuit.id, ctx.horodatage).await;
                }
                Ok(Some(SignatureFaite {
                    signature_id: sid,
                    niveau: Niveau::P,
                    circuit_complet: complet,
                    avertissement: p.avertissement.map(str::to_string),
                }))
            }
        }
    }

    // ─────────────── Lecture de ce qu'on signe ───────────────

    /// La **version exacte** que le signataire va signer (journalisée comme consultation) : « ce que la personne a vu ».
    pub async fn lire_version_a_signer_for_actor(&self, a: &Acteur<'_>, signataire_id: Uuid) -> Result<Vec<u8>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let r: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT circuit_id, etat FROM ent_doc_signataire WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(signataire_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let (circuit_id, etat) = r.ok_or_else(introuvable)?;
        if compte_du_signataire(&mut tx, a.etablissement_id, signataire_id).await? != Some(a.user_id)
            || !matches!(etat.as_str(), "invite" | "papier_depose" | "signe")
        {
            return Err(introuvable());
        }
        let c = charger_circuit(&mut tx, a.etablissement_id, circuit_id, false).await?;
        if !signataire_en_droit(&mut tx, a.etablissement_id, signataire_id).await? {
            return Err(introuvable());
        }
        let (cle, sha): (String, String) = sqlx::query_as(
            "SELECT cle_objet, empreinte_sha256 FROM ent_doc_version WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(c.version_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_doc_consultation (id, etablissement_id, document_id, version_id, acteur_user_id, profil, canal, motif)
             VALUES ($1,$2,$3,$4,$5,$6,'web','lecture')",
        )
        .bind(Uuid::new_v4())
        .bind(a.etablissement_id)
        .bind(c.document_id)
        .bind(c.version_id)
        .bind(a.user_id)
        .bind(a.role.chars().take(40).collect::<String>())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let o = self.objet_get(&cle).await?;
        if sha256_hex(&o) != sha {
            return Err(refus(RefusMetier::Conflit, "le fichier ne correspond plus à son empreinte"));
        }
        Ok(o)
    }

    /// PDF signé ou dossier de preuve d'un circuit **complet** : le personnel (`gerer`) ou l'un des signataires
    /// (y compris celui qui n'a pas signé : le dossier lui est lisible). Consultation journalisée.
    pub async fn lire_piece_circuit_for_actor(
        &self,
        a: &Acteur<'_>,
        circuit_id: Uuid,
        piece: PieceCircuit,
    ) -> Result<Vec<u8>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let c = charger_circuit(&mut tx, a.etablissement_id, circuit_id, false).await?;
        let eleve = eleve_du_circuit(&mut tx, a.etablissement_id, circuit_id).await?;
        let gere = self.doc_peut_sur_sujet(a, Action::DocumentsSignaturesGerer, eleve).await?;
        if !gere && !est_signataire_du_circuit(&mut tx, a, circuit_id).await? {
            return Err(introuvable());
        }
        if c.etat != "complet" {
            return Err(refus(RefusMetier::Conflit, "le circuit n'est pas complet"));
        }
        let r = sqlx::query(
            "SELECT pdf_signe_cle_objet, preuve_cle_objet, pdf_signe_sha256, preuve_sha256
               FROM ent_doc_circuit WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(circuit_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let (cle, sha): (Option<String>, Option<String>) = match piece {
            PieceCircuit::PdfSigne => (r.get("pdf_signe_cle_objet"), r.get("pdf_signe_sha256")),
            PieceCircuit::Preuve => (r.get("preuve_cle_objet"), r.get("preuve_sha256")),
        };
        let (cle, sha) = (cle.ok_or_else(introuvable)?, sha.ok_or_else(introuvable)?);
        journaliser(
            &mut tx,
            a,
            Some(c.document_id),
            None,
            match piece {
                PieceCircuit::PdfSigne => "pdf_signe_consulte",
                PieceCircuit::Preuve => "dossier_preuve_consulte",
            },
            (None, None),
            Some(&circuit_id.to_string()),
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        let o = self.objet_get(&cle).await?;
        if sha256_hex(&o) != sha {
            return Err(refus(RefusMetier::Conflit, "le fichier ne correspond plus à son empreinte"));
        }
        Ok(o)
    }

    // ─────────────── Vérification interne (DCT-79) ───────────────

    /// Recalcule la chaîne, les liens de défi et l'empreinte de la version d'un circuit ; écrit le résultat au journal.
    pub async fn verifier_signatures_for_actor(&self, a: &Acteur<'_>, circuit_id: Uuid) -> Result<RapportVerification, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let c = charger_circuit(&mut tx, a.etablissement_id, circuit_id, false).await?;
        let eleve = eleve_du_circuit(&mut tx, a.etablissement_id, circuit_id).await?;
        self.exiger_sur_sujet(a, Action::DocumentsSignaturesGerer, eleve).await?;
        let lignes = sqlx::query(
            "SELECT g.signataire_id, g.version_id, g.rang_chaine, g.empreinte_signee, g.methode, g.niveau, g.intention_sha256,
                    g.horodatage_serveur, g.lien_sha256, g.precedent_sha256, g.entree_sha256, g.defi_id
               FROM ent_doc_signature g WHERE g.etablissement_id = $1 AND g.circuit_id = $2 ORDER BY g.rang_chaine",
        )
        .bind(a.etablissement_id)
        .bind(circuit_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let (empreinte_version,): (String,) = sqlx::query_as(
            "SELECT empreinte_sha256 FROM ent_doc_version WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(a.etablissement_id)
        .bind(c.version_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let mut entrees = Vec::new();
        let mut empreinte_ok = true;
        let mut liens_ok = true;
        for l in &lignes {
            let emp: String = l.get("empreinte_signee");
            if emp != empreinte_version {
                empreinte_ok = false;
            }
            let defi_id: Option<Uuid> = l.get("defi_id");
            let lien: Option<String> = l.get("lien_sha256");
            if let (Some(d), Some(lien_sig)) = (defi_id, &lien) {
                // Passkey : le lien se recalcule depuis le défi, l'empreinte, le signataire, l'intention et le nonce.
                let r = sqlx::query(
                    "SELECT defi_webauthn, nonce, lien_sha256 FROM ent_doc_defi WHERE etablissement_id = $1 AND id = $2",
                )
                .bind(a.etablissement_id)
                .bind(d)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                let stocke: Option<String> = r.get("lien_sha256");
                if stocke.as_deref() != Some(lien_sig.as_str()) {
                    liens_ok = false;
                } else if let Some(defi) = r.get::<Option<String>, _>("defi_webauthn") {
                    let sid: Uuid = l.get("signataire_id");
                    let isha: String = l.get("intention_sha256");
                    let nonce: String = r.get("nonce");
                    if lien_defi(&defi, &emp, sid, &isha, &nonce) != *lien_sig {
                        liens_ok = false;
                    }
                }
            }
            entrees.push(EntreeLue {
                entree: EntreeChaine {
                    circuit_id,
                    signataire_id: l.get("signataire_id"),
                    version_id: l.get("version_id"),
                    rang: l.get("rang_chaine"),
                    empreinte_signee: emp,
                    methode: l.get("methode"),
                    niveau: l.get("niveau"),
                    intention_sha256: l.get("intention_sha256"),
                    horodatage: l.get("horodatage_serveur"),
                    lien_sha256: lien,
                    precedent: l.get("precedent_sha256"),
                },
                entree_sha256: l.get("entree_sha256"),
            });
        }
        let verdict = verifier_chaine(circuit_id, &entrees);
        let rapport = RapportVerification {
            circuit_id,
            nb_signatures: entrees.len(),
            chaine_valide: verdict.is_ok(),
            rupture: verdict.as_ref().err().map(|r| (r.rang, r.raison)),
            empreinte_version_conforme: empreinte_ok,
            liens_conformes: liens_ok,
            tete_chaine: verdict.ok(),
        };
        journaliser(
            &mut tx,
            a,
            Some(c.document_id),
            None,
            "signatures_verifiees",
            (None, Some(if rapport.chaine_valide && empreinte_ok && liens_ok { "conforme" } else { "non_conforme" })),
            Some(&circuit_id.to_string()),
            None,
        )
        .await?;
        tx.commit().await.map_err(db)?;
        Ok(rapport)
    }

    // ─────────────── Finalisation ───────────────

    /// Complétion d'un circuit (dans la transaction de la dernière signature) : PDF signé (page de signatures, cachet),
    /// dossier de preuve (PDF lisible, cacheté), tête de chaîne, demande d'horodatage. Un échec annule tout.
    async fn finaliser_circuit(
        &self,
        conn: &mut PgConnection,
        a: &Acteur<'_>,
        ctx: &ContexteSignature<'_>,
        circuit: &LigneCircuit,
    ) -> Result<MetierEvent, EntDbError> {
        let etab = a.etablissement_id;
        let n = nature_signature(&circuit.nature_code).ok_or_else(invalide)?;
        let v = sqlx::query(
            "SELECT v.cle_objet, v.empreinte_sha256, v.numero_version, d.serie_code, d.numero_annee, d.numero, d.eleve_id
               FROM ent_doc_version v
               JOIN ent_doc_document d ON d.etablissement_id = v.etablissement_id AND d.id = v.document_id
              WHERE v.etablissement_id = $1 AND v.id = $2",
        )
        .bind(etab)
        .bind(circuit.version_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
        let cle_version: String = v.get("cle_objet");
        let empreinte: String = v.get("empreinte_sha256");
        let numero_version: i32 = v.get("numero_version");
        let eleve_id: Option<Uuid> = v.get("eleve_id");
        let numero = match (v.get::<Option<String>, _>("serie_code"), v.get::<Option<i32>, _>("numero_annee"), v.get::<Option<i32>, _>("numero")) {
            (Some(s), Some(an), Some(nu)) => numero_affiche(an, &s, nu),
            _ => "sans numéro".to_string(),
        };
        let original = self.objet_get(&cle_version).await?;
        if sha256_hex(&original) != empreinte {
            return Err(refus(RefusMetier::Conflit, "la version à signer ne correspond plus à son empreinte"));
        }
        let signatures = sqlx::query(
            "SELECT g.id, g.signataire_id, g.rang_chaine, g.methode, g.niveau, g.horodatage_serveur, g.entree_sha256,
                    g.precedent_sha256, g.passkey_credential_id, g.code_canal_masque, g.identite_vue, g.confirmation,
                    g.formule_delegation, g.lien_sha256, g.constate_par_user_id, s.libelle_role
               FROM ent_doc_signature g
               JOIN ent_doc_signataire s ON s.etablissement_id = g.etablissement_id AND s.id = g.signataire_id
              WHERE g.etablissement_id = $1 AND g.circuit_id = $2 ORDER BY g.rang_chaine",
        )
        .bind(etab)
        .bind(circuit.id)
        .fetch_all(&mut *conn)
        .await
        .map_err(db)?;
        let tete: String = signatures.last().map(|l| l.get("entree_sha256")).ok_or_else(invalide)?;
        let mut blocs: Vec<String> = Vec::new();
        for l in &signatures {
            let sid: Uuid = l.get("signataire_id");
            let nom = nom_du_signataire(conn, etab, sid).await?;
            let methode: String = l.get("methode");
            let niveau = Niveau::parse(&l.get::<String, _>("niveau")).ok_or_else(invalide)?;
            let quand: DateTime<Utc> = l.get("horodatage_serveur");
            let mut ligne = format!(
                "{}. {} — {} — {} ({}) — {}",
                l.get::<i32, _>("rang_chaine"),
                nom,
                l.get::<String, _>("libelle_role"),
                niveau.libelle(),
                methode,
                horodatage_canonique(quand)
            );
            if let Some(p) = l.get::<Option<String>, _>("passkey_credential_id") {
                ligne.push_str(&format!(" — clé d'accès {}", p.chars().take(16).collect::<String>()));
            }
            if let Some(c) = l.get::<Option<String>, _>("code_canal_masque") {
                ligne.push_str(&format!(" — code envoyé au {c}"));
            }
            if let Some(i) = l.get::<Option<String>, _>("identite_vue") {
                ligne.push_str(&format!(" — identité vue : {i} ; confirmation : {}", l.get::<Option<String>, _>("confirmation").unwrap_or_default()));
            }
            if let Some(f) = l.get::<Option<String>, _>("formule_delegation") {
                ligne.push_str(&format!(" — {f}"));
            }
            blocs.push(ligne);
            blocs.push(format!("   sceau de l'entrée : {}", l.get::<String, _>("entree_sha256")));
        }
        // PDF signé : l'original (jamais modifié) suivi d'une page de signatures, puis le cachet.
        let mut lignes_page = vec![
            format!("Document : {} — {numero} — version {numero_version}", n.libelle),
            format!("Empreinte SHA-256 du document signé : {empreinte}"),
            String::new(),
        ];
        lignes_page.extend(blocs.iter().cloned());
        lignes_page.push(String::new());
        lignes_page.push(format!("Tête de la chaîne des signatures : {tete}"));
        lignes_page.push("La vérification se fait sur la page publique de vérification, avec le code imprimé sur le document ou en y déposant ce PDF.".to_string());
        let pdf_signe = ent_pdf::ajouter_page_signatures(&original, "Signatures électroniques", &lignes_page).ok();
        // M8 : jamais de « complet » silencieux sans PDF signé : le dossier de preuve le dit explicitement.
        let mention_pdf_signe = if pdf_signe.is_some() {
            None
        } else {
            Some("PDF signé non produit : la structure de ce document ne permet pas d'y ajouter la page de signatures ; la preuve repose sur ce dossier et sur l'empreinte de la version.")
        };
        let (mut cle_signe, mut sha_signe) = (None, None);
        if let Some(pdf) = &pdf_signe {
            let scelle = ctx.moteur.cachet.sceller(etab, pdf).map_err(|_| {
                refus(RefusMetier::Conflit, "cachet de l'établissement indisponible : signature impossible")
            })?;
            let cle = ent_object::ObjectStore::key_for(etab, "signatures", Uuid::new_v4(), "pdf");
            let info = self.objet_put(&cle, &scelle.octets).await?;
            crate::objets::enregistrer_objet(&mut *conn, etab, &info, Some("application/pdf")).await?;
            cle_signe = Some(cle);
            sha_signe = Some(info.sha256);
        }
        // Dossier de preuve : lisible par un humain, cacheté.
        let mut d = ent_pdf::Document::new(ent_pdf::Meta {
            titre: "Chemin de preuve".to_string(),
            auteur: "ScolIA".to_string(),
            creation: None,
        })
        .avec_pied("ScolIA — chemin de preuve des signatures");
        d.titre("Chemin de preuve des signatures");
        d.paragraphe(&format!("Document : {} — {numero} — version {numero_version}", n.libelle));
        d.paragraphe(&format!("Empreinte SHA-256 complète du document signé : {empreinte}"));
        if let Some(s) = &sha_signe {
            d.paragraphe(&format!("Empreinte SHA-256 du PDF signé (avec page de signatures, avant cachet) : voir le PDF signé ; empreinte du fichier remis : {s}"));
        }
        d.filet();
        d.sous_titre("Qui a signé, quand, comment");
        for b in &blocs {
            d.paragraphe(b);
        }
        d.filet();
        if let Some(m) = mention_pdf_signe {
            d.paragraphe(m);
        }
        d.paragraphe(&format!("Tête de la chaîne (chaque entrée scelle la précédente) : {tete}"));
        d.paragraphe(&format!(
            "Horodatage qualifié : {}",
            if circuit.horodater {
                "prévu pour cette nature de document ; si le service en renvoie un, le jeton RFC 3161 est conservé séparément à l'état « reçu, non vérifié » : ScolIA n'en valide pas la signature, il ne vaut donc pas, à ce stade, horodatage qualifié vérifié"
            } else {
                "non demandé pour cette nature de document"
            }
        ));
        d.paragraphe("Les signatures par clé d'accès, par code, au guichet ou sur papier sont des signatures électroniques simples ou renforcées, au sens du règlement eIDAS : ce ne sont pas des signatures qualifiées. La force de la preuve repose sur ce dossier, l'empreinte du document et le cachet de l'établissement.");
        let preuve_octets = d.octets();
        let scelle = ctx.moteur.cachet.sceller(etab, &preuve_octets).map_err(|_| {
            refus(RefusMetier::Conflit, "cachet de l'établissement indisponible : signature impossible")
        })?;
        let cle_preuve = ent_object::ObjectStore::key_for(etab, "signatures", Uuid::new_v4(), "pdf");
        let info = self.objet_put(&cle_preuve, &scelle.octets).await?;
        crate::objets::enregistrer_objet(&mut *conn, etab, &info, Some("application/pdf")).await?;
        sqlx::query(
            "UPDATE ent_doc_circuit
                SET etat = 'complet', clos_le = now(), pdf_signe_cle_objet = $3, pdf_signe_sha256 = $4,
                    preuve_cle_objet = $5, preuve_sha256 = $6, tete_chaine_sha256 = $7
              WHERE etablissement_id = $1 AND id = $2 AND etat = 'ouvert'",
        )
        .bind(etab)
        .bind(circuit.id)
        .bind(&cle_signe)
        .bind(&sha_signe)
        .bind(&cle_preuve)
        .bind(&info.sha256)
        .bind(&tete)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        sqlx::query(
            "UPDATE ent_doc_signataire SET etat = 'retire'
              WHERE etablissement_id = $1 AND circuit_id = $2 AND etat IN ('attendu', 'invite', 'papier_depose')",
        )
        .bind(etab)
        .bind(circuit.id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        if circuit.horodater {
            let svc: Option<(String, bool)> =
                sqlx::query_as("SELECT mode, actif FROM ent_doc_horodatage_service WHERE etablissement_id = $1")
                    .bind(etab)
                    .fetch_optional(&mut *conn)
                    .await
                    .map_err(db)?;
            if let Some((mode, true)) = svc {
                sqlx::query(
                    "INSERT INTO ent_doc_horodatage (id, etablissement_id, circuit_id, empreinte_horodatee, mode)
                     VALUES ($1,$2,$3,$4,$5)",
                )
                .bind(Uuid::new_v4())
                .bind(etab)
                .bind(circuit.id)
                .bind(&info.sha256)
                .bind(mode)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            }
        }
        journaliser(
            conn,
            a,
            Some(circuit.document_id),
            None,
            "circuit_complet",
            (Some("ouvert"), Some("complet")),
            Some(&circuit.id.to_string()),
            None,
        )
        .await?;
        publier(
            conn,
            a,
            EVT_CIRCUIT_COMPLET,
            circuit.document_id,
            eleve_id,
            json!({ "circuit_id": circuit.id, "nature": circuit.nature_code }),
        )
        .await
    }

    // ─────────────── Horodatage qualifié (RFC 3161) ───────────────

    /// Demande le jeton d'horodatage d'un circuit complet, si un service est branché et actif.
    ///
    /// * la demande passe à `en_cours` **avant** l'appel réseau (un second appel simultané ne repart pas : un jeton se
    ///   facture) ; une demande `en_cours` depuis plus de dix minutes (arrêt en plein appel) est reprise ;
    /// * le jeton reçu est conservé **tel quel**, état `recu_non_verifie` : ScolIA n'en valide pas la signature CMS
    ///   (aucune brique libre auditée retenue, E1) ; il n'est donc jamais présenté comme vérifié ;
    /// * sans service ou en cas d'échec, la demande retourne `en_attente` ; après cinq échecs (ou un refus du service)
    ///   elle passe `echec`, avec **alerte** (journal et événement) et reprise possible (`relancer_horodatage_for_actor`).
    ///
    /// `Ok(true)` si un jeton a été reçu.
    pub async fn horodater_circuit(
        &self,
        etab: Uuid,
        circuit_id: Uuid,
        fournisseur: &dyn FournisseurHorodatage,
    ) -> Result<bool, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let svc = sqlx::query(
            "SELECT mode, url, identifiant, secret_scelle FROM ent_doc_horodatage_service
              WHERE etablissement_id = $1 AND actif",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some(svc) = svc else {
            tx.commit().await.map_err(db)?;
            return Ok(false);
        };
        let reclame: Option<(Uuid, String, String, i16)> = sqlx::query_as(
            "UPDATE ent_doc_horodatage SET etat = 'en_cours', en_cours_le = now()
              WHERE etablissement_id = $1 AND circuit_id = $2
                AND (etat = 'en_attente' OR (etat = 'en_cours' AND en_cours_le < now() - interval '10 minutes'))
          RETURNING id, empreinte_horodatee, mode, tentatives",
        )
        .bind(etab)
        .bind(circuit_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((id, empreinte, mode, tentatives)) = reclame else {
            tx.commit().await.map_err(db)?;
            return Ok(false);
        };
        let mode_h = ModeHorodatage::parse(&mode).ok_or_else(invalide)?;
        let config = if mode_h == ModeHorodatage::Propre {
            let secret: Option<String> = svc.get("secret_scelle");
            let secret = ChampsSensibles::depuis_env()
                .ouvrir_opt("ent_doc_horodatage_service", "secret_scelle", etab, secret)
                .map_err(EntDbError::Domain)?;
            Some(ConfigServiceHorodatage {
                url: svc.get::<Option<String>, _>("url").unwrap_or_default(),
                identifiant: svc.get("identifiant"),
                secret,
            })
        } else {
            None
        };
        // La réclamation est validée AVANT l'appel réseau.
        tx.commit().await.map_err(db)?;
        let service = fournisseur.service(etab, mode_h, config.as_ref());
        let resultat = match service {
            None => Err(ErreurHorodatage::Indisponible("aucun service d'horodatage disponible".into())),
            Some(service) => {
                let mut brut = [0u8; 32];
                for (i, o) in brut.iter_mut().enumerate() {
                    *o = u8::from_str_radix(&empreinte[2 * i..2 * i + 2], 16).map_err(|_| invalide())?;
                }
                tokio::task::spawn_blocking(move || service.horodater(&brut))
                    .await
                    .map_err(|_| refus(RefusMetier::Conflit, "tâche d'horodatage interrompue"))?
            }
        };
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let (ok, evts) = match resultat {
            Ok(jeton) => {
                sqlx::query(
                    "UPDATE ent_doc_horodatage SET etat = 'recu_non_verifie', jeton_hex = $3, obtenu_le = now(), tentatives = $4
                      WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_cours'",
                )
                .bind(etab)
                .bind(id)
                .bind(hex(&jeton))
                .bind(tentatives + 1)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                (true, Vec::new())
            }
            Err(e) => {
                let msg: String = e.to_string().chars().take(300).collect();
                let definitif = tentatives + 1 >= 5 || matches!(e, ErreurHorodatage::Refuse(_));
                sqlx::query(
                    "UPDATE ent_doc_horodatage SET etat = $3, derniere_erreur = $4, tentatives = $5
                      WHERE etablissement_id = $1 AND id = $2 AND etat = 'en_cours'",
                )
                .bind(etab)
                .bind(id)
                .bind(if definitif { "echec" } else { "en_attente" })
                .bind(msg)
                .bind(tentatives + 1)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                let mut evts = Vec::new();
                if definitif {
                    // Alerte : journal et événement (sans contenu), l'établissement le voit au relevé.
                    let (doc, eleve): (Uuid, Option<Uuid>) = sqlx::query_as(
                        "SELECT c.document_id, d.eleve_id FROM ent_doc_circuit c
                           JOIN ent_doc_document d ON d.etablissement_id = c.etablissement_id AND d.id = c.document_id
                          WHERE c.etablissement_id = $1 AND c.id = $2",
                    )
                    .bind(etab)
                    .bind(circuit_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db)?;
                    let systeme = Acteur { etablissement_id: etab, user_id: Uuid::nil(), role: "systeme" };
                    journaliser(&mut tx, &systeme, Some(doc), None, "horodatage_en_echec", (None, Some("echec")), Some(&circuit_id.to_string()), None)
                        .await?;
                    evts.push(
                        publier(
                            &mut tx,
                            &systeme,
                            EVT_HORODATAGE_ECHEC,
                            doc,
                            eleve,
                            json!({ "circuit_id": circuit_id }),
                        )
                        .await?,
                    );
                }
                (false, evts)
            }
        };
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&evts);
        Ok(ok)
    }

    /// Remet en attente un horodatage en `echec` (reprise après correction du service) : `documents.signatures.gerer`
    /// sur le sujet du document.
    pub async fn relancer_horodatage_for_actor(&self, a: &Acteur<'_>, circuit_id: Uuid) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id).await.map_err(db)?;
        let eleve = eleve_du_circuit(&mut tx, a.etablissement_id, circuit_id).await?;
        self.exiger_sur_sujet(a, Action::DocumentsSignaturesGerer, eleve).await?;
        let n = sqlx::query(
            "UPDATE ent_doc_horodatage SET etat = 'en_attente', tentatives = 0, derniere_erreur = NULL
              WHERE etablissement_id = $1 AND circuit_id = $2 AND etat = 'echec'",
        )
        .bind(a.etablissement_id)
        .bind(circuit_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(refus(RefusMetier::Conflit, "aucun horodatage en échec pour ce circuit"));
        }
        tx.commit().await.map_err(db)?;
        Ok(())
    }

    /// Tâche de fond : réessaie les horodatages en attente (ou restés `en_cours` trop longtemps) d'un établissement ;
    /// rend le nombre de jetons reçus.
    pub async fn horodater_en_attente(
        &self,
        etab: Uuid,
        fournisseur: &dyn FournisseurHorodatage,
        max: i64,
    ) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await.map_err(db)?;
        let ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT circuit_id FROM ent_doc_horodatage
              WHERE etablissement_id = $1
                AND (etat = 'en_attente' OR (etat = 'en_cours' AND en_cours_le < now() - interval '10 minutes'))
              ORDER BY demande_le LIMIT $2",
        )
        .bind(etab)
        .bind(max.clamp(1, 200))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        let mut obtenus = 0;
        for (c,) in ids {
            if self.horodater_circuit(etab, c, fournisseur).await? {
                obtenus += 1;
            }
        }
        Ok(obtenus)
    }
}

async fn telephone_du_signataire(
    conn: &mut PgConnection,
    etab: Uuid,
    signataire_id: Uuid,
) -> Result<String, EntDbError> {
    let r = sqlx::query(
        "SELECT s.externe_contact_scelle,
                (SELECT c.valeur FROM ent_personne_coordonnee c
                  WHERE c.etablissement_id = s.etablissement_id
                    AND c.personne_id = COALESCE(s.personne_id, (SELECT ag.personne_id FROM ent_agent ag
                                                                  WHERE ag.etablissement_id = s.etablissement_id AND ag.id = s.agent_id))
                    AND c.type = 'mobile' AND c.au IS NULL
                  ORDER BY c.principal DESC, c.du DESC LIMIT 1) AS telephone
           FROM ent_doc_signataire s
          WHERE s.etablissement_id = $1 AND s.id = $2",
    )
    .bind(etab)
    .bind(signataire_id)
    .fetch_optional(conn)
    .await
    .map_err(db)?
    .ok_or_else(introuvable)?;
    let scelle: Option<String> = r.get("externe_contact_scelle");
    let tel: Option<String> = match scelle {
        Some(s) => Some(
            ChampsSensibles::depuis_env()
                .ouvrir("ent_doc_signataire", "externe_contact_scelle", signataire_id, &s)
                .map_err(EntDbError::Domain)?,
        ),
        None => r.get("telephone"),
    };
    match tel.map(|t| t.trim().to_string()).filter(|t| t.chars().filter(char::is_ascii_digit).count() >= 8) {
        Some(t) => Ok(t),
        None => Err(refus(RefusMetier::Conflit, "aucun téléphone enregistré pour ce signataire : choisissez un autre mode")),
    }
}
