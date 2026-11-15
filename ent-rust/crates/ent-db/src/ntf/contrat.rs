//! Socle 8, lot N1 — le contrat de notification (`08_NOTIFICATIONS.md` §6).
//!
//! Ce que chaque brique émettrice déclare : une [`NatureAvisDecl`] par situation (code, urgence, sensibilité,
//! canaux, gabarit français, annulation) et, pour les faits portés par un événement, un [`AvisSurEvenement`]
//! qui dit comment passer de l'événement à l'avis. Les natures et les gabarits sont **dans le code** (comme
//! les actions) ; la base ne garde que les avis, leurs destinataires et leurs remises.
//!
//! Placement : le kit des briques (`ARCHITECTURE_BRIQUES.md`, K6) n'est pas appliqué ailleurs dans le code ;
//! ce module vit donc dans `ent-db` sous `ntf/`, avec la découpe prévue pour la brique (contrat, assemblage,
//! service, consommateur, expédition), pour être déplacé tel quel dans `brique-notifications(-types)`.

use chrono::NaiveDate;
use ent_events::MetierEvent;
use uuid::Uuid;

/// Urgence d'un avis (§9.2) : U0 information, U1 à lire, U2 à traiter vite, U3 sûreté.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgence {
    U0,
    U1,
    U2,
    U3,
}

impl Urgence {
    pub fn code(self) -> &'static str {
        match self {
            Urgence::U0 => "u0",
            Urgence::U1 => "u1",
            Urgence::U2 => "u2",
            Urgence::U3 => "u3",
        }
    }

    /// Monter d'un cran, jamais au-delà de `max` ni en dessous de `self` (`urgence_plus`, §6.3).
    pub fn plus(self, demande: Urgence, max: Urgence) -> Urgence {
        if demande > self && demande <= max {
            demande
        } else {
            self
        }
    }
}

/// Classe de sensibilité du **contenu** (l'enveloppe d'un avis est toujours C0 ou C1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sensibilite {
    C0,
    C1,
    C2,
    C3,
}

impl Sensibilite {
    pub fn code(self) -> &'static str {
        match self {
            Sensibilite::C0 => "c0",
            Sensibilite::C1 => "c1",
            Sensibilite::C2 => "c2",
            Sensibilite::C3 => "c3",
        }
    }
}

/// Catégorie de préférence (§9.2) : la personne règle ses canaux par catégorie (lot N3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Categorie {
    VieScolaire,
    Evaluations,
    Sante,
    Finances,
    Documents,
    Messagerie,
    Agenda,
    Inscriptions,
    Securite,
    Systeme,
}

impl Categorie {
    pub const TOUTES: [Categorie; 10] = [
        Categorie::VieScolaire,
        Categorie::Evaluations,
        Categorie::Sante,
        Categorie::Finances,
        Categorie::Documents,
        Categorie::Messagerie,
        Categorie::Agenda,
        Categorie::Inscriptions,
        Categorie::Securite,
        Categorie::Systeme,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Categorie::VieScolaire => "vie_scolaire",
            Categorie::Evaluations => "evaluations",
            Categorie::Sante => "sante",
            Categorie::Finances => "finances",
            Categorie::Documents => "documents",
            Categorie::Messagerie => "messagerie",
            Categorie::Agenda => "agenda",
            Categorie::Inscriptions => "inscriptions",
            Categorie::Securite => "securite",
            Categorie::Systeme => "systeme",
        }
    }
}

/// Canal d'une remise. `InApp` est toujours remis pour qui a un compte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Canal {
    InApp,
    Push,
    Email,
    Sms,
}

impl Canal {
    /// Code stocké dans `ent_ntf_remise.canal`.
    pub fn code(self) -> &'static str {
        match self {
            Canal::InApp => "inapp",
            Canal::Push => "push",
            Canal::Email => "email",
            Canal::Sms => "sms",
        }
    }

    pub fn externe(self) -> bool {
        !matches!(self, Canal::InApp)
    }
}

/// Plancher : combien de canaux au moins doivent être tentés (§9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plancher {
    InApp,
    UnExterne,
    Tous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeSujet {
    Eleve,
    Personne,
    Cours,
    Etablissement,
}

impl TypeSujet {
    pub fn code(self) -> &'static str {
        match self {
            TypeSujet::Eleve => "eleve",
            TypeSujet::Personne => "personne",
            TypeSujet::Cours => "cours",
            TypeSujet::Etablissement => "etablissement",
        }
    }
}

/// L'élève est-il lui-même destinataire ? (`SiMajeur` : relu à la remise par le lot N2.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EleveDest {
    Jamais,
    Toujours,
    SiMajeur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regroupement {
    Aucun,
    Resume,
    FoyerDemiJournee,
    ParObjet,
}

/// Droit du lien que la nature exige du responsable, **relu à la remise** (R-N1, §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DroitRequis {
    /// Tout responsable au lien actif (information de santé : « la famille doit savoir », sans détail).
    Lien,
    VieScolaire,
    Notes,
}

impl DroitRequis {
    /// Colonne de `ent_eleve_lien` ; `None` pour le seul lien actif.
    pub fn colonne(self) -> Option<&'static str> {
        match self {
            DroitRequis::Lien => None,
            DroitRequis::VieScolaire => Some("peut_voir_vs"),
            DroitRequis::Notes => Some("peut_voir_notes"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Escalade {
    pub si_non_lu_apres_min: u32,
    pub vers: Canal,
    /// L'escalade vers le SMS n'a lieu que si la famille l'a consenti (défaut désactivé, §9.2).
    pub sms_consenti: bool,
}

/// Variable d'un gabarit, typée et classée : le lint de minimisation s'appuie sur sa sensibilité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variable {
    pub nom: &'static str,
    pub sensibilite: Sensibilite,
}

/// Gabarit **français** d'une nature (§6.1) : texte complet et enveloppes par canal. Les variables s'écrivent
/// `{nom}`. Les traductions et le rendu tardif avec les droits du destinataire relèvent du lot N4.
#[derive(Debug, Clone, Copy)]
pub struct Gabarit {
    pub id: &'static str,
    /// Titre dans l'ENT, objet d'e-mail et titre de push : jamais plus de C1.
    pub titre: &'static str,
    /// Corps dans l'ENT et dans l'e-mail : jamais plus de C2.
    pub corps: &'static str,
    /// Enveloppe SMS : jamais plus de C1.
    pub sms: &'static str,
    pub variables: &'static [Variable],
}

/// Texte libre : refusé hors des natures qui l'autorisent (diffusion, messagerie, urgence), §6.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexteLibre {
    Interdit,
    Autorise,
}

/// Déclaration d'une nature d'avis (§6.1). Une brique n'émet que ses natures.
#[derive(Debug, Clone, Copy)]
pub struct NatureAvisDecl {
    /// Stable, préfixé par le domaine de la brique : `vie_scolaire.absence_constatee`.
    pub code: &'static str,
    /// Une nouvelle version = nouveau gabarit ; l'ancien reste lisible.
    pub version: u16,
    /// Domaine propriétaire (premier segment du code).
    pub brique: &'static str,
    pub categorie: Categorie,
    pub sujet: TypeSujet,
    pub eleve_destinataire: EleveDest,
    pub droit_requis: DroitRequis,
    pub urgence: Urgence,
    /// Classe du contenu complet ; l'enveloppe externe reste C0/C1.
    pub sensibilite: Sensibilite,
    pub plancher: Plancher,
    pub canaux_defaut: &'static [Canal],
    pub escalade: Option<Escalade>,
    /// CN-01 : laisser corriger avant de prévenir.
    pub delai_confirmation_min: Option<u32>,
    pub regroupement: Regroupement,
    pub gabarit: Gabarit,
    /// Événements qui annulent l'avis tant qu'il n'est pas remis (`annulee_par`).
    pub annulee_par: &'static [&'static str],
    pub correction_si_remis: bool,
    /// R-N59 : textes gardés assez longtemps pour la preuve (avis d'assiduité).
    pub conservation_preuve: bool,
    /// R-N58 : classe « confidentiel famille » (information préoccupante, signalement) : **refusée** à
    /// l'assemblage pour tout avis qui peut atteindre la famille.
    pub confidentiel_famille: bool,
    /// Le texte libre n'est admis que pour les natures déclarées ainsi.
    pub texte_libre: TexteLibre,
    /// Actions autorisées à émettre l'avis par le service ; vide = événement seulement.
    pub actions_emission: &'static [&'static str],
    /// `U3` est réservé aux briques de sûreté, de santé et à la plateforme.
    pub brique_u3_autorisee: bool,
}

impl NatureAvisDecl {
    /// Délai de confirmation effectif, en minutes (réglage de l'établissement prioritaire pour l'absence).
    pub fn delai_defaut_min(&self) -> i64 {
        i64::from(self.delai_confirmation_min.unwrap_or(0))
    }
}

// ───────────────────────── Émission par événement (§6.2) ─────────────────────────

/// Valeur de paramètre d'un avis : identifiants et valeurs courtes typées, jamais de chaîne libre longue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValeurParam {
    Texte(String),
    Uuid(Uuid),
    Jour(NaiveDate),
}

pub type Param = (&'static str, ValeurParam);

/// Longueur maximale d'une valeur texte de paramètre (contrôlée par le service et par la base).
pub const PARAM_TEXTE_MAX: usize = 64;

/// Événement tel que les fonctions de déclaration le lisent.
pub struct EvenementVu<'a>(pub &'a MetierEvent);

impl<'a> EvenementVu<'a> {
    pub fn id(&self) -> Uuid {
        self.0.id
    }
    pub fn type_name(&self) -> &str {
        &self.0.type_name
    }
    pub fn eleve_id(&self) -> Option<Uuid> {
        self.0.eleve_id
    }
    pub fn payload_str(&self, cle: &str) -> Option<&str> {
        self.0.payload.get(cle).and_then(|v| v.as_str())
    }
    pub fn payload_bool(&self, cle: &str) -> Option<bool> {
        self.0.payload.get(cle).and_then(|v| v.as_bool())
    }
    pub fn payload_uuid(&self, cle: &str) -> Option<Uuid> {
        self.payload_str(cle).and_then(|s| Uuid::parse_str(s).ok())
    }
    pub fn payload_jour(&self, cle: &str) -> Option<NaiveDate> {
        self.payload_str(cle).and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    }
}

/// Comment un événement devient un avis. Le consommateur `notifications.avis` les parcourt tous.
#[derive(Clone, Copy)]
pub struct AvisSurEvenement {
    /// Nom historique gelé de l'événement.
    pub evenement: &'static str,
    pub nature: &'static NatureAvisDecl,
    pub filtre: fn(&EvenementVu<'_>) -> bool,
    /// Clé d'idempotence : un rejeu ne crée pas un second avis.
    pub cle: fn(&EvenementVu<'_>) -> String,
    pub parametres: fn(&EvenementVu<'_>) -> Vec<Param>,
}

/// Clé d'idempotence fondée sur l'identifiant du fait (`CleAvis::fait`, §6.2).
pub fn cle_fait(evt: &EvenementVu<'_>, suffixe: &str) -> String {
    format!("evt:{}:{}", evt.id(), suffixe)
}

/// Variables `{nom}` d'un texte de gabarit, dans l'ordre d'apparition.
pub fn variables_de(texte: &str) -> Vec<&str> {
    let mut v = Vec::new();
    let mut reste = texte;
    while let Some(i) = reste.find('{') {
        let apres = &reste[i + 1..];
        match apres.find('}') {
            Some(j) => {
                v.push(&apres[..j]);
                reste = &apres[j + 1..];
            }
            None => break,
        }
    }
    v
}
