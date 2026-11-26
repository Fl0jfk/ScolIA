//! Socle 27, lot G3b-3 : **fichiers remis, remise, téléchargement, purge, clôture, renvois** (`27_RGPD_ARCHIVES.md`
//! §9.4.5, RGP-154, RGP-163, RGP-166, R-RG42, R-RG47).
//!
//! * Les fichiers (un PDF lisible, un CSV par section) sont produits à la demande de remise depuis le contenu
//!   **masqué** de l'extrait ; seuls leurs métadonnées et empreintes SHA-256 sont en base.
//! * Chiffrement **sans nouvelle dépendance** (D-062, Q-EXP1 non tranchée, valeur prudente D-069) : pas d'archive ZIP
//!   AES ni `age` ; chaque fichier est **scellé** au repos par `SecretBox` (usage dérivé `extrait-rgpd:v1`, AAD liée à
//!   l'établissement, à l'extrait, à sa version et au fichier). Un fichier recopié sur un autre extrait ne s'ouvre pas.
//! * Remise : passkey **récente** du gestionnaire (défi lié à l'empreinte de l'extrait et à l'intention « remettre »),
//!   **en ligne** seulement sur le compte du demandeur dans cet établissement, ou **en main propre** (émargement).
//!   Jamais une adresse ou un numéro fournis par la demande (bloquant B).
//! * Téléchargement : passkey récente du **demandeur** (défi lié à l'extrait), chaque téléchargement journalisé.
//! * Mise à disposition de 7 jours (réglage), puis objets supprimés ; « purgé » n'est écrit qu'après confirmation.

use crate::droits_demandes::{db, invalide, journal, texte};
use crate::droits_extrait::{ContenuExtrait, SectionContenu};
use crate::notifications::{notifier_personne, EnvoiAPlanifier};
use crate::passkey_repo::{CounterOutcome, PgPasskeyRepo};
use crate::service::{EntDbError, PgEnt, RefusMetier};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use domain_eleve::DomainError;
use ent_auth::{Action, PublicKeyCredential};
use ent_pdf::{DateHeure, Meta};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Row};
use uuid::Uuid;

/// Nature de notification de la remise : **aucun nom d'élève** (R-RG32).
pub const KIND_EXTRAIT_PRET: &str = "rgpd.extrait_pret";
/// Usage dérivé de la clé de scellement des fichiers remis.
pub const USAGE_SCELLE: &str = "extrait-rgpd:v1";
/// Durée de vie d'un défi de passkey (minutes) : « récente ».
const DEFI_MIN: i64 = 5;

// ─────────────────────────────── Outils purs ───────────────────────────────

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha(octets: &[u8]) -> String {
    hex(&Sha256::digest(octets))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 standard (avec remplissage) : `SecretBox` ne scelle que du texte, les fichiers y passent encodés.
pub fn base64_encoder(donnees: &[u8]) -> String {
    let mut out = String::with_capacity(donnees.len().div_ceil(3) * 4);
    for bloc in donnees.chunks(3) {
        let b = [
            bloc[0],
            *bloc.get(1).unwrap_or(&0),
            *bloc.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if bloc.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if bloc.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Décodage du format ci-dessus ; `None` si le texte n'est pas du base64 standard valide.
pub fn base64_decoder(texte: &str) -> Option<Vec<u8>> {
    let t = texte.as_bytes();
    if !t.len().is_multiple_of(4) {
        return None;
    }
    let val = |c: u8| B64.iter().position(|&x| x == c).map(|p| p as u32);
    let mut out = Vec::with_capacity(t.len() / 4 * 3);
    for bloc in t.chunks(4) {
        let pad = bloc.iter().filter(|&&c| c == b'=').count();
        if pad > 2 || bloc[..4 - pad].contains(&b'=') {
            return None;
        }
        let mut n = 0u32;
        for &c in &bloc[..4 - pad] {
            n = (n << 6) | val(c)?;
        }
        n <<= 6 * pad as u32;
        let o = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&o[..3 - pad]);
    }
    Some(out)
}

/// Une cellule CSV (RFC 4180) : guillemets doublés, champ entouré dès qu'il contient séparateur, guillemet ou
/// saut de ligne. Une cellule qui commencerait par `=`, `+`, `-`, `@` est préfixée d'une apostrophe (injection de
/// formule dans un tableur).
pub fn csv_cellule(v: &str) -> String {
    let mut s = v.to_string();
    if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        s.insert(0, '\'');
    }
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

/// CSV d'**une section** : UTF-8 avec BOM (tableurs), fins de ligne CRLF, format long
/// `table,ligne,colonne,valeur` (des tables aux colonnes différentes tiennent dans un seul fichier).
pub fn csv_section(s: &SectionContenu) -> Vec<u8> {
    let mut out = String::from("\u{feff}table,ligne,colonne,valeur\r\n");
    for t in &s.tables {
        for (rang, l) in t.lignes.iter().enumerate() {
            let ligne =
                l.id.map_or_else(|| (rang + 1).to_string(), |i| i.to_string());
            for (col, val) in &l.valeurs {
                out.push_str(&format!(
                    "{},{},{},{}\r\n",
                    csv_cellule(&t.nom),
                    csv_cellule(&ligne),
                    csv_cellule(col),
                    csv_cellule(val)
                ));
            }
        }
    }
    out.into_bytes()
}

fn nom_csv(fournisseur: &str) -> String {
    format!("section-{fournisseur}.csv")
}

/// Lignes du PDF : destinataires, durées, sources, sections, exclusions et ce qui n'est pas remis (RGP-163).
pub fn lignes_pdf(c: &ContenuExtrait, reference: &str, version: i32) -> Vec<String> {
    let mut l: Vec<String> = vec![
        "Extrait des données personnelles".into(),
        format!("Demande {reference} — extrait n° {version}"),
        "--------".into(),
    ];
    if !c.complet {
        l.push("Cet extrait est incomplet : certaines sections n'ont pas pu être assemblées (voir ci-dessous).".into());
    }
    l.push("Destinataires des données".into());
    for d in &c.destinataires_internes {
        l.push(format!("• en interne : {d}"));
    }
    for d in &c.destinataires_externes {
        l.push(format!("• externe : {d}"));
    }
    for d in &c.soustraitants {
        l.push(format!("• sous-traitant : {d}"));
    }
    l.push(String::new());
    l.push("Durées de conservation".into());
    for d in &c.durees {
        let u = d
            .utilite_mois
            .map_or("—".to_string(), |m| format!("{m} mois"));
        l.push(format!("• {} : {u} ({})", d.libelle, d.source));
    }
    l.push(String::new());
    l.push("Sources des données".into());
    for s in &c.sources {
        l.push(format!("• {s}"));
    }
    l.push("--------".into());
    for s in &c.sections {
        l.push(format!(
            "Section {} (socle {}) — {}",
            s.fournisseur,
            s.socle,
            s.statut.code()
        ));
        for t in &s.tables {
            l.push(format!("Table {} : {} ligne(s)", t.nom, t.lignes.len()));
            for ligne in &t.lignes {
                let v: Vec<String> = ligne
                    .valeurs
                    .iter()
                    .map(|(k, v)| format!("{k} : {v}"))
                    .collect();
                l.push(format!("• {}", v.join(" ; ")));
            }
        }
        for e in &s.exclusions {
            l.push(format!("Non remis — {} : {}", e.table, e.motif));
        }
        l.push(String::new());
    }
    l.push("--------".into());
    l.push(
        "Pièces du coffre et documents émis : listés mais non joints à cet extrait (une pièce qui concerne un tiers \
         n'est jamais remise ni caviardée). Elles se demandent au guichet."
            .into(),
    );
    l
}

fn date_heure(t: DateTime<Utc>) -> DateHeure {
    DateHeure {
        an: t.year() as u16,
        mois: t.month() as u8,
        jour: t.day() as u8,
        heure: t.hour() as u8,
        minute: t.minute() as u8,
        seconde: t.second() as u8,
    }
}

/// PDF de l'extrait, lisible par `ent-pdf` (métadonnées minimales : aucun nom de personne dans le titre).
pub fn pdf_extrait(
    c: &ContenuExtrait,
    reference: &str,
    version: i32,
    maintenant: DateTime<Utc>,
) -> Vec<u8> {
    let meta = Meta {
        titre: "Extrait des données personnelles".into(),
        auteur: "ScolIA".into(),
        creation: Some(date_heure(maintenant)),
    };
    let pied = format!(
        "Demande {reference} — empreinte {}",
        &c.empreinte[..12.min(c.empreinte.len())]
    );
    ent_pdf::rendre_lignes(meta, &pied, &lignes_pdf(c, reference, version))
}

/// Les données **authentifiées** d'un fichier scellé : établissement, extrait, version et nom du fichier.
pub fn aad_fichier(etab: Uuid, extrait: Uuid, version: i32, nom: &str) -> Vec<u8> {
    format!("rgp-extrait:{etab}:{extrait}:v{version}:{nom}").into_bytes()
}

/// Un fichier prêt à être remis.
#[derive(Clone, Debug)]
pub struct FichierRendu {
    pub nom: String,
    pub type_: &'static str,
    pub octets: Vec<u8>,
}

/// Tous les fichiers d'une remise : le PDF, puis un CSV par section qui porte des lignes.
pub fn fichiers_de(
    c: &ContenuExtrait,
    reference: &str,
    version: i32,
    maintenant: DateTime<Utc>,
) -> Vec<FichierRendu> {
    let mut v = vec![FichierRendu {
        nom: "extrait.pdf".into(),
        type_: "pdf",
        octets: pdf_extrait(c, reference, version, maintenant),
    }];
    for s in c.sections.iter().filter(|s| s.lignes > 0) {
        v.push(FichierRendu {
            nom: nom_csv(&s.fournisseur),
            type_: "csv",
            octets: csv_section(s),
        });
    }
    v
}

// ─────────────────────────────── Vues ───────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct FichierVue {
    pub id: Uuid,
    pub nom: String,
    pub type_: String,
    pub octets: i32,
    pub empreinte: String,
    pub disponible_jusqu_le: DateTime<Utc>,
    pub purge: bool,
    pub telechargements: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct RemiseVue {
    pub id: Uuid,
    pub extrait_id: Uuid,
    pub mode: String,
    pub remis_le: DateTime<Utc>,
    pub empreinte_extrait: String,
    pub fichiers: Vec<FichierVue>,
}

/// « Mes données » : un extrait remis au demandeur, avec ses fichiers.
#[derive(Clone, Debug, Serialize)]
pub struct MonExtrait {
    pub demande_reference: String,
    pub remise: RemiseVue,
}

/// Défi de passkey récente : options WebAuthn à jouer par le navigateur, intention à afficher.
#[derive(Clone, Debug, Serialize)]
pub struct DefiRemise {
    pub defi_id: Uuid,
    pub options: serde_json::Value,
    pub intention: String,
    pub empreinte: String,
}

/// Émargement d'une remise en main propre.
#[derive(Clone, Debug)]
pub struct Emargement {
    pub nom: String,
    pub piece_vue: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeRemise {
    EnLigne,
    MainPropre,
}

impl ModeRemise {
    pub fn code(self) -> &'static str {
        match self {
            Self::EnLigne => "en_ligne",
            Self::MainPropre => "main_propre",
        }
    }
}

/// Résultat d'une remise.
#[derive(Clone, Debug, Serialize)]
pub struct RemiseFaite {
    pub remise: RemiseVue,
    /// En main propre : le PDF à imprimer (jamais stocké en clair). Vide pour une remise en ligne.
    #[serde(skip)]
    pub pdf_a_imprimer: Option<Vec<u8>>,
}

/// Un fichier déchiffré, servi au demandeur authentifié.
#[derive(Clone, Debug)]
pub struct FichierServi {
    pub nom: String,
    pub type_: String,
    pub octets: Vec<u8>,
}

/// Un refus partiel ou total consigné à la clôture.
#[derive(Clone, Debug, Serialize)]
pub struct RenvoiVue {
    pub id: Uuid,
    pub nature: String,
    pub cible: String,
    pub texte: String,
    pub cree_le: DateTime<Utc>,
}

fn infra() -> EntDbError {
    EntDbError::Domain(DomainError::Infrastructure)
}

fn non_trouve() -> EntDbError {
    EntDbError::Domain(DomainError::NotFound)
}

// ─────────────────────────────── Services ───────────────────────────────

impl PgEnt {
    /// Crée un défi de passkey **lié** à l'empreinte de l'extrait et à l'intention (5 minutes, usage unique).
    async fn defi_creer(
        &self,
        tx: &mut PgConnection,
        etab: Uuid,
        compte: Uuid,
        extrait: Uuid,
        usage: &str,
        empreinte: &str,
        intention: &str,
        maintenant: DateTime<Utc>,
    ) -> Result<DefiRemise, EntDbError> {
        let passkeys = PgPasskeyRepo::new(self.pool().clone())
            .list_passkeys(compte)
            .await
            .map_err(|_| infra())?;
        if passkeys.is_empty() {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "aucune passkey enregistrée : enregistrez-en une, ou choisissez la remise en main propre",
            ));
        }
        let (rcr, etat) = self.passkey_engine()?.start_authentication(&passkeys)?;
        let options = serde_json::to_value(&rcr).map_err(|_| infra())?;
        let etat_json = serde_json::to_string(&etat).map_err(|_| infra())?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO rgp_defi (id, etablissement_id, extrait_id, usage, compte_id, empreinte, intention,
                                   etat_webauthn, expire_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(extrait)
        .bind(usage)
        .bind(compte)
        .bind(empreinte)
        .bind(intention)
        .bind(&etat_json)
        .bind(maintenant + Duration::minutes(DEFI_MIN))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        Ok(DefiRemise {
            defi_id: id,
            options,
            intention: intention.to_string(),
            empreinte: empreinte.to_string(),
        })
    }

    /// Vérifie l'assertion d'un défi : **brûlé avant** toute vérification (un rejeu échoue toujours), lié au compte,
    /// à l'extrait et à l'usage. Transaction propre, validée quoi qu'il arrive ensuite.
    async fn defi_consommer(
        &self,
        etab: Uuid,
        compte: Uuid,
        extrait: Uuid,
        usage: &str,
        defi: Uuid,
        credential: &PublicKeyCredential,
        maintenant: DateTime<Utc>,
    ) -> Result<(), EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let d = sqlx::query(
            "SELECT etat_webauthn, consomme_le, expire_le, compte_id, extrait_id, usage
               FROM rgp_defi WHERE etablissement_id = $1 AND id = $2 FOR UPDATE",
        )
        .bind(etab)
        .bind(defi)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(non_trouve)?;
        let (d_compte, d_extrait, d_usage): (Uuid, Uuid, String) =
            (d.get("compte_id"), d.get("extrait_id"), d.get("usage"));
        if d_compte != compte || d_extrait != extrait || d_usage != usage {
            return Err(non_trouve());
        }
        let consomme: Option<DateTime<Utc>> = d.get("consomme_le");
        let expire: DateTime<Utc> = d.get("expire_le");
        if consomme.is_some() || expire <= maintenant {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "défi expiré ou déjà utilisé : recommencez",
            ));
        }
        sqlx::query("UPDATE rgp_defi SET consomme_le = $3 WHERE etablissement_id = $1 AND id = $2")
            .bind(etab)
            .bind(defi)
            .bind(maintenant)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let etat_txt: String = d.get("etat_webauthn");
        tx.commit().await?;
        let etat: ent_auth::PasskeyAuthentication = serde_json::from_str(&etat_txt)
            .map_err(|_| EntDbError::Passkey(ent_auth::PasskeyError::BadState))?;
        let resultat = self
            .passkey_engine()?
            .finish_authentication(credential, &etat)?;
        match PgPasskeyRepo::new(self.pool().clone())
            .apply_authentication(&resultat)
            .await
            .map_err(|_| infra())?
        {
            CounterOutcome::Regressed { .. } => Err(EntDbError::Passkey(
                ent_auth::PasskeyError::PossibleCompromise,
            )),
            CounterOutcome::Unknown => Err(EntDbError::Passkey(ent_auth::PasskeyError::AuthFinish)),
            CounterOutcome::Updated | CounterOutcome::Unchanged => Ok(()),
        }
    }

    /// Démarre la remise : défi de passkey du **gestionnaire** (`droits.remettre`), lié à l'empreinte de l'extrait
    /// et à l'intention « remettre » (R-RG42). L'extrait doit être remissible : sinon 409 dès maintenant.
    pub async fn droits_remise_defi(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
    ) -> Result<DefiRemise, EntDbError> {
        self.droits_remise_defi_a(etab, acteur, role, demande, extrait, Utc::now())
            .await
    }

    pub async fn droits_remise_defi_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<DefiRemise, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsRemettre)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (vue, _contenu, _dem) = self
            .contenu_a_remettre(&mut tx, etab, demande, extrait)
            .await?;
        deja_remis(&mut tx, etab, extrait).await?;
        let intention = "Je remets à la personne concernée l'extrait de ses données, dont j'ai relu le contenu.";
        let defi = self
            .defi_creer(
                &mut tx,
                etab,
                acteur,
                extrait,
                "remettre",
                &vue.empreinte,
                intention,
                maintenant,
            )
            .await?;
        tx.commit().await?;
        Ok(defi)
    }

    /// **Remet** l'extrait (RGP-154). `droits.remettre` + passkey récente du gestionnaire. En ligne : sur le compte du
    /// demandeur **dans cet établissement** (jamais une adresse de la demande) ; fichiers scellés, notification sans
    /// nom d'élève. En main propre : émargement obligatoire, PDF rendu une fois pour l'impression.
    pub async fn droits_remettre(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
        mode: ModeRemise,
        defi: Uuid,
        credential: &PublicKeyCredential,
        emargement: Option<Emargement>,
    ) -> Result<RemiseFaite, EntDbError> {
        self.droits_remettre_a(
            etab,
            acteur,
            role,
            demande,
            extrait,
            mode,
            defi,
            credential,
            emargement,
            Utc::now(),
        )
        .await
    }

    pub async fn droits_remettre_a(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        extrait: Uuid,
        mode: ModeRemise,
        defi: Uuid,
        credential: &PublicKeyCredential,
        emargement: Option<Emargement>,
        maintenant: DateTime<Utc>,
    ) -> Result<RemiseFaite, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsRemettre)
            .await?;
        let emarge = match (mode, emargement) {
            (ModeRemise::MainPropre, Some(e)) => {
                let nom = texte(&e.nom, 2, 200)
                    .ok_or(invalide("nom de la personne qui émarge obligatoire"))?;
                let piece = texte(&e.piece_vue, 2, 100).ok_or(invalide("pièce vue obligatoire"))?;
                Some((nom, piece))
            }
            (ModeRemise::MainPropre, None) => {
                return Err(invalide("émargement obligatoire en main propre"))
            }
            (ModeRemise::EnLigne, _) => None,
        };
        // Le défi est brûlé d'abord : sans passkey récente du gestionnaire, rien n'est remis.
        self.defi_consommer(
            etab, acteur, extrait, "remettre", defi, credential, maintenant,
        )
        .await?;
        let jours = self.droits_reglage_extrait_jours(etab).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (vue, contenu, demandeur) = self
            .contenu_a_remettre(&mut tx, etab, demande, extrait)
            .await?;
        deja_remis(&mut tx, etab, extrait).await?;
        let r = sqlx::query(
            "SELECT annee, numero FROM rgp_demande WHERE etablissement_id = $1 AND id = $2",
        )
        .bind(etab)
        .bind(demande)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        let reference = crate::droits_demandes::reference_de(r.get("annee"), r.get("numero"));
        let destinataire = match mode {
            ModeRemise::EnLigne => {
                let c = demandeur.ok_or(EntDbError::Refus(
                    RefusMetier::Conflit,
                    "le demandeur n'a pas de compte : remise en main propre",
                ))?;
                let membre: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_membership WHERE etablissement_id = $1 AND user_id = $2)",
                )
                .bind(etab)
                .bind(c)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if !membre {
                    return Err(EntDbError::Refus(
                        RefusMetier::Conflit,
                        "le demandeur n'a pas de compte dans cet établissement : remise en main propre",
                    ));
                }
                Some(c)
            }
            ModeRemise::MainPropre => None,
        };
        let rendus = fichiers_de(&contenu, &reference, vue.version, maintenant);
        let remise_id = Uuid::new_v4();
        let mut fichiers = Vec::new();
        let mut pdf = None;
        match mode {
            ModeRemise::EnLigne => {
                let boite = self.secrets()?.derive(USAGE_SCELLE);
                let dispo = maintenant + Duration::days(i64::from(jours));
                for f in &rendus {
                    let aad = aad_fichier(etab, extrait, vue.version, &f.nom);
                    let scelle = boite
                        .seal(&base64_encoder(&f.octets), &aad)
                        .map_err(|_| infra())?;
                    let id = Uuid::new_v4();
                    let cle = ent_object::ObjectStore::key_for(etab, "rgpd-extraits", id, "bin");
                    self.objet_put(&cle, scelle.as_bytes()).await?;
                    sqlx::query(
                        "INSERT INTO rgp_extrait_fichier
                            (id, etablissement_id, extrait_id, nom, type, cle_objet, octets, empreinte,
                             empreinte_scelle, disponible_jusqu_le)
                         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
                    )
                    .bind(id)
                    .bind(etab)
                    .bind(extrait)
                    .bind(&f.nom)
                    .bind(f.type_)
                    .bind(&cle)
                    .bind(f.octets.len() as i32)
                    .bind(sha(&f.octets))
                    .bind(sha(scelle.as_bytes()))
                    .bind(dispo)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    fichiers.push(FichierVue {
                        id,
                        nom: f.nom.clone(),
                        type_: f.type_.to_string(),
                        octets: f.octets.len() as i32,
                        empreinte: sha(&f.octets),
                        disponible_jusqu_le: dispo,
                        purge: false,
                        telechargements: 0,
                    });
                }
            }
            ModeRemise::MainPropre => {
                pdf = rendus
                    .into_iter()
                    .find(|f| f.type_ == "pdf")
                    .map(|f| f.octets);
            }
        }
        sqlx::query(
            "INSERT INTO rgp_remise
                (id, etablissement_id, demande_id, extrait_id, mode, remis_par_compte_id, remis_le,
                 destinataire_compte_id, empreinte_extrait, emargement_nom, emargement_piece_vue, emargement_le)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(remise_id)
        .bind(etab)
        .bind(demande)
        .bind(extrait)
        .bind(mode.code())
        .bind(acteur)
        .bind(maintenant)
        .bind(destinataire)
        .bind(&vue.empreinte)
        .bind(emarge.as_ref().map(|e| e.0.clone()))
        .bind(emarge.as_ref().map(|e| e.1.clone()))
        .bind(emarge.as_ref().map(|_| maintenant))
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "extrait_remis",
            Some(match mode {
                ModeRemise::EnLigne => "extrait remis en ligne (passkey récente du gestionnaire)",
                ModeRemise::MainPropre => "extrait remis en main propre (émargement)",
            }),
            Some(acteur),
            Some(role),
        )
        .await?;
        if let Some(c) = destinataire {
            notifier_personne(
                &mut tx,
                EnvoiAPlanifier {
                    etablissement_id: etab,
                    user_id: c,
                    kind: KIND_EXTRAIT_PRET,
                    titre: "Votre extrait est prêt",
                    corps: &format!(
                        "L'extrait de vos données (demande {reference}) est disponible dans « Mes données » \
                         pendant {jours} jours. Le téléchargement demande votre passkey."
                    ),
                    eleve_id: None,
                    notification_id: None,
                    source_type: "systeme",
                    source_id: remise_id,
                    cle_dedup: format!("rgpd:extrait:{extrait}:pret"),
                    droit_requis: None,
                    regroupable: false,
                    canaux_voulus: &["email", "push"],
                },
                remise_id,
                maintenant,
            )
            .await?;
        }
        tx.commit().await?;
        Ok(RemiseFaite {
            remise: RemiseVue {
                id: remise_id,
                extrait_id: extrait,
                mode: mode.code().to_string(),
                remis_le: maintenant,
                empreinte_extrait: vue.empreinte,
                fichiers,
            },
            pdf_a_imprimer: pdf,
        })
    }

    async fn droits_reglage_extrait_jours(&self, etab: Uuid) -> Result<i32, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let v: Option<i32> = sqlx::query_scalar(
            "SELECT extrait_disponible_jours FROM rgp_reglage WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        Ok(v.unwrap_or(7))
    }

    /// « Mes données » : les extraits remis **en ligne** au compte connecté, dans cet établissement.
    pub async fn droits_mes_extraits(
        &self,
        etab: Uuid,
        compte: Uuid,
    ) -> Result<Vec<MonExtrait>, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows = sqlx::query(
            "SELECT r.id, r.extrait_id, r.mode, r.remis_le, r.empreinte_extrait, d.annee, d.numero
               FROM rgp_remise r JOIN rgp_demande d ON d.etablissement_id = r.etablissement_id AND d.id = r.demande_id
              WHERE r.etablissement_id = $1 AND r.destinataire_compte_id = $2
              ORDER BY r.remis_le DESC",
        )
        .bind(etab)
        .bind(compte)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        for r in rows {
            let extrait: Uuid = r.get("extrait_id");
            let fichiers = fichiers_de_extrait(&mut tx, etab, extrait).await?;
            out.push(MonExtrait {
                demande_reference: crate::droits_demandes::reference_de(
                    r.get("annee"),
                    r.get("numero"),
                ),
                remise: RemiseVue {
                    id: r.get("id"),
                    extrait_id: extrait,
                    mode: r.get("mode"),
                    remis_le: r.get("remis_le"),
                    empreinte_extrait: r.get("empreinte_extrait"),
                    fichiers,
                },
            });
        }
        Ok(out)
    }

    /// Défi de passkey du **demandeur** pour télécharger : lié à l'extrait. Un autre compte reçoit 404.
    pub async fn droits_telechargement_defi(
        &self,
        etab: Uuid,
        compte: Uuid,
        extrait: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<DefiRemise, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let empreinte: String = sqlx::query_scalar(
            "SELECT empreinte_extrait FROM rgp_remise
              WHERE etablissement_id = $1 AND extrait_id = $2 AND destinataire_compte_id = $3",
        )
        .bind(etab)
        .bind(extrait)
        .bind(compte)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(non_trouve)?;
        let intention = "Je télécharge l'extrait des données personnelles qui me concernent.";
        let defi = self
            .defi_creer(
                &mut tx,
                etab,
                compte,
                extrait,
                "telecharger",
                &empreinte,
                intention,
                maintenant,
            )
            .await?;
        tx.commit().await?;
        Ok(defi)
    }

    /// **Télécharge** un fichier : passkey récente du demandeur, objet relu, empreinte du scellé vérifiée, déchiffré
    /// **seulement ici**, téléchargement journalisé et compté. Fichier purgé ou échu : 410 logique (Conflit).
    pub async fn droits_telecharger(
        &self,
        etab: Uuid,
        compte: Uuid,
        fichier: Uuid,
        defi: Uuid,
        credential: &PublicKeyCredential,
        maintenant: DateTime<Utc>,
    ) -> Result<FichierServi, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let f = sqlx::query(
            "SELECT f.extrait_id, f.nom, f.type, f.cle_objet, f.empreinte, f.empreinte_scelle, f.purge_le,
                    f.disponible_jusqu_le, e.version, r.demande_id
               FROM rgp_extrait_fichier f
               JOIN rgp_remise r ON r.etablissement_id = f.etablissement_id AND r.extrait_id = f.extrait_id
               JOIN rgp_extrait e ON e.etablissement_id = f.etablissement_id AND e.id = f.extrait_id
              WHERE f.etablissement_id = $1 AND f.id = $2 AND r.destinataire_compte_id = $3",
        )
        .bind(etab)
        .bind(fichier)
        .bind(compte)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        .ok_or_else(non_trouve)?;
        tx.commit().await?;
        let extrait: Uuid = f.get("extrait_id");
        let purge: Option<DateTime<Utc>> = f.get("purge_le");
        let echeance: DateTime<Utc> = f.get("disponible_jusqu_le");
        if purge.is_some() || echeance <= maintenant {
            return Err(EntDbError::Refus(
                RefusMetier::Conflit,
                "mise à disposition terminée : demandez un nouvel extrait",
            ));
        }
        self.defi_consommer(
            etab,
            compte,
            extrait,
            "telecharger",
            defi,
            credential,
            maintenant,
        )
        .await?;
        let cle: String = f.get("cle_objet");
        let scelle = self.objet_get(&cle).await?;
        if sha(&scelle) != f.get::<String, _>("empreinte_scelle") {
            tracing::error!("extrait RGPD : empreinte du scellé différente de celle enregistrée");
            return Err(infra());
        }
        let nom: String = f.get("nom");
        let version: i32 = f.get("version");
        let aad = aad_fichier(etab, extrait, version, &nom);
        let texte_b64 = self
            .secrets()?
            .derive(USAGE_SCELLE)
            .open(&String::from_utf8(scelle).map_err(|_| infra())?, &aad)
            .map_err(|_| infra())?;
        let octets = base64_decoder(&texte_b64).ok_or_else(infra)?;
        if sha(&octets) != f.get::<String, _>("empreinte") {
            return Err(infra());
        }
        let demande: Uuid = f.get("demande_id");
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        sqlx::query(
            "INSERT INTO rgp_telechargement (id, etablissement_id, fichier_id, compte_id, le)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(fichier)
        .bind(compte)
        .bind(maintenant)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "extrait_telecharge",
            Some(&nom),
            Some(compte),
            Some("demandeur"),
        )
        .await?;
        tx.commit().await?;
        Ok(FichierServi {
            nom,
            type_: f.get("type"),
            octets,
        })
    }

    /// Tâche horaire : supprime les objets des fichiers dont la mise à disposition est échue (`maintenant` injecté).
    /// « Purgé » n'est écrit qu'après **confirmation** que l'objet n'existe plus ; un échec est rejoué au tour suivant.
    pub async fn droits_extraits_purger_a(
        &self,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        let etabs: Vec<(Uuid,)> = crate::tenant_tx::lire_par_etablissement(
            self.pool(),
            crate::tenant_tx::ContexteSysteme::JobPlanifie("rgpd_extraits_purge"),
            "SELECT DISTINCT etablissement_id FROM rgp_extrait_fichier WHERE purge_le IS NULL",
        )
        .await
        .map_err(db)?;
        let mut n = 0;
        for (etab,) in etabs {
            n += self.droits_extraits_purger_etab_a(etab, maintenant).await?;
        }
        Ok(n)
    }

    pub async fn droits_extraits_purger_tache(&self) -> Result<usize, EntDbError> {
        self.droits_extraits_purger_a(Utc::now()).await
    }

    pub async fn droits_extraits_purger_etab_a(
        &self,
        etab: Uuid,
        maintenant: DateTime<Utc>,
    ) -> Result<usize, EntDbError> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows = sqlx::query(
            "SELECT f.id, f.cle_objet, f.extrait_id, r.demande_id
               FROM rgp_extrait_fichier f
               JOIN rgp_remise r ON r.etablissement_id = f.etablissement_id AND r.extrait_id = f.extrait_id
              WHERE f.etablissement_id = $1 AND f.purge_le IS NULL AND f.disponible_jusqu_le <= $2
              ORDER BY f.id FOR UPDATE OF f SKIP LOCKED",
        )
        .bind(etab)
        .bind(maintenant)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut n = 0;
        let mut journalises: Vec<Uuid> = Vec::new();
        for r in rows {
            let (id, cle, extrait, demande): (Uuid, String, Uuid, Uuid) = (
                r.get("id"),
                r.get("cle_objet"),
                r.get("extrait_id"),
                r.get("demande_id"),
            );
            if let Err(e) = self.objet_supprimer(&cle).await {
                tracing::warn!("purge d'un extrait RGPD : suppression non confirmée ({e}), rejouée au prochain tour");
                continue;
            }
            // Confirmation : l'objet ne doit plus exister avant d'écrire « purgé ».
            if self.objet_existe(&cle).await.unwrap_or(true) {
                tracing::warn!(
                    "purge d'un extrait RGPD : l'objet existe encore, rejouée au prochain tour"
                );
                continue;
            }
            sqlx::query("UPDATE rgp_extrait_fichier SET purge_le = $3 WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .bind(maintenant)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            n += 1;
            if !journalises.contains(&extrait) {
                journalises.push(extrait);
                journal(
                    &mut tx,
                    etab,
                    demande,
                    "extrait_purge",
                    Some("mise à disposition terminée : fichiers de l'extrait supprimés"),
                    None,
                    None,
                )
                .await?;
            }
        }
        tx.commit().await?;
        Ok(n)
    }

    // ───────────────────────── Clôture et renvois ─────────────────────────

    /// **Clôture** (RGP-166) : la clôture de G3a (`droits_clore`) puis, dans la foulée, l'enregistrement des
    /// **éléments remis par empreinte** (jamais les données) et des refus partiels motivés. Les renvois tracés
    /// avant la clôture (`droits_renvoi_tracer`) font partie du dossier.
    pub async fn droits_cloturer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        reponse_resume: &str,
    ) -> Result<crate::droits_demandes::DemandeVue, EntDbError> {
        let vue = self
            .droits_clore(etab, acteur, role, demande, reponse_resume)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let remises = sqlx::query(
            "SELECT r.extrait_id, r.mode, r.remis_le, r.empreinte_extrait, e.version, e.sujet_type, e.sujet_id
               FROM rgp_remise r JOIN rgp_extrait e ON e.etablissement_id = r.etablissement_id AND e.id = r.extrait_id
              WHERE r.etablissement_id = $1 AND r.demande_id = $2 ORDER BY r.remis_le, r.id",
        )
        .bind(etab)
        .bind(demande)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let cloture = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO rgp_cloture (id, etablissement_id, demande_id, cloture_par_compte_id)
             VALUES ($1,$2,$3,$4)",
        )
        .bind(cloture)
        .bind(etab)
        .bind(demande)
        .bind(acteur)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        for (rang, r) in remises.iter().enumerate() {
            let extrait: Uuid = r.get("extrait_id");
            let element = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO rgp_cloture_element
                   (id, etablissement_id, cloture_id, rang, extrait_id, version, sujet_type, mode, remis_le, empreinte)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            )
            .bind(element)
            .bind(etab)
            .bind(cloture)
            .bind(rang as i32)
            .bind(extrait)
            .bind(r.get::<i32, _>("version"))
            .bind(r.get::<String, _>("sujet_type"))
            .bind(r.get::<String, _>("mode"))
            .bind(r.get::<DateTime<Utc>, _>("remis_le"))
            .bind(r.get::<String, _>("empreinte_extrait"))
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "INSERT INTO rgp_cloture_section (id, etablissement_id, element_id, fournisseur_code, statut, empreinte)
                 SELECT gen_random_uuid(), etablissement_id, $3, fournisseur_code, statut, empreinte
                   FROM rgp_extrait_section WHERE etablissement_id = $1 AND extrait_id = $2",
            )
            .bind(etab)
            .bind(extrait)
            .bind(element)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        sqlx::query(
            "INSERT INTO rgp_cloture_refus (id, etablissement_id, cloture_id, droit, decision, motif_code, motif)
             SELECT gen_random_uuid(), etablissement_id, $3, droit, decision, motif_code, motif_texte
               FROM rgp_demande_droit
              WHERE etablissement_id = $1 AND demande_id = $2 AND decision IN ('refuse', 'partiel')",
        )
        .bind(etab)
        .bind(demande)
        .bind(cloture)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await?;
        Ok(vue)
    }

    /// Les éléments et refus consignés par la dernière clôture : `(éléments, refus)` en JSON.
    pub async fn droits_cloture_lire(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
    ) -> Result<Option<(serde_json::Value, serde_json::Value)>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let Some(c) = sqlx::query(
            "SELECT id FROM rgp_cloture WHERE etablissement_id = $1 AND demande_id = $2
              ORDER BY cloture_le DESC, id LIMIT 1",
        )
        .bind(etab)
        .bind(demande)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?
        else {
            return Ok(None);
        };
        let cloture: Uuid = c.get("id");
        let elements = sqlx::query(
            "SELECT id, extrait_id, version, sujet_type, mode, remis_le, empreinte FROM rgp_cloture_element
              WHERE etablissement_id = $1 AND cloture_id = $2 ORDER BY rang",
        )
        .bind(etab)
        .bind(cloture)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let mut elems = Vec::new();
        for e in elements {
            let sections = sqlx::query(
                "SELECT fournisseur_code, statut, empreinte FROM rgp_cloture_section
                  WHERE etablissement_id = $1 AND element_id = $2 ORDER BY fournisseur_code",
            )
            .bind(etab)
            .bind(e.get::<Uuid, _>("id"))
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let sections: Vec<serde_json::Value> = sections
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "fournisseur": s.get::<String, _>("fournisseur_code"),
                        "statut": s.get::<String, _>("statut"),
                        "empreinte": s.get::<Option<String>, _>("empreinte"),
                    })
                })
                .collect();
            elems.push(serde_json::json!({
                "extrait_id": e.get::<Uuid, _>("extrait_id"),
                "version": e.get::<i32, _>("version"),
                "sujet_type": e.get::<String, _>("sujet_type"),
                "mode": e.get::<String, _>("mode"),
                "remis_le": e.get::<DateTime<Utc>, _>("remis_le"),
                "empreinte": e.get::<String, _>("empreinte"),
                "sections": sections,
            }));
        }
        let refus = sqlx::query(
            "SELECT droit, decision, motif_code, motif FROM rgp_cloture_refus
              WHERE etablissement_id = $1 AND cloture_id = $2 ORDER BY droit",
        )
        .bind(etab)
        .bind(cloture)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        let refus: Vec<serde_json::Value> = refus
            .iter()
            .map(|r| {
                serde_json::json!({
                    "droit": r.get::<String, _>("droit"),
                    "decision": r.get::<String, _>("decision"),
                    "motif_code": r.get::<Option<String>, _>("motif_code"),
                    "motif": r.get::<Option<String>, _>("motif"),
                })
            })
            .collect();
        Ok(Some((serde_json::Value::Array(elems), serde_json::Value::Array(refus))))
    }

    /// **Renvoi tracé** (RGP-156, RGP-159, RGP-316) : rectification vers le socle propriétaire, retrait de
    /// consentement relié au socle 18, CADA d'un tiers (document agrégé ou occulté). `droits.traiter`.
    pub async fn droits_renvoi_tracer(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
        nature: &str,
        cible: &str,
        texte_libre: &str,
    ) -> Result<RenvoiVue, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        if !matches!(
            nature,
            "rectification" | "retrait_consentement" | "cada_tiers"
        ) {
            return Err(invalide("nature de renvoi inconnue"));
        }
        let cible = if nature == "retrait_consentement" && cible.trim().is_empty() {
            "socle-18".to_string()
        } else {
            texte(cible, 2, 100).ok_or(invalide("cible du renvoi obligatoire"))?
        };
        let mut corps =
            texte(texte_libre, 3, 1900).ok_or(invalide("texte du renvoi obligatoire"))?;
        // SIECLE reste la source : l'établissement ne peut rien y corriger lui-même.
        if nature == "rectification" && cible.eq_ignore_ascii_case("siecle") {
            corps.push_str(" — à corriger dans la source (SIECLE)");
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        crate::droits_demandes::etat_courant(&mut tx, etab, demande).await?;
        let id = Uuid::new_v4();
        let cree: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO rgp_redirection (id, etablissement_id, demande_id, nature, cible, texte, cree_par_compte_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING cree_le",
        )
        .bind(id)
        .bind(etab)
        .bind(demande)
        .bind(nature)
        .bind(&cible)
        .bind(&corps)
        .bind(acteur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        journal(
            &mut tx,
            etab,
            demande,
            "renvoi_trace",
            Some(nature),
            Some(acteur),
            Some(role),
        )
        .await?;
        tx.commit().await?;
        Ok(RenvoiVue {
            id,
            nature: nature.to_string(),
            cible,
            texte: corps,
            cree_le: cree,
        })
    }

    pub async fn droits_renvois(
        &self,
        etab: Uuid,
        acteur: Uuid,
        role: &str,
        demande: Uuid,
    ) -> Result<Vec<RenvoiVue>, EntDbError> {
        self.exiger_action_etab(etab, acteur, role, Action::DroitsTraiter)
            .await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let rows = sqlx::query(
            "SELECT id, nature, cible, texte, cree_le FROM rgp_redirection
              WHERE etablissement_id = $1 AND demande_id = $2 ORDER BY cree_le, id",
        )
        .bind(etab)
        .bind(demande)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        Ok(rows
            .iter()
            .map(|r| RenvoiVue {
                id: r.get("id"),
                nature: r.get("nature"),
                cible: r.get("cible"),
                texte: r.get("texte"),
                cree_le: r.get("cree_le"),
            })
            .collect())
    }
}

async fn deja_remis(conn: &mut PgConnection, etab: Uuid, extrait: Uuid) -> Result<(), EntDbError> {
    let existe: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM rgp_remise WHERE etablissement_id = $1 AND extrait_id = $2)",
    )
    .bind(etab)
    .bind(extrait)
    .fetch_one(&mut *conn)
    .await
    .map_err(db)?;
    if existe {
        return Err(EntDbError::ConflitDossier("cet extrait a déjà été remis"));
    }
    Ok(())
}

async fn fichiers_de_extrait(
    conn: &mut PgConnection,
    etab: Uuid,
    extrait: Uuid,
) -> Result<Vec<FichierVue>, EntDbError> {
    let rows = sqlx::query(
        "SELECT f.id, f.nom, f.type, f.octets, f.empreinte, f.disponible_jusqu_le, f.purge_le,
                (SELECT count(*) FROM rgp_telechargement t
                  WHERE t.etablissement_id = f.etablissement_id AND t.fichier_id = f.id) AS nb
           FROM rgp_extrait_fichier f WHERE f.etablissement_id = $1 AND f.extrait_id = $2 ORDER BY f.nom",
    )
    .bind(etab)
    .bind(extrait)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(rows
        .iter()
        .map(|r| FichierVue {
            id: r.get("id"),
            nom: r.get("nom"),
            type_: r.get("type"),
            octets: r.get("octets"),
            empreinte: r.get("empreinte"),
            disponible_jusqu_le: r.get("disponible_jusqu_le"),
            purge: r.get::<Option<DateTime<Utc>>, _>("purge_le").is_some(),
            telechargements: r.get("nb"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_aller_retour() {
        for n in 0..40usize {
            let d: Vec<u8> = (0..n).map(|i| (i * 7 + 3) as u8).collect();
            assert_eq!(
                base64_decoder(&base64_encoder(&d)).unwrap(),
                d,
                "longueur {n}"
            );
        }
        assert_eq!(base64_encoder(b"Man"), "TWFu");
        assert_eq!(base64_encoder(b"Ma"), "TWE=");
        assert!(base64_decoder("TWF").is_none());
        assert!(base64_decoder("T=Fu").is_none());
    }

    #[test]
    fn csv_echappement_rfc4180_et_formules() {
        assert_eq!(csv_cellule("simple"), "simple");
        assert_eq!(csv_cellule("a,b"), "\"a,b\"");
        assert_eq!(csv_cellule("dit \"oui\""), "\"dit \"\"oui\"\"\"");
        assert_eq!(csv_cellule("ligne1\nligne2"), "\"ligne1\nligne2\"");
        assert_eq!(csv_cellule("=1+1"), "'=1+1");
    }

    #[test]
    fn aad_distincte_par_extrait_version_et_fichier() {
        let (e, a, b) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        assert_ne!(
            aad_fichier(e, a, 1, "extrait.pdf"),
            aad_fichier(e, b, 1, "extrait.pdf")
        );
        assert_ne!(
            aad_fichier(e, a, 1, "extrait.pdf"),
            aad_fichier(e, a, 2, "extrait.pdf")
        );
        assert_ne!(
            aad_fichier(e, a, 1, "extrait.pdf"),
            aad_fichier(e, a, 1, "section-x.csv")
        );
    }
}
