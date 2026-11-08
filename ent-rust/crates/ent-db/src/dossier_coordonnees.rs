//! Lot A2 du socle 4 (`docs/socles/04_ADMINISTRATIF.md` §8.4, §7, ADM-05) : les coordonnées d'une
//! personne (e-mails, téléphones, adresse postale de contact) sont portées par la **fiche de la
//! personne**, pas par son compte.
//!
//! * Une ligne par coordonnée, **datée** : une correction ou un remplacement ferme la ligne et en
//!   ouvre une autre, rien ne s'écrase. L'historique (qui, quand, pourquoi) est dans
//!   `ent_personne_coordonnee_histo` et ne contient **jamais** de valeur.
//! * Numéros normalisés en E.164 ([`normaliser_telephone`], miroir testé de la fonction SQL
//!   `ent_tel_e164`).
//! * Vérification par code (e-mail, SMS) : empreinte HMAC seule en base, cinq essais, quinze minutes.
//!   Une coordonnée saisie par la famille n'est jamais principale avant d'être vérifiée (sauf si
//!   elle est la première de son genre) ; un changement fait par la famille ne remplace l'ancienne
//!   valeur qu'à la vérification.
//! * Règles des personas : une adresse à protéger n'est jamais écrite en clair (refus si une adresse
//!   protégée attend la direction) ; une personne « ne pas solliciter » ne reçoit aucun code à
//!   l'initiative de l'établissement ; aucune valeur dans les événements ni les journaux.
//! * Les colonnes héritées de `ent_personne` sont une projection maintenue par la base
//!   (migration 274) ; un écrivain historique ne perd aucune coordonnée.

use super::*;
use crate::service::RefusMetier;
use ent_mail::OutgoingMail;
use ent_sms::OutgoingSms;

pub const TYPES_COORDONNEE: [&str; 5] = ["email", "mobile", "fixe", "professionnel", "adresse"];
/// Usages qu'une commande peut poser (`connexion` n'appartient qu'au compte).
pub const USAGES_COORDONNEE: [&str; 3] = ["notifications", "urgence", "facturation"];
/// Nombre maximal de coordonnées ouvertes par personne.
pub const COORDONNEES_MAX: i64 = 12;
/// Validité d'un code de vérification, essais, demandes par heure et par coordonnée.
pub const VERIFICATION_TTL_MINUTES: i64 = 15;
pub const VERIFICATION_ESSAIS_MAX: i16 = 5;
pub const VERIFICATION_DEMANDES_PAR_HEURE: i64 = 3;

pub(crate) fn refus_invalide(msg: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Invalide, msg)
}
pub(crate) fn refus_conflit(msg: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Conflit, msg)
}
pub(crate) fn refus_interdit(msg: &'static str) -> EntDbError {
    EntDbError::Refus(RefusMetier::Interdit, msg)
}

// ---------------------------------------------------------------------------
// Normalisation (fonctions pures, testées sans base)
// ---------------------------------------------------------------------------

/// Numéro de téléphone en E.164 (`+33612345678`), ou `None` quand il n'est pas sûr. Accepte les
/// formats nationaux français (`06 12 34 56 78`), `00` et `+` internationaux, le zéro de tronc
/// `+33 (0)6…`. Même règle que la fonction SQL `ent_tel_e164` (test de concordance).
pub fn normaliser_telephone(brut: &str) -> Option<String> {
    let mut s: String = brut
        .chars()
        .filter(|c| !(c.is_whitespace() || ".-()/".contains(*c)))
        .collect();
    if s.is_empty() {
        return None;
    }
    if let Some(reste) = s.strip_prefix("00") {
        s = format!("+{reste}");
    }
    let chiffres = |t: &str| t.chars().all(|c| c.is_ascii_digit());
    if let Some(reste) = s.strip_prefix("+330") {
        if reste.len() == 9 && chiffres(reste) && !reste.starts_with('0') {
            s = format!("+33{reste}");
        }
    }
    if let Some(reste) = s.strip_prefix("+33") {
        let ok = reste.len() == 9 && chiffres(reste) && !reste.starts_with('0');
        return ok.then_some(s);
    }
    if let Some(reste) = s.strip_prefix('+') {
        let ok = (8..=15).contains(&reste.len()) && chiffres(reste) && !reste.starts_with('0');
        return ok.then_some(s);
    }
    if s.len() == 10 && chiffres(&s) && s.starts_with('0') && !s[1..].starts_with('0') {
        return Some(format!("+33{}", &s[1..]));
    }
    None
}

/// `mobile` ou `fixe` d'après un numéro E.164 (les mobiles français commencent par 6 ou 7 ; un
/// numéro étranger est classé `mobile` par défaut).
pub fn classer_telephone(e164: &str) -> &'static str {
    if e164.starts_with("+336") || e164.starts_with("+337") {
        "mobile"
    } else if e164.starts_with("+33") {
        "fixe"
    } else {
        "mobile"
    }
}

/// E-mail en minuscules, forme contrôlée.
pub fn normaliser_email(brut: &str) -> Result<String, EntDbError> {
    let e = brut.trim().to_lowercase();
    let ok = !e.is_empty()
        && e.len() <= 254
        && e.split_once('@')
            .map(|(a, d)| {
                !a.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.') && !d.contains('@')
            })
            .unwrap_or(false)
        && !e.chars().any(char::is_whitespace);
    if ok {
        Ok(e)
    } else {
        Err(invalide("adresse e-mail mal formée"))
    }
}

/// Adresse postale de contact : lignes nettoyées, 5 à 500 caractères.
pub fn normaliser_adresse(brut: &str) -> Result<String, EntDbError> {
    let lignes: Vec<String> = brut
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect();
    let a = lignes.join("\n");
    let n = a.chars().count();
    if !(5..=500).contains(&n) || a.chars().any(|c| c.is_control() && c != '\n') {
        return Err(invalide("adresse postale de 5 à 500 caractères"));
    }
    Ok(a)
}

/// `j•••@gmail.com` : pour les listes (jamais la valeur entière hors fiche).
pub fn masque_courriel(e: &str) -> String {
    match e.split_once('@') {
        Some((a, d)) => format!("{}•••@{d}", a.chars().next().map(String::from).unwrap_or_default()),
        None => "••••".into(),
    }
}

/// `+33•••••••78`.
pub fn masque_telephone(t: &str) -> String {
    let c: Vec<char> = t.chars().collect();
    if c.len() <= 6 {
        return "••••".into();
    }
    let debut: String = c[..3].iter().collect();
    let fin: String = c[c.len() - 2..].iter().collect();
    format!("{debut}{}{fin}", "•".repeat(c.len() - 5))
}

fn groupe_coordonnee(type_: &str) -> &'static str {
    match type_ {
        "email" => "email",
        "adresse" => "adresse",
        _ => "telephone",
    }
}


// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Pourquoi une coordonnée change (code obligatoire ; le texte libre est facultatif).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaisonCoordonnee {
    ErreurDeSaisie,
    ChangementDeclare,
    DemandeDeLaPersonne,
    Autre,
}

impl RaisonCoordonnee {
    pub fn code(self) -> &'static str {
        match self {
            Self::ErreurDeSaisie => "erreur_de_saisie",
            Self::ChangementDeclare => "changement_declare",
            Self::DemandeDeLaPersonne => "demande_de_la_personne",
            Self::Autre => "autre",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "erreur_de_saisie" => Self::ErreurDeSaisie,
            "changement_declare" => Self::ChangementDeclare,
            "demande_de_la_personne" => Self::DemandeDeLaPersonne,
            "autre" => Self::Autre,
            _ => return None,
        })
    }
}

/// Qui agit : le personnel (droit `personnes.gerer`) ou la personne elle-même (« soi »).
#[derive(Debug, Clone, Copy)]
pub enum Auteur<'a> {
    Personnel { actor: Uuid, role: &'a str },
    Personne { actor: Uuid },
}

impl Auteur<'_> {
    pub fn actor(&self) -> Uuid {
        match self {
            Self::Personnel { actor, .. } | Self::Personne { actor } => *actor,
        }
    }
    pub fn via(&self) -> &'static str {
        match self {
            Self::Personnel { .. } => "personnel",
            Self::Personne { .. } => "personne",
        }
    }
    fn est_personne(&self) -> bool {
        matches!(self, Self::Personne { .. })
    }
}

/// Coordonnée datée d'une personne.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Coordonnee {
    pub id: Uuid,
    pub personne_id: Uuid,
    #[sqlx(rename = "type")]
    pub type_: String,
    pub valeur: String,
    pub libelle: Option<String>,
    pub usages: Vec<String>,
    pub principal: bool,
    pub verifiee_le: Option<DateTime<Utc>>,
    pub verification_voie: Option<String>,
    pub a_verifier: bool,
    pub delivrabilite: String,
    pub source: String,
    pub du: DateTime<Utc>,
    pub au: Option<DateTime<Utc>>,
    pub remplace_id: Option<Uuid>,
}

const COLS_COORD: &str = "c.id, c.personne_id, c.type, c.valeur, c.libelle, c.usages, c.principal, \
     c.verifiee_le, c.verification_voie, c.a_verifier, c.delivrabilite, c.source, c.du, c.au, c.remplace_id";

/// Coordonnées lues, avec l'indication des adresses fermées pour ce lecteur (F2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordonneesPersonne {
    pub coordonnees: Vec<Coordonnee>,
    pub adresses_masquees: bool,
}

/// Saisie d'une coordonnée (ajout).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordonneeSaisie {
    pub type_: String,
    pub valeur: String,
    pub libelle: Option<String>,
    /// Vide : `notifications`.
    pub usages: Vec<String>,
    /// Le personnel peut demander à en faire le principal ; une famille jamais (vérification d'abord).
    pub principal: bool,
    pub raison: RaisonCoordonnee,
    pub motif: Option<String>,
}

/// Remplacement d'une valeur : faute de frappe (`correction`) ou changement réel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatureRemplacement {
    Correction,
    Remplacement,
}

/// Entrée de l'historique des coordonnées : jamais de valeur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoriqueCoordonnee {
    pub id: Uuid,
    pub coordonnee_id: Uuid,
    pub type_: String,
    pub nature: String,
    pub raison: String,
    pub motif: Option<String>,
    pub via: String,
    /// Nom de l'auteur, pour le personnel seulement (jamais montré à une famille).
    pub par_nom: Option<String>,
    pub quand: DateTime<Utc>,
}

/// Code envoyé : jamais renvoyé à l'appelant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationEnvoyee {
    pub coordonnee_id: Uuid,
    pub canal: &'static str,
    pub expire_le: DateTime<Utc>,
}

struct SaisieNormalisee {
    type_: String,
    valeur: String,
    libelle: Option<String>,
    usages: Vec<String>,
}

fn normaliser_usages(u: &[String]) -> Result<Vec<String>, EntDbError> {
    let mut out: Vec<String> = Vec::new();
    for x in u {
        let x = x.trim();
        if !USAGES_COORDONNEE.contains(&x) {
            return Err(invalide("usage inconnu (notifications, urgence, facturation)"));
        }
        if !out.iter().any(|y| y == x) {
            out.push(x.to_string());
        }
    }
    if out.is_empty() {
        out.push("notifications".into());
    }
    Ok(out)
}

fn normaliser_valeur(type_: &str, valeur: &str) -> Result<(String, String), EntDbError> {
    match type_ {
        "email" => Ok(("email".into(), normaliser_email(valeur)?)),
        "adresse" => Ok(("adresse".into(), normaliser_adresse(valeur)?)),
        "mobile" | "fixe" | "professionnel" => {
            let n = normaliser_telephone(valeur).ok_or_else(|| {
                invalide("numéro de téléphone non reconnu (indiquer l'indicatif du pays, ex. +33)")
            })?;
            let reel = classer_telephone(&n);
            if type_ == "mobile" && n.starts_with("+33") && reel != "mobile" {
                return Err(invalide("ce numéro n'est pas un mobile"));
            }
            if type_ == "fixe" && reel == "mobile" && (n.starts_with("+336") || n.starts_with("+337")) {
                return Err(invalide("ce numéro est un mobile"));
            }
            Ok((type_.to_string(), n))
        }
        _ => Err(invalide("type de coordonnée inconnu")),
    }
}

fn normaliser_saisie(s: &CoordonneeSaisie) -> Result<SaisieNormalisee, EntDbError> {
    let (type_, valeur) = normaliser_valeur(s.type_.trim(), &s.valeur)?;
    let libelle = s
        .libelle
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string);
    if libelle.as_ref().is_some_and(|l| l.chars().count() > 80) {
        return Err(invalide("libellé de 80 caractères au plus"));
    }
    Ok(SaisieNormalisee {
        type_,
        valeur,
        libelle,
        usages: normaliser_usages(&s.usages)?,
    })
}

pub(crate) fn motif_valide(m: &Option<String>) -> Result<Option<String>, EntDbError> {
    match m.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) if (3..=500).contains(&s.chars().count()) => Ok(Some(s.to_string())),
        Some(_) => Err(invalide("motif de 3 à 500 caractères")),
    }
}

// ---------------------------------------------------------------------------
// Accès à la base (dans la transaction de l'appelant)
// ---------------------------------------------------------------------------

/// Attribue les écritures de la transaction à l'auteur : l'historique, y compris celui des
/// déclencheurs, porte alors « qui » et « pourquoi ».
pub(crate) async fn attribuer_tx(
    conn: &mut PgConnection,
    auteur: &Auteur<'_>,
    raison: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        "SELECT set_config('ent.coord_acteur', $1, true), set_config('ent.coord_via', $2, true),
                set_config('ent.coord_raison', $3, true)",
    )
    .bind(auteur.actor().to_string())
    .bind(auteur.via())
    .bind(raison)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn histo_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    coord: &Coordonnee,
    nature: &str,
    raison: &str,
    motif: Option<&str>,
    auteur: &Auteur<'_>,
) -> Result<(), EntDbError> {
    sqlx::query(
        "INSERT INTO ent_personne_coordonnee_histo
            (id, etablissement_id, personne_id, coordonnee_id, type, nature, raison, motif, via, par)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(personne_id)
    .bind(coord.id)
    .bind(&coord.type_)
    .bind(nature)
    .bind(raison)
    .bind(motif)
    .bind(auteur.via())
    .bind(auteur.actor())
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

async fn coord_lire_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<Option<Coordonnee>, EntDbError> {
    let sql = format!(
        "SELECT {COLS_COORD} FROM ent_personne_coordonnee c
          WHERE c.etablissement_id = $1 AND c.personne_id = $2 AND c.id = $3{}",
        if verrou { " FOR UPDATE" } else { "" }
    );
    sqlx::query_as::<_, Coordonnee>(&sql)
        .bind(etab)
        .bind(personne_id)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(db)
}

async fn groupe_a_principal_tx(
    conn: &mut PgConnection,
    personne_id: Uuid,
    type_: &str,
) -> Result<bool, EntDbError> {
    let (v,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_personne_coordonnee
                         WHERE personne_id = $1 AND au IS NULL AND principal
                           AND ent_coord_groupe(type) = ent_coord_groupe($2))",
    )
    .bind(personne_id)
    .bind(type_)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    Ok(v)
}

async fn retirer_principal_tx(
    conn: &mut PgConnection,
    personne_id: Uuid,
    type_: &str,
) -> Result<(), EntDbError> {
    sqlx::query(
        "UPDATE ent_personne_coordonnee SET principal = false
          WHERE personne_id = $1 AND au IS NULL AND principal
            AND ent_coord_groupe(type) = ent_coord_groupe($2)",
    )
    .bind(personne_id)
    .bind(type_)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Après la fermeture du principal : la première coordonnée éligible prend sa place.
async fn promouvoir_suivant_tx(
    conn: &mut PgConnection,
    personne_id: Uuid,
    type_: &str,
) -> Result<(), EntDbError> {
    if groupe_a_principal_tx(conn, personne_id, type_).await? {
        return Ok(());
    }
    sqlx::query(
        "UPDATE ent_personne_coordonnee SET principal = true
          WHERE id = (SELECT id FROM ent_personne_coordonnee
                       WHERE personne_id = $1 AND au IS NULL
                         AND ent_coord_groupe(type) = ent_coord_groupe($2)
                         AND 'notifications' = ANY (usages)
                         AND delivrabilite NOT IN ('rebond_definitif', 'non_attribue', 'desinscrit')
                       ORDER BY du ASC, id ASC LIMIT 1)",
    )
    .bind(personne_id)
    .bind(type_)
    .execute(conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Une adresse protégée attend la direction : aucune adresse en clair n'est écrite en attendant.
async fn exiger_adresse_ecrivable_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    personne_id: Uuid,
) -> Result<(), EntDbError> {
    let (v,): (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM ent_adresse_protegee
                         WHERE etablissement_id = $1 AND personne_id = $2)",
    )
    .bind(etab)
    .bind(personne_id)
    .fetch_one(conn)
    .await
    .map_err(db)?;
    if v {
        return Err(refus_interdit(
            "adresse protégée : aucune adresse n'est enregistrée ici, voir la direction",
        ));
    }
    Ok(())
}

/// Journal (noms et codes seulement) puis événement : jamais de valeur.
async fn tracer_coordonnee_tx(
    conn: &mut PgConnection,
    etab: Uuid,
    auteur: &Auteur<'_>,
    personne_id: Uuid,
    coord_id: Uuid,
    type_: &str,
    nature: &str,
) -> Result<(), EntDbError> {
    journal_tx(
        conn,
        etab,
        None,
        "coordonnee_modifiee",
        "Coordonnée d'une personne modifiée",
        json!({ "personne_id": personne_id, "coordonnee_id": coord_id, "type": type_, "nature": nature,
                "via": auteur.via() }),
        false,
        auteur.actor(),
    )
    .await?;
    publier_tx(
        conn,
        etab,
        types::PERSONNE_COORDONNEES_MODIFIEES,
        AggregateKind::Personne,
        personne_id,
        None,
        json!({ "personne_id": personne_id, "coordonnee_id": coord_id, "types": [type_], "nature": nature }),
        auteur.actor(),
    )
    .await
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

impl PgEnt {
    /// La personne visée : par le personnel (droits, conflit d'intérêts) ou « soi ».
    pub(crate) async fn coord_resoudre_personne(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        ecriture: bool,
    ) -> Result<Uuid, EntDbError> {
        match auteur {
            Auteur::Personnel { actor, role } => {
                let pid = personne_id.ok_or(EntDbError::Domain(DomainError::NotFound))?;
                let ok = self
                    .a_action_etablissement(etab, *actor, role, Action::PersonnesGerer)
                    .await?
                    || (!ecriture
                        && self
                            .a_action_etablissement(etab, *actor, role, Action::ResponsablesLire)
                            .await?);
                if !ok {
                    return Err(EntDbError::Forbidden);
                }
                // Une personne d'un autre établissement est indiscernable d'une personne inconnue.
                let (existe,): (bool,) = sqlx::query_as(
                    "SELECT EXISTS (SELECT 1 FROM ent_personne WHERE etablissement_id = $1 AND id = $2)",
                )
                .bind(etab)
                .bind(pid)
                .fetch_one(self.pool())
                .await
                .map_err(db)?;
                if !existe {
                    return Err(EntDbError::Domain(DomainError::NotFound));
                }
                if ecriture {
                    self.exiger_pas_de_conflit_personne(etab, *actor, role, pid).await?;
                }
                Ok(pid)
            }
            Auteur::Personne { actor } => {
                let r: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM ent_personne
                      WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL",
                )
                .bind(etab)
                .bind(actor)
                .fetch_optional(self.pool())
                .await
                .map_err(db)?;
                r.map(|(id,)| id).ok_or(EntDbError::Domain(DomainError::NotFound))
            }
        }
    }

    /// L'auteur lit-il les adresses (F2) de cette personne ?
    async fn coord_adresses_visibles(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Uuid,
    ) -> Result<bool, EntDbError> {
        match auteur {
            Auteur::Personne { .. } => Ok(true),
            Auteur::Personnel { actor, role } => {
                if !self
                    .a_action_etablissement(etab, *actor, role, Action::ResponsablesAdressesLire)
                    .await?
                {
                    return Ok(false);
                }
                Ok(!self
                    .personnes_en_conflit(etab, *actor, role, &[personne_id])
                    .await?
                    .contains(&personne_id))
            }
        }
    }

    pub(crate) async fn coord_lire(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        avec_fermees: bool,
    ) -> Result<CoordonneesPersonne, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, false).await?;
        let adresses = self.coord_adresses_visibles(etab, auteur, pid).await?;
        let sql = format!(
            "SELECT {COLS_COORD} FROM ent_personne_coordonnee c
              WHERE c.etablissement_id = $1 AND c.personne_id = $2 AND ($3 OR c.au IS NULL)
              ORDER BY ent_coord_groupe(c.type), (c.au IS NOT NULL), c.principal DESC, c.du, c.id"
        );
        let mut coordonnees = sqlx::query_as::<_, Coordonnee>(&sql)
            .bind(etab)
            .bind(pid)
            .bind(avec_fermees)
            .fetch_all(self.pool())
            .await
            .map_err(db)?;
        if !adresses {
            coordonnees.retain(|c| c.type_ != "adresse");
        }
        if let Auteur::Personnel { actor, .. } = auteur {
            // Lecture de coordonnées d'une autre personne : trace d'accès (au plus une par jour
            // et par lecteur serait préférable ; la lecture reste rare et réservée).
            tracing::debug!(%actor, personne = %pid, "lecture des coordonnées d'une personne");
        }
        Ok(CoordonneesPersonne {
            coordonnees,
            adresses_masquees: !adresses,
        })
    }

    pub(crate) async fn coord_ajouter(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        saisie: &CoordonneeSaisie,
    ) -> Result<Coordonnee, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        let n = normaliser_saisie(saisie)?;
        let motif = motif_valide(&saisie.motif)?;
        if auteur.est_personne() && n.type_ == "adresse" {
            return Err(refus_interdit(
                "le changement d'adresse postale passe par une demande au secrétariat",
            ));
        }
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if p.fusionnee_dans.is_some() {
            return Err(refus_conflit("personne fusionnée dans une autre fiche"));
        }
        if n.type_ == "adresse" {
            exiger_adresse_ecrivable_tx(&mut tx, etab, pid).await?;
        }
        attribuer_tx(&mut tx, auteur, saisie.raison.code()).await?;
        let (ouvertes,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_personne_coordonnee WHERE personne_id = $1 AND au IS NULL",
        )
        .bind(pid)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if ouvertes >= COORDONNEES_MAX {
            return Err(refus_conflit("trop de coordonnées enregistrées pour cette personne"));
        }
        let (deja,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_personne_coordonnee
                             WHERE personne_id = $1 AND au IS NULL AND type = $2 AND valeur = $3)",
        )
        .bind(pid)
        .bind(&n.type_)
        .bind(&n.valeur)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(refus_conflit("cette coordonnée est déjà enregistrée"));
        }
        let a_principal = groupe_a_principal_tx(&mut tx, pid, &n.type_).await?;
        let eligible = n.usages.iter().any(|u| u == "notifications");
        let principal = eligible && (!a_principal || (saisie.principal && !auteur.est_personne()));
        if principal && a_principal {
            retirer_principal_tx(&mut tx, pid, &n.type_).await?;
        }
        let id = Uuid::new_v4();
        let source = if auteur.est_personne() { "famille" } else { "saisie" };
        sqlx::query(
            "INSERT INTO ent_personne_coordonnee
                (id, etablissement_id, personne_id, type, valeur, libelle, usages, principal, source,
                 cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(etab)
        .bind(pid)
        .bind(&n.type_)
        .bind(&n.valeur)
        .bind(&n.libelle)
        .bind(&n.usages)
        .bind(principal)
        .bind(source)
        .bind(auteur.actor())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let c = coord_lire_tx(&mut tx, etab, pid, id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        histo_tx(&mut tx, etab, pid, &c, "ajout", saisie.raison.code(), motif.as_deref(), auteur).await?;
        tracer_coordonnee_tx(&mut tx, etab, auteur, pid, id, &n.type_, "ajout").await?;
        commit(tx).await?;
        Ok(c)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn coord_remplacer(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        coord_id: Uuid,
        valeur: &str,
        nature: NatureRemplacement,
        raison: RaisonCoordonnee,
        motif: &Option<String>,
    ) -> Result<Coordonnee, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        let motif = motif_valide(motif)?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        let p = personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if p.fusionnee_dans.is_some() {
            return Err(refus_conflit("personne fusionnée dans une autre fiche"));
        }
        let ancienne = coord_lire_tx(&mut tx, etab, pid, coord_id, true)
            .await?
            .filter(|c| c.au.is_none())
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        if ancienne.usages.iter().any(|u| u == "connexion") && ancienne.source == "compte" {
            return Err(refus_conflit(
                "cette coordonnée est celle du compte : elle change par le compte lui-même",
            ));
        }
        if auteur.est_personne() && ancienne.type_ == "adresse" {
            return Err(refus_interdit(
                "le changement d'adresse postale passe par une demande au secrétariat",
            ));
        }
        if auteur.est_personne() && nature == NatureRemplacement::Correction {
            return Err(refus_interdit("une correction est faite par le secrétariat"));
        }
        let type_voulu = if ancienne.type_ == "professionnel" { "professionnel" } else { ancienne.type_.as_str() };
        let (type_, nouvelle) = if groupe_coordonnee(&ancienne.type_) == "telephone" {
            let n = normaliser_telephone(valeur).ok_or_else(|| {
                invalide("numéro de téléphone non reconnu (indiquer l'indicatif du pays, ex. +33)")
            })?;
            let t = if type_voulu == "professionnel" { "professionnel".to_string() } else { classer_telephone(&n).to_string() };
            (t, n)
        } else {
            normaliser_valeur(&ancienne.type_, valeur)?
        };
        if nouvelle == ancienne.valeur {
            return Err(invalide("la valeur n'a pas changé"));
        }
        if type_ == "adresse" {
            exiger_adresse_ecrivable_tx(&mut tx, etab, pid).await?;
        }
        attribuer_tx(&mut tx, auteur, raison.code()).await?;
        let (deja,): (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_personne_coordonnee
                             WHERE personne_id = $1 AND au IS NULL AND type = $2 AND valeur = $3)",
        )
        .bind(pid)
        .bind(&type_)
        .bind(&nouvelle)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if deja {
            return Err(refus_conflit("cette coordonnée est déjà enregistrée"));
        }
        let mut usages: Vec<String> = ancienne.usages.iter().filter(|u| *u != "connexion").cloned().collect();
        if usages.is_empty() {
            usages.push("notifications".into());
        }
        let verifiable = type_ == "email" || type_ == "mobile";
        // La famille ne remplace qu'après vérification d'un code (e-mail, mobile) ; le reste de ce
        // qu'elle change est immédiat (fixe, poste professionnel).
        let en_attente = auteur.est_personne() && verifiable;
        let nouvelle_id = Uuid::new_v4();
        let principal = ancienne.principal && !en_attente && usages.iter().any(|u| u == "notifications");
        if !en_attente {
            sqlx::query(
                "UPDATE ent_personne_coordonnee SET au = GREATEST(now(), du), principal = false WHERE id = $1",
            )
            .bind(ancienne.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        }
        let source = if auteur.est_personne() { "famille" } else { "saisie" };
        sqlx::query(
            "INSERT INTO ent_personne_coordonnee
                (id, etablissement_id, personne_id, type, valeur, libelle, usages, principal, source,
                 remplace_id, cree_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(nouvelle_id)
        .bind(etab)
        .bind(pid)
        .bind(&type_)
        .bind(&nouvelle)
        .bind(&ancienne.libelle)
        .bind(&usages)
        .bind(principal)
        .bind(source)
        .bind(ancienne.id)
        .bind(auteur.actor())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let c = coord_lire_tx(&mut tx, etab, pid, nouvelle_id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let nature_code = match nature {
            NatureRemplacement::Correction => "correction",
            NatureRemplacement::Remplacement => "remplacement",
        };
        histo_tx(&mut tx, etab, pid, &c, nature_code, raison.code(), motif.as_deref(), auteur).await?;
        tracer_coordonnee_tx(&mut tx, etab, auteur, pid, nouvelle_id, &type_, nature_code).await?;
        commit(tx).await?;
        Ok(c)
    }

    pub(crate) async fn coord_retirer(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        coord_id: Uuid,
        raison: RaisonCoordonnee,
        motif: &Option<String>,
    ) -> Result<(), EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        let motif = motif_valide(motif)?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let c = coord_lire_tx(&mut tx, etab, pid, coord_id, true)
            .await?
            .filter(|c| c.au.is_none())
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        attribuer_tx(&mut tx, auteur, raison.code()).await?;
        sqlx::query("UPDATE ent_personne_coordonnee SET au = GREATEST(now(), du), principal = false WHERE id = $1")
            .bind(c.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query(
            "UPDATE ent_coordonnee_verification SET statut = 'annulee'
              WHERE coordonnee_id = $1 AND statut = 'envoye'",
        )
        .bind(c.id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        if c.principal {
            promouvoir_suivant_tx(&mut tx, pid, &c.type_).await?;
        }
        histo_tx(&mut tx, etab, pid, &c, "retrait", raison.code(), motif.as_deref(), auteur).await?;
        tracer_coordonnee_tx(&mut tx, etab, auteur, pid, c.id, &c.type_, "retrait").await?;
        commit(tx).await?;
        Ok(())
    }

    /// Usages et rang de principal. `faire_principal` n'a d'effet que s'il est vrai : on ne
    /// rétrograde pas directement, on promeut une autre coordonnée.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn coord_modifier_usages(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        coord_id: Uuid,
        usages: &[String],
        faire_principal: bool,
    ) -> Result<Coordonnee, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        let nouveaux = normaliser_usages(usages)?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let c = coord_lire_tx(&mut tx, etab, pid, coord_id, true)
            .await?
            .filter(|c| c.au.is_none())
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        attribuer_tx(&mut tx, auteur, "changement_declare").await?;
        let mut finaux = nouveaux.clone();
        if c.usages.iter().any(|u| u == "connexion") {
            finaux.push("connexion".into());
        }
        let eligible = finaux.iter().any(|u| u == "notifications")
            && !matches!(
                c.delivrabilite.as_str(),
                "rebond_definitif" | "non_attribue" | "desinscrit"
            );
        let devient_principal = faire_principal && !c.principal;
        if devient_principal {
            if !eligible {
                return Err(refus_invalide(
                    "une coordonnée principale doit servir aux notifications et être joignable",
                ));
            }
            if auteur.est_personne() && (c.type_ == "email" || c.type_ == "mobile") && c.verifiee_le.is_none() {
                return Err(refus_conflit("vérifier cette coordonnée par un code avant d'en faire la principale"));
            }
            retirer_principal_tx(&mut tx, pid, &c.type_).await?;
        }
        let reste_principal = (c.principal && eligible) || devient_principal;
        sqlx::query("UPDATE ent_personne_coordonnee SET usages = $2, principal = $3 WHERE id = $1")
            .bind(c.id)
            .bind(&finaux)
            .bind(reste_principal)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        if c.principal && !reste_principal {
            promouvoir_suivant_tx(&mut tx, pid, &c.type_).await?;
        }
        let apres = coord_lire_tx(&mut tx, etab, pid, c.id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let nature = if devient_principal { "principal" } else { "usages" };
        histo_tx(&mut tx, etab, pid, &apres, nature, "changement_declare", None, auteur).await?;
        tracer_coordonnee_tx(&mut tx, etab, auteur, pid, c.id, &c.type_, nature).await?;
        commit(tx).await?;
        Ok(apres)
    }

    pub(crate) async fn coord_historique(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
    ) -> Result<Vec<HistoriqueCoordonnee>, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, false).await?;
        let adresses = self.coord_adresses_visibles(etab, auteur, pid).await?;
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            coordonnee_id: Uuid,
            #[sqlx(rename = "type")]
            type_: String,
            nature: String,
            raison: String,
            motif: Option<String>,
            via: String,
            par_nom: Option<String>,
            quand: DateTime<Utc>,
        }
        let rows = sqlx::query_as::<_, Row>(
            "SELECT h.id, h.coordonnee_id, h.type, h.nature, h.raison, h.motif, h.via,
                    u.display_name AS par_nom, h.quand
               FROM ent_personne_coordonnee_histo h
               LEFT JOIN ent_user u ON u.id = h.par AND h.via = 'personnel'
              WHERE h.etablissement_id = $1 AND h.personne_id = $2
              ORDER BY h.quand DESC, h.id DESC LIMIT 500",
        )
        .bind(etab)
        .bind(pid)
        .fetch_all(self.pool())
        .await
        .map_err(db)?;
        let famille = auteur.est_personne();
        Ok(rows
            .into_iter()
            .filter(|r| adresses || r.type_ != "adresse")
            .map(|r| HistoriqueCoordonnee {
                id: r.id,
                coordonnee_id: r.coordonnee_id,
                type_: r.type_,
                nature: r.nature,
                raison: r.raison,
                motif: r.motif,
                via: r.via,
                par_nom: if famille { None } else { r.par_nom },
                quand: r.quand,
            })
            .collect())
    }

    // -- Vérification par code ------------------------------------------------

    /// Envoie un code à la coordonnée (e-mail ou mobile). Refusé à l'initiative de
    /// l'établissement vers une personne « ne pas solliciter » ; au plus trois codes par heure.
    pub(crate) async fn coord_demander_verification(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        coord_id: Uuid,
    ) -> Result<VerificationEnvoyee, EntDbError> {
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let c = coord_lire_tx(&mut tx, etab, pid, coord_id, true)
            .await?
            .filter(|c| c.au.is_none())
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let canal = match c.type_.as_str() {
            "email" => "email",
            "mobile" => "sms",
            _ => return Err(refus_invalide("seuls un e-mail ou un mobile se vérifient par code")),
        };
        if c.verifiee_le.is_some() {
            return Err(refus_conflit("cette coordonnée est déjà vérifiée"));
        }
        if matches!(c.delivrabilite.as_str(), "rebond_definitif" | "non_attribue" | "desinscrit") {
            return Err(refus_conflit("cette coordonnée est signalée comme injoignable"));
        }
        if !auteur.est_personne() && personne_ne_pas_solliciter_tx(&mut tx, etab, pid).await? {
            return Err(refus_interdit("cette personne ne doit pas être sollicitée"));
        }
        let (recentes,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_coordonnee_verification
              WHERE coordonnee_id = $1 AND envoye_le > now() - interval '1 hour'",
        )
        .bind(c.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db)?;
        if recentes >= VERIFICATION_DEMANDES_PAR_HEURE {
            return Err(refus_conflit("trop de codes demandés, réessayer plus tard"));
        }
        sqlx::query(
            "UPDATE ent_coordonnee_verification SET statut = 'annulee'
              WHERE coordonnee_id = $1 AND statut = 'envoye'",
        )
        .bind(c.id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let id = Uuid::new_v4();
        let code = code_six_chiffres();
        let expire = Utc::now() + Duration::minutes(VERIFICATION_TTL_MINUTES);
        sqlx::query(
            "INSERT INTO ent_coordonnee_verification
                (id, etablissement_id, personne_id, coordonnee_id, canal, code_hash, expire_le,
                 demande_par)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(id)
        .bind(etab)
        .bind(pid)
        .bind(c.id)
        .bind(canal)
        .bind(empreinte_verification(id, &code)?)
        .bind(expire)
        .bind(auteur.actor())
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        // Aucun contenu qui nomme l'établissement ou un enfant : un code ne révèle rien.
        let envoi_ok = if canal == "email" {
            let r = self
                .mailer()
                .send(OutgoingMail {
                    to: c.valeur.clone(),
                    subject: "Votre code de vérification".into(),
                    body_text: format!(
                        "Bonjour,\n\nVotre code de vérification est : {code}\n\
Il est valable {VERIFICATION_TTL_MINUTES} minutes.\n\n\
Si vous n'êtes pas à l'origine de cette demande, ignorez ce message.\n"
                    ),
                    attachments: vec![],
                })
                .await;
            matches!(r, Ok(ref r) if self.envoi_reussi(r))
        } else {
            let r = self
                .sms()
                .send(OutgoingSms {
                    to: c.valeur.clone(),
                    body: format!("Code de vérification : {code} (valable {VERIFICATION_TTL_MINUTES} min)."),
                })
                .await;
            matches!(r, Ok(ref r) if self.sms_envoi_reussi(r))
        };
        if !envoi_ok {
            // Rien n'a été envoyé : aucune demande en attente ne reste (transaction non validée).
            tracing::warn!(canal, "vérification de coordonnée : code non envoyé");
            return Err(EntDbError::EnvoiImpossible);
        }
        commit(tx).await?;
        Ok(VerificationEnvoyee {
            coordonnee_id: c.id,
            canal,
            expire_le: expire,
        })
    }

    /// Confirme un code. Erreur unique quelle que soit la cause ; cinq essais au plus.
    pub(crate) async fn coord_verifier(
        &self,
        etab: Uuid,
        auteur: &Auteur<'_>,
        personne_id: Option<Uuid>,
        coord_id: Uuid,
        code: &str,
    ) -> Result<Coordonnee, EntDbError> {
        let generique = || refus_invalide("code invalide ou expiré");
        let code = code.trim();
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(generique());
        }
        let pid = self.coord_resoudre_personne(etab, auteur, personne_id, true).await?;
        let mut tx = self.pool().begin().await.map_err(db)?;
        personne_tx(&mut tx, etab, pid, true)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        let c = coord_lire_tx(&mut tx, etab, pid, coord_id, true)
            .await?
            .filter(|c| c.au.is_none())
            .ok_or_else(generique)?;
        let row: Option<(Uuid, String, i16, DateTime<Utc>, String)> = sqlx::query_as(
            "SELECT id, code_hash, tentatives, expire_le, canal FROM ent_coordonnee_verification
              WHERE coordonnee_id = $1 AND statut = 'envoye' FOR UPDATE",
        )
        .bind(c.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let Some((vid, hash, tentatives, expire, canal)) = row else {
            return Err(generique());
        };
        if expire < Utc::now() {
            sqlx::query("UPDATE ent_coordonnee_verification SET statut = 'expiree' WHERE id = $1")
                .bind(vid)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            commit(tx).await?;
            return Err(generique());
        }
        if !ent_auth::ct_eq(&hash, &empreinte_verification(vid, code)?) {
            let epuise = tentatives + 1 >= VERIFICATION_ESSAIS_MAX;
            sqlx::query(
                "UPDATE ent_coordonnee_verification
                    SET tentatives = tentatives + 1, statut = CASE WHEN $2 THEN 'epuisee' ELSE statut END
                  WHERE id = $1",
            )
            .bind(vid)
            .bind(epuise)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            commit(tx).await?;
            return Err(generique());
        }
        attribuer_tx(&mut tx, auteur, "verification").await?;
        let voie = if canal == "email" { "code_email" } else { "code_sms" };
        sqlx::query("UPDATE ent_coordonnee_verification SET statut = 'verifiee', verifiee_le = now() WHERE id = $1")
            .bind(vid)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        // Une coordonnée qui en remplace une autre la remplace à la vérification seulement.
        let mut principal = c.principal;
        if let Some(ancienne_id) = c.remplace_id {
            if let Some(ancienne) = coord_lire_tx(&mut tx, etab, pid, ancienne_id, true)
                .await?
                .filter(|a| a.au.is_none())
            {
                principal = ancienne.principal && c.usages.iter().any(|u| u == "notifications");
                sqlx::query(
                    "UPDATE ent_personne_coordonnee SET au = GREATEST(now(), du), principal = false WHERE id = $1",
                )
                .bind(ancienne.id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
                histo_tx(&mut tx, etab, pid, &ancienne, "remplacement", "verification", None, auteur).await?;
            }
        }
        sqlx::query(
            "UPDATE ent_personne_coordonnee
                SET verifiee_le = now(), verification_voie = $2, principal = $3, a_verifier = false
              WHERE id = $1",
        )
        .bind(c.id)
        .bind(voie)
        .bind(principal)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        let apres = coord_lire_tx(&mut tx, etab, pid, c.id, false)
            .await?
            .ok_or(EntDbError::Domain(DomainError::NotFound))?;
        histo_tx(&mut tx, etab, pid, &apres, "verification", "verification", None, auteur).await?;
        tracer_coordonnee_tx(&mut tx, etab, auteur, pid, c.id, &c.type_, "verification").await?;
        commit(tx).await?;
        Ok(apres)
    }

    // -- Façades pour le personnel (ADM-05) -----------------------------------

    pub async fn coordonnees_personne_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        avec_fermees: bool,
    ) -> Result<CoordonneesPersonne, EntDbError> {
        self.coord_lire(etab, &Auteur::Personnel { actor, role }, Some(personne_id), avec_fermees).await
    }

    pub async fn ajouter_coordonnee_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        saisie: &CoordonneeSaisie,
    ) -> Result<Coordonnee, EntDbError> {
        self.coord_ajouter(etab, &Auteur::Personnel { actor, role }, Some(personne_id), saisie).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn remplacer_coordonnee_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        coord_id: Uuid,
        valeur: &str,
        nature: NatureRemplacement,
        raison: RaisonCoordonnee,
        motif: &Option<String>,
    ) -> Result<Coordonnee, EntDbError> {
        self.coord_remplacer(
            etab,
            &Auteur::Personnel { actor, role },
            Some(personne_id),
            coord_id,
            valeur,
            nature,
            raison,
            motif,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn retirer_coordonnee_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        coord_id: Uuid,
        raison: RaisonCoordonnee,
        motif: &Option<String>,
    ) -> Result<(), EntDbError> {
        self.coord_retirer(etab, &Auteur::Personnel { actor, role }, Some(personne_id), coord_id, raison, motif)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn usages_coordonnee_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        coord_id: Uuid,
        usages: &[String],
        faire_principal: bool,
    ) -> Result<Coordonnee, EntDbError> {
        self.coord_modifier_usages(
            etab,
            &Auteur::Personnel { actor, role },
            Some(personne_id),
            coord_id,
            usages,
            faire_principal,
        )
        .await
    }

    pub async fn historique_coordonnees_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
    ) -> Result<Vec<HistoriqueCoordonnee>, EntDbError> {
        self.coord_historique(etab, &Auteur::Personnel { actor, role }, Some(personne_id)).await
    }

    pub async fn demander_verification_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        coord_id: Uuid,
    ) -> Result<VerificationEnvoyee, EntDbError> {
        self.coord_demander_verification(etab, &Auteur::Personnel { actor, role }, Some(personne_id), coord_id)
            .await
    }

    pub async fn verifier_coordonnee_for_actor(
        &self,
        etab: Uuid,
        actor: Uuid,
        role: &str,
        personne_id: Uuid,
        coord_id: Uuid,
        code: &str,
    ) -> Result<Coordonnee, EntDbError> {
        self.coord_verifier(etab, &Auteur::Personnel { actor, role }, Some(personne_id), coord_id, code)
            .await
    }

    // -- Façades « soi » (la personne elle-même) -------------------------------

    pub async fn mes_coordonnees(&self, etab: Uuid, user: Uuid) -> Result<CoordonneesPersonne, EntDbError> {
        self.coord_lire(etab, &Auteur::Personne { actor: user }, None, false).await
    }

    pub async fn ajouter_ma_coordonnee(
        &self,
        etab: Uuid,
        user: Uuid,
        saisie: &CoordonneeSaisie,
    ) -> Result<Coordonnee, EntDbError> {
        self.coord_ajouter(etab, &Auteur::Personne { actor: user }, None, saisie).await
    }

    pub async fn remplacer_ma_coordonnee(
        &self,
        etab: Uuid,
        user: Uuid,
        coord_id: Uuid,
        valeur: &str,
        motif: &Option<String>,
    ) -> Result<Coordonnee, EntDbError> {
        self.coord_remplacer(
            etab,
            &Auteur::Personne { actor: user },
            None,
            coord_id,
            valeur,
            NatureRemplacement::Remplacement,
            RaisonCoordonnee::DemandeDeLaPersonne,
            motif,
        )
        .await
    }

    pub async fn retirer_ma_coordonnee(
        &self,
        etab: Uuid,
        user: Uuid,
        coord_id: Uuid,
        motif: &Option<String>,
    ) -> Result<(), EntDbError> {
        self.coord_retirer(
            etab,
            &Auteur::Personne { actor: user },
            None,
            coord_id,
            RaisonCoordonnee::DemandeDeLaPersonne,
            motif,
        )
        .await
    }

    pub async fn usages_ma_coordonnee(
        &self,
        etab: Uuid,
        user: Uuid,
        coord_id: Uuid,
        usages: &[String],
        faire_principal: bool,
    ) -> Result<Coordonnee, EntDbError> {
        self.coord_modifier_usages(etab, &Auteur::Personne { actor: user }, None, coord_id, usages, faire_principal)
            .await
    }

    pub async fn mon_historique_coordonnees(
        &self,
        etab: Uuid,
        user: Uuid,
    ) -> Result<Vec<HistoriqueCoordonnee>, EntDbError> {
        self.coord_historique(etab, &Auteur::Personne { actor: user }, None).await
    }

    pub async fn demander_verification_ma_coordonnee(
        &self,
        etab: Uuid,
        user: Uuid,
        coord_id: Uuid,
    ) -> Result<VerificationEnvoyee, EntDbError> {
        self.coord_demander_verification(etab, &Auteur::Personne { actor: user }, None, coord_id).await
    }

    pub async fn verifier_ma_coordonnee(
        &self,
        etab: Uuid,
        user: Uuid,
        coord_id: Uuid,
        code: &str,
    ) -> Result<Coordonnee, EntDbError> {
        self.coord_verifier(etab, &Auteur::Personne { actor: user }, None, coord_id, code).await
    }
}

/// Code à six chiffres (aléatoire du système, via un UUID v4).
fn code_six_chiffres() -> String {
    let b = Uuid::new_v4().into_bytes();
    let n = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) % 1_000_000;
    format!("{n:06}")
}

/// Empreinte HMAC-SHA-256 du code sous le secret serveur : un code à six chiffres n'a que 10⁶
/// valeurs, une base volée ne doit pas permettre de le retrouver (fail-closed sans secret).
fn empreinte_verification(id: Uuid, code: &str) -> Result<String, EntDbError> {
    let secret = ent_auth::SecretBox::from_env()?;
    Ok(secret.hmac_hex("coordonnee-verification-v1", format!("{id}:{}", code.trim()).as_bytes())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telephone_e164() {
        for (brut, attendu) in [
            ("06 12 34 56 78", Some("+33612345678")),
            ("06.12.34.56.78", Some("+33612345678")),
            ("+33 6 12 34 56 78", Some("+33612345678")),
            ("+33 (0)6 12 34 56 78", Some("+33612345678")),
            ("0033612345678", Some("+33612345678")),
            ("01 02 03 04 05", Some("+33102030405")),
            ("+44 7911 123456", Some("+447911123456")),
            ("0044 7911 123456", Some("+447911123456")),
            ("+262 692 12 34 56", Some("+262692123456")),
            ("12345", None),
            ("+3361234", None),
            ("+33 0 12 34 56 78", None),
            ("00 12 34 56 78", None),
            ("06 12 34 56 7a", None),
            ("", None),
            ("+0612345678", None),
        ] {
            assert_eq!(normaliser_telephone(brut).as_deref(), attendu, "{brut}");
        }
    }

    #[test]
    fn classement_mobile_fixe() {
        assert_eq!(classer_telephone("+33612345678"), "mobile");
        assert_eq!(classer_telephone("+33712345678"), "mobile");
        assert_eq!(classer_telephone("+33102030405"), "fixe");
        assert_eq!(classer_telephone("+447911123456"), "mobile");
    }

    #[test]
    fn valeurs_normalisees_et_refusees() {
        assert_eq!(normaliser_valeur("email", " Jean.Dupont@Exemple.FR ").unwrap().1, "jean.dupont@exemple.fr");
        assert!(normaliser_valeur("email", "pas-un-email").is_err());
        assert!(normaliser_valeur("email", "a@b@c.fr").is_err());
        assert_eq!(normaliser_valeur("mobile", "07 12 34 56 78").unwrap().1, "+33712345678");
        assert!(normaliser_valeur("mobile", "01 02 03 04 05").is_err(), "un fixe n'est pas un mobile");
        assert!(normaliser_valeur("fixe", "06 12 34 56 78").is_err(), "un mobile n'est pas un fixe");
        assert!(normaliser_valeur("professionnel", "06 12 34 56 78").is_ok());
        assert!(normaliser_valeur("telex", "1").is_err());
        assert!(normaliser_valeur("adresse", "12").is_err());
        assert_eq!(
            normaliser_valeur("adresse", "  12  rue des Lilas \n\n 75011   Paris ").unwrap().1,
            "12 rue des Lilas\n75011 Paris"
        );
    }

    #[test]
    fn masques_sans_valeur_entiere() {
        assert_eq!(masque_courriel("jean@exemple.fr"), "j•••@exemple.fr");
        assert!(!masque_telephone("+33612345678").contains("123456"));
        assert!(masque_telephone("+33612345678").ends_with("78"));
        assert_eq!(masque_telephone("123"), "••••");
    }

    #[test]
    fn usages_normalises() {
        assert_eq!(normaliser_usages(&[]).unwrap(), vec!["notifications".to_string()]);
        assert!(normaliser_usages(&["connexion".into()]).is_err(), "connexion appartient au compte");
        assert_eq!(
            normaliser_usages(&["urgence".into(), "urgence".into(), "facturation".into()]).unwrap(),
            vec!["urgence".to_string(), "facturation".to_string()]
        );
    }

    #[test]
    fn motif_borne() {
        assert!(motif_valide(&None).unwrap().is_none());
        assert!(motif_valide(&Some("  ".into())).unwrap().is_none());
        assert!(motif_valide(&Some("ab".into())).is_err());
        assert!(motif_valide(&Some("x".repeat(501))).is_err());
        assert_eq!(motif_valide(&Some(" demande orale ".into())).unwrap().as_deref(), Some("demande orale"));
    }
}
