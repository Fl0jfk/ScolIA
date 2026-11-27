//! Socle 5, lot S2 : profil de l'établissement, fuseau, préréglages, paramètres versionnés et
//! journal de structure (`05_STRUCTURE.md` §9.1, STR-01 à STR-04, STR-10).
//!
//! Règles du lot :
//! * **droits côté serveur** : lecture = `referentiel.lire`, écriture = `referentiel.gerer`
//!   (direction, administratif, administrateur ; l'action dédiée `etablissement.parametrer` du §13.2
//!   n'existe pas encore dans le catalogue, `referentiel.gerer` en est le raccourci prévu) ;
//! * **cloisonnement** : l'établissement vient de l'acteur, jamais d'un paramètre de requête ;
//!   les écritures passent par `TenantTx` (RLS) ;
//! * **aucune création implicite** : lire le profil ou les paramètres n'écrit rien ; les valeurs
//!   par défaut sont servies par le code, la première écriture crée la ligne ;
//! * **journal en ajout seul** (`ent_structure_journal`) pour toute modification, avec avant/après ;
//! * **fuseau** : source unique `ent_etablissement_parametre.fuseau` (migration 265) ; le module
//!   [`structure`] est le port que les autres socles lisent (`structure::fuseau`).

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use ent_auth::Action;
use ent_events::{AggregateKind, MetierEventDraft};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::outbox::Publier;
use crate::ref_commun::{conflit, db, introuvable, invalide, Acteur};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;

/// Fuseau par défaut d'un établissement sans réglage (métropole).
pub const FUSEAU_DEFAUT: &str = "Europe/Paris";

pub const SECTEURS: [&str; 3] = ["public", "prive_sous_contrat", "prive_hors_contrat"];
pub const CONTRATS: [&str; 2] = ["association", "simple"];
pub const DEGRES: [&str; 3] = ["premier", "second", "post_bac"];
const JOURS: [&str; 7] = ["lun", "mar", "mer", "jeu", "ven", "sam", "dim"];
const DEMI_JOURNEES: [&str; 2] = ["matin", "apres_midi"];

// ───────────────────────── Port lu par les autres socles ─────────────────────────

/// Port `structure::…` du §6.3 : fonctions de lecture prenant la connexion/transaction de
/// l'appelant. **Point de branchement du socle 6 (lot P0)** : remplacer le « Europe/Paris » en
/// dur par `structure::fuseau` / `structure::aujourdhui` / `structure::heure_locale`.
pub mod structure {
    use super::*;

    /// Fuseau IANA de l'établissement (`Europe/Paris` s'il n'a rien réglé). Lecture seule.
    pub async fn fuseau(conn: &mut PgConnection, etab: Uuid) -> Result<String, EntDbError> {
        let f: Option<String> = sqlx::query_scalar(
            "SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1",
        )
        .bind(etab)
        .fetch_optional(conn)
        .await
        .map_err(db)?;
        Ok(f.unwrap_or_else(|| FUSEAU_DEFAUT.to_string()))
    }

    /// Jour métier courant de l'établissement (calculé par la base, même source que `fuseau`).
    pub async fn aujourdhui(conn: &mut PgConnection, etab: Uuid) -> Result<NaiveDate, EntDbError> {
        sqlx::query_scalar("SELECT ent_aujourdhui_etab($1)")
            .bind(etab)
            .fetch_one(conn)
            .await
            .map_err(db)
    }

    /// Heure murale d'un instant dans le fuseau de l'établissement.
    pub async fn heure_locale(
        conn: &mut PgConnection,
        etab: Uuid,
        instant: DateTime<Utc>,
    ) -> Result<NaiveDateTime, EntDbError> {
        sqlx::query_scalar(
            "SELECT $2::timestamptz AT TIME ZONE
                    COALESCE((SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $1),
                             'Europe/Paris')",
        )
        .bind(etab)
        .bind(instant)
        .fetch_one(conn)
        .await
        .map_err(db)
    }

    /// Valeur effective d'un paramètre (défaut du registre si jamais modifié). Clé inconnue : erreur.
    pub async fn parametre(
        conn: &mut PgConnection,
        etab: Uuid,
        cle: &str,
    ) -> Result<Value, EntDbError> {
        let def = super::definition(cle).ok_or_else(invalide)?;
        let v: Option<Value> = sqlx::query_scalar(
            "SELECT ent_parametre_valeur_json(p) FROM ent_parametre p
              WHERE p.etablissement_id = $1 AND p.cle = $2 AND p.type_param IS NOT NULL",
        )
        .bind(etab)
        .bind(cle)
        .fetch_optional(conn)
        .await
        .map_err(db)?;
        Ok(v.unwrap_or_else(|| def.defaut.clone()))
    }
}

// ───────────────────────── Aides communes (journal, événements) ─────────────────────────

/// Écrit une ligne du journal de structure dans la transaction de l'écriture.
pub(crate) async fn journaliser(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    objet: &str,
    objet_ref: &str,
    action: &str,
    avant: Option<Value>,
    apres: Option<Value>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_structure_journal (id, etablissement_id, objet, objet_ref, action, avant, apres, par)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(Uuid::new_v4())
    .bind(a.etablissement_id)
    .bind(objet)
    .bind(objet_ref)
    .bind(action)
    .bind(avant)
    .bind(apres)
    .bind(a.user_id)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Publie un fait `referentiel.*` dans la transaction de l'écriture (même commit).
pub(crate) async fn publier(
    conn: &mut PgConnection,
    a: &Acteur<'_>,
    type_name: &str,
    aggregate_id: Uuid,
    payload: Value,
) -> Result<ent_events::MetierEvent, EntDbError> {
    Ok(conn
        .publier(MetierEventDraft {
            etablissement_id: a.etablissement_id,
            type_name: type_name.to_string(),
            aggregate: AggregateKind::Referentiel,
            aggregate_id,
            eleve_id: None,
            payload,
            actor_user_id: Some(a.user_id.to_string()),
        })
        .await?)
}

/// Texte facultatif : espaces retirés, vide = absent, trop long = refus.
pub(crate) fn net(v: Option<&str>, max: usize) -> Result<Option<String>, EntDbError> {
    match v.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) if s.chars().count() > max => Err(invalide()),
        Some(s) => Ok(Some(s.to_string())),
    }
}

/// Texte obligatoire de 1 à `max` caractères.
pub(crate) fn requis(v: &str, max: usize) -> Result<String, EntDbError> {
    net(Some(v), max)?.ok_or_else(invalide)
}

/// UAI : sept chiffres et une lettre majuscule (format seulement).
pub(crate) fn uai_valide(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 8 && b[..7].iter().all(u8::is_ascii_digit) && b[7].is_ascii_uppercase()
}

// ───────────────────────── Profil ─────────────────────────

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Profil {
    pub etablissement_id: Uuid,
    pub nom_officiel: String,
    pub nom_usage: Option<String>,
    pub logo_document_id: Option<Uuid>,
    pub uai: Option<String>,
    pub secteur: String,
    pub contrat_type: Option<String>,
    pub degres: Vec<String>,
    pub types: Vec<String>,
    pub academie_code: Option<String>,
    pub departement_code: Option<String>,
    pub zone_calendrier: Option<String>,
    pub regime_local: Option<String>,
    pub tutelle_libelle: Option<String>,
    pub adresse: Option<String>,
    pub telephone: Option<String>,
    pub courriel_contact: Option<String>,
    pub jours_ouvres: Value,
    pub profil_preregle: Option<String>,
    pub version: i32,
    pub modifie_par: Option<Uuid>,
    pub modifie_le: DateTime<Utc>,
    /// Fuseau IANA effectif (source : `ent_etablissement_parametre`, Europe/Paris par défaut).
    pub fuseau: String,
}

const COLONNES_PROFIL: &str = "p.etablissement_id, p.nom_officiel, p.nom_usage, p.logo_document_id, p.uai,
    p.secteur, p.contrat_type, p.degres, p.types, p.academie_code, p.departement_code, p.zone_calendrier,
    p.regime_local, p.tutelle_libelle, p.adresse, p.telephone, p.courriel_contact,
    ent_etablissement_jours_ouvres(p.etablissement_id) AS jours_ouvres,
    p.profil_preregle, p.version, p.modifie_par, p.modifie_le,
    COALESCE((SELECT f.fuseau FROM ent_etablissement_parametre f WHERE f.etablissement_id = p.etablissement_id),
             'Europe/Paris') AS fuseau";

/// Saisie complète du profil (remplacement). Le fuseau se règle à part ([`PgEnt::structure_fuseau_definir`]).
#[derive(Debug, Clone, Deserialize)]
pub struct ProfilSaisie {
    pub nom_officiel: String,
    pub nom_usage: Option<String>,
    pub logo_document_id: Option<Uuid>,
    pub uai: Option<String>,
    pub secteur: String,
    pub contrat_type: Option<String>,
    #[serde(default)]
    pub degres: Vec<String>,
    #[serde(default)]
    pub types: Vec<String>,
    pub academie_code: Option<String>,
    pub departement_code: Option<String>,
    pub zone_calendrier: Option<String>,
    pub regime_local: Option<String>,
    pub tutelle_libelle: Option<String>,
    pub adresse: Option<String>,
    pub telephone: Option<String>,
    pub courriel_contact: Option<String>,
    /// `{lun:[matin,apres_midi], mer:[matin], …}`. Absent : la semaine type actuelle est conservée.
    pub jours_ouvres: Option<Value>,
    /// Version lue par l'appelant ; si le profil a changé depuis, refus (conflit).
    pub version_attendue: Option<i32>,
}

/// Remplace la semaine type de l'établissement (table `ent_etablissement_jour_ouvre`, D-036).
/// `v` est validée par l'appelant ([`jours_ouvres_valides`]) ; le format est celui de l'API.
async fn ecrire_jours_ouvres(
    conn: &mut PgConnection,
    etab: Uuid,
    v: &Value,
) -> Result<(), EntDbError> {
    sqlx::query("DELETE FROM ent_etablissement_jour_ouvre WHERE etablissement_id = $1")
        .bind(etab)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    for jour in JOURS {
        let Some(dj) = v.get(jour).and_then(Value::as_array) else { continue };
        let a = |d: &str| dj.iter().any(|x| x.as_str() == Some(d));
        sqlx::query(
            "INSERT INTO ent_etablissement_jour_ouvre (id, etablissement_id, jour, matin, apres_midi)
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(jour)
        .bind(a("matin"))
        .bind(a("apres_midi"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

/// Semaine type valide : au moins un jour, jours connus, demi-journées connues et sans doublon.
pub fn jours_ouvres_valides(v: &Value) -> bool {
    let Some(o) = v.as_object() else { return false };
    if o.is_empty() {
        return false;
    }
    o.iter().all(|(jour, dj)| {
        JOURS.contains(&jour.as_str())
            && dj.as_array().is_some_and(|a| {
                !a.is_empty()
                    && a.len() <= 2
                    && a.iter()
                        .all(|x| x.as_str().is_some_and(|s| DEMI_JOURNEES.contains(&s)))
                    && (a.len() == 1 || a[0] != a[1])
            })
    })
}

fn liste_fermee(v: &[String], permis: &[&str], max: usize) -> Result<Vec<String>, EntDbError> {
    let mut out: Vec<String> = Vec::new();
    for x in v {
        let x = x.trim();
        if x.is_empty() || x.chars().count() > 40 || (!permis.is_empty() && !permis.contains(&x)) {
            return Err(invalide());
        }
        if !out.iter().any(|y| y == x) {
            out.push(x.to_string());
        }
    }
    if out.len() > max {
        return Err(invalide());
    }
    Ok(out)
}

struct ProfilValide {
    nom_officiel: String,
    nom_usage: Option<String>,
    uai: Option<String>,
    secteur: String,
    contrat_type: Option<String>,
    degres: Vec<String>,
    types: Vec<String>,
    academie_code: Option<String>,
    departement_code: Option<String>,
    zone_calendrier: Option<String>,
    regime_local: Option<String>,
    tutelle_libelle: Option<String>,
    adresse: Option<String>,
    telephone: Option<String>,
    courriel_contact: Option<String>,
}

fn valider_profil(s: &ProfilSaisie) -> Result<ProfilValide, EntDbError> {
    let secteur = s.secteur.trim().to_string();
    if !SECTEURS.contains(&secteur.as_str()) {
        return Err(invalide());
    }
    let contrat_type = net(s.contrat_type.as_deref(), 20)?;
    if let Some(c) = &contrat_type {
        // Un contrat n'existe que pour le privé sous contrat.
        if secteur != "prive_sous_contrat" || !CONTRATS.contains(&c.as_str()) {
            return Err(invalide());
        }
    }
    let uai = net(s.uai.as_deref(), 8)?.map(|u| u.to_ascii_uppercase());
    if uai.as_deref().is_some_and(|u| !uai_valide(u)) {
        return Err(invalide());
    }
    let departement_code = net(s.departement_code.as_deref(), 3)?.map(|d| d.to_ascii_uppercase());
    if departement_code.as_deref().is_some_and(|d| {
        d.len() < 2
            || !d
                .bytes()
                .all(|c| c.is_ascii_digit() || c == b'A' || c == b'B')
    }) {
        return Err(invalide());
    }
    let regime_local = net(s.regime_local.as_deref(), 20)?;
    if regime_local
        .as_deref()
        .is_some_and(|r| r != "alsace_moselle")
    {
        return Err(invalide());
    }
    Ok(ProfilValide {
        nom_officiel: requis(&s.nom_officiel, 200)?,
        nom_usage: net(s.nom_usage.as_deref(), 200)?,
        uai,
        secteur,
        contrat_type,
        degres: liste_fermee(&s.degres, &DEGRES, 3)?,
        types: liste_fermee(&s.types, &[], 12)?,
        academie_code: net(s.academie_code.as_deref(), 16)?,
        departement_code,
        zone_calendrier: net(s.zone_calendrier.as_deref(), 32)?,
        regime_local,
        tutelle_libelle: net(s.tutelle_libelle.as_deref(), 200)?,
        adresse: net(s.adresse.as_deref(), 400)?,
        telephone: net(s.telephone.as_deref(), 32)?,
        courriel_contact: net(s.courriel_contact.as_deref(), 254)?,
    })
}

fn json_profil(p: &Profil) -> Value {
    json!({
        "nom_officiel": p.nom_officiel, "nom_usage": p.nom_usage, "uai": p.uai, "secteur": p.secteur,
        "contrat_type": p.contrat_type, "degres": p.degres, "types": p.types,
        "academie_code": p.academie_code, "departement_code": p.departement_code,
        "zone_calendrier": p.zone_calendrier, "regime_local": p.regime_local,
        "tutelle_libelle": p.tutelle_libelle, "adresse": p.adresse, "telephone": p.telephone,
        "courriel_contact": p.courriel_contact, "jours_ouvres": p.jours_ouvres,
        "profil_preregle": p.profil_preregle, "logo_document_id": p.logo_document_id,
        "version": p.version,
    })
}

async fn lire_profil(
    conn: &mut PgConnection,
    etab: Uuid,
    verrou: bool,
) -> Result<Option<Profil>, EntDbError> {
    let sql = format!(
        "SELECT {COLONNES_PROFIL} FROM ent_etablissement_profil p WHERE p.etablissement_id = $1{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as::<_, Profil>(&sql)
        .bind(etab)
        .fetch_optional(conn)
        .await
        .map_err(db)
}

/// Une ligne de journal, telle que l'écran « historique » la lira.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct JournalLigne {
    pub id: Uuid,
    pub objet: String,
    pub objet_ref: String,
    pub action: String,
    pub avant: Option<Value>,
    pub apres: Option<Value>,
    pub par: Option<Uuid>,
    pub le: DateTime<Utc>,
}

impl PgEnt {
    /// Profil de l'établissement. `None` tant que personne ne l'a renseigné : **rien n'est créé**.
    pub async fn structure_profil(&self, a: &Acteur<'_>) -> Result<Option<Profil>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let p = lire_profil(&mut tx, a.etablissement_id, false).await?;
        tx.commit().await.map_err(db)?;
        Ok(p)
    }

    /// STR-01 / STR-03 : renseigne (ou remplace) le profil. Crée la ligne à la première saisie.
    pub async fn structure_profil_definir(
        &self,
        a: &Acteur<'_>,
        s: ProfilSaisie,
    ) -> Result<Profil, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let v = valider_profil(&s)?;
        if let Some(j) = &s.jours_ouvres {
            if !jours_ouvres_valides(j) {
                return Err(invalide());
            }
        }
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let avant = lire_profil(&mut tx, a.etablissement_id, true).await?;
        if let (Some(att), Some(p)) = (s.version_attendue, &avant) {
            if att != p.version {
                return Err(conflit());
            }
        }
        sqlx::query(
            "INSERT INTO ent_etablissement_profil
                (etablissement_id, nom_officiel, nom_usage, logo_document_id, uai, secteur, contrat_type,
                 degres, types, academie_code, departement_code, zone_calendrier, regime_local,
                 tutelle_libelle, adresse, telephone, courriel_contact, modifie_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)
             ON CONFLICT (etablissement_id) DO UPDATE SET
                nom_officiel = EXCLUDED.nom_officiel, nom_usage = EXCLUDED.nom_usage,
                logo_document_id = EXCLUDED.logo_document_id, uai = EXCLUDED.uai,
                secteur = EXCLUDED.secteur, contrat_type = EXCLUDED.contrat_type,
                degres = EXCLUDED.degres, types = EXCLUDED.types, academie_code = EXCLUDED.academie_code,
                departement_code = EXCLUDED.departement_code, zone_calendrier = EXCLUDED.zone_calendrier,
                regime_local = EXCLUDED.regime_local, tutelle_libelle = EXCLUDED.tutelle_libelle,
                adresse = EXCLUDED.adresse, telephone = EXCLUDED.telephone,
                courriel_contact = EXCLUDED.courriel_contact,
                version = ent_etablissement_profil.version + 1,
                modifie_par = EXCLUDED.modifie_par, modifie_le = now()",
        )
        .bind(a.etablissement_id)
        .bind(&v.nom_officiel)
        .bind(&v.nom_usage)
        .bind(s.logo_document_id)
        .bind(&v.uai)
        .bind(&v.secteur)
        .bind(&v.contrat_type)
        .bind(&v.degres)
        .bind(&v.types)
        .bind(&v.academie_code)
        .bind(&v.departement_code)
        .bind(&v.zone_calendrier)
        .bind(&v.regime_local)
        .bind(&v.tutelle_libelle)
        .bind(&v.adresse)
        .bind(&v.telephone)
        .bind(&v.courriel_contact)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Semaine type (D-036 : table typée) : remplacée si fournie, sinon posée par défaut à la création.
        match &s.jours_ouvres {
            Some(j) => ecrire_jours_ouvres(&mut tx, a.etablissement_id, j).await?,
            None => {
                let existe: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM ent_etablissement_jour_ouvre WHERE etablissement_id = $1)",
                )
                .bind(a.etablissement_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
                if !existe {
                    let defaut: Value = sqlx::query_scalar("SELECT ent_etablissement_jours_ouvres($1)")
                        .bind(a.etablissement_id)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(db)?;
                    ecrire_jours_ouvres(&mut tx, a.etablissement_id, &defaut).await?;
                }
            }
        }
        let apres = lire_profil(&mut tx, a.etablissement_id, false)
            .await?
            .ok_or_else(introuvable)?;
        journaliser(
            &mut tx,
            a,
            "profil",
            &a.etablissement_id.to_string(),
            if avant.is_some() { "modifie" } else { "cree" },
            avant.as_ref().map(json_profil),
            Some(json_profil(&apres)),
        )
        .await?;
        // L'événement ne porte que la version : les consommateurs relisent le profil.
        let ev = publier(
            &mut tx,
            a,
            "referentiel.profil_modifie",
            a.etablissement_id,
            json!({ "version": apres.version }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(apres)
    }

    /// STR-03 : règle le fuseau horaire IANA de l'établissement. Changer de fuseau pendant une
    /// année courante déplace les bords de journée : refus tant que l'appelant ne le confirme pas.
    pub async fn structure_fuseau_definir(
        &self,
        a: &Acteur<'_>,
        fuseau: &str,
        confirmer_en_cours_annee: bool,
    ) -> Result<String, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let fuseau = requis(fuseau, 64)?;
        // PostgreSQL est la référence des noms IANA : un nom inconnu lève une erreur.
        sqlx::query("SELECT timezone($1, TIMESTAMPTZ '2000-01-01 00:00:00+00')")
            .bind(&fuseau)
            .fetch_one(&mut *TenantTx::begin(self.pool(), a.etablissement_id).await?)
            .await
            .map_err(|_| invalide())?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let actuel = structure::fuseau(&mut tx, a.etablissement_id).await?;
        if actuel == fuseau {
            tx.commit().await.map_err(db)?;
            return Ok(fuseau);
        }
        let en_cours: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ent_annee_scolaire WHERE etablissement_id = $1 AND courante)",
        )
        .bind(a.etablissement_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if en_cours && !confirmer_en_cours_annee {
            return Err(conflit());
        }
        sqlx::query(
            "INSERT INTO ent_etablissement_parametre (etablissement_id, fuseau, updated_by)
             VALUES ($1,$2,$3)
             ON CONFLICT (etablissement_id) DO UPDATE SET fuseau = EXCLUDED.fuseau,
                    updated_by = EXCLUDED.updated_by, updated_at = now()",
        )
        .bind(a.etablissement_id)
        .bind(&fuseau)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            a,
            "fuseau",
            &a.etablissement_id.to_string(),
            "modifie",
            Some(json!({ "fuseau": actuel })),
            Some(json!({ "fuseau": fuseau, "annee_en_cours": en_cours })),
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            "referentiel.profil_modifie",
            a.etablissement_id,
            json!({ "fuseau": fuseau }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(fuseau)
    }

    /// Historique de structure d'un objet (profil, paramètre, site, salle…), plus récent d'abord.
    pub async fn structure_journal(
        &self,
        a: &Acteur<'_>,
        objet: &str,
        objet_ref: Option<&str>,
        limite: i64,
    ) -> Result<Vec<JournalLigne>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let rows = sqlx::query_as::<_, JournalLigne>(
            "SELECT id, objet, objet_ref, action, avant, apres, par, le FROM ent_structure_journal
              WHERE etablissement_id = $1 AND objet = $2 AND ($3::text IS NULL OR objet_ref = $3)
              ORDER BY le DESC, id LIMIT $4",
        )
        .bind(a.etablissement_id)
        .bind(objet)
        .bind(objet_ref)
        .bind(limite.clamp(1, 500))
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(rows)
    }
}

// ───────────────────────── Paramètres (STR-10) ─────────────────────────

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TypeParam {
    Entier,
    Booleen,
    Heure,
}

/// Définition d'une clé connue : type, bornes, défaut, socle propriétaire.
#[derive(Debug, Clone, Serialize)]
pub struct DefinitionParam {
    pub cle: &'static str,
    pub libelle: &'static str,
    pub type_param: TypeParam,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub defaut: Value,
    pub socle_proprietaire: i16,
}

/// Registre des clés connues. **Toute clé absente d'ici est refusée** (clé inconnue = erreur).
/// Les autres socles y ajoutent leurs clés avec leur numéro de socle.
pub fn registre() -> Vec<DefinitionParam> {
    vec![
        DefinitionParam {
            cle: "structure.duree_seance_defaut_min",
            libelle: "Durée par défaut d'une séance (minutes)",
            type_param: TypeParam::Entier,
            min: Some(15),
            max: Some(240),
            defaut: json!(55),
            socle_proprietaire: 5,
        },
        DefinitionParam {
            cle: "structure.demi_journee_coupure",
            libelle: "Heure qui sépare le matin de l'après-midi",
            type_param: TypeParam::Heure,
            min: None,
            max: None,
            defaut: json!("12:30"),
            socle_proprietaire: 5,
        },
        DefinitionParam {
            cle: "structure.trajet_defaut_min",
            libelle: "Temps de trajet par défaut vers un lieu extérieur (minutes)",
            type_param: TypeParam::Entier,
            min: Some(0),
            max: Some(120),
            defaut: json!(0),
            socle_proprietaire: 5,
        },
        DefinitionParam {
            cle: "structure.salles_reservables_par_defaut",
            libelle: "Une salle nouvellement créée est réservable",
            type_param: TypeParam::Booleen,
            min: None,
            max: None,
            defaut: json!(false),
            socle_proprietaire: 5,
        },
    ]
}

/// Nom du type en base (`ent_parametre.type_param`, D-036).
fn nom_type(t: TypeParam) -> &'static str {
    match t {
        TypeParam::Entier => "entier",
        TypeParam::Booleen => "booleen",
        TypeParam::Heure => "heure",
    }
}

/// Valeur d'échange (déjà validée par [`valider_valeur`]) → colonnes typées : (entier, booléen, heure).
fn valeur_typee(v: &Value) -> (Option<i64>, Option<bool>, Option<String>) {
    (v.as_i64(), v.as_bool(), v.as_str().map(str::to_string))
}

pub(crate) fn definition(cle: &str) -> Option<DefinitionParam> {
    registre().into_iter().find(|d| d.cle == cle)
}

fn valider_valeur(def: &DefinitionParam, v: &Value) -> Result<(), EntDbError> {
    let ok = match def.type_param {
        TypeParam::Entier => v
            .as_i64()
            .is_some_and(|n| def.min.is_none_or(|m| n >= m) && def.max.is_none_or(|m| n <= m)),
        TypeParam::Booleen => v.is_boolean(),
        TypeParam::Heure => v
            .as_str()
            .is_some_and(|s| chrono::NaiveTime::parse_from_str(s, "%H:%M").is_ok()),
    };
    if ok {
        Ok(())
    } else {
        Err(invalide())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ParametreVue {
    #[serde(flatten)]
    pub definition: DefinitionParam,
    /// Valeur effective (défaut si jamais modifié).
    pub valeur: Value,
    pub personnalise: bool,
    pub version: i32,
    pub modifie_par: Option<Uuid>,
    pub modifie_le: Option<DateTime<Utc>>,
}

impl PgEnt {
    /// Tous les paramètres connus avec leur valeur effective. N'écrit rien.
    pub async fn structure_parametres(
        &self,
        a: &Acteur<'_>,
    ) -> Result<Vec<ParametreVue>, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielLire).await?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        type Ligne = (String, Value, i32, Option<Uuid>, DateTime<Utc>);
        let lignes: Vec<Ligne> = sqlx::query_as(
            "SELECT cle, ent_parametre_valeur_json(p), version, modifie_par, modifie_le FROM ent_parametre p
              WHERE etablissement_id = $1 AND type_param IS NOT NULL",
        )
        .bind(a.etablissement_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(registre()
            .into_iter()
            .map(|d| match lignes.iter().find(|l| l.0 == d.cle) {
                Some((_, v, ver, par, le)) => ParametreVue {
                    valeur: v.clone(),
                    personnalise: true,
                    version: *ver,
                    modifie_par: *par,
                    modifie_le: Some(*le),
                    definition: d,
                },
                None => ParametreVue {
                    valeur: d.defaut.clone(),
                    personnalise: false,
                    version: 0,
                    modifie_par: None,
                    modifie_le: None,
                    definition: d,
                },
            })
            .collect())
    }

    /// STR-10 : modifie un paramètre. Clé connue, valeur validée, version incrémentée, journal.
    /// `version_attendue` (0 = jamais modifié) protège d'une modification concurrente.
    pub async fn structure_parametre_definir(
        &self,
        a: &Acteur<'_>,
        cle: &str,
        valeur: Value,
        version_attendue: Option<i32>,
    ) -> Result<ParametreVue, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let def = definition(cle).ok_or_else(invalide)?;
        valider_valeur(&def, &valeur)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let avant: Option<(Value, i32)> = sqlx::query_as(
            "SELECT ent_parametre_valeur_json(p), version FROM ent_parametre p
              WHERE etablissement_id = $1 AND cle = $2 AND type_param IS NOT NULL FOR UPDATE",
        )
        .bind(a.etablissement_id)
        .bind(cle)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let version_actuelle = avant.as_ref().map_or(0, |x| x.1);
        if version_attendue.is_some_and(|v| v != version_actuelle) {
            return Err(conflit());
        }
        let (v_entier, v_booleen, v_heure) = valeur_typee(&valeur);
        let (version, modifie_le): (i32, DateTime<Utc>) = sqlx::query_as(
            "INSERT INTO ent_parametre (etablissement_id, cle, type_param, valeur_entier, valeur_booleen,
                    valeur_heure, version, socle_proprietaire, modifie_par)
             VALUES ($1,$2,$3,$6,$7,$8::text::time,1,$4,$5)
             ON CONFLICT (etablissement_id, cle) DO UPDATE SET type_param = EXCLUDED.type_param,
                    valeur_entier = EXCLUDED.valeur_entier, valeur_booleen = EXCLUDED.valeur_booleen,
                    valeur_heure = EXCLUDED.valeur_heure,
                    version = ent_parametre.version + 1, modifie_par = EXCLUDED.modifie_par, modifie_le = now()
             RETURNING version, modifie_le",
        )
        .bind(a.etablissement_id)
        .bind(cle)
        .bind(nom_type(def.type_param))
        .bind(def.socle_proprietaire)
        .bind(a.user_id)
        .bind(v_entier)
        .bind(v_booleen)
        .bind(v_heure)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        journaliser(
            &mut tx,
            a,
            "parametre",
            cle,
            if avant.is_some() { "modifie" } else { "cree" },
            avant.map(|x| json!({ "valeur": x.0, "version": x.1 })),
            Some(json!({ "valeur": valeur, "version": version })),
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            "referentiel.parametre_modifie",
            a.etablissement_id,
            json!({ "cle": cle, "version": version }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(ParametreVue {
            valeur,
            personnalise: true,
            version,
            modifie_par: Some(a.user_id),
            modifie_le: Some(modifie_le),
            definition: def,
        })
    }
}

// ───────────────────────── Préréglages (STR-02) ─────────────────────────

/// Préréglage proposé au départ : une COPIE modifiable, jamais un lien.
#[derive(Debug, Clone, Serialize)]
pub struct Prereglage {
    pub code: &'static str,
    pub libelle: &'static str,
    pub secteur: &'static str,
    pub degres: Vec<&'static str>,
    pub types: Vec<&'static str>,
    pub jours_ouvres: Value,
    pub parametres: Vec<(&'static str, Value)>,
}

fn semaine(mercredi_matin_seulement: bool, samedi_matin: bool) -> Value {
    let plein = json!(["matin", "apres_midi"]);
    let mut o = serde_json::Map::new();
    o.insert("lun".into(), plein.clone());
    o.insert("mar".into(), plein.clone());
    o.insert(
        "mer".into(),
        if mercredi_matin_seulement {
            json!(["matin"])
        } else {
            plein.clone()
        },
    );
    o.insert("jeu".into(), plein.clone());
    o.insert("ven".into(), plein);
    if samedi_matin {
        o.insert("sam".into(), json!(["matin"]));
    }
    Value::Object(o)
}

pub fn prereglages() -> Vec<Prereglage> {
    vec![
        Prereglage {
            code: "college_public",
            libelle: "Collège public",
            secteur: "public",
            degres: vec!["second"],
            types: vec!["college"],
            jours_ouvres: semaine(true, false),
            parametres: vec![("structure.duree_seance_defaut_min", json!(55))],
        },
        Prereglage {
            code: "lycee_general_public",
            libelle: "Lycée général et technologique public",
            secteur: "public",
            degres: vec!["second"],
            types: vec!["lycee_gt"],
            jours_ouvres: semaine(false, false),
            parametres: vec![("structure.duree_seance_defaut_min", json!(55))],
        },
        Prereglage {
            code: "lycee_professionnel_public",
            libelle: "Lycée professionnel public",
            secteur: "public",
            degres: vec!["second"],
            types: vec!["lycee_pro"],
            jours_ouvres: semaine(false, false),
            parametres: vec![("structure.duree_seance_defaut_min", json!(55))],
        },
        Prereglage {
            code: "ecole_publique",
            libelle: "École publique",
            secteur: "public",
            degres: vec!["premier"],
            types: vec!["ecole"],
            jours_ouvres: json!({
                "lun": ["matin", "apres_midi"], "mar": ["matin", "apres_midi"],
                "jeu": ["matin", "apres_midi"], "ven": ["matin", "apres_midi"]
            }),
            parametres: vec![("structure.duree_seance_defaut_min", json!(45))],
        },
        Prereglage {
            code: "etablissement_prive_sous_contrat",
            libelle: "Établissement privé sous contrat (collège et lycée)",
            secteur: "prive_sous_contrat",
            degres: vec!["second"],
            types: vec!["college", "lycee_gt"],
            jours_ouvres: semaine(true, true),
            parametres: vec![("structure.duree_seance_defaut_min", json!(55))],
        },
        Prereglage {
            code: "etablissement_hors_contrat",
            libelle: "Établissement privé hors contrat",
            secteur: "prive_hors_contrat",
            degres: vec!["second"],
            types: vec![],
            jours_ouvres: semaine(false, false),
            parametres: vec![("structure.duree_seance_defaut_min", json!(55))],
        },
    ]
}

/// Ce que l'application d'un préréglage changerait (aperçu) ou a changé.
#[derive(Debug, Clone, Serialize)]
pub struct ApercuPrereglage {
    pub code: String,
    pub applique: bool,
    pub champs: Value,
    /// Paramètres qui seront posés (ceux déjà personnalisés ne sont jamais écrasés).
    pub parametres: Vec<(String, Value)>,
    pub parametres_conserves: Vec<String>,
}

impl PgEnt {
    pub fn structure_prereglages(&self) -> Vec<Prereglage> {
        prereglages()
    }

    /// STR-02 : aperçu (`apercu = true`, aucune écriture) ou application d'un préréglage. Le
    /// profil doit exister (identité d'abord) et **n'avoir jamais reçu de préréglage** : on ne
    /// réapplique pas sur des données existantes. Ne crée ni site, ni salle, ni classe.
    pub async fn structure_prereglage_appliquer(
        &self,
        a: &Acteur<'_>,
        code: &str,
        apercu: bool,
    ) -> Result<ApercuPrereglage, EntDbError> {
        self.exiger_referentiel(a, Action::ReferentielGerer).await?;
        let pr = prereglages()
            .into_iter()
            .find(|p| p.code == code)
            .ok_or_else(introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), a.etablissement_id)
            .await
            .map_err(db)?;
        let profil = lire_profil(&mut tx, a.etablissement_id, true)
            .await?
            .ok_or_else(conflit)?;
        if profil.profil_preregle.is_some() {
            return Err(conflit());
        }
        let existants: Vec<String> =
            sqlx::query_scalar("SELECT cle FROM ent_parametre WHERE etablissement_id = $1")
                .bind(a.etablissement_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(db)?;
        let (a_poser, conserves): (Vec<_>, Vec<_>) = pr
            .parametres
            .iter()
            .partition(|(c, _)| !existants.iter().any(|e| e == c));
        let rapport = ApercuPrereglage {
            code: pr.code.to_string(),
            applique: !apercu,
            champs: json!({
                "secteur": pr.secteur, "degres": pr.degres, "types": pr.types, "jours_ouvres": pr.jours_ouvres
            }),
            parametres: a_poser
                .iter()
                .map(|(c, v)| (c.to_string(), v.clone()))
                .collect(),
            parametres_conserves: conserves.iter().map(|(c, _)| c.to_string()).collect(),
        };
        if apercu {
            tx.commit().await.map_err(db)?;
            return Ok(rapport);
        }
        // Un contrat n'a de sens que sous contrat : on le retire si le secteur change.
        sqlx::query(
            "UPDATE ent_etablissement_profil SET secteur = $2, degres = $3, types = $4,
                    profil_preregle = $5, contrat_type = CASE WHEN $2 = 'prive_sous_contrat' THEN contrat_type END,
                    version = version + 1, modifie_par = $6, modifie_le = now()
              WHERE etablissement_id = $1",
        )
        .bind(a.etablissement_id)
        .bind(pr.secteur)
        .bind(pr.degres.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .bind(pr.types.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .bind(pr.code)
        .bind(a.user_id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        ecrire_jours_ouvres(&mut tx, a.etablissement_id, &pr.jours_ouvres).await?;
        for (cle, valeur) in &a_poser {
            let def = definition(cle).ok_or_else(invalide)?;
            sqlx::query(
                "INSERT INTO ent_parametre (etablissement_id, cle, type_param, valeur_entier, valeur_booleen,
                        valeur_heure, version, socle_proprietaire, modifie_par)
                 VALUES ($1,$2,$3,$6,$7,$8::text::time,1,$4,$5) ON CONFLICT DO NOTHING",
            )
            .bind(a.etablissement_id)
            .bind(*cle)
            .bind(nom_type(def.type_param))
            .bind(def.socle_proprietaire)
            .bind(a.user_id)
            .bind(valeur_typee(valeur).0)
            .bind(valeur_typee(valeur).1)
            .bind(valeur_typee(valeur).2)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        journaliser(
            &mut tx,
            a,
            "prereglage",
            pr.code,
            "applique",
            Some(json_profil(&profil)),
            Some(json!({ "champs": rapport.champs, "parametres": rapport.parametres })),
        )
        .await?;
        let ev = publier(
            &mut tx,
            a,
            "referentiel.prereglages_appliques",
            a.etablissement_id,
            json!({ "code": pr.code }),
        )
        .await?;
        tx.commit().await.map_err(db)?;
        self.diffuser_apres_commit(&[ev]);
        Ok(rapport)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uai_format() {
        assert!(uai_valide("0751234A"));
        assert!(!uai_valide("075123A"));
        assert!(!uai_valide("0751234a"));
        assert!(!uai_valide("07512345"));
    }

    #[test]
    fn semaine_type_validee() {
        assert!(jours_ouvres_valides(
            &json!({"lun": ["matin", "apres_midi"], "mer": ["matin"]})
        ));
        assert!(!jours_ouvres_valides(&json!({})));
        assert!(!jours_ouvres_valides(&json!({"lun": []})));
        assert!(!jours_ouvres_valides(&json!({"xxx": ["matin"]})));
        assert!(!jours_ouvres_valides(&json!({"lun": ["matin", "matin"]})));
        assert!(!jours_ouvres_valides(&json!({"lun": ["soir"]})));
        assert!(!jours_ouvres_valides(&json!(["lun"])));
    }

    #[test]
    fn prereglages_valides_et_cles_connues() {
        for p in prereglages() {
            assert!(jours_ouvres_valides(&p.jours_ouvres), "{}", p.code);
            assert!(SECTEURS.contains(&p.secteur));
            for (c, v) in &p.parametres {
                let d = definition(c).unwrap_or_else(|| panic!("clé {c}"));
                valider_valeur(&d, v).unwrap();
            }
        }
    }

    #[test]
    fn registre_defauts_valides() {
        for d in registre() {
            valider_valeur(&d, &d.defaut).unwrap_or_else(|_| panic!("défaut de {}", d.cle));
        }
    }
}
