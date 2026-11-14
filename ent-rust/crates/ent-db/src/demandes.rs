//! Socle N1 (« Demandes et flux de travail »), lot N1-0 : modèle et garde-fous du cœur.
//!
//! Ce lot pose le **cœur** d'une demande (dépôt, prise, relâche, annulation, clôture, purge) et surtout les
//! garde-fous qui ferment les défauts du Next.js (règle 14 de `PILOTAGE.md`, `ALERTES_NEXTJS_PROD.md` §5 ter) :
//!
//! | Défaut | Garde-fou |
//! |---|---|
//! | NJ-72 | limitation de débit (seau en base, par personne) sur le dépôt et les gestes |
//! | NJ-80 | toute écriture passe par `charger_pour_acces` : ni demandeur, ni déposant, ni membre de la file → **404 neutre** |
//! | NJ-82 | deux projections séparées, jusque dans la base (vues `ent_dem_vue_demandeur` / `ent_dem_vue_traitant`) |
//! | NJ-84 | le demandeur est la personne de la **session** ; `DepotDemande` n'a aucun champ d'identité |
//! | NJ-86 | événements en **enveloppe neutre** (identifiants, numéro, codes) contrôlée avant publication |
//! | NJ-88 | aucun dépôt public : tout passe par une session ; aucune route publique |
//! | NJ-89 | faits en ajout seul et chaînés par la base ; aucune suppression ; purge par l'ordonnanceur seulement |
//! | NJ-90 | version vérifiée à chaque écriture (409) ; valeurs typées sans sac d'attributs |
//! | NJ-93 | erreurs publiques neutres (`RefusDemande`) ; le détail va au journal technique, jamais au client |
//!
//! **Hors du lot N1-0** (lots suivants) : files, membres calculés et règles de routage (N1-1), échanges, pièces,
//! délégation, transfert, consultation (N1-2), HSE et reprographie (N1-3). Les lectures de ces sources passent par
//! [`SourcesDemandes`] : tant qu'elles sont absentes ([`SourcesAbsentes`]), **aucune** file n'existe, donc personne
//! ne traite rien et aucun dépôt n'aboutit (R-DM6, R-DM7 : jamais « tout le personnel »).
//!
//! Le jour métier est celui de l'établissement (`ent_aujourdhui_etab`), jamais la date du serveur ni le jour UTC.

use crate::champs_chiffres::ChampsSensibles;
use crate::finances::publier;
use crate::outbox::Causalite;
use crate::personnels::agent_du_user_tx;
use crate::rate_limit::{Decision, Quota};
use crate::service::{EntDbError, PgEnt};
use crate::tenant_tx::TenantTx;
use chrono::{DateTime, Datelike, Months, NaiveDate, NaiveTime, Utc, Weekday};
use ent_auth::{Action, Role};
use ent_events::{types, AggregateKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection};
use std::collections::BTreeMap;
use uuid::Uuid;

// ───────────────────────── Refus publics (NJ-93) ─────────────────────────

/// Refus d'une opération sur les demandes. **Tout ce que le client peut lire est ici** : des phrases fixes, sans
/// identifiant, sans texte de base de données, sans nom de table (NJ-93).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusDemande {
    /// La demande n'existe pas **ou** l'acteur n'a aucun droit de la voir : même réponse (404 neutre, R-DM10).
    Introuvable,
    /// L'acteur n'a pas le droit d'effectuer ce geste, sur une chose qu'il connaît déjà.
    Interdit,
    /// Saisie refusée ; la raison est une phrase fixe sans donnée personnelle.
    Invalide(&'static str),
    /// Conflit d'état ou de version (409) : relire puis recommencer.
    Conflit(&'static str),
    /// Limitation de débit (429).
    TropDeDemandes { retry_after_secs: u64 },
    /// Panne ou source absente : aucun détail au client (le détail est journalisé côté serveur).
    Indisponible,
}

impl RefusDemande {
    /// Code HTTP à renvoyer.
    pub fn statut_http(&self) -> u16 {
        match self {
            Self::Introuvable => 404,
            Self::Interdit => 403,
            Self::Invalide(_) => 422,
            Self::Conflit(_) => 409,
            Self::TropDeDemandes { .. } => 429,
            Self::Indisponible => 503,
        }
    }

    /// Phrase destinée à l'utilisateur : fixe, en français, sans donnée.
    pub fn message_public(&self) -> &'static str {
        match self {
            Self::Introuvable => "Demande introuvable.",
            Self::Interdit => "Vous n'avez pas le droit d'effectuer cette action.",
            Self::Invalide(m) | Self::Conflit(m) => m,
            Self::TropDeDemandes { .. } => "Trop de demandes en peu de temps : réessayez dans quelques instants.",
            Self::Indisponible => "Le service est momentanément indisponible. Réessayez plus tard.",
        }
    }
}

impl std::fmt::Display for RefusDemande {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message_public())
    }
}
impl std::error::Error for RefusDemande {}

impl From<sqlx::Error> for RefusDemande {
    fn from(e: sqlx::Error) -> Self {
        // Le détail va au journal technique (jamais au client, NJ-93) ; aucune donnée de la demande n'y figure.
        tracing::error!(erreur = %e, "demandes : erreur de base de données");
        match e.as_database_error().and_then(|d| d.code()).as_deref() {
            Some("40001") | Some("23505") => Self::Conflit("La demande a changé entre-temps : rechargez-la."),
            _ => Self::Indisponible,
        }
    }
}

impl From<EntDbError> for RefusDemande {
    fn from(e: EntDbError) -> Self {
        match e {
            EntDbError::Forbidden => Self::Interdit,
            autre => {
                tracing::error!(erreur = %autre, "demandes : erreur interne");
                Self::Indisponible
            }
        }
    }
}

// ───────────────────────── Vocabulaire ─────────────────────────

macro_rules! enum_texte {
    ($(#[$m:meta])* $nom:ident { $($var:ident => $code:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $nom { $($var),+ }
        impl $nom {
            pub const fn code(self) -> &'static str { match self { $(Self::$var => $code),+ } }
            pub fn parse(s: &str) -> Option<Self> { match s { $($code => Some(Self::$var),)+ _ => None } }
        }
    };
}

enum_texte!(
    /// Statut d'une demande (§7).
    Statut {
        Brouillon => "brouillon", Deposee => "deposee", Prise => "prise",
        EnAttenteDemandeur => "en_attente_demandeur", EnAttenteTiers => "en_attente_tiers",
        EnValidation => "en_validation", Traitee => "traitee", Close => "close",
        Rouverte => "rouverte", Annulee => "annulee",
    }
);
enum_texte!(
    /// Classe de confidentialité de la nature (C0 à C3).
    Confidentialite { C0 => "C0", C1 => "C1", C2 => "C2", C3 => "C3" }
);
enum_texte!(
    Urgence { Normale => "normale", Haute => "haute", Securite => "securite" }
);
enum_texte!(
    /// Canal de dépôt. Ce lot n'ouvre que `web` et `mobile` (session) ; les autres viennent avec leurs lots.
    Canal {
        Web => "web", Mobile => "mobile", Comptoir => "comptoir", Telephone => "telephone",
        Courrier => "courrier", CourrielRecu => "courriel_recu", ConversionMessage => "conversion_message",
        Reprise => "reprise",
    }
);
enum_texte!(
    /// Issue d'une demande close (§7).
    Issue {
        Traitee => "traitee", Accordee => "accordee", AccordeePartiellement => "accordee_partiellement",
        Refusee => "refusee", SansObjet => "sans_objet", AnnuleeParDemandeur => "annulee_par_demandeur",
        Doublon => "doublon", TransmiseHorsScolia => "transmise_hors_scolia",
        CloseSansReponseDuDemandeur => "close_sans_reponse_du_demandeur",
    }
);

impl Issue {
    /// Décision associée, pour les issues qui en portent une.
    pub fn decision(self) -> Option<&'static str> {
        match self {
            Self::Accordee => Some("accordee"),
            Self::AccordeePartiellement => Some("accordee_partiellement"),
            Self::Refusee => Some("refusee"),
            _ => None,
        }
    }
}

impl Statut {
    pub(crate) fn est_final(self) -> bool {
        matches!(self, Self::Close | Self::Annulee)
    }
}

// ───────────────────────── Natures déclarées (kit §8.5) ─────────────────────────

/// Public auquel une nature s'ouvre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicNature {
    Personnel,
    Responsable,
    EleveMajeur,
    EleveMineur,
}

/// Catégorie de conservation (§15.3, soumise au DPO : Q-DM4 ouverte, durées en `[NV]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conservation {
    /// Maintenance, entretien, assistance, fournitures : 1 an.
    TechniqueCourte,
    /// Secrétariat, vie scolaire, direction : 2 ans.
    InterneCourante,
    /// Attestation, carte, secrétariat famille : année en cours + 1 (24 mois au plus).
    FamilleCourante,
    /// Comptabilité, remboursement : 10 ans [NV].
    Comptable,
    /// RH et HSE : durée de la relation de travail + 5 ans [NV] — **aucune purge automatique** (archive).
    Rh,
    /// Reprographie : la demande 1 an.
    Reprographie,
}

impl Conservation {
    /// Durée en mois après clôture ; `None` = pas de purge automatique.
    pub const fn duree_mois(self) -> Option<u32> {
        match self {
            Self::TechniqueCourte | Self::Reprographie => Some(12),
            Self::InterneCourante | Self::FamilleCourante => Some(24),
            Self::Comptable => Some(120),
            Self::Rh => None,
        }
    }
}

/// Type d'un champ déclaré.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChampType {
    Texte { max: usize },
    Entier { min: i64, max: i64 },
    /// Décimal à deux chiffres après la virgule (stocké `NUMERIC(12,2)`).
    Decimal,
    Date,
    Heure,
    Bool,
    /// Un code parmi une liste fermée.
    Liste(&'static [&'static str]),
    /// Identifiant d'un objet d'une autre brique (lieu, élève…), sans clé étrangère inter-brique.
    Ref,
}

/// Champ déclaré par une nature.
#[derive(Debug, Clone, Copy)]
pub struct ChampDecl {
    pub code: &'static str,
    pub type_: ChampType,
    pub obligatoire: bool,
    /// Nombre maximal de valeurs (champ répété) ; 1 = champ simple.
    pub rangs_max: u16,
}

/// Déclaration d'une nature de demande (forme du §8.2, réduite à ce que le cœur applique).
#[derive(Debug, Clone, Copy)]
pub struct NatureDecl {
    /// Stable ; préfixe = domaine de la brique propriétaire (`secretariat.…`, `rh.…`).
    pub code: &'static str,
    pub version: i16,
    pub libelle: &'static str,
    pub mots_cles: &'static [&'static str],
    pub publics: &'static [PublicNature],
    pub confidentialite: Confidentialite,
    pub champs: &'static [ChampDecl],
    /// Longueur maximale du texte libre ; `None` = pas de texte libre.
    pub texte_max: Option<usize>,
    /// La demande porte sur un élève (dans les droits du demandeur).
    pub eleve_requis: bool,
    pub delai_prise_jours: u16,
    pub delai_traitement_jours: u16,
    pub conservation: Conservation,
    /// Code de la file qui reçoit par défaut (résolu en identifiant par N1-1).
    pub file_defaut: &'static str,
}

pub(crate) const fn champ(code: &'static str, type_: ChampType, obligatoire: bool) -> ChampDecl {
    ChampDecl { code, type_, obligatoire, rangs_max: 1 }
}

/// Natures intégrées de ce lot (le catalogue complet est celui de N1-1, §8.4). Ce sont de vraies natures de la
/// bascule ; elles servent aussi à exercer les types de valeurs, la confidentialité C2 et les publics.
pub const NATURES: &[NatureDecl] = &[
    NatureDecl {
        code: "secretariat.demande_interne",
        version: 1,
        libelle: "Demande au secrétariat",
        mots_cles: &["secrétariat", "courrier", "document", "papier"],
        publics: &[PublicNature::Personnel],
        confidentialite: Confidentialite::C1,
        champs: &[],
        texte_max: Some(2000),
        eleve_requis: false,
        delai_prise_jours: 2,
        delai_traitement_jours: 5,
        conservation: Conservation::InterneCourante,
        file_defaut: "secretariat",
    },
    NatureDecl {
        code: "maintenance.intervention",
        version: 1,
        libelle: "Signaler un problème technique",
        mots_cles: &["panne", "réparation", "fuite", "ampoule", "maintenance"],
        publics: &[PublicNature::Personnel],
        confidentialite: Confidentialite::C1,
        champs: &[
            champ("gravite", ChampType::Liste(&["mineure", "gene", "bloquante"]), true),
            champ("nb_personnes", ChampType::Entier { min: 1, max: 5000 }, false),
            champ("constate_le", ChampType::Date, false),
            champ("heure_constat", ChampType::Heure, false),
            champ("appareil_arrete", ChampType::Bool, false),
            champ("lieu_id", ChampType::Ref, false),
            champ("cout_estime", ChampType::Decimal, false),
            champ("precision_lieu", ChampType::Texte { max: 120 }, false),
        ],
        texte_max: Some(2000),
        eleve_requis: false,
        delai_prise_jours: 2,
        delai_traitement_jours: 10,
        conservation: Conservation::TechniqueCourte,
        file_defaut: "maintenance",
    },
    NatureDecl {
        code: "rh.demande_agent",
        version: 1,
        libelle: "Demande d'un personnel aux ressources humaines",
        mots_cles: &["attestation", "temps partiel", "formation", "ressources humaines"],
        publics: &[PublicNature::Personnel],
        confidentialite: Confidentialite::C2,
        champs: &[
            champ(
                "type_demande",
                ChampType::Liste(&["attestation_employeur", "temps_partiel", "formation", "autre"]),
                true,
            ),
            champ("date_effet", ChampType::Date, false),
        ],
        texte_max: Some(2000),
        eleve_requis: false,
        delai_prise_jours: 3,
        delai_traitement_jours: 15,
        conservation: Conservation::Rh,
        file_defaut: "rh",
    },
    NatureDecl {
        code: "scolarite.attestation",
        version: 1,
        libelle: "Demander une attestation",
        mots_cles: &["attestation", "certificat", "justificatif", "assurance"],
        publics: &[PublicNature::Responsable, PublicNature::EleveMajeur],
        confidentialite: Confidentialite::C1,
        champs: &[
            champ(
                "type_attestation",
                ChampType::Liste(&["scolarite_periode", "assiduite", "inscription_examen", "autre"]),
                true,
            ),
            champ("periode_du", ChampType::Date, false),
            champ("periode_au", ChampType::Date, false),
        ],
        texte_max: Some(2000),
        eleve_requis: true,
        delai_prise_jours: 2,
        delai_traitement_jours: 5,
        conservation: Conservation::FamilleCourante,
        file_defaut: "secretariat_scolarite",
    },
];

/// La nature déclarée sous ce code.
pub fn nature(code: &str) -> Option<&'static NatureDecl> {
    crate::demandes_catalogue::nature_declaree(code)
}

fn code_valide(s: &str, max: usize) -> bool {
    !s.is_empty()
        && s.len() <= max
        && s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Ce que vérifie l'assemblage d'une nature (§8.5, `t_kit_nature_complete`) : renvoie la première règle violée.
pub fn verifier_nature(n: &NatureDecl) -> Result<(), String> {
    let (domaine, reste) = n.code.split_once('.').ok_or("code sans préfixe de domaine")?;
    if !code_valide(domaine, 40) || reste.split('.').any(|p| !code_valide(p, 40)) || n.code.len() > 80 {
        return Err(format!("code « {} » mal formé", n.code));
    }
    if n.version < 1 {
        return Err("version < 1".into());
    }
    if n.libelle.trim().is_empty() || n.libelle.chars().count() > 120 {
        return Err("libellé absent ou trop long".into());
    }
    if n.mots_cles.is_empty() || n.mots_cles.iter().any(|m| m.trim().is_empty()) {
        return Err("mots-clés absents".into());
    }
    if n.publics.is_empty() {
        return Err("aucun public".into());
    }
    // Aucun public famille ou élève sur une nature C2+ (§8.5).
    if matches!(n.confidentialite, Confidentialite::C2 | Confidentialite::C3)
        && n.publics.iter().any(|p| *p != PublicNature::Personnel)
    {
        return Err("public famille ou élève sur une nature C2+".into());
    }
    if n.eleve_requis && n.publics == [PublicNature::Personnel] {
        return Err("élève requis mais aucun public famille ou élève".into());
    }
    if n.delai_prise_jours == 0 || n.delai_traitement_jours < n.delai_prise_jours || n.delai_traitement_jours > 90 {
        return Err("délais absents ou incohérents".into());
    }
    if !code_valide(n.file_defaut, 40) {
        return Err("file par défaut absente".into());
    }
    if n.texte_max.is_some_and(|m| m == 0 || m > 8000) {
        return Err("texte libre hors bornes".into());
    }
    let mut vus = std::collections::BTreeSet::new();
    for c in n.champs {
        if !code_valide(c.code, 60) || !vus.insert(c.code) {
            return Err(format!("champ « {} » mal formé ou en double", c.code));
        }
        if c.rangs_max == 0 || c.rangs_max > 50 {
            return Err(format!("champ « {} » : rangs hors bornes", c.code));
        }
        match c.type_ {
            ChampType::Liste(l) if l.is_empty() || l.iter().any(|v| !code_valide(v, 60)) => {
                return Err(format!("champ « {} » : liste invalide", c.code));
            }
            ChampType::Texte { max } if max == 0 || max > 2000 => {
                return Err(format!("champ « {} » : texte hors bornes", c.code));
            }
            ChampType::Entier { min, max } if min > max => {
                return Err(format!("champ « {} » : bornes inversées", c.code));
            }
            _ => {}
        }
    }
    Ok(())
}

// ───────────────────────── Valeurs saisies et lues ─────────────────────────

/// Valeur typée d'un champ.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "valeur", rename_all = "snake_case")]
pub enum Valeur {
    Texte(String),
    Entier(i64),
    /// Décimal en texte (`"12.50"`) : jamais un flottant.
    Decimal(String),
    Date(NaiveDate),
    Heure(NaiveTime),
    Bool(bool),
    Code(String),
    Ref(Uuid),
}

/// Valeur saisie pour un champ déclaré.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValeurSaisie {
    pub champ: String,
    #[serde(default)]
    pub rang: u16,
    pub valeur: Valeur,
}

/// Valeur lue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValeurLue {
    pub champ: String,
    pub rang: i16,
    pub valeur: Valeur,
}

fn decimal_valide(s: &str) -> bool {
    let t = s.strip_prefix('-').unwrap_or(s);
    let (ent, frac) = t.split_once('.').unwrap_or((t, ""));
    !ent.is_empty()
        && ent.len() <= 10
        && frac.len() <= 2
        && ent.chars().all(|c| c.is_ascii_digit())
        && frac.chars().all(|c| c.is_ascii_digit())
}

fn valider_valeur(decl: &ChampDecl, v: &Valeur) -> Result<(), RefusDemande> {
    let ok = match (&decl.type_, v) {
        (ChampType::Texte { max }, Valeur::Texte(t)) => {
            !t.trim().is_empty() && t.chars().count() <= *max && !t.contains('\0')
        }
        (ChampType::Entier { min, max }, Valeur::Entier(n)) => n >= min && n <= max,
        (ChampType::Decimal, Valeur::Decimal(s)) => decimal_valide(s),
        (ChampType::Date, Valeur::Date(_)) | (ChampType::Heure, Valeur::Heure(_)) => true,
        (ChampType::Bool, Valeur::Bool(_)) => true,
        (ChampType::Liste(l), Valeur::Code(c)) => l.contains(&c.as_str()),
        (ChampType::Ref, Valeur::Ref(id)) => !id.is_nil(),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(RefusDemande::Invalide("Une des informations saisies n'est pas valide."))
    }
}

/// Valide les valeurs saisies contre la déclaration de la nature : champ connu, type exact, bornes, obligatoires,
/// rangs. Aucune valeur d'un champ non déclaré n'est jamais écrite (NJ-90 : pas de sac d'attributs).
fn valider_valeurs(n: &NatureDecl, saisies: &[ValeurSaisie]) -> Result<(), RefusDemande> {
    let mut vus: BTreeMap<(&str, u16), ()> = BTreeMap::new();
    for s in saisies {
        let decl = n
            .champs
            .iter()
            .find(|c| c.code == s.champ)
            .ok_or(RefusDemande::Invalide("Une des informations saisies n'est pas prévue pour cette demande."))?;
        if s.rang >= decl.rangs_max {
            return Err(RefusDemande::Invalide("Trop de valeurs pour une des informations."));
        }
        if vus.insert((decl.code, s.rang), ()).is_some() {
            return Err(RefusDemande::Invalide("Une information est saisie deux fois."));
        }
        valider_valeur(decl, &s.valeur)?;
    }
    for decl in n.champs.iter().filter(|c| c.obligatoire) {
        if !saisies.iter().any(|s| s.champ == decl.code) {
            return Err(RefusDemande::Invalide("Une information obligatoire est manquante."));
        }
    }
    Ok(())
}

// ───────────────────────── Sources externes (adaptateurs « source absente ») ─────────────────────────

/// Ce que N1-0 lit chez les autres briques. Les adaptateurs réels arrivent avec leurs lots (N1-1 : files et membres
/// calculés depuis le socle 6 ; socle 4 : droits de lien des familles). **Sans source, rien ne passe** (fail-closed).
pub trait SourcesDemandes: Send + Sync {
    /// File de service qui reçoit une demande de cette nature dans cet établissement (R-DM6 : toujours une file).
    fn file_de_depot(&self, etab: Uuid, nature_code: &str) -> Option<Uuid>;
    /// Files dont la personne est **membre** (membres calculés ; jamais « tout le personnel », R-DM7).
    fn files_de(&self, etab: Uuid, personne_id: Uuid) -> Vec<Uuid>;
    /// Un responsable légal ou un élève (selon `role`) peut-il déposer une demande pour cet élève ?
    fn peut_deposer_famille(&self, etab: Uuid, user_id: Uuid, role: Role, eleve_id: Option<Uuid>) -> bool;
    /// Routage complet (N1-1) : file, raison, calendrier et délais effectifs. Par défaut : la file de
    /// [`Self::file_de_depot`], sans raison tracée ni délai propre (adaptateurs de test du lot N1-0).
    fn routage_de_depot(&self, etab: Uuid, nature_code: &str) -> Option<RoutageResolu> {
        self.file_de_depot(etab, nature_code).map(RoutageResolu::simple)
    }
    /// Phrase publique si la nature n'est **pas proposée** à ce public dans l'établissement (nature non activée,
    /// public fermé : CDM-07). `None` : rien à redire (ou source sans notion d'activation).
    fn refus_nature(&self, _etab: Uuid, _nature_code: &str) -> Option<&'static str> {
        None
    }
}

/// Résultat du routage déterministe d'une demande (§10.1) : la destination, **pourquoi**, et ce qui borne les
/// échéances. La raison n'est jamais lisible du demandeur (il voit le service, pas la règle).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutageResolu {
    pub file_id: Uuid,
    /// `regle` · `file_de_la_nature` · `defaut_nature` · `tri_du_site` · `repli_file_vide` · `repli_personne_visee`.
    /// `None` : source sans trace de routage.
    pub motif: Option<&'static str>,
    /// Règle de routage appliquée, ou file initialement visée quand un repli a pris le relais.
    pub ref_id: Option<Uuid>,
    pub service_calendrier: String,
    pub delai_prise_jours: Option<u16>,
    pub delai_traitement_jours: Option<u16>,
    pub site_id: Option<Uuid>,
}

impl RoutageResolu {
    pub fn simple(file_id: Uuid) -> Self {
        Self {
            file_id,
            motif: None,
            ref_id: None,
            service_calendrier: "secretariat".into(),
            delai_prise_jours: None,
            delai_traitement_jours: None,
            site_id: None,
        }
    }
}

/// Aucune source : aucune file, aucun droit de famille. Comportement par défaut tant que N1-1 n'est pas versé.
#[derive(Debug, Clone, Copy, Default)]
pub struct SourcesAbsentes;

impl SourcesDemandes for SourcesAbsentes {
    fn file_de_depot(&self, _: Uuid, _: &str) -> Option<Uuid> {
        None
    }
    fn files_de(&self, _: Uuid, _: Uuid) -> Vec<Uuid> {
        Vec::new()
    }
    fn peut_deposer_famille(&self, _: Uuid, _: Uuid, _: Role, _: Option<Uuid>) -> bool {
        false
    }
}

// ───────────────────────── Enveloppe neutre (NJ-86) ─────────────────────────

/// Charge utile d'un événement de demande : **identifiants, numéro et codes seulement**. Jamais l'objet, le texte,
/// un nom, une adresse ni une pièce (R-DM27). Les avis externes (socle 8) portent cette enveloppe et un lien.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnveloppeNeutre {
    pub demande_id: Uuid,
    pub numero: String,
    pub nature: String,
    pub evenement: &'static str,
    pub lien: String,
}

const CLES_ENVELOPPE: [&str; 5] = ["demande_id", "evenement", "lien", "nature", "numero"];

impl EnveloppeNeutre {
    pub fn nouvelle(demande_id: Uuid, numero: &str, nature: &str, evenement: &'static str) -> Self {
        Self {
            demande_id,
            numero: numero.to_string(),
            nature: nature.to_string(),
            evenement,
            lien: format!("/demandes/{demande_id}"),
        }
    }

    pub fn vers_valeur(&self) -> Value {
        json!({
            "demande_id": self.demande_id,
            "numero": self.numero,
            "nature": self.nature,
            "evenement": self.evenement,
            "lien": self.lien,
        })
    }
}

/// Lint de minimisation : une charge utile d'événement de demande n'a que les clés de l'enveloppe, des chaînes
/// courtes, aucune adresse (`@`) ni espace (donc aucun texte libre). Appelé avant **chaque** publication.
pub fn controler_enveloppe(v: &Value) -> Result<(), &'static str> {
    let o = v.as_object().ok_or("enveloppe : objet attendu")?;
    let mut cles: Vec<&str> = o.keys().map(String::as_str).collect();
    cles.sort_unstable();
    if cles != CLES_ENVELOPPE {
        return Err("enveloppe : clés non autorisées");
    }
    for val in o.values() {
        let s = val.as_str().ok_or("enveloppe : chaînes seulement")?;
        if s.len() > 120 || s.contains('@') || s.contains(char::is_whitespace) {
            return Err("enveloppe : valeur non neutre");
        }
    }
    if !o["lien"].as_str().is_some_and(|l| l.starts_with("/demandes/")) {
        return Err("enveloppe : lien interne attendu");
    }
    Ok(())
}

// ───────────────────────── Projections ─────────────────────────

/// Ce que **voit le demandeur** (projection issue de la vue `ent_dem_vue_demandeur`) : jamais la file, le traitant,
/// le routage, une note interne ni une adresse du personnel (R-DM18, NJ-82).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DemandeDemandeur {
    pub id: Uuid,
    pub numero: String,
    pub nature_code: String,
    pub statut: String,
    pub issue: Option<String>,
    pub decision: Option<String>,
    pub urgence: String,
    pub objet: Option<String>,
    pub texte: Option<String>,
    pub depose_le: Option<DateTime<Utc>>,
    pub clos_le: Option<DateTime<Utc>>,
    pub version: i32,
}

/// Ce que **voit le traitant** (vue `ent_dem_vue_traitant`) : l'exploitation de la demande, sans coordonnées du
/// demandeur (lues au socle 4 à l'instant de l'avis, §24.4). Le texte d'une nature C2+ n'est ouvert que pour un
/// membre de la file, et la lecture est tracée.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DemandeTraitant {
    pub id: Uuid,
    pub numero: String,
    pub nature_code: String,
    pub statut: String,
    pub confidentialite: String,
    pub urgence: String,
    pub demandeur_personne_id: Option<Uuid>,
    pub objet: Option<String>,
    pub texte: Option<String>,
    pub file_id: Uuid,
    pub attribuee_a_agent_id: Option<Uuid>,
    pub depose_le: Option<DateTime<Utc>>,
    pub clos_le: Option<DateTime<Utc>>,
    pub gel: bool,
    pub version: i32,
}

/// Une ligne de l'historique **visible du demandeur** : type, date, statut atteint. Ni agent, ni file, ni motif.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EtapeVisible {
    pub etape: String,
    pub le: DateTime<Utc>,
    pub statut: Option<String>,
}

/// Faits que le demandeur voit (le reste est interne : notes, lectures, échéances, routage…).
const FAITS_VISIBLES_DEMANDEUR: [&str; 8] =
    ["depose", "vue_par_service", "prise", "close", "annulee", "rouverte", "complement_demande", "complement_recu"];

/// État minimal rendu après un geste : rien de plus que ce que tout acteur légitime peut savoir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EtatDemande {
    pub id: Uuid,
    pub numero: String,
    pub statut: String,
    pub version: i32,
}

// ───────────────────────── Dépôt et gestes ─────────────────────────

/// Corps d'un dépôt. **Aucun champ d'identité** (nom, prénom, e-mail, téléphone) : le demandeur est la personne de
/// la session (NJ-84). Un client qui en envoie reste sans effet : ces champs n'existent pas ici.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepotDemande {
    pub nature_code: String,
    pub objet: String,
    #[serde(default)]
    pub texte: Option<String>,
    #[serde(default)]
    pub valeurs: Vec<ValeurSaisie>,
    #[serde(default)]
    pub urgence: Option<Urgence>,
    #[serde(default)]
    pub eleve_id: Option<Uuid>,
    /// Dépôt « pour le compte » : la personne (du socle 4) au nom de laquelle on dépose. Réservé à
    /// `demandes.deposer_pour_compte` ; le déposant est tracé.
    #[serde(default)]
    pub pour_le_compte_de: Option<Uuid>,
    #[serde(default)]
    pub canal: Option<Canal>,
    /// Volet d'une brique propriétaire (HSE, reprographie), écrit **dans la même transaction** que la demande.
    /// Jamais lu d'un corps de requête (`serde(skip)`) : seuls les points d'entrée typés de N1-3 le posent.
    #[serde(skip)]
    pub volet: Option<Volet>,
}

/// Le volet d'une demande déclarée par une brique propriétaire (§8.3).
#[derive(Debug, Clone)]
pub enum Volet {
    Hse(crate::demandes_hse::VoletHse),
    Repro(crate::demandes_repro::VoletRepro),
}

/// Ce que le dépôt connaît de la demande en cours de création, remis au volet pour qu'il s'écrive avec elle.
pub(crate) struct ContexteVolet {
    pub(crate) etab: Uuid,
    pub(crate) demande_id: Uuid,
    pub(crate) annee_id: Uuid,
    pub(crate) jour: NaiveDate,
    pub(crate) deposant: Uuid,
    pub(crate) demandeur: Uuid,
    pub(crate) site_id: Option<Uuid>,
}

/// Geste sur une demande existante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Geste {
    /// Prendre en charge (un membre de la file ; une seule prise active, R-DM9).
    Prendre,
    /// Relâcher sa prise.
    Relacher,
    /// Annulation par le demandeur, tant que la demande n'est pas prise.
    Annuler,
    /// Clôturer (traitant).
    Cloturer { issue: Issue },
}

/// Quotas de débit (NJ-72). Par personne, partagés entre instances (seau en base).
#[derive(Debug, Clone, Copy)]
pub struct QuotasDemandes {
    pub depot: Quota,
    pub geste: Quota,
}

impl QuotasDemandes {
    /// Valeurs par défaut, réglables par `ENT_DEMANDES_DEPOT_QUOTA` et `ENT_DEMANDES_GESTE_QUOTA` (`max/secondes`).
    pub fn depuis_env() -> Self {
        Self {
            depot: Quota::depuis_env("ENT_DEMANDES_DEPOT_QUOTA", Some(Quota::new(20, 3600)))
                .unwrap_or(Quota::new(20, 3600)),
            geste: Quota::depuis_env("ENT_DEMANDES_GESTE_QUOTA", Some(Quota::new(240, 3600)))
                .unwrap_or(Quota::new(240, 3600)),
        }
    }
}

/// Rapport d'une purge : nombres par nature, **aucun nom, aucun identifiant de personne** (§15.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RapportPurge {
    pub purgees: u32,
    pub par_nature: BTreeMap<String, u32>,
    pub gelees_ignorees: u32,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct LigneDemande {
    pub(crate) numero: String,
    pub(crate) nature_code: String,
    pub(crate) statut: String,
    pub(crate) confidentialite: String,
    pub(crate) demandeur_personne_id: Option<Uuid>,
    pub(crate) deposant_personne_id: Option<Uuid>,
    pub(crate) file_id: Uuid,
    pub(crate) attribuee_a_agent_id: Option<Uuid>,
    pub(crate) eleve_id: Option<Uuid>,
    pub(crate) version: i32,
}

/// Le lien d'un acteur avec une demande. **Un seul chemin** les calcule : [`charger_pour_acces`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lien {
    Demandeur,
    Deposant,
    /// Membre (calculé) de la file de destination, hors personne visée (R-DM47).
    Traitant,
    /// Autre responsable du même foyer, ajouté par le demandeur : lit et écrit côté demandeur.
    CoDemandeur,
    /// Collègue tenu informé : lit ce qui est visible du demandeur, n'écrit pas.
    CopieInterne,
    /// « +1 » : ne reçoit que l'avis de clôture, ne lit rien.
    Appui,
    /// Tiers consulté (§11.6) : ne voit que sa consultation.
    Consulte(Uuid),
}

pub(crate) struct FaitNouveau<'a> {
    pub(crate) demande_id: Uuid,
    pub(crate) type_: &'a str,
    pub(crate) auteur_personne: Option<Uuid>,
    pub(crate) auteur_tache: Option<&'a str>,
    pub(crate) statut_avant: Option<&'a str>,
    pub(crate) statut_apres: Option<&'a str>,
    pub(crate) file_avant: Option<Uuid>,
    pub(crate) file_apres: Option<Uuid>,
    pub(crate) agent_avant: Option<Uuid>,
    pub(crate) agent_apres: Option<Uuid>,
    pub(crate) motif: Option<&'a str>,
    pub(crate) ref_id: Option<Uuid>,
}

impl<'a> FaitNouveau<'a> {
    pub(crate) fn nouveau(demande_id: Uuid, type_: &'a str) -> Self {
        Self {
            demande_id,
            type_,
            auteur_personne: None,
            auteur_tache: None,
            statut_avant: None,
            statut_apres: None,
            file_avant: None,
            file_apres: None,
            agent_avant: None,
            agent_apres: None,
            motif: None,
            ref_id: None,
        }
    }
}

/// Ajoute un fait. Le rang, l'empreinte précédente et l'empreinte sont posés par la base (migration 3040).
pub(crate) async fn ajouter_fait(conn: &mut PgConnection, etab: Uuid, f: FaitNouveau<'_>) -> Result<(), RefusDemande> {
    sqlx::query(
        "INSERT INTO ent_dem_fait (id, etablissement_id, demande_id, type, auteur_personne_id, auteur_tache,
                                   statut_avant, statut_apres, file_avant_id, file_apres_id,
                                   agent_avant_id, agent_apres_id, motif_code, ref_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(f.demande_id)
    .bind(f.type_)
    .bind(f.auteur_personne)
    .bind(f.auteur_tache)
    .bind(f.statut_avant)
    .bind(f.statut_apres)
    .bind(f.file_avant)
    .bind(f.file_apres)
    .bind(f.agent_avant)
    .bind(f.agent_apres)
    .bind(f.motif)
    .bind(f.ref_id)
    .execute(conn)
    .await?;
    Ok(())
}

/// Publie un événement de demande **après contrôle de l'enveloppe neutre**.
pub(crate) async fn publier_enveloppe(
    conn: &mut PgConnection,
    etab: Uuid,
    type_name: &str,
    env: &EnveloppeNeutre,
    acteur: Option<Uuid>,
) -> Result<(), RefusDemande> {
    let charge = env.vers_valeur();
    if let Err(raison) = controler_enveloppe(&charge) {
        tracing::error!(raison, "demandes : enveloppe d'événement refusée");
        return Err(RefusDemande::Indisponible);
    }
    publier(conn, etab, type_name, AggregateKind::Demande, env.demande_id, None, charge, acteur, Causalite::default())
        .await?;
    Ok(())
}

/// La personne (non fusionnée) du compte dans l'établissement. Sans fiche de personne : aucun accès.
pub(crate) async fn personne_du_user(conn: &mut PgConnection, etab: Uuid, user_id: Uuid) -> Result<Uuid, RefusDemande> {
    let r: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM ent_personne
          WHERE etablissement_id = $1 AND user_id = $2 AND fusionnee_dans IS NULL
          ORDER BY created_at LIMIT 1",
    )
    .bind(etab)
    .bind(user_id)
    .fetch_optional(conn)
    .await?;
    r.map(|(id,)| id).ok_or(RefusDemande::Interdit)
}

/// Ajoute `n` jours ouvrés (hors week-end) ; les fermetures et jours fériés du site viennent avec N1-1 (R-DM22).
pub(crate) fn ajouter_jours_ouvres(depart: NaiveDate, n: u16) -> NaiveDate {
    let (mut d, mut restant) = (depart, n);
    while restant > 0 {
        d = d.succ_opt().unwrap_or(d);
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            restant -= 1;
        }
    }
    d
}

/// Échéance en jours ouvrés du service (calendrier du socle 5). Si le calendrier est illisible, repli sur les seuls
/// week-ends (journalisé) : une échéance approximative vaut mieux qu'un dépôt refusé.
pub(crate) async fn echeance_ouvree(conn: &mut PgConnection, etab: Uuid, depuis: NaiveDate, n: u16, service: &str) -> NaiveDate {
    match crate::structure_calendrier::calendrier::echeance_jours_ouvres(conn, etab, depuis, u32::from(n), service).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, "demandes : calendrier illisible, échéance sans fermetures");
            ajouter_jours_ouvres(depuis, n)
        }
    }
}

impl PgEnt {
    pub(crate) async fn limiter(&self, route: &str, user: Uuid, quota: Quota) -> Result<(), RefusDemande> {
        match self.limiteur_debit().verifier(route, "user", &user.to_string(), quota).await {
            Decision::Autorise => Ok(()),
            Decision::Limite { retry_after_secs } => Err(RefusDemande::TropDeDemandes { retry_after_secs }),
        }
    }

    /// Dépose une demande **au nom de la personne de la session** (NJ-84), avec les quotas par défaut.
    pub async fn deposer_demande_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        depot: DepotDemande,
    ) -> Result<EtatDemande, RefusDemande> {
        self.deposer_demande_avec_quotas(etab, user_id, role, sources, depot, QuotasDemandes::depuis_env()).await
    }

    /// Dépôt avec quotas explicites (tests, réglage par l'appelant).
    pub async fn deposer_demande_avec_quotas(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        depot: DepotDemande,
        quotas: QuotasDemandes,
    ) -> Result<EtatDemande, RefusDemande> {
        // NJ-72 : le débit se contrôle AVANT tout travail ; la clé est la personne authentifiée.
        self.limiter("demandes.deposer", user_id, quotas.depot).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        let decl = nature(&depot.nature_code).ok_or(RefusDemande::Invalide("Cette démarche n'existe pas."))?;

        // Contrôles sans base : saisie.
        let objet = depot.objet.trim();
        if objet.is_empty() || objet.chars().count() > 140 || objet.contains('\0') {
            return Err(RefusDemande::Invalide("L'objet de la demande est obligatoire (140 caractères au plus)."));
        }
        let texte = match (&depot.texte, decl.texte_max) {
            (None, _) => None,
            (Some(t), _) if t.trim().is_empty() => None,
            (Some(_), None) => return Err(RefusDemande::Invalide("Cette démarche n'accepte pas de texte libre.")),
            (Some(t), Some(max)) if t.chars().count() > max || t.contains('\0') => {
                return Err(RefusDemande::Invalide("Le texte est trop long."));
            }
            (Some(t), Some(_)) => Some(t.trim().to_string()),
        };
        valider_valeurs(decl, &depot.valeurs)?;
        // Une nature qui porte un volet ne se dépose que par son point d'entrée dédié, et le volet doit être le sien.
        match (&depot.volet, decl.code) {
            (Some(Volet::Hse(_)), "rh.hse") | (Some(Volet::Repro(_)), "reprographie.tirage") => {}
            (None, "rh.hse" | "reprographie.tirage") | (Some(_), _) => {
                return Err(RefusDemande::Invalide("Cette démarche se dépose par son formulaire dédié."));
            }
            (None, _) => {}
        }
        if decl.eleve_requis && depot.eleve_id.is_none() {
            return Err(RefusDemande::Invalide("Cette démarche concerne un élève : choisissez-le."));
        }
        if !decl.eleve_requis && depot.eleve_id.is_some() {
            return Err(RefusDemande::Invalide("Cette démarche ne concerne pas un élève."));
        }

        // Droit de déposer : le personnel par l'action, les familles et élèves par les droits de lien (D-017).
        if role.is_staff() {
            if !decl.publics.contains(&PublicNature::Personnel)
                || !self.a_action_etablissement(etab, user_id, role.as_str(), Action::DemandesDeposer).await?
            {
                return Err(RefusDemande::Interdit);
            }
            if depot.pour_le_compte_de.is_some()
                && !self
                    .a_action_etablissement(etab, user_id, role.as_str(), Action::DemandesDeposerPourCompte)
                    .await?
            {
                return Err(RefusDemande::Interdit);
            }
        } else {
            let public_ok = match role {
                Role::Parent => decl.publics.contains(&PublicNature::Responsable),
                _ => decl.publics.contains(&PublicNature::EleveMajeur) || decl.publics.contains(&PublicNature::EleveMineur),
            };
            // Une famille ne dépose jamais « pour le compte » d'autrui (R-DM4).
            if !public_ok
                || depot.pour_le_compte_de.is_some()
                || !sources.peut_deposer_famille(etab, user_id, role, depot.eleve_id)
            {
                return Err(RefusDemande::Interdit);
            }
        }
        // R-DM6 : une demande a toujours une file. Sans file, pas de dépôt (jamais « tout le personnel »).
        // CDM-07 : nature non (ou plus) proposée à ce public → phrase claire, aucune demande créée.
        if let Some(phrase) = sources.refus_nature(etab, decl.code) {
            return Err(RefusDemande::Invalide(phrase));
        }
        let Some(routage) = sources.routage_de_depot(etab, decl.code) else {
            tracing::error!(nature = decl.code, "demandes : aucune file de dépôt (source absente)");
            return Err(RefusDemande::Indisponible);
        };
        let file_id = routage.file_id;
        let canal = match depot.canal.unwrap_or(Canal::Web) {
            c @ (Canal::Web | Canal::Mobile) => c,
            // Les canaux comptoir, téléphone, courrier… viennent avec leurs lots, et toujours par une session.
            _ => return Err(RefusDemande::Invalide("Ce mode de dépôt n'est pas disponible.")),
        };

        let id = Uuid::new_v4();
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let deposant = personne_du_user(&mut tx, etab, user_id).await?;
        // Demandeur : la personne de la session, ou la personne désignée par un déposant habilité (tracé).
        let demandeur = match depot.pour_le_compte_de {
            None => deposant,
            Some(p) if p == deposant => deposant,
            Some(p) => {
                let existe: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT id FROM ent_personne WHERE etablissement_id = $1 AND id = $2 AND fusionnee_dans IS NULL",
                )
                .bind(etab)
                .bind(p)
                .fetch_optional(&mut *tx)
                .await?;
                // Réponse neutre : on ne dit pas si la personne existe ailleurs.
                existe.map(|(p,)| p).ok_or(RefusDemande::Invalide("La personne désignée est introuvable."))?
            }
        };
        let pour_compte = demandeur != deposant;

        // Année et jour métier de l'établissement (jamais la date du serveur ni le jour UTC).
        let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())").bind(etab).fetch_one(&mut *tx).await?;
        let annee: Option<(Uuid, NaiveDate)> = sqlx::query_as(
            "SELECT id, date_debut FROM ent_annee_scolaire
              WHERE etablissement_id = $1 AND (courante OR (date_debut <= $2 AND date_fin >= $2))
              ORDER BY courante DESC, date_debut DESC LIMIT 1",
        )
        .bind(etab)
        .bind(jour)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((annee_id, annee_debut)) = annee else {
            tracing::error!("demandes : aucune année scolaire pour numéroter");
            return Err(RefusDemande::Indisponible);
        };
        let (n,): (i32,) = sqlx::query_as("SELECT ent_dem_numero_suivant($1, $2)").bind(etab).bind(annee_id).fetch_one(&mut *tx).await?;
        let numero = format!("D{}-{n:05}", annee_debut.year());

        // C2+ : le texte libre n'est jamais écrit en clair (champs chiffrés, fail-closed sans clé).
        let scelle = !matches!(decl.confidentialite, Confidentialite::C0 | Confidentialite::C1);
        let texte_stocke = match &texte {
            Some(t) if scelle => Some(
                ChampsSensibles::depuis_env()
                    .sceller("ent_dem_demande", "texte", id, t)
                    .map_err(|_| RefusDemande::Indisponible)?,
            ),
            autre => autre.clone(),
        };
        let urgence = depot.urgence.unwrap_or(Urgence::Normale);

        sqlx::query(
            "INSERT INTO ent_dem_demande (id, etablissement_id, numero, annee_id, nature_code, nature_version, statut,
                    confidentialite, urgence, demandeur_personne_id, deposant_personne_id, canal_depot, eleve_id,
                    objet, texte, texte_scelle, file_id, site_id, depose_le)
             VALUES ($1, $2, $3, $4, $5, $6, 'deposee', $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, now())",
        )
        .bind(id)
        .bind(etab)
        .bind(&numero)
        .bind(annee_id)
        .bind(decl.code)
        .bind(decl.version)
        .bind(decl.confidentialite.code())
        .bind(urgence.code())
        .bind(demandeur)
        .bind(pour_compte.then_some(deposant))
        .bind(canal.code())
        .bind(depot.eleve_id)
        .bind(objet)
        .bind(texte_stocke.as_deref())
        .bind(scelle && texte_stocke.is_some())
        .bind(file_id)
        .bind(routage.site_id)
        .execute(&mut *tx)
        .await?;

        for s in &depot.valeurs {
            inserer_valeur(&mut tx, etab, id, s).await?;
        }
        if let Some(v) = &depot.volet {
            let ctx = ContexteVolet {
                etab,
                demande_id: id,
                annee_id,
                jour,
                deposant,
                demandeur,
                site_id: routage.site_id,
            };
            match v {
                Volet::Hse(h) => crate::demandes_hse::ecrire_volet(&mut tx, &ctx, h).await?,
                Volet::Repro(r) => crate::demandes_repro::ecrire_volet(&mut tx, &ctx, r).await?,
            }
        }

        // Échéances : prise en charge et traitement (jours ouvrés ; fermetures du site en N1-1).
        // Jours ouvrés du calendrier du service de la file : fermetures et jours fériés exclus ; un dépôt pendant une
        // fermeture voit son délai partir à la réouverture (R-DM22). Les délais effectifs ne dépassent jamais ceux
        // que la nature déclare (l'établissement raccourcit, il ne rallonge pas).
        let delai_prise = routage.delai_prise_jours.map_or(decl.delai_prise_jours, |d| d.min(decl.delai_prise_jours));
        let delai_traitement = routage
            .delai_traitement_jours
            .map_or(decl.delai_traitement_jours, |d| d.min(decl.delai_traitement_jours))
            .max(delai_prise);
        for (type_, jours) in [("prise_en_charge", delai_prise), ("traitement", delai_traitement)] {
            let due = echeance_ouvree(&mut tx, etab, jour, jours, &routage.service_calendrier).await;
            sqlx::query(
                "INSERT INTO ent_dem_echeance (id, etablissement_id, demande_id, type, due_le)
                 VALUES ($1, $2, $3, $4,
                         ($5::date + time '23:59:59') AT TIME ZONE COALESCE(
                            (SELECT fuseau FROM ent_etablissement_parametre WHERE etablissement_id = $2), 'Europe/Paris'))",
            )
            .bind(Uuid::new_v4())
            .bind(etab)
            .bind(id)
            .bind(type_)
            .bind(due)
            .execute(&mut *tx)
            .await?;
        }

        // Le déposant est toujours enregistré (R-DM5) : auteur du fait, motif « pour le compte » le cas échéant.
        let mut fait = FaitNouveau::nouveau(id, "depose");
        fait.auteur_personne = Some(deposant);
        fait.statut_apres = Some(Statut::Deposee.code());
        fait.file_apres = Some(file_id);
        fait.motif = pour_compte.then_some("pour_le_compte");
        ajouter_fait(&mut tx, etab, fait).await?;
        // La raison du routage est un fait : lisible des membres de la file, jamais du demandeur (§10.1).
        if let Some(motif) = routage.motif {
            let mut r = FaitNouveau::nouveau(id, "routage");
            r.auteur_tache = Some("routage_deterministe");
            r.file_apres = Some(file_id);
            r.motif = Some(motif);
            r.ref_id = routage.ref_id;
            ajouter_fait(&mut tx, etab, r).await?;
        }

        let env = EnveloppeNeutre::nouvelle(id, &numero, decl.code, "deposee");
        publier_enveloppe(&mut tx, etab, types::DEMANDE_DEPOSEE, &env, Some(user_id)).await?;
        tx.commit().await?;
        Ok(EtatDemande { id, numero, statut: Statut::Deposee.code().into(), version: 1 })
    }

    /// Mes demandes (projection **demandeur**) : celles dont je suis le demandeur, et celles que j'ai déposées pour
    /// autrui jusqu'à 30 jours après leur clôture (§24.2).
    pub async fn mes_demandes_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<DemandeDemandeur>, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let lignes: Vec<LigneVueDemandeur> = sqlx::query_as(
            "SELECT id, numero, nature_code, statut, issue, decision, urgence, objet, texte, texte_scelle,
                    depose_le, clos_le, version
               FROM ent_dem_vue_demandeur
              WHERE etablissement_id = $1
                AND (demandeur_personne_id = $2
                     OR (deposant_personne_id = $2 AND (clos_le IS NULL OR clos_le > now() - interval '30 days'))
                     OR EXISTS (SELECT 1 FROM ent_dem_abonne ab
                                 WHERE ab.etablissement_id = ent_dem_vue_demandeur.etablissement_id
                                   AND ab.demande_id = ent_dem_vue_demandeur.id AND ab.personne_id = $2
                                   AND ab.qualite = 'co_demandeur' AND ab.au IS NULL))
              ORDER BY cree_le DESC, id DESC
              LIMIT 500",
        )
        .bind(etab)
        .bind(moi)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        lignes.into_iter().map(LigneVueDemandeur::ouvrir).collect()
    }

    /// Une demande vue par son **demandeur ou son déposant** (projection demandeur). Toute autre personne : 404.
    pub async fn demande_demandeur_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<DemandeDemandeur, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await.map_err(|_| RefusDemande::Introuvable)?;
        let l: Option<LigneVueDemandeur> = sqlx::query_as(
            "SELECT id, numero, nature_code, statut, issue, decision, urgence, objet, texte, texte_scelle,
                    depose_le, clos_le, version
               FROM ent_dem_vue_demandeur
              WHERE etablissement_id = $1 AND id = $2
                AND (demandeur_personne_id = $3
                     OR (deposant_personne_id = $3 AND (clos_le IS NULL OR clos_le > now() - interval '30 days'))
                     OR EXISTS (SELECT 1 FROM ent_dem_abonne ab
                                 WHERE ab.etablissement_id = ent_dem_vue_demandeur.etablissement_id
                                   AND ab.demande_id = ent_dem_vue_demandeur.id AND ab.personne_id = $3
                                   AND ab.qualite = 'co_demandeur' AND ab.au IS NULL))",
        )
        .bind(etab)
        .bind(id)
        .bind(moi)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        l.ok_or(RefusDemande::Introuvable)?.ouvrir()
    }

    /// Historique d'une demande **tel que le demandeur le voit** : étapes publiques seulement, sans agent ni file.
    pub async fn historique_demandeur_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Vec<EtapeVisible>, RefusDemande> {
        // Le contrôle d'accès est celui de la projection demandeur (404 neutre sinon).
        self.demande_demandeur_for_actor(etab, user_id, id).await?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let lignes: Vec<(String, DateTime<Utc>, Option<String>)> = sqlx::query_as(
            "SELECT type, le, statut_apres FROM ent_dem_fait
              WHERE etablissement_id = $1 AND demande_id = $2 AND type = ANY($3)
              ORDER BY rang",
        )
        .bind(etab)
        .bind(id)
        .bind(FAITS_VISIBLES_DEMANDEUR.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(lignes.into_iter().map(|(etape, le, statut)| EtapeVisible { etape, le, statut }).collect())
    }

    /// Une demande vue par un **membre de sa file** (projection traitant), par identifiant ou par numéro. Hors file
    /// et non demandeur : **404 neutre**, même si la demande existe (R-DM10, matrice §24.3 piège 1). La lecture du
    /// texte d'une nature C2+ est tracée (R-DM16).
    pub async fn demande_traitant_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        reference: RefDemande,
    ) -> Result<DemandeTraitant, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        if !role.is_staff() {
            return Err(RefusDemande::Introuvable);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await.map_err(|_| RefusDemande::Introuvable)?;
        let files = sources.files_de(etab, moi);
        let l: Option<LigneVueTraitant> = match reference {
            RefDemande::Id(id) => {
                sqlx::query_as(&format!("{SELECT_TRAITANT} WHERE etablissement_id = $1 AND id = $2 AND file_id = ANY($3) AND {SANS_VISEE_4}"))
                    .bind(etab)
                    .bind(id)
                    .bind(&files)
                    .bind(moi)
                    .fetch_optional(&mut *tx)
                    .await?
            }
            RefDemande::Numero(n) => {
                sqlx::query_as(&format!("{SELECT_TRAITANT} WHERE etablissement_id = $1 AND numero = $2 AND file_id = ANY($3) AND {SANS_VISEE_4}"))
                    .bind(etab)
                    .bind(n)
                    .bind(&files)
                    .bind(moi)
                    .fetch_optional(&mut *tx)
                    .await?
            }
        };
        let l = l.ok_or(RefusDemande::Introuvable)?;
        let confidentielle = !matches!(l.confidentialite.as_str(), "C0" | "C1");
        if confidentielle {
            // Toute lecture C2+ est tracée (R-DM16) ; le journal d'accès complet vient avec N1-8.
            let mut f = FaitNouveau::nouveau(l.id, "lecture_confidentielle");
            f.auteur_personne = Some(moi);
            ajouter_fait(&mut tx, etab, f).await?;
        }
        // R-DM53, CDM-82 : la PREMIÈRE ouverture du détail par un membre de la file de destination est montrée au
        // demandeur (« vue par le secrétariat »). Ni la liste (`ma_file`), ni une recherche, ni un consulté ne la
        // posent : seul ce chemin, qui ne s'ouvre qu'aux membres de la file. L'unicité est gardée par la base.
        let vue: Option<(Uuid,)> = sqlx::query_as(
            "INSERT INTO ent_dem_fait (id, etablissement_id, demande_id, type, auteur_personne_id, file_apres_id)
             VALUES ($1, $2, $3, 'vue_par_service', $4, $5)
             ON CONFLICT (etablissement_id, demande_id, file_apres_id) WHERE type = 'vue_par_service' DO NOTHING
             RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(etab)
        .bind(l.id)
        .bind(moi)
        .bind(l.file_id)
        .fetch_optional(&mut *tx)
        .await?;
        if vue.is_some() {
            let env = EnveloppeNeutre::nouvelle(l.id, &l.numero, &l.nature_code, "vue");
            publier_enveloppe(&mut tx, etab, types::DEMANDE_VUE, &env, Some(user_id)).await?;
        }
        tx.commit().await?;
        l.ouvrir()
    }

    /// « Ma file » : les demandes des files dont je suis membre, **sans texte** (liste). Vide sans appartenance.
    pub async fn ma_file_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
    ) -> Result<Vec<DemandeTraitant>, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Interdit)?;
        if !role.is_staff() {
            return Err(RefusDemande::Interdit);
        }
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let moi = personne_du_user(&mut tx, etab, user_id).await?;
        let files = sources.files_de(etab, moi);
        let lignes: Vec<LigneVueTraitant> = sqlx::query_as(&format!(
            "{SELECT_TRAITANT} WHERE etablissement_id = $1 AND file_id = ANY($2)
               AND statut NOT IN ('brouillon', 'close', 'annulee') AND {SANS_VISEE_3}
             ORDER BY depose_le, id LIMIT 500"
        ))
        .bind(etab)
        .bind(&files)
        .bind(moi)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        lignes
            .into_iter()
            .map(|mut l| {
                l.texte = None; // une liste ne porte jamais de texte
                l.ouvrir()
            })
            .collect()
    }

    /// Les valeurs typées d'une demande, pour un membre de sa file ou son demandeur (404 neutre sinon).
    pub async fn valeurs_demande_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
    ) -> Result<Vec<ValeurLue>, RefusDemande> {
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let acces = charger_pour_acces(&mut tx, etab, user_id, role, sources, id, false).await?;
        // Les valeurs typées : le demandeur et son déposant, le co-demandeur, les membres de la file. Pas un
        // consulté, pas un appui, pas une copie interne.
        if !matches!(acces.lien, Lien::Demandeur | Lien::Deposant | Lien::CoDemandeur | Lien::Traitant) {
            return Err(RefusDemande::Introuvable);
        }
        let lignes: Vec<LigneValeur> = sqlx::query_as(
            "SELECT champ_code, rang, valeur_texte, valeur_entier, valeur_decimal::text AS valeur_decimal,
                    valeur_date, valeur_heure, valeur_bool, valeur_code, valeur_ref
               FROM ent_dem_valeur
              WHERE etablissement_id = $1 AND demande_id = $2
                AND valeurs_version = (SELECT max(valeurs_version) FROM ent_dem_valeur WHERE etablissement_id = $1 AND demande_id = $2)
              ORDER BY champ_code, rang",
        )
        .bind(etab)
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(lignes.into_iter().filter_map(LigneValeur::vers_lue).collect())
    }

    /// Un geste sur une demande : **seule porte d'écriture** du cœur. Dans l'ordre : débit (NJ-72), chargement avec
    /// verrou de ligne, accès (404 neutre si ni demandeur, ni déposant, ni membre de la file : NJ-80), droit du
    /// geste, version (409 : NJ-90), écriture, fait chaîné, événement à enveloppe neutre.
    pub async fn agir_sur_demande_for_actor(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        geste: Geste,
    ) -> Result<EtatDemande, RefusDemande> {
        self.agir_avec_quotas(etab, user_id, role, sources, id, version_attendue, geste, QuotasDemandes::depuis_env()).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn agir_avec_quotas(
        &self,
        etab: Uuid,
        user_id: Uuid,
        role: &str,
        sources: &dyn SourcesDemandes,
        id: Uuid,
        version_attendue: i32,
        geste: Geste,
        quotas: QuotasDemandes,
    ) -> Result<EtatDemande, RefusDemande> {
        self.limiter("demandes.geste", user_id, quotas.geste).await?;
        let role = Role::parse(role).ok_or(RefusDemande::Introuvable)?;
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let Acces { ligne, moi, agent, lien } =
            charger_pour_acces(&mut tx, etab, user_id, role, sources, id, true).await?;
        let statut = Statut::parse(&ligne.statut).ok_or(RefusDemande::Indisponible)?;

        // Droit du geste (l'accès a déjà été prouvé : on peut dire « interdit »).
        match geste {
            Geste::Annuler if !matches!(lien, Lien::Demandeur | Lien::Deposant) => return Err(RefusDemande::Interdit),
            Geste::Prendre | Geste::Relacher | Geste::Cloturer { .. } if lien != Lien::Traitant => {
                return Err(RefusDemande::Interdit);
            }
            _ => {}
        }
        if ligne.version != version_attendue {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        if statut.est_final() {
            return Err(RefusDemande::Conflit("Cette demande est terminée."));
        }

        let (type_fait, apres, agent_apres, motif): (&str, Statut, Option<Uuid>, Option<&str>) = match geste {
            Geste::Prendre => {
                // R-DM9, NJ-81 : une seule prise active ; on ne reprend pas ce qu'un autre a pris.
                if ligne.attribuee_a_agent_id.is_some() {
                    return Err(RefusDemande::Conflit("Cette demande est déjà prise en charge."));
                }
                let a = agent.ok_or(RefusDemande::Interdit)?;
                ("prise", Statut::Prise, Some(a), None)
            }
            Geste::Relacher => {
                // Seul le porteur de la prise la relâche (la réattribution par le responsable vient avec N1-2).
                if ligne.attribuee_a_agent_id.is_none() || ligne.attribuee_a_agent_id != agent {
                    return Err(RefusDemande::Interdit);
                }
                ("relache", Statut::Deposee, None, None)
            }
            Geste::Annuler => {
                // Annulation par le demandeur tant que la demande n'est pas prise (§8.2 : TantQueNonPrise).
                if ligne.attribuee_a_agent_id.is_some() || !matches!(statut, Statut::Deposee | Statut::Brouillon) {
                    return Err(RefusDemande::Conflit("Cette demande est déjà prise en charge : écrivez au service."));
                }
                ("annulee", Statut::Annulee, None, Some("annulee_par_demandeur"))
            }
            Geste::Cloturer { issue } => {
                // Une demande prise se clôt par son porteur (NJ-81) ; non prise, par un membre de la file.
                if ligne.attribuee_a_agent_id.is_some() && ligne.attribuee_a_agent_id != agent {
                    return Err(RefusDemande::Interdit);
                }
                if matches!(issue, Issue::AnnuleeParDemandeur) {
                    return Err(RefusDemande::Invalide("Cette issue est réservée au demandeur."));
                }
                ("close", Statut::Close, ligne.attribuee_a_agent_id, Some(issue.code()))
            }
        };

        let final_ = apres.est_final();
        // Conservation : calculée à la clôture depuis la catégorie de la nature (§15.3), jour métier de l'établissement.
        let conservation_mois = if final_ { nature(&ligne.nature_code).and_then(|n| n.conservation.duree_mois()) } else { None };
        let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())").bind(etab).fetch_one(&mut *tx).await?;
        let conservation = conservation_mois.and_then(|m| jour.checked_add_months(Months::new(m)));
        let (issue_code, decision) = match geste {
            Geste::Cloturer { issue } => (Some(issue.code()), issue.decision()),
            Geste::Annuler => (Some(Issue::AnnuleeParDemandeur.code()), None),
            _ => (None, None),
        };
        let prise_agent = match geste {
            Geste::Prendre => agent_apres,
            Geste::Cloturer { .. } => ligne.attribuee_a_agent_id,
            _ => None,
        };

        // Version : l'écriture ne réussit que si personne n'est passé entre-temps (R-DM11) — et la base le garde aussi.
        let modifiees = sqlx::query(
            "UPDATE ent_dem_demande
                SET statut = $4, issue = $5, decision = $6,
                    attribuee_a_agent_id = $7,
                    attribuee_le = CASE WHEN $7::uuid IS NULL THEN NULL
                                        WHEN attribuee_a_agent_id IS NOT DISTINCT FROM $7::uuid THEN attribuee_le
                                        ELSE now() END,
                    clos_le = CASE WHEN $8 THEN now() ELSE NULL END,
                    conservation_jusqu_au = $9,
                    version = version + 1
              WHERE etablissement_id = $1 AND id = $2 AND version = $3",
        )
        .bind(etab)
        .bind(id)
        .bind(version_attendue)
        .bind(apres.code())
        .bind(issue_code)
        .bind(decision)
        .bind(prise_agent)
        .bind(final_)
        .bind(conservation)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if modifiees != 1 {
            return Err(RefusDemande::Conflit("La demande a changé entre-temps : rechargez-la."));
        }
        if final_ {
            // Une demande terminée n'a plus d'échéance à surveiller ; rien n'est supprimé, on les marque traitées.
            sqlx::query("UPDATE ent_dem_echeance SET traitee_le = now() WHERE etablissement_id = $1 AND demande_id = $2 AND traitee_le IS NULL")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        } else if matches!(geste, Geste::Prendre) {
            sqlx::query("UPDATE ent_dem_echeance SET traitee_le = now() WHERE etablissement_id = $1 AND demande_id = $2 AND type = 'prise_en_charge' AND traitee_le IS NULL")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }

        let mut fait = FaitNouveau::nouveau(id, type_fait);
        fait.auteur_personne = Some(moi);
        fait.statut_avant = Some(&ligne.statut);
        fait.statut_apres = Some(apres.code());
        fait.agent_avant = ligne.attribuee_a_agent_id;
        fait.agent_apres = prise_agent;
        fait.motif = motif;
        ajouter_fait(&mut tx, etab, fait).await?;

        let evenement = match geste {
            Geste::Prendre => Some((types::DEMANDE_PRISE, "prise")),
            Geste::Relacher => Some((types::DEMANDE_RELACHEE, "relachee")),
            Geste::Annuler | Geste::Cloturer { .. } => Some((types::DEMANDE_CLOSE, "close")),
        };
        if let Some((t, e)) = evenement {
            let env = EnveloppeNeutre::nouvelle(id, &ligne.numero, &ligne.nature_code, e);
            publier_enveloppe(&mut tx, etab, t, &env, Some(user_id)).await?;
        }
        tx.commit().await?;
        Ok(EtatDemande { id, numero: ligne.numero, statut: apres.code().into(), version: version_attendue + 1 })
    }

    /// Purge de conservation : **tâche de l'ordonnanceur (socle 31), jamais une lecture** (R-DM29, NJ-89). Efface le
    /// contenu (valeurs, objet, texte, identités) des demandes closes dont la conservation est échue et qui ne sont
    /// pas gelées. Les **faits restent** (preuve de ce qui s'est passé) : ils ne portent aucun texte. Rapport sans nom.
    pub async fn purger_demandes_echues(&self, etab: Uuid) -> Result<RapportPurge, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        // Réglage LOCAL à cette transaction : la base n'accepte l'effacement que sous ce réglage.
        sqlx::query("SELECT set_config('ent.dem_purge', 'on', true)").execute(&mut *tx).await?;
        let (jour,): (NaiveDate,) = sqlx::query_as("SELECT ent_aujourdhui_etab($1, now())").bind(etab).fetch_one(&mut *tx).await?;
        let (gelees,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM ent_dem_demande
              WHERE etablissement_id = $1 AND purgee_le IS NULL AND gel AND conservation_jusqu_au < $2",
        )
        .bind(etab)
        .bind(jour)
        .fetch_one(&mut *tx)
        .await?;
        let echues: Vec<(Uuid, String, i32)> = sqlx::query_as(
            "SELECT id, nature_code, version FROM ent_dem_demande
              WHERE etablissement_id = $1 AND purgee_le IS NULL AND NOT gel
                AND statut IN ('close', 'annulee') AND conservation_jusqu_au < $2
              ORDER BY clos_le, id LIMIT 500 FOR UPDATE SKIP LOCKED",
        )
        .bind(etab)
        .bind(jour)
        .fetch_all(&mut *tx)
        .await?;
        let mut rapport = RapportPurge { gelees_ignorees: gelees as u32, ..Default::default() };
        for (id, nature_code, version) in echues {
            sqlx::query("DELETE FROM ent_dem_valeur WHERE etablissement_id = $1 AND demande_id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM ent_dem_echeance WHERE etablissement_id = $1 AND demande_id = $2")
                .bind(etab)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE ent_dem_demande
                    SET objet = NULL, texte = NULL, texte_scelle = false, demandeur_personne_id = NULL,
                        eleve_id = NULL, beneficiaire_agent_id = NULL, lieu_id = NULL,
                        purgee_le = now(), version = version + 1
                  WHERE etablissement_id = $1 AND id = $2 AND version = $3",
            )
            .bind(etab)
            .bind(id)
            .bind(version)
            .execute(&mut *tx)
            .await?;
            let mut f = FaitNouveau::nouveau(id, "purgee");
            f.auteur_tache = Some("purge_conservation");
            ajouter_fait(&mut tx, etab, f).await?;
            let (numero,): (String,) = sqlx::query_as("SELECT numero FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2")
                .bind(etab)
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
            let env = EnveloppeNeutre::nouvelle(id, &numero, &nature_code, "purgee");
            publier_enveloppe(&mut tx, etab, types::DEMANDE_PURGEE, &env, None).await?;
            rapport.purgees += 1;
            *rapport.par_nature.entry(nature_code).or_default() += 1;
        }
        tx.commit().await?;
        Ok(rapport)
    }

    /// Vérifie la chaîne des faits d'une demande : `None` = intacte, `Some(rang)` = premier maillon altéré.
    pub async fn verifier_chaine_demande(&self, etab: Uuid, id: Uuid) -> Result<Option<i32>, RefusDemande> {
        let mut tx = TenantTx::begin(self.pool(), etab).await?;
        let (r,): (Option<i32>,) = sqlx::query_as("SELECT ent_dem_verifier_chaine($1, $2)").bind(etab).bind(id).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(r)
    }
}

/// Référence à une demande : par identifiant ou par numéro lisible.
#[derive(Debug, Clone)]
pub enum RefDemande {
    Id(Uuid),
    Numero(String),
}

// ───────────────────────── Lectures internes ─────────────────────────

/// R-DM47 : la personne visée par la demande (paramètre `$4`, ou `$3` en liste) ne la voit pas, même membre de la file.
const SANS_VISEE_4: &str = "NOT EXISTS (SELECT 1 FROM ent_dem_personne_visee v
        WHERE v.etablissement_id = ent_dem_vue_traitant.etablissement_id
          AND v.demande_id = ent_dem_vue_traitant.id AND v.personne_id = $4)";
const SANS_VISEE_3: &str = "NOT EXISTS (SELECT 1 FROM ent_dem_personne_visee v
        WHERE v.etablissement_id = ent_dem_vue_traitant.etablissement_id
          AND v.demande_id = ent_dem_vue_traitant.id AND v.personne_id = $3)";

const SELECT_TRAITANT: &str = "SELECT id, numero, nature_code, statut, confidentialite, urgence, demandeur_personne_id,
        objet, texte, texte_scelle, file_id, attribuee_a_agent_id, depose_le, clos_le, gel, version
   FROM ent_dem_vue_traitant";

pub(crate) async fn charger_demande(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    verrou: bool,
) -> Result<LigneDemande, RefusDemande> {
    let sql = if verrou {
        "SELECT numero, nature_code, statut, confidentialite, demandeur_personne_id, deposant_personne_id,
                file_id, attribuee_a_agent_id, eleve_id, version
           FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2 AND purgee_le IS NULL FOR UPDATE"
    } else {
        "SELECT numero, nature_code, statut, confidentialite, demandeur_personne_id, deposant_personne_id,
                file_id, attribuee_a_agent_id, eleve_id, version
           FROM ent_dem_demande WHERE etablissement_id = $1 AND id = $2 AND purgee_le IS NULL"
    };
    let l: Option<LigneDemande> = sqlx::query_as(sql).bind(etab).bind(id).fetch_optional(conn).await?;
    l.ok_or(RefusDemande::Introuvable)
}

/// Ce que la garde d'accès rend : la demande, l'acteur et son lien.
pub(crate) struct Acces {
    pub(crate) ligne: LigneDemande,
    pub(crate) moi: Uuid,
    pub(crate) agent: Option<Uuid>,
    pub(crate) lien: Lien,
}

/// **La garde d'accès unique** (NJ-80, NJ-81, NJ-83) : toute lecture ou écriture d'une demande, de son fil, de ses
/// pièces, de ses consultations, passe ici. Sans lien — ni demandeur, ni déposant, ni membre (non visé) de la file
/// de destination, ni abonné, ni consulté — la demande **n'existe pas** pour l'acteur : 404 neutre, que la demande
/// existe ou non, qu'elle soit dans une autre file ou dans un autre établissement. Une demande d'une autre file ne
/// s'ouvre donc jamais, même à un responsable de file (NJ-81).
pub(crate) async fn charger_pour_acces(
    conn: &mut PgConnection,
    etab: Uuid,
    user_id: Uuid,
    role: Role,
    sources: &dyn SourcesDemandes,
    id: Uuid,
    verrou: bool,
) -> Result<Acces, RefusDemande> {
    // Sans fiche de personne dans cet établissement : la demande n'existe pas pour cet acteur.
    let moi = personne_du_user(conn, etab, user_id).await.map_err(|_| RefusDemande::Introuvable)?;
    let ligne = charger_demande(conn, etab, id, verrou).await?;
    let files = sources.files_de(etab, moi);
    let lien = lien_de(conn, etab, id, &ligne, moi, role, &files).await?.ok_or(RefusDemande::Introuvable)?;
    let agent = agent_du_user_tx(conn, etab, user_id).await?;
    Ok(Acces { ligne, moi, agent, lien })
}

async fn lien_de(
    conn: &mut PgConnection,
    etab: Uuid,
    id: Uuid,
    l: &LigneDemande,
    moi: Uuid,
    role: Role,
    files: &[Uuid],
) -> Result<Option<Lien>, RefusDemande> {
    if l.demandeur_personne_id == Some(moi) {
        return Ok(Some(Lien::Demandeur));
    }
    if l.deposant_personne_id == Some(moi) {
        return Ok(Some(Lien::Deposant));
    }
    // Le traitant est un membre de la file ET un personnel ; la personne visée n'est jamais traitante (R-DM47).
    if role.is_staff() && files.contains(&l.file_id) {
        let visee: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM ent_dem_personne_visee
                             WHERE etablissement_id = $1 AND demande_id = $2 AND personne_id = $3)",
        )
        .bind(etab)
        .bind(id)
        .bind(moi)
        .fetch_one(&mut *conn)
        .await?;
        if !visee.0 {
            return Ok(Some(Lien::Traitant));
        }
        return Ok(None);
    }
    // Abonnés : ce que leur qualité ouvre, sur la période de l'abonnement (jour métier de l'établissement).
    let ab: Option<(String,)> = sqlx::query_as(
        "SELECT qualite FROM ent_dem_abonne
          WHERE etablissement_id = $1 AND demande_id = $2 AND personne_id = $3
            AND au IS NULL AND du <= ent_aujourdhui_etab($1, now())
          ORDER BY CASE qualite WHEN 'co_demandeur' THEN 0 WHEN 'copie_interne' THEN 1 ELSE 2 END LIMIT 1",
    )
    .bind(etab)
    .bind(id)
    .bind(moi)
    .fetch_optional(&mut *conn)
    .await?;
    match ab.as_ref().map(|(q,)| q.as_str()) {
        Some("co_demandeur") if !role.is_staff() => return Ok(Some(Lien::CoDemandeur)),
        Some("copie_interne") if role.is_staff() => return Ok(Some(Lien::CopieInterne)),
        Some("appui") => return Ok(Some(Lien::Appui)),
        _ => {}
    }
    // Consulté (agent nommé) : sa consultation seulement.
    let c: Option<(Uuid,)> = sqlx::query_as(
        "SELECT c.id FROM ent_dem_consultation c
           JOIN ent_agent a ON a.etablissement_id = c.etablissement_id AND a.id = c.consulte_agent_id
          WHERE c.etablissement_id = $1 AND c.demande_id = $2 AND a.personne_id = $3
          ORDER BY c.cree_le DESC LIMIT 1",
    )
    .bind(etab)
    .bind(id)
    .bind(moi)
    .fetch_optional(conn)
    .await?;
    Ok(c.map(|(c,)| Lien::Consulte(c)).filter(|_| role.is_staff()))
}

async fn inserer_valeur(conn: &mut PgConnection, etab: Uuid, demande: Uuid, s: &ValeurSaisie) -> Result<(), RefusDemande> {
    let (mut texte, mut entier, mut decimal, mut date, mut heure, mut booleen, mut code, mut reference) =
        (None, None, None, None, None, None, None, None);
    match &s.valeur {
        Valeur::Texte(v) => texte = Some(v.trim().to_string()),
        Valeur::Entier(v) => entier = Some(*v),
        Valeur::Decimal(v) => decimal = Some(v.clone()),
        Valeur::Date(v) => date = Some(*v),
        Valeur::Heure(v) => heure = Some(*v),
        Valeur::Bool(v) => booleen = Some(*v),
        Valeur::Code(v) => code = Some(v.clone()),
        Valeur::Ref(v) => reference = Some(*v),
    }
    sqlx::query(
        "INSERT INTO ent_dem_valeur (id, etablissement_id, demande_id, valeurs_version, champ_code, rang,
                valeur_texte, valeur_entier, valeur_decimal, valeur_date, valeur_heure, valeur_bool, valeur_code, valeur_ref)
         VALUES ($1, $2, $3, 1, $4, $5, $6, $7, $8::numeric, $9, $10, $11, $12, $13)",
    )
    .bind(Uuid::new_v4())
    .bind(etab)
    .bind(demande)
    .bind(&s.champ)
    .bind(s.rang as i16)
    .bind(texte)
    .bind(entier)
    .bind(decimal)
    .bind(date)
    .bind(heure)
    .bind(booleen)
    .bind(code)
    .bind(reference)
    .execute(conn)
    .await?;
    Ok(())
}

#[derive(Debug, FromRow)]
struct LigneVueDemandeur {
    id: Uuid,
    numero: String,
    nature_code: String,
    statut: String,
    issue: Option<String>,
    decision: Option<String>,
    urgence: String,
    objet: Option<String>,
    texte: Option<String>,
    texte_scelle: bool,
    depose_le: Option<DateTime<Utc>>,
    clos_le: Option<DateTime<Utc>>,
    version: i32,
}

impl LigneVueDemandeur {
    fn ouvrir(self) -> Result<DemandeDemandeur, RefusDemande> {
        let texte = ouvrir_texte(self.id, self.texte, self.texte_scelle)?;
        Ok(DemandeDemandeur {
            id: self.id,
            numero: self.numero,
            nature_code: self.nature_code,
            statut: self.statut,
            issue: self.issue,
            decision: self.decision,
            urgence: self.urgence,
            objet: self.objet,
            texte,
            depose_le: self.depose_le,
            clos_le: self.clos_le,
            version: self.version,
        })
    }
}

#[derive(Debug, FromRow)]
struct LigneVueTraitant {
    id: Uuid,
    numero: String,
    nature_code: String,
    statut: String,
    confidentialite: String,
    urgence: String,
    demandeur_personne_id: Option<Uuid>,
    objet: Option<String>,
    texte: Option<String>,
    texte_scelle: bool,
    file_id: Uuid,
    attribuee_a_agent_id: Option<Uuid>,
    depose_le: Option<DateTime<Utc>>,
    clos_le: Option<DateTime<Utc>>,
    gel: bool,
    version: i32,
}

impl LigneVueTraitant {
    fn ouvrir(self) -> Result<DemandeTraitant, RefusDemande> {
        let texte = ouvrir_texte(self.id, self.texte, self.texte_scelle)?;
        Ok(DemandeTraitant {
            id: self.id,
            numero: self.numero,
            nature_code: self.nature_code,
            statut: self.statut,
            confidentialite: self.confidentialite,
            urgence: self.urgence,
            demandeur_personne_id: self.demandeur_personne_id,
            objet: self.objet,
            texte,
            file_id: self.file_id,
            attribuee_a_agent_id: self.attribuee_a_agent_id,
            depose_le: self.depose_le,
            clos_le: self.clos_le,
            gel: self.gel,
            version: self.version,
        })
    }
}

pub(crate) fn ouvrir_texte(id: Uuid, stocke: Option<String>, scelle: bool) -> Result<Option<String>, RefusDemande> {
    match stocke {
        Some(s) if scelle => ChampsSensibles::depuis_env()
            .ouvrir("ent_dem_demande", "texte", id, &s)
            .map(Some)
            .map_err(|_| RefusDemande::Indisponible),
        autre => Ok(autre),
    }
}

#[derive(Debug, FromRow)]
struct LigneValeur {
    champ_code: String,
    rang: i16,
    valeur_texte: Option<String>,
    valeur_entier: Option<i64>,
    valeur_decimal: Option<String>,
    valeur_date: Option<NaiveDate>,
    valeur_heure: Option<NaiveTime>,
    valeur_bool: Option<bool>,
    valeur_code: Option<String>,
    valeur_ref: Option<Uuid>,
}

impl LigneValeur {
    fn vers_lue(self) -> Option<ValeurLue> {
        let valeur = if let Some(v) = self.valeur_texte {
            Valeur::Texte(v)
        } else if let Some(v) = self.valeur_entier {
            Valeur::Entier(v)
        } else if let Some(v) = self.valeur_decimal {
            Valeur::Decimal(v)
        } else if let Some(v) = self.valeur_date {
            Valeur::Date(v)
        } else if let Some(v) = self.valeur_heure {
            Valeur::Heure(v)
        } else if let Some(v) = self.valeur_bool {
            Valeur::Bool(v)
        } else if let Some(v) = self.valeur_code {
            Valeur::Code(v)
        } else {
            Valeur::Ref(self.valeur_ref?)
        };
        Some(ValeurLue { champ: self.champ_code, rang: self.rang, valeur })
    }
}
