//! Lot A2 du socle 4 (suite) : préférences de contact et langue (ADM-07), consentements
//! administratifs (ADM-12), divergences entre le compte et la fiche (§8.4, jamais écrasées en
//! silence), identité versionnée (ADM-03) et séparation en lot du nom et du prénom (ADM-10).
//!
//! Même règle partout : une écriture = une transaction = fait + version/historique + journal +
//! événement ; les journaux et événements ne portent que des identifiants, des codes et des noms
//! de champs, **jamais une valeur** (coordonnée, nom, adresse, motif).

use super::*;
use crate::champs_chiffres::ChampsSensibles;
use super::coordonnees::{
    attribuer_tx, classer_telephone, histo_tx, masque_courriel, masque_telephone, motif_valide, refus_conflit,
    refus_invalide, Auteur, Coordonnee,
};

const TABLE_VERSION: &str = "ent_personne_version";

// ---------------------------------------------------------------------------
// Préférences (ADM-07)
// ---------------------------------------------------------------------------

pub const CANAUX_REFUSABLES: [&str; 2] = ["email", "sms"];
pub const ACCESSIBILITE: [&str; 3] = ["gros_caracteres", "pas_d_appel_vocal", "interprete"];

/// Préférences de contact d'une personne. Un canal refusé n'est jamais utilisé pour elle ; la
/// personne reste toujours joignable par un autre moyen (e-mail, SMS ou courrier).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PreferencesPersonne {
    pub langue: Option<String>,
    pub canaux_refuses: Vec<String>,
    pub courrier_papier: bool,
    pub accessibilite: Vec<String>,
    /// Plages où l'appeler : `[{"jours":[1,2],"de":"08:00","a":"12:00"}]` (jamais pour bloquer une urgence).
    pub joignable: Option<Value>,
    pub fuseau: Option<String>,
    pub maj_le: Option<DateTime<Utc>>,
}

fn heure_valide(h: &str) -> Option<u32> {
    let (a, b) = h.split_once(':')?;
    if a.len() != 2 || b.len() != 2 {
        return None;
    }
    let (a, b) = (a.parse::<u32>().ok()?, b.parse::<u32>().ok()?);
    (a < 24 && b < 60).then_some(a * 60 + b)
}

/// Plages de joignabilité : au plus 14, jours 1 (lundi) à 7, `de` < `a`.
pub fn valider_joignable(v: &Value) -> Result<Value, EntDbError> {
    let mal = || invalide("horaires de joignabilité mal formés");
    let plages = v.as_array().ok_or_else(mal)?;
    if plages.len() > 14 {
        return Err(invalide("14 plages horaires au plus"));
    }
    let mut out = Vec::new();
    for p in plages {
        let jours: Vec<i64> = p
            .get("jours")
            .and_then(Value::as_array)
            .ok_or_else(mal)?
            .iter()
            .map(|j| j.as_i64().filter(|j| (1..=7).contains(j)).ok_or_else(mal))
            .collect::<Result<_, _>>()?;
        if jours.is_empty() {
            return Err(mal());
        }
        let de = p.get("de").and_then(Value::as_str).and_then(heure_valide).ok_or_else(mal)?;
        let a = p.get("a").and_then(Value::as_str).and_then(heure_valide).ok_or_else(mal)?;
        if de >= a {
            return Err(mal());
        }
        out.push(json!({
            "jours": jours,
            "de": p["de"],
            "a": p["a"],
        }));
    }
    Ok(Value::Array(out))
}

fn normaliser_preferences(p: &PreferencesPersonne) -> Result<PreferencesPersonne, EntDbError> {
    let langue = p.langue.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_lowercase);
    if let Some(l) = &langue {
        let ok = {
            let (base, region) = l.split_once('-').map_or((l.as_str(), None), |(a, b)| (a, Some(b)));
            (2..=3).contains(&base.len())
                && base.chars().all(|c| c.is_ascii_lowercase())
                && region.is_none_or(|r| (2..=8).contains(&r.len()) && r.chars().all(|c| c.is_ascii_alphanumeric()))
        };
        if !ok {
            return Err(invalide("code de langue invalide (ex. fr, ar, fa-AF)"));
        }
    }
    let mut refuses: Vec<String> = Vec::new();
    for c in &p.canaux_refuses {
        if !CANAUX_REFUSABLES.contains(&c.as_str()) {
            return Err(invalide("canal inconnu (email, sms)"));
        }
        if !refuses.contains(c) {
            refuses.push(c.clone());
        }
    }
    refuses.sort();
    if refuses.len() == CANAUX_REFUSABLES.len() && !p.courrier_papier {
        return Err(refus_invalide(
            "au moins un moyen d'être prévenu : e-mail, SMS ou courrier papier",
        ));
    }
    let mut acc: Vec<String> = Vec::new();
    for a in &p.accessibilite {
        if !ACCESSIBILITE.contains(&a.as_str()) {
            return Err(invalide("besoin d'accessibilité inconnu"));
        }
        if !acc.contains(a) {
            acc.push(a.clone());
        }
    }
    acc.sort();
    let joignable = match &p.joignable {
        None | Some(Value::Null) => None,
        Some(v) => Some(valider_joignable(v)?),
    };
    let fuseau = p.fuseau.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    if fuseau.as_ref().is_some_and(|f| f.len() > 64) {
        return Err(invalide("fuseau horaire invalide"));
    }
    Ok(PreferencesPersonne {
        langue,
        canaux_refuses: refuses,
        courrier_papier: p.courrier_papier,
        accessibilite: acc,
        joignable,
        fuseau,
        maj_le: None,
    })
}

// ---------------------------------------------------------------------------
// Consentements (ADM-12)
// ---------------------------------------------------------------------------

/// Objets de consentement et version du texte actuellement présenté à la personne. Une réponse
/// donnée sur un texte périmé est refusée : la preuve porte sur le texte exact.
pub const CONSENTEMENTS: [(&str, &str); 4] = [
    ("diffusion_associations_parents", "2026-11-v1"),
    ("diffusion_transport", "2026-11-v1"),
    ("photo_remise", "2026-11-v1"),
    ("annuaire_parents_classe", "2026-11-v1"),
];

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Consentement {
    pub id: Uuid,
    pub objet: String,
    pub reponse: String,
    pub texte_version: String,
    pub canal: String,
    pub piece_reference: Option<String>,
    pub du: DateTime<Utc>,
    pub retire_le: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Divergences compte ↔ fiche
// ---------------------------------------------------------------------------

/// Divergence rapportée (valeurs masquées : la liste ne propage pas les coordonnées).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivergenceCompte {
    pub personne_id: Uuid,
    pub personne_libelle: String,
    pub champ: String,
    /// `absent_de_la_fiche` ou `different_du_principal`.
    pub nature: String,
    pub valeur_compte_masquee: String,
    pub valeur_fiche_masquee: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoixDivergence {
    /// Le compte et la fiche portent volontairement des valeurs différentes.
    DistinctVoulu,
    /// La fiche fait foi ; on ne revient pas dessus tant que les deux valeurs ne changent pas.
    ConserverFiche,
    /// La valeur du compte devient la coordonnée principale de la fiche (l'ancienne est conservée).
    AdopterCompte,
}

impl ChoixDivergence {
    pub fn code(self) -> &'static str {
        match self {
            Self::DistinctVoulu => "distinct_voulu",
            Self::ConserverFiche => "conserver_fiche",
            Self::AdopterCompte => "adopter_compte",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "distinct_voulu" => Self::DistinctVoulu,
            "conserver_fiche" => Self::ConserverFiche,
            "adopter_compte" => Self::AdopterCompte,
            _ => return None,
        })
    }
}

// ---------------------------------------------------------------------------
// Identité versionnée (ADM-03) et séparation nom / prénom (ADM-10)
// ---------------------------------------------------------------------------

/// Correction (faute de frappe) ou changement réel (mariage, jugement, prénom d'usage).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatureIdentite {
    Correction,
    Changement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentiteSaisie {
    pub nom: String,
    pub prenom: Option<String>,
    pub civilite: Option<String>,
}

/// Anciennes valeurs d'un changement sensible (lisibles avec `responsables.protege.lire`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AncienneIdentite {
    pub prenom: Option<String>,
    pub civilite: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionIdentite {
    pub id: Uuid,
    pub nom: Option<String>,
    pub prenom: Option<String>,
    pub civilite: Option<String>,
    pub nature: String,
    pub effet_du: NaiveDate,
    pub motif: Option<String>,
    pub sensible: bool,
    /// Valeurs en clair retirées (scellées dans une version plus récente).
    pub masquee: bool,
    pub enregistre_le: DateTime<Utc>,
    /// Présentes seulement pour qui lit les données protégées.
    pub anterieures: Option<Vec<AncienneIdentite>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionSeparation {
    pub personne_id: Uuid,
    pub nom_actuel: String,
    pub nom_propose: String,
    pub prenom_propose: String,
    /// Découpage ambigu (plus de deux mots) : à relire avec attention.
    pub a_verifier: bool,
}

/// Séparation choisie (ou corrigée) par une personne humaine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparationNom {
    pub personne_id: Uuid,
    pub nom: String,
    pub prenom: String,
}

const PARTICULES: [&str; 14] = [
    "de", "du", "des", "la", "le", "les", "van", "von", "der", "den", "di", "da", "del", "el",
];

fn est_tout_en_majuscules(m: &str) -> bool {
    let lettres: Vec<char> = m.chars().filter(|c| c.is_alphabetic()).collect();
    lettres.len() > 1 && lettres.iter().all(|c| c.is_uppercase())
}

/// Propose un découpage « Prénom Nom » ou « NOM Prénom » d'un nom affiché mélangé. Heuristique
/// volontairement prudente : le résultat est une **proposition**, validée par une personne.
pub fn proposer_separation(nom_affiche: &str) -> Option<(String, String, bool)> {
    let mots: Vec<&str> = nom_affiche.split_whitespace().collect();
    if mots.len() < 2 {
        return None;
    }
    if est_tout_en_majuscules(mots[0]) && mots.iter().skip(1).any(|m| !est_tout_en_majuscules(m)) {
        let n = mots.iter().take_while(|m| est_tout_en_majuscules(m)).count();
        let nom = mots[..n].join(" ");
        let prenom = mots[n..].join(" ");
        return Some((nom, prenom, mots.len() > 2));
    }
    let prenom = mots[0].to_string();
    let nom = mots[1..].join(" ");
    let avec_particule = mots[1..].iter().any(|m| PARTICULES.contains(&m.to_lowercase().as_str()));
    Some((nom, prenom, mots.len() > 2 && !avec_particule))
}

fn mots_tries(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s.split_whitespace().map(str::to_lowercase).collect();
    v.sort();
    v
}

fn coffre() -> ChampsSensibles {
    ChampsSensibles::depuis_env()
}

/// Attribue les écritures de la transaction à un acteur (déclencheurs de la migration 274 et 276).
pub(crate) async fn attribuer_acteur_tx(
    conn: &mut PgConnection,
    actor: Uuid,
    via: &str,
    raison: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        "SELECT set_config('ent.coord_acteur', $1, true), set_config('ent.coord_via', $2, true),
                set_config('ent.coord_raison', $3, true)",
    )
    .bind(actor.to_string())
    .bind(via)
    .bind(raison)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Applique une identité à une personne : projection `ent_personne` + une version, dans la
/// transaction de l'appelant. Renvoie les noms des champs modifiés (jamais leurs valeurs).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn appliquer_identite_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    actor: Uuid,
    via: &str,
    avant: &Personne,
    nouveau: &IdentiteSaisie,
    nature_version: &str,
    sensible_si_changement: bool,
    effet_du: Option<NaiveDate>,
    motif: &str,
) -> Result<Vec<&'static str>, EntDbError> {
    let mut champs: Vec<&'static str> = Vec::new();
    if avant.nom != nouveau.nom {
        champs.push("nom");
    }
    if avant.prenom != nouveau.prenom {
        champs.push("prenom");
    }
    if avant.civilite != nouveau.civilite {
        champs.push("civilite");
    }
    if champs.is_empty() {
        return Ok(champs);
    }
    let (aujourdhui,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())")
        .bind(etab)
        .fetch_one(&mut *conn)
        .await
        .map_err(db)?;
    let effet = effet_du.unwrap_or(aujourdhui);
    if effet > aujourdhui {
        return Err(invalide("la date d'effet ne peut pas être dans le futur"));
    }
    let sensible = sensible_si_changement && (champs.contains(&"prenom") || champs.contains(&"civilite"));
    sqlx::query("SELECT set_config('ent.identite_ecrite', 'on', true)")
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    attribuer_acteur_tx(conn, actor, via, "changement_declare").await?;
    sqlx::query(
        "UPDATE ent_personne SET nom = $3, prenom = $4, civilite = $5, updated_at = now()
          WHERE etablissement_id = $1 AND id = $2",
    )
    .bind(etab)
    .bind(avant.id)
    .bind(&nouveau.nom)
    .bind(&nouveau.prenom)
    .bind(&nouveau.civilite)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    #[derive(sqlx::FromRow)]
    struct Prec {
        id: Uuid,
        prenom: Option<String>,
        civilite: Option<String>,
        valeurs_anterieures: Option<String>,
        masquee: bool,
    }
    let precedentes = sqlx::query_as::<_, Prec>(
        "SELECT id, prenom, civilite, valeurs_anterieures, masquee FROM ent_personne_version
          WHERE etablissement_id = $1 AND personne_id = $2 ORDER BY enregistre_le, id FOR UPDATE",
    )
    .bind(etab)
    .bind(avant.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let id = Uuid::new_v4();
    let mut scelle: Option<String> = None;
    if sensible {
        // Les anciennes valeurs (et celles déjà scellées plus tôt) passent dans la nouvelle version.
        let mut anciennes: Vec<AncienneIdentite> = Vec::new();
        for p in &precedentes {
            if let Some(s) = &p.valeurs_anterieures {
                let clair = coffre()
                    .ouvrir(TABLE_VERSION, "valeurs_anterieures", p.id, s)
                    .map_err(EntDbError::Domain)?;
                anciennes.extend(serde_json::from_str::<Vec<AncienneIdentite>>(&clair).unwrap_or_default());
            }
            if !p.masquee && (p.prenom.is_some() || p.civilite.is_some()) {
                anciennes.push(AncienneIdentite {
                    prenom: p.prenom.clone(),
                    civilite: p.civilite.clone(),
                });
            }
        }
        let clair = serde_json::to_string(&anciennes).map_err(|_| EntDbError::Domain(DomainError::Infrastructure))?;
        scelle = Some(
            coffre()
                .sceller(TABLE_VERSION, "valeurs_anterieures", id, &clair)
                .map_err(EntDbError::Domain)?,
        );
        sqlx::query(
            "UPDATE ent_personne_version SET prenom = NULL, civilite = NULL, masquee = true,
                    valeurs_anterieures = NULL
              WHERE etablissement_id = $1 AND personne_id = $2",
        )
        .bind(etab)
        .bind(avant.id)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    sqlx::query(
        "INSERT INTO ent_personne_version
            (id, etablissement_id, personne_id, nom, prenom, civilite, nature, effet_du,
             remplace_version_id, motif, sensible, valeurs_anterieures, enregistre_par)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
    )
    .bind(id)
    .bind(etab)
    .bind(avant.id)
    .bind(&nouveau.nom)
    .bind(&nouveau.prenom)
    .bind(&nouveau.civilite)
    .bind(nature_version)
    .bind(effet)
    .bind(precedentes.last().map(|p| p.id))
    .bind(motif)
    .bind(sensible)
    .bind(scelle)
    .bind(actor)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    journal_tx(
        conn,
        etab,
        None,
        "personne_identite_modifiee",
        "Identité d'une personne modifiée",
        json!({ "personne_id": avant.id, "nature": nature_version, "champs": champs, "sensible": sensible }),
        false,
        actor,
    )
    .await?;
    publier_tx(
        conn,
        etab,
        types::PERSONNE_IDENTITE_MODIFIEE,
        AggregateKind::Personne,
        avant.id,
        None,
        json!({ "personne_id": avant.id, "nature": nature_version, "champs": champs }),
        actor,
    )
    .await?;
    Ok(champs)
}

fn identite_normalisee(s: &IdentiteSaisie) -> Result<IdentiteSaisie, EntDbError> {
    let n = PersonneSaisie {
        nom: s.nom.clone(),
        prenom: s.prenom.clone(),
        civilite: s.civilite.clone(),
        ..Default::default()
    }
    .normaliser()?;
    Ok(IdentiteSaisie {
        nom: n.nom,
        prenom: n.prenom,
        civilite: n.civilite,
    })
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

impl PgEnt {
    // -- Préférences ----------------------------------------------------------

    pub(crate) async fn preferences_lire(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
    ) -> Result<PreferencesPersonne, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, false).await?;
        #[derive(sqlx::FromRow)]
        struct Row {
            langue: Option<String>,
            canal_email: bool,
            canal_sms: bool,
            courrier_papier: bool,
            accessibilite: Vec<String>,
            joignable: Option<Value>,
            fuseau: Option<String>,
            updated_at: DateTime<Utc>,
        }
        let r = sqlx::query_as::<_, Row>(
            "SELECT langue, canal_email, canal_sms, courrier_papier, accessibilite, joignable, fuseau,
                    updated_at
               FROM ent_personne_preference WHERE etablissement_id = $1 AND personne_id = $2",
        )
        .bind(etab)
        .bind(pid)
        .fetch_optional(self.pool())
        .await
        .map_err(db)?;
        let Some(r) = r else {
            // Sans ligne : la langue déclarée à la préinscription (contrat v1) sert de repli.
            let langue: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT langue FROM ent_personne_reserve WHERE etablissement_id = $1 AND personne_id = $2",
            )
            .bind(etab)
            .bind(pid)
            .fetch_optional(self.pool())
            .await
            .map_err(db)?;
            return Ok(PreferencesPersonne {
                langue: langue.and_then(|(l,)| l),
                ..Default::default()
            });
        };
        let mut refuses = Vec::new();
        if !r.canal_email {
            refuses.push("email".to_string());
        }
        if !r.canal_sms {
            refuses.push("sms".to_string());
        }
        Ok(PreferencesPersonne {
            langue: r.langue,
            canaux_refuses: refuses,
            courrier_papier: r.courrier_papier,
            accessibilite: r.accessibilite,
            joignable: r.joignable,
            fuseau: r.fuseau,
            maj_le: Some(r.updated_at),
        })
    }

    pub(crate) async fn preferences_definir(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        saisie: &PreferencesPersonne,
    ) -> Result<PreferencesPersonne, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        let n = normaliser_preferences(saisie)?;
        if let Some(f) = &n.fuseau {
            let (ok,): (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = $1)")
                .bind(f)
                .fetch_one(self.pool())
                .await
                .map_err(db)?;
            if !ok {
                return Err(invalide("fuseau horaire inconnu"));
            }
        }
        let avant = self.preferences_lire(etab, auteur, personne_id).await?;
        let mut champs: Vec<&str> = Vec::new();
        if avant.langue != n.langue {
            champs.push("langue");
        }
        if avant.canaux_refuses != n.canaux_refuses {
            champs.push("canaux_refuses");
        }
        if avant.courrier_papier != n.courrier_papier {
            champs.push("courrier_papier");
        }
        if avant.accessibilite != n.accessibilite {
            champs.push("accessibilite");
        }
        if avant.joignable != n.joignable {
            champs.push("joignable");
        }
        if avant.fuseau != n.fuseau {
            champs.push("fuseau");
        }
        if champs.is_empty() {
            return Ok(avant);
        }
        let canal_email = !n.canaux_refuses.iter().any(|c| c == "email");
        let canal_sms = !n.canaux_refuses.iter().any(|c| c == "sms");
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if p.fusionnee_dans.is_some() {
            return Err(refus_conflit("personne fusionnée dans une autre fiche"));
        }
        sqlx::query(
            "INSERT INTO ent_personne_preference
                (personne_id, etablissement_id, canal_email, canal_sms, courrier_papier, langue,
                 accessibilite, joignable, fuseau, updated_by)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             ON CONFLICT (personne_id) DO UPDATE SET canal_email = $3, canal_sms = $4,
                courrier_papier = $5, langue = $6, accessibilite = $7, joignable = $8, fuseau = $9,
                updated_by = $10, updated_at = now()",
        )
        .bind(pid)
        .bind(etab)
        .bind(canal_email)
        .bind(canal_sms)
        .bind(n.courrier_papier)
        .bind(&n.langue)
        .bind(&n.accessibilite)
        .bind(&n.joignable)
        .bind(&n.fuseau)
        .bind(auteur.actor())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO ent_personne_preference_version
                (id, etablissement_id, personne_id, langue, canal_email, canal_sms, courrier_papier,
                 accessibilite, joignable, fuseau, via, par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(pid)
        .bind(&n.langue)
        .bind(canal_email)
        .bind(canal_sms)
        .bind(n.courrier_papier)
        .bind(&n.accessibilite)
        .bind(&n.joignable)
        .bind(&n.fuseau)
        .bind(auteur.via())
        .bind(auteur.actor())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            None,
            "preferences_modifiees",
            "Préférences de contact modifiées",
            json!({ "personne_id": pid, "champs": champs, "via": auteur.via() }),
            false,
            auteur.actor(),
        )
        .await?;
        publier_tx(
            &mut tx,
            etab,
            types::PERSONNE_PREFERENCES_MODIFIEES,
            AggregateKind::Personne,
            pid,
            None,
            json!({ "personne_id": pid, "champs": champs }),
            auteur.actor(),
        )
        .await?;
        commit(tx).await?;
        self.preferences_lire(etab, auteur, personne_id).await
    }

    pub async fn preferences_personne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
    ) -> Result<PreferencesPersonne, EntDbError> {
        self.preferences_lire(etab, &Auteur::Personnel { actor, role }, Some(personne_id)).await
    }

    pub async fn definir_preferences_contact_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        saisie: &PreferencesPersonne,
    ) -> Result<PreferencesPersonne, EntDbError> {
        self.preferences_definir(etab, &Auteur::Personnel { actor, role }, Some(personne_id), saisie).await
    }

    pub async fn mes_preferences(&self, etab: Uuid, user: Uuid) -> Result<PreferencesPersonne, EntDbError> {
        self.preferences_lire(etab, &Auteur::Personne { actor: user }, None).await
    }

    pub async fn definir_mes_preferences(
        &self,
        etab: Uuid,
        user: Uuid,
        saisie: &PreferencesPersonne,
    ) -> Result<PreferencesPersonne, EntDbError> {
        self.preferences_definir(etab, &Auteur::Personne { actor: user }, None, saisie).await
    }

    // -- Consentements ---------------------------------------------------------

    pub(crate) async fn consentements_lire(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
    ) -> Result<Vec<Consentement>, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, false).await?;
        sqlx::query_as::<_, Consentement>(
            "SELECT id, objet, reponse, texte_version, canal, piece_reference, du, retire_le
               FROM ent_personne_consentement
              WHERE etablissement_id = $1 AND personne_id = $2
              ORDER BY du DESC, id DESC",
        )
        .bind(etab)
        .bind(pid)
        .fetch_all(self.pool())
        .await
        .map_err(db)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn consentement_enregistrer(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        objet: &str,
        accorde: bool,
        texte_version: &str,
        canal: Option<&str>,
        piece_reference: Option<&str>,
    ) -> Result<Consentement, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        let Some((_, courante)) = CONSENTEMENTS.iter().find(|(o, _)| *o == objet) else {
            return Err(invalide("objet de consentement inconnu"));
        };
        if texte_version != *courante {
            return Err(refus_conflit("le texte présenté a changé : relire le texte actuel"));
        }
        let (canal, piece) = match auteur {
            Auteur::Personne { .. } => ("portail", None),
            Auteur::Personnel { .. } => {
                let canal = canal.unwrap_or("guichet");
                if !["papier", "guichet"].contains(&canal) {
                    return Err(invalide("canal inconnu (papier, guichet)"));
                }
                let piece = piece_reference.map(str::trim).filter(|s| !s.is_empty());
                if canal == "papier" && piece.is_none() {
                    return Err(refus_invalide("un consentement papier exige la référence de la pièce"));
                }
                if piece.is_some_and(|p| !(1..=80).contains(&p.chars().count())) {
                    return Err(invalide("référence de pièce de 80 caractères au plus"));
                }
                (canal, piece)
            }
        };
        let reponse = if accorde { "accorde" } else { "refuse" };
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if p.fusionnee_dans.is_some() {
            return Err(refus_conflit("personne fusionnée dans une autre fiche"));
        }
        sqlx::query(
            "UPDATE ent_personne_consentement SET retire_le = now(), retire_par = $3
              WHERE personne_id = $1 AND objet = $2 AND retire_le IS NULL",
        )
        .bind(pid)
        .bind(objet)
        .bind(auteur.actor())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO ent_personne_consentement
                (id, etablissement_id, personne_id, objet, reponse, texte_version, canal,
                 piece_reference, donne_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(etab)
        .bind(pid)
        .bind(objet)
        .bind(reponse)
        .bind(texte_version)
        .bind(canal)
        .bind(piece)
        .bind(auteur.actor())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        self.consentement_tracer(&mut tx, etab, auteur, pid, objet, reponse).await?;
        let c = sqlx::query_as::<_, Consentement>(
            "SELECT id, objet, reponse, texte_version, canal, piece_reference, du, retire_le
               FROM ent_personne_consentement WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        commit(tx).await?;
        Ok(c)
    }

    async fn consentement_tracer(
        &self,
        conn: &mut PgConnection,
        etab: Uuid,
        auteur: &Auteur<'_>,
        pid: Uuid,
        objet: &str,
        reponse: &str,
    ) -> Result<(), EntDbError> {
        journal_tx(
            conn,
            etab,
            None,
            "consentement_modifie",
            "Consentement administratif modifié",
            json!({ "personne_id": pid, "objet": objet, "reponse": reponse, "via": auteur.via() }),
            false,
            auteur.actor(),
        )
        .await?;
        publier_tx(
            conn,
            etab,
            types::PERSONNE_CONSENTEMENT_MODIFIE,
            AggregateKind::Personne,
            pid,
            None,
            json!({ "personne_id": pid, "objet": objet, "reponse": reponse }),
            auteur.actor(),
        )
        .await
    }

    pub(crate) async fn consentement_retirer(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        objet: &str,
    ) -> Result<(), EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        if !CONSENTEMENTS.iter().any(|(o, _)| *o == objet) {
            return Err(invalide("objet de consentement inconnu"));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let n = sqlx::query(
            "UPDATE ent_personne_consentement SET retire_le = now(), retire_par = $3
              WHERE personne_id = $1 AND objet = $2 AND retire_le IS NULL",
        )
        .bind(pid)
        .bind(objet)
        .bind(auteur.actor())
        .execute(&mut *tx)
        .await
        .map_err(db)?
        .rows_affected();
        if n == 0 {
            return Err(refus_conflit("aucun consentement en cours pour cet objet"));
        }
        self.consentement_tracer(&mut tx, etab, auteur, pid, objet, "retire").await?;
        commit(tx).await
    }

    pub async fn consentements_personne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
    ) -> Result<Vec<Consentement>, EntDbError> {
        self.consentements_lire(etab, &Auteur::Personnel { actor, role }, Some(personne_id)).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn enregistrer_consentement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        objet: &str,
        accorde: bool,
        texte_version: &str,
        canal: Option<&str>,
        piece_reference: Option<&str>,
    ) -> Result<Consentement, EntDbError> {
        self.consentement_enregistrer(
            etab,
            &Auteur::Personnel { actor, role },
            Some(personne_id),
            objet,
            accorde,
            texte_version,
            canal,
            piece_reference,
        )
        .await
    }

    pub async fn retirer_consentement_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        objet: &str,
    ) -> Result<(), EntDbError> {
        self.consentement_retirer(etab, &Auteur::Personnel { actor, role }, Some(personne_id), objet).await
    }

    pub async fn mes_consentements(&self, etab: Uuid, user: Uuid) -> Result<Vec<Consentement>, EntDbError> {
        self.consentements_lire(etab, &Auteur::Personne { actor: user }, None).await
    }

    pub async fn enregistrer_mon_consentement(
        &self,
        etab: Uuid,
        user: Uuid,
        objet: &str,
        accorde: bool,
        texte_version: &str,
    ) -> Result<Consentement, EntDbError> {
        self.consentement_enregistrer(
            etab,
            &Auteur::Personne { actor: user },
            None,
            objet,
            accorde,
            texte_version,
            None,
            None,
        )
        .await
    }

    pub async fn retirer_mon_consentement(&self, etab: Uuid, user: Uuid, objet: &str) -> Result<(), EntDbError> {
        self.consentement_retirer(etab, &Auteur::Personne { actor: user }, None, objet).await
    }

    // -- Divergences compte ↔ fiche ----------------------------------------------

    async fn divergences_lire(
        &self,
        etab: Uuid,
        personne_id: Option<Uuid>,
    ) -> Result<Vec<DivergenceCompte>, EntDbError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            personne_id: Uuid,
            nom: String,
            prenom: Option<String>,
            champ: String,
            nature: String,
            valeur_compte: String,
            valeur_fiche: Option<String>,
        }
        let rows = sqlx::query_as::<_, Row>(
            "SELECT v.personne_id, p.nom, p.prenom, v.champ, v.nature, v.valeur_compte, v.valeur_fiche
               FROM ent_personne_divergence_compte_v v
               JOIN ent_personne p ON p.id = v.personne_id AND p.etablissement_id = v.etablissement_id
              WHERE v.etablissement_id = $1 AND ($2::uuid IS NULL OR v.personne_id = $2)
              ORDER BY lower(p.nom), lower(coalesce(p.prenom, '')), v.champ LIMIT 500",
        )
        .bind(etab)
        .bind(personne_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let masque = |champ: &str, v: &str| {
            if champ == "email" {
                masque_courriel(v)
            } else {
                masque_telephone(v)
            }
        };
        Ok(rows
            .into_iter()
            .map(|r| DivergenceCompte {
                personne_libelle: Personne {
                    id: r.personne_id,
                    nom: r.nom,
                    prenom: r.prenom,
                    civilite: None,
                    email: None,
                    telephone: None,
                    telephone_2: None,
                    adresse: None,
                    user_id: None,
                    eleve_id: None,
                    fusionnee_dans: None,
                    source: String::new(),
                }
                .libelle(),
                personne_id: r.personne_id,
                valeur_compte_masquee: masque(&r.champ, &r.valeur_compte),
                valeur_fiche_masquee: r.valeur_fiche.as_deref().map(|v| masque(&r.champ, v)),
                champ: r.champ,
                nature: r.nature,
            })
            .collect())
    }

    /// Divergences de l'établissement : rapport pour le personnel habilité.
    pub async fn divergences_compte_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
    ) -> Result<Vec<DivergenceCompte>, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer).await?;
        self.divergences_lire(etab, None).await
    }

    pub async fn mes_divergences_compte(&self, etab: Uuid, user: Uuid) -> Result<Vec<DivergenceCompte>, EntDbError> {
        let pid = self
            .coord_resoudre_personne(etab, &Auteur::Personne { actor: user }, None, false)
            .await?;
        self.divergences_lire(etab, Some(pid)).await
    }

    pub(crate) async fn divergence_resoudre(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        champ: &str,
        choix: ChoixDivergence,
        motif: &Option<String>,
    ) -> Result<(), EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        if !["email", "telephone"].contains(&champ) {
            return Err(invalide("champ inconnu (email, telephone)"));
        }
        let motif = motif_valide(motif)?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let d: Option<(String, Option<String>, String, String)> = sqlx::query_as(
            "SELECT valeur_compte, valeur_fiche, empreinte, nature FROM ent_personne_divergence_compte_v
              WHERE etablissement_id = $1 AND personne_id = $2 AND champ = $3",
        )
        .bind(etab)
        .bind(pid)
        .bind(champ)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((valeur_compte, _fiche, empreinte, _nature)) = d else {
            return Err(refus_conflit("aucune divergence à résoudre pour ce champ"));
        };
        if choix == ChoixDivergence::AdopterCompte {
            attribuer_tx(&mut tx, auteur, "divergence_compte").await?;
            let type_cible = if champ == "email" { "email".to_string() } else { classer_telephone(&valeur_compte).to_string() };
            let groupe = if champ == "email" { "email" } else { "telephone" };
            let existante: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM ent_personne_coordonnee
                  WHERE personne_id = $1 AND au IS NULL AND ent_coord_groupe(type) = $2 AND valeur = $3",
            )
            .bind(pid)
            .bind(groupe)
            .bind(&valeur_compte)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            sqlx::query(
                "UPDATE ent_personne_coordonnee SET principal = false
                  WHERE personne_id = $1 AND au IS NULL AND principal AND ent_coord_groupe(type) = $2",
            )
            .bind(pid)
            .bind(groupe)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            let coord_id = match existante {
                Some((id,)) => {
                    sqlx::query(
                        "UPDATE ent_personne_coordonnee
                            SET principal = true,
                                usages = (SELECT array_agg(DISTINCT u) FROM unnest(array_append(usages, 'notifications')) u),
                                verifiee_le = COALESCE(verifiee_le, now()),
                                verification_voie = COALESCE(verification_voie, 'compte')
                          WHERE id = $1",
                    )
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    id
                }
                None => {
                    let id = Uuid::new_v4();
                    sqlx::query(
                        "INSERT INTO ent_personne_coordonnee
                            (id, etablissement_id, personne_id, type, valeur, usages, principal,
                             verifiee_le, verification_voie, source, cree_par)
                         VALUES ($1,$2,$3,$4,$5,ARRAY['notifications','connexion']::TEXT[],true,now(),'compte','compte',$6)",
                    )
                    .bind(id)
                    .bind(etab)
                    .bind(pid)
                    .bind(&type_cible)
                    .bind(&valeur_compte)
                    .bind(auteur.actor())
                    .execute(&mut *tx)
                    .await
                    .map_err(db)?;
                    id
                }
            };
            let c: Coordonnee = sqlx::query_as(
                "SELECT c.id, c.personne_id, c.type, c.valeur, c.libelle, c.usages, c.principal,
                        c.verifiee_le, c.verification_voie, c.a_verifier, c.delivrabilite, c.source,
                        c.du, c.au, c.remplace_id
                   FROM ent_personne_coordonnee c WHERE c.id = $1",
            )
            .bind(coord_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            histo_tx(&mut tx, etab, pid, &c, "divergence_resolue", "divergence_compte", motif.as_deref(), auteur)
                .await?;
            publier_tx(
                &mut tx,
                etab,
                types::PERSONNE_COORDONNEES_MODIFIEES,
                AggregateKind::Personne,
                pid,
                None,
                json!({ "personne_id": pid, "coordonnee_id": coord_id, "types": [c.type_], "nature": "divergence_resolue" }),
                auteur.actor(),
            )
            .await?;
        }
        sqlx::query(
            "INSERT INTO ent_personne_divergence_decision
                (id, etablissement_id, personne_id, champ, empreinte, choix, motif, decide_par, decide_via)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             ON CONFLICT (personne_id, champ) DO UPDATE SET empreinte = $5, choix = $6, motif = $7,
                decide_par = $8, decide_via = $9, decide_le = now()",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(pid)
        .bind(champ)
        .bind(&empreinte)
        .bind(choix.code())
        .bind(&motif)
        .bind(auteur.actor())
        .bind(auteur.via())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        journal_tx(
            &mut tx,
            etab,
            None,
            "divergence_compte_resolue",
            "Divergence compte / fiche résolue",
            json!({ "personne_id": pid, "champ": champ, "choix": choix.code(), "via": auteur.via() }),
            false,
            auteur.actor(),
        )
        .await?;
        commit(tx).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn resoudre_divergence_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        champ: &str,
        choix: ChoixDivergence,
        motif: &Option<String>,
    ) -> Result<(), EntDbError> {
        self.divergence_resoudre(etab, &Auteur::Personnel { actor, role }, Some(personne_id), champ, choix, motif)
            .await
    }

    pub async fn resoudre_ma_divergence(
        &self,
        etab: Uuid,
        user: Uuid,
        champ: &str,
        choix: ChoixDivergence,
        motif: &Option<String>,
    ) -> Result<(), EntDbError> {
        self.divergence_resoudre(etab, &Auteur::Personne { actor: user }, None, champ, choix, motif).await
    }

    // -- Identité (ADM-03) ---------------------------------------------------------

    /// Corrige ou change l'identité d'une personne. Le motif est obligatoire ; un changement de
    /// prénom ou de civilité est « sensible » : les anciennes valeurs sont scellées (R-A28).
    #[allow(clippy::too_many_arguments)]
    pub async fn modifier_identite_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        saisie: &IdentiteSaisie,
        nature: NatureIdentite,
        effet_du: Option<NaiveDate>,
        motif: &str,
    ) -> Result<Personne, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer).await?;
        self.exiger_pas_de_conflit_personne(etab, actor, role, personne_id).await?;
        let n = identite_normalisee(saisie)?;
        let motif = motif_valide(&Some(motif.to_string()))?
            .ok_or_else(|| refus_invalide("le motif est obligatoire pour corriger ou changer une identité"))?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let avant = personne_tx(&mut tx, etab, personne_id, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if avant.fusionnee_dans.is_some() {
            return Err(refus_conflit("personne fusionnée dans une autre fiche"));
        }
        let (code, sensible) = match nature {
            NatureIdentite::Correction => ("correction", false),
            NatureIdentite::Changement => ("changement", true),
        };
        let champs = appliquer_identite_tx(&mut tx, etab, actor, "personnel", &avant, &n, code, sensible, effet_du, &motif)
            .await?;
        if champs.is_empty() {
            return Err(invalide("aucun changement d'identité"));
        }
        let p = personne_tx(&mut tx, etab, personne_id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        commit(tx).await?;
        Ok(p)
    }

    /// Historique d'identité. Les anciennes valeurs scellées ne sont lues qu'avec
    /// `responsables.protege.lire` ; la lecture est alors tracée comme accès sensible.
    pub async fn historique_identite_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
    ) -> Result<Vec<VersionIdentite>, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer).await?;
        let (existe,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_personne WHERE etablissement_id = $1 AND id = $2)",
        )
        .bind(etab)
        .bind(personne_id)
        .fetch_one(self.pool())
        .await
        .map_err(db)?;
        if !existe {
            return Err(EntDbError::Domain(DomainError::NotFound));
        }
        let protege = self
            .a_action_etablissement(etab, actor, role, Action::ResponsablesProtegeLire)
            .await?
            && !self
                .personnes_en_conflit(etab, actor, role, &[personne_id])
                .await?
                .contains(&personne_id);
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            nom: Option<String>,
            prenom: Option<String>,
            civilite: Option<String>,
            nature: String,
            effet_du: NaiveDate,
            motif: Option<String>,
            sensible: bool,
            masquee: bool,
            valeurs_anterieures: Option<String>,
            enregistre_le: DateTime<Utc>,
        }
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, nom, prenom, civilite, nature, effet_du, motif, sensible, masquee,
                    valeurs_anterieures, enregistre_le
               FROM ent_personne_version WHERE etablissement_id = $1 AND personne_id = $2
              ORDER BY enregistre_le DESC, id DESC",
        )
        .bind(etab)
        .bind(personne_id)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let mut out = Vec::new();
        let mut a_lu_protege = false;
        for r in rows {
            let anterieures = match (&r.valeurs_anterieures, protege) {
                (Some(s), true) => {
                    a_lu_protege = true;
                    let clair = coffre()
                        .ouvrir(TABLE_VERSION, "valeurs_anterieures", r.id, s)
                        .map_err(EntDbError::Domain)?;
                    serde_json::from_str::<Vec<AncienneIdentite>>(&clair).ok()
                }
                _ => None,
            };
            out.push(VersionIdentite {
                id: r.id,
                nom: r.nom,
                prenom: r.prenom,
                civilite: r.civilite,
                nature: r.nature,
                effet_du: r.effet_du,
                motif: r.motif,
                sensible: r.sensible,
                masquee: r.masquee,
                enregistre_le: r.enregistre_le,
                anterieures,
            });
        }
        if a_lu_protege {
            self.tracer_acces_sensible(
                etab,
                actor,
                role,
                None,
                "dossier",
                "lecture_identite_protegee",
                "Lecture des anciennes valeurs d'identité d'une personne",
            )
            .await;
        }
        Ok(out)
    }

    // -- ADM-10 : séparer nom et prénom -------------------------------------------

    /// Personnes dont le nom est probablement « Prénom Nom » dans un seul champ (reprise des
    /// comptes), avec un découpage **proposé**.
    pub async fn propositions_separation_noms_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        limite: i64,
    ) -> Result<Vec<PropositionSeparation>, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer).await?;
        let rows: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id, nom FROM ent_personne
              WHERE etablissement_id = $1 AND fusionnee_dans IS NULL AND eleve_id IS NULL
                AND (prenom IS NULL OR btrim(prenom) = '') AND btrim(nom) ~ '[[:space:]]'
              ORDER BY lower(nom), id LIMIT $2",
        )
        .bind(etab)
        .bind(limite.clamp(1, 500))
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, nom)| {
                let (n, p, a_verifier) = proposer_separation(&nom)?;
                Some(PropositionSeparation {
                    personne_id: id,
                    nom_actuel: nom,
                    nom_propose: n,
                    prenom_propose: p,
                    a_verifier,
                })
            })
            .collect())
    }

    /// Applique, en lot et en une transaction, les séparations **validées par une personne**.
    /// Chaque séparation doit reprendre exactement les mêmes mots que le nom actuel (aucune lettre
    /// ajoutée ni retirée) ; les fiches déjà dotées d'un prénom sont refusées.
    pub async fn separer_noms_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        items: &[SeparationNom],
    ) -> Result<usize, EntDbError> {
        self.dossier_exiger_action(etab, actor, role, Action::PersonnesGerer).await?;
        if items.is_empty() || items.len() > 200 {
            return Err(invalide("de 1 à 200 séparations par lot"));
        }
        let ids: Vec<Uuid> = items.iter().map(|i| i.personne_id).collect();
        let en_conflit = self.personnes_en_conflit(etab, actor, role, &ids).await?;
        if !en_conflit.is_empty() {
            return Err(EntDbError::ConflitDossier(
                "une des personnes concerne un dossier où vous êtes en conflit d'intérêts",
            ));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let mut vus = std::collections::HashSet::new();
        for it in items {
            if !vus.insert(it.personne_id) {
                return Err(invalide("une personne ne peut figurer qu'une fois dans le lot"));
            }
            let avant = personne_tx(&mut tx, etab, it.personne_id, true)
                .await?
                .ok_or(EntDbError::Domain(DomainError::NotFound))?;
            if avant.fusionnee_dans.is_some() {
                return Err(refus_conflit("personne fusionnée dans une autre fiche"));
            }
            if avant.prenom.as_deref().is_some_and(|p| !p.trim().is_empty()) {
                return Err(refus_conflit("cette fiche a déjà un prénom"));
            }
            let nom = it.nom.trim();
            let prenom = it.prenom.trim();
            if nom.is_empty() || prenom.is_empty() {
                return Err(invalide("le nom et le prénom sont obligatoires"));
            }
            let mut apres = mots_tries(&format!("{nom} {prenom}"));
            apres.sort();
            if apres != mots_tries(&avant.nom) {
                return Err(invalide("la séparation doit reprendre exactement les mots du nom actuel"));
            }
            let nouveau = identite_normalisee(&IdentiteSaisie {
                nom: nom.to_string(),
                prenom: Some(prenom.to_string()),
                civilite: avant.civilite.clone(),
            })?;
            appliquer_identite_tx(
                &mut tx,
                etab,
                actor,
                "personnel",
                &avant,
                &nouveau,
                "separation_noms",
                false,
                None,
                "Séparation du nom et du prénom (reprise des comptes)",
            )
            .await?;
        }
        commit(tx).await?;
        Ok(items.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separation_prenom_nom_et_nom_prenom() {
        assert_eq!(
            proposer_separation("Marie Dupont"),
            Some(("Dupont".into(), "Marie".into(), false))
        );
        assert_eq!(
            proposer_separation("DUPONT Marie"),
            Some(("DUPONT".into(), "Marie".into(), false))
        );
        assert_eq!(
            proposer_separation("MARTIN DUPONT Jean-Pierre"),
            Some(("MARTIN DUPONT".into(), "Jean-Pierre".into(), true))
        );
        assert_eq!(
            proposer_separation("Marie de la Fontaine"),
            Some(("de la Fontaine".into(), "Marie".into(), false))
        );
        // Plus de deux mots sans particule : ambigu, à relire.
        assert_eq!(
            proposer_separation("Jean Pierre Dupont"),
            Some(("Pierre Dupont".into(), "Jean".into(), true))
        );
        assert_eq!(proposer_separation("Dupont"), None);
        assert_eq!(proposer_separation("   "), None);
    }

    #[test]
    fn joignable_borne() {
        let ok = json!([{"jours":[1,2,3],"de":"08:00","a":"12:30"}]);
        assert_eq!(valider_joignable(&ok).unwrap(), ok);
        for mauvais in [
            json!("lundi"),
            json!([{"jours":[],"de":"08:00","a":"12:00"}]),
            json!([{"jours":[8],"de":"08:00","a":"12:00"}]),
            json!([{"jours":[1],"de":"12:00","a":"08:00"}]),
            json!([{"jours":[1],"de":"8h","a":"12:00"}]),
            json!([{"jours":[1],"de":"25:00","a":"26:00"}]),
            Value::Array((0..15).map(|_| json!({"jours":[1],"de":"08:00","a":"09:00"})).collect()),
        ] {
            assert!(valider_joignable(&mauvais).is_err(), "{mauvais}");
        }
    }

    #[test]
    fn preferences_normalisees() {
        let base = PreferencesPersonne::default();
        let n = normaliser_preferences(&PreferencesPersonne {
            langue: Some(" FA-af ".into()),
            canaux_refuses: vec!["sms".into(), "sms".into()],
            accessibilite: vec!["gros_caracteres".into()],
            ..base.clone()
        })
        .unwrap();
        assert_eq!(n.langue.as_deref(), Some("fa-af"));
        assert_eq!(n.canaux_refuses, vec!["sms".to_string()]);
        assert!(normaliser_preferences(&PreferencesPersonne { langue: Some("francais".into()), ..base.clone() }).is_err());
        assert!(normaliser_preferences(&PreferencesPersonne { canaux_refuses: vec!["pigeon".into()], ..base.clone() }).is_err());
        // Jamais « oublié » : refuser e-mail et SMS exige le courrier.
        let tout_refuse = PreferencesPersonne { canaux_refuses: vec!["email".into(), "sms".into()], ..base.clone() };
        assert!(normaliser_preferences(&tout_refuse).is_err());
        assert!(normaliser_preferences(&PreferencesPersonne { courrier_papier: true, ..tout_refuse }).is_ok());
    }

    #[test]
    fn choix_divergence_codes() {
        for c in [ChoixDivergence::DistinctVoulu, ChoixDivergence::ConserverFiche, ChoixDivergence::AdopterCompte] {
            assert_eq!(ChoixDivergence::parse(c.code()), Some(c));
        }
        assert_eq!(ChoixDivergence::parse("ecraser"), None);
    }

    #[test]
    fn consentements_versions_connues() {
        assert!(CONSENTEMENTS.iter().all(|(_, v)| !v.is_empty()));
    }
}
