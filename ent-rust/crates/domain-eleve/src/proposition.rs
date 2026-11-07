//! Socle 2 (l'élève), lot E5 — propositions et paramétrage des champs (logique pure).
//!
//! PRINCIPE (`docs/socles/02_ELEVE.md` §7.8, §8.3) : aucune donnée venue de l'extérieur
//! (import SIECLE, famille, élève, ancien établissement, personnel qui signale une erreur)
//! n'écrit directement dans la fiche. Elle passe par une **proposition** (valeur actuelle →
//! valeur proposée, source, pièce), acceptée par une règle automatique explicite ou par une
//! personne habilitée.
//!
//! Ce module porte, sans base de données :
//! * le **catalogue des champs** (du logiciel : types, sensibilité, ce que la famille peut
//!   proposer, quelles sources peuvent « faire foi ») ;
//! * le **réglage** d'un champ par l'établissement ([`ReglageChamp`]) et ses bornes ;
//! * les **profils d'établissement** (préréglages public, privé sous contrat, hors contrat…) ;
//! * les **décisions** : qui peut déposer quoi, quand une règle automatique s'applique, ce
//!   qu'un champ verrouillé interdit.
//!
//! Aucune valeur d'identité ne sort de ce module vers un message d'erreur : les erreurs sont
//! des textes fixes.

use serde::{Deserialize, Serialize};

use crate::identifiant::SystemeIdentifiant;
use crate::identite::ChampIdentite;

// ───────────────────────── Vocabulaires ─────────────────────────

/// Ce que la proposition modifie (CHECK de `ent_eleve_proposition.cible`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CibleProposition {
    Identite,
    Identifiant,
    Scolarisation,
    Inscription,
    Coordonnee,
    Photo,
    NouvelEleve,
}

impl CibleProposition {
    pub fn code(self) -> &'static str {
        match self {
            Self::Identite => "identite",
            Self::Identifiant => "identifiant",
            Self::Scolarisation => "scolarisation",
            Self::Inscription => "inscription",
            Self::Coordonnee => "coordonnee",
            Self::Photo => "photo",
            Self::NouvelEleve => "nouvel_eleve",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::Identite,
            Self::Identifiant,
            Self::Scolarisation,
            Self::Inscription,
            Self::Coordonnee,
            Self::Photo,
            Self::NouvelEleve,
        ]
        .into_iter()
        .find(|c| c.code() == s)
    }

    /// Le service sait-il appliquer cette cible ? (le schéma prévoit les autres ; leur
    /// application relève des lots suivants — le dépôt est refusé tant qu'il n'y a pas
    /// d'applicateur, jamais accepté pour rien).
    pub fn applicable(self) -> bool {
        matches!(self, Self::Identite | Self::Identifiant)
    }
}

/// Qui dépose (CHECK de `ent_eleve_proposition.source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceProposition {
    Import,
    Famille,
    Eleve,
    AncienEtablissement,
    Educonnect,
    Admission,
    Systeme,
    Personnel,
    VerificationCampagne,
}

impl SourceProposition {
    pub const TOUTES: [Self; 9] = [
        Self::Import,
        Self::Famille,
        Self::Eleve,
        Self::AncienEtablissement,
        Self::Educonnect,
        Self::Admission,
        Self::Systeme,
        Self::Personnel,
        Self::VerificationCampagne,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Famille => "famille",
            Self::Eleve => "eleve",
            Self::AncienEtablissement => "ancien_etablissement",
            Self::Educonnect => "educonnect",
            Self::Admission => "admission",
            Self::Systeme => "systeme",
            Self::Personnel => "personnel",
            Self::VerificationCampagne => "verification_campagne",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|c| c.code() == s)
    }

    /// Source qu'un **traitement du serveur** peut déposer (imports, socle 3) ; les autres
    /// sont déposées par une personne connectée.
    pub fn deposable_par_systeme(self) -> bool {
        matches!(
            self,
            Self::Import
                | Self::Educonnect
                | Self::Admission
                | Self::Systeme
                | Self::AncienEtablissement
                | Self::VerificationCampagne
        )
    }

    /// Une règle automatique ne s'applique qu'à un flux machine officiel.
    pub fn automatisable(self) -> bool {
        matches!(self, Self::Import | Self::Educonnect | Self::Systeme)
    }

    /// Valeur de `ent_eleve_identite.source` pour une version issue de cette proposition.
    pub fn source_identite(self) -> &'static str {
        match self {
            Self::Import | Self::Educonnect | Self::Systeme => "import",
            Self::Famille | Self::VerificationCampagne => "famille",
            Self::Eleve => "eleve",
            Self::AncienEtablissement => "ancien_etablissement",
            Self::Admission => "admission",
            Self::Personnel => "saisie",
        }
    }
}

/// Système d'où vient un import (CHECK de `ent_eleve_proposition.origine`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrigineProposition {
    Siecle,
    Onde,
    Aaf,
    Bee,
    Reprise,
    Educonnect,
    Autre,
}

impl OrigineProposition {
    pub fn code(self) -> &'static str {
        match self {
            Self::Siecle => "siecle",
            Self::Onde => "onde",
            Self::Aaf => "aaf",
            Self::Bee => "bee",
            Self::Reprise => "reprise",
            Self::Educonnect => "educonnect",
            Self::Autre => "autre",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::Siecle,
            Self::Onde,
            Self::Aaf,
            Self::Bee,
            Self::Reprise,
            Self::Educonnect,
            Self::Autre,
        ]
        .into_iter()
        .find(|c| c.code() == s)
    }

    /// Source qui fait foi à laquelle cette origine correspond. SIECLE, l'AAF (qui en est
    /// issue) et les exports BEE sont une seule autorité ; ÉduConnect et « autre » n'en
    /// sont aucune.
    pub fn fait_foi_pour(self) -> Option<SourceFoi> {
        match self {
            Self::Siecle | Self::Aaf | Self::Bee => Some(SourceFoi::Siecle),
            Self::Onde => Some(SourceFoi::Onde),
            Self::Reprise => Some(SourceFoi::Reprise),
            Self::Educonnect | Self::Autre => None,
        }
    }

    /// Provenance de l'identifiant qu'un import de cette origine fait entrer.
    pub fn provenance_identifiant(self) -> &'static str {
        match self {
            Self::Siecle | Self::Aaf => "import_aaf",
            Self::Bee => "import_bee",
            Self::Onde => "import_onde",
            Self::Reprise => "reprise_logiciel",
            Self::Educonnect => "educonnect",
            Self::Autre => "ancien_etablissement",
        }
    }
}

/// Qui fait foi pour un champ (CHECK de `ent_param_eleve_champ.source_qui_fait_foi`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFoi {
    /// L'établissement lui-même (ScolIA).
    Scolia,
    Siecle,
    Onde,
    Reprise,
    Famille,
}

impl SourceFoi {
    pub fn code(self) -> &'static str {
        match self {
            Self::Scolia => "scolia",
            Self::Siecle => "siecle",
            Self::Onde => "onde",
            Self::Reprise => "reprise",
            Self::Famille => "famille",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::Scolia,
            Self::Siecle,
            Self::Onde,
            Self::Reprise,
            Self::Famille,
        ]
        .into_iter()
        .find(|c| c.code() == s)
    }

    /// Une autorité externe (≠ l'établissement, ≠ la famille).
    pub fn externe(self) -> bool {
        matches!(self, Self::Siecle | Self::Onde | Self::Reprise)
    }
}

/// Ce que la famille peut faire sur un champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaisieFamille {
    Non,
    Proposer,
    Modifier,
}

impl SaisieFamille {
    pub fn code(self) -> &'static str {
        match self {
            Self::Non => "non",
            Self::Proposer => "proposer",
            Self::Modifier => "modifier",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [Self::Non, Self::Proposer, Self::Modifier]
            .into_iter()
            .find(|c| c.code() == s)
    }
}

/// La modification **locale** (par le secrétariat) d'un champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModificationLocale {
    /// Les commandes directes (corrections, changements, usages) restent permises.
    Libre,
    /// Toute modification locale passe par la file des propositions (dépôt puis décision).
    ParProposition,
    /// Aucune modification locale : seule la source qui fait foi écrit (import) ; une
    /// demande locale ne peut être que refusée (« à corriger dans la source »).
    Interdite,
}

impl ModificationLocale {
    pub fn code(self) -> &'static str {
        match self {
            Self::Libre => "libre",
            Self::ParProposition => "par_proposition",
            Self::Interdite => "interdite",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [Self::Libre, Self::ParProposition, Self::Interdite]
            .into_iter()
            .find(|c| c.code() == s)
    }
}

/// Moments où un champ peut être obligatoire (CHECK de `obligatoire_a`).
pub const MOMENTS_OBLIGATOIRE: [&str; 4] = ["creation", "inscription", "export_bee", "export_gar"];

// ───────────────────────── Catalogue des champs ─────────────────────────

/// Forme de la valeur d'un champ (représentation canonique en JSON).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeValeur {
    /// Texte court (nom, lieu…) — JSON string ou `null` pour retirer.
    Texte,
    /// Liste de prénoms — JSON array de strings.
    Prenoms,
    /// `M` ou `F`.
    Sexe,
    /// Date `AAAA-MM-JJ`.
    Date,
    /// Code géographique court (COG).
    Code,
    /// Identifiant externe du système donné.
    Identifiant(SystemeIdentifiant),
}

/// Définition d'un champ du catalogue (du logiciel, identique pour tous).
#[derive(Debug, Clone, Copy)]
pub struct DefChamp {
    pub code: &'static str,
    pub libelle: &'static str,
    pub cible: CibleProposition,
    pub type_valeur: TypeValeur,
    /// Valeur intime : jamais appliquée automatiquement, jamais montrée à la famille
    /// au-delà de celui qui l'a déposée.
    pub sensible: bool,
    /// Ce que la famille peut au plus faire de ce champ, quel que soit le réglage
    /// (l'INE n'est jamais proposable par la famille).
    pub famille_max: SaisieFamille,
    /// Une pièce justificative est exigée d'une famille qui propose ce champ.
    pub piece_famille: bool,
    /// Autorités externes qui peuvent faire foi pour ce champ (en plus de l'établissement).
    pub sources_foi: &'static [SourceFoi],
    /// Nom d'usage ou prénom d'usage : appliqué comme version `usage`, jamais comme
    /// correction ou changement d'état civil.
    pub usage: bool,
}

const FOI_ETAT_CIVIL: &[SourceFoi] = &[SourceFoi::Siecle, SourceFoi::Onde, SourceFoi::Reprise];
const FOI_AUCUNE: &[SourceFoi] = &[];
const FOI_SIECLE: &[SourceFoi] = &[SourceFoi::Siecle, SourceFoi::Reprise];
const FOI_ONDE: &[SourceFoi] = &[SourceFoi::Onde, SourceFoi::Reprise];

const fn civil(
    code: &'static str,
    libelle: &'static str,
    type_valeur: TypeValeur,
    sensible: bool,
) -> DefChamp {
    DefChamp {
        code,
        libelle,
        cible: CibleProposition::Identite,
        type_valeur,
        sensible,
        famille_max: SaisieFamille::Proposer,
        piece_famille: true,
        sources_foi: FOI_ETAT_CIVIL,
        usage: false,
    }
}

/// Catalogue des champs proposables et paramétrables. L'ordre est celui des écrans.
pub const CATALOGUE: &[DefChamp] = &[
    civil("nom_naissance", "Nom de naissance", TypeValeur::Texte, false),
    DefChamp {
        code: "nom_usage",
        libelle: "Nom d'usage",
        cible: CibleProposition::Identite,
        type_valeur: TypeValeur::Texte,
        sensible: false,
        famille_max: SaisieFamille::Proposer,
        piece_famille: false,
        sources_foi: FOI_AUCUNE,
        usage: true,
    },
    // Les prénoms de l'état civil et la mention de sexe sont intimes (une version qui les
    // change l'est aussi, cf. `identite::est_sensible`) : jamais appliqués automatiquement.
    civil("prenoms", "Prénoms", TypeValeur::Prenoms, true),
    DefChamp {
        code: "prenom_usage",
        libelle: "Prénom d'usage",
        cible: CibleProposition::Identite,
        type_valeur: TypeValeur::Texte,
        sensible: true,
        famille_max: SaisieFamille::Proposer,
        piece_famille: false,
        sources_foi: FOI_AUCUNE,
        usage: true,
    },
    civil("sexe", "Sexe à l'état civil", TypeValeur::Sexe, true),
    civil("date_naissance", "Date de naissance", TypeValeur::Date, false),
    civil("naissance_commune", "Commune de naissance", TypeValeur::Texte, false),
    civil("naissance_commune_cog", "Code commune de naissance", TypeValeur::Code, false),
    civil("naissance_pays_cog", "Code pays de naissance", TypeValeur::Code, false),
    civil("naissance_lieu_etranger", "Ville de naissance à l'étranger", TypeValeur::Texte, false),
    DefChamp {
        code: "ine",
        libelle: "INE",
        cible: CibleProposition::Identifiant,
        type_valeur: TypeValeur::Identifiant(SystemeIdentifiant::Ine),
        sensible: false,
        // L'INE n'est jamais modifiable par la famille (§7.8).
        famille_max: SaisieFamille::Non,
        piece_famille: false,
        sources_foi: FOI_ETAT_CIVIL,
        usage: false,
    },
    DefChamp {
        code: "siecle_eleve",
        libelle: "Identifiant élève SIECLE",
        cible: CibleProposition::Identifiant,
        type_valeur: TypeValeur::Identifiant(SystemeIdentifiant::SiecleEleve),
        sensible: false,
        famille_max: SaisieFamille::Non,
        piece_famille: false,
        sources_foi: FOI_SIECLE,
        usage: false,
    },
    DefChamp {
        code: "onde",
        libelle: "Identifiant ONDE",
        cible: CibleProposition::Identifiant,
        type_valeur: TypeValeur::Identifiant(SystemeIdentifiant::Onde),
        sensible: false,
        famille_max: SaisieFamille::Non,
        piece_famille: false,
        sources_foi: FOI_ONDE,
        usage: false,
    },
];

/// Champ du catalogue par son code.
pub fn champ(code: &str) -> Option<&'static DefChamp> {
    CATALOGUE.iter().find(|c| c.code == code)
}

/// Champ du catalogue qui correspond à un champ d'identité modifié (verrous des commandes
/// directes). Les champs hors catalogue (précision et statut de la date de naissance mis à
/// part, qui suivent la date ; graphies d'origine) n'ont pas de verrou propre.
pub fn champ_catalogue_de(c: ChampIdentite) -> Option<&'static str> {
    Some(match c {
        ChampIdentite::NomNaissance => "nom_naissance",
        ChampIdentite::NomUsage => "nom_usage",
        ChampIdentite::Prenoms | ChampIdentite::RangPrenomUsuel => "prenoms",
        ChampIdentite::PrenomUsage => "prenom_usage",
        ChampIdentite::Sexe => "sexe",
        ChampIdentite::DateNaissance
        | ChampIdentite::DateNaissancePrecision
        | ChampIdentite::DateNaissanceStatut => "date_naissance",
        ChampIdentite::NaissanceCommune => "naissance_commune",
        ChampIdentite::NaissanceCommuneCog => "naissance_commune_cog",
        ChampIdentite::NaissancePaysCog => "naissance_pays_cog",
        ChampIdentite::NaissanceLieuEtranger => "naissance_lieu_etranger",
        ChampIdentite::NomEcritureOrigine | ChampIdentite::PrenomsEcritureOrigine => return None,
    })
}

/// Champ du catalogue qui correspond à un système d'identifiant (verrou d'ELV-07).
pub fn champ_catalogue_du_systeme(s: SystemeIdentifiant) -> Option<&'static str> {
    match s {
        SystemeIdentifiant::Ine => Some("ine"),
        SystemeIdentifiant::SiecleEleve => Some("siecle_eleve"),
        SystemeIdentifiant::Onde => Some("onde"),
        _ => None,
    }
}

// ───────────────────────── Réglage d'un champ ─────────────────────────

/// Paramétrage d'un champ par l'établissement (`ent_param_eleve_champ`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReglageChamp {
    pub visible: bool,
    pub obligatoire_a: Vec<String>,
    pub saisie_famille: SaisieFamille,
    pub source_qui_fait_foi: SourceFoi,
    pub modification_locale: ModificationLocale,
    /// Gel total : rien ne s'applique tant que le champ est verrouillé (commande directe,
    /// acceptation, règle automatique) ; les propositions peuvent être déposées et attendent.
    pub verrouille: bool,
}

impl ReglageChamp {
    /// Valeur par défaut d'un champ non réglé : un établissement existant ne change pas de
    /// comportement tant qu'il n'a rien réglé.
    pub fn defaut() -> Self {
        Self {
            visible: true,
            obligatoire_a: Vec::new(),
            saisie_famille: SaisieFamille::Non,
            source_qui_fait_foi: SourceFoi::Scolia,
            modification_locale: ModificationLocale::Libre,
            verrouille: false,
        }
    }
}

/// Erreur de réglage ou de décision (textes fixes, sans donnée personnelle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErreurProposition {
    #[error("{0}")]
    Invalide(&'static str),
    #[error("{0}")]
    Refus(&'static str),
    #[error("{0}")]
    Conflit(&'static str),
}

/// Contrôle un réglage contre les bornes du logiciel (§9.2 : « dans les bornes »).
pub fn valider_reglage(def: &DefChamp, r: &ReglageChamp) -> Result<(), ErreurProposition> {
    if r.saisie_famille > def.famille_max {
        return Err(ErreurProposition::Invalide(
            "ce champ n'est pas ouvert à la famille à ce niveau",
        ));
    }
    if r.saisie_famille != SaisieFamille::Non && !r.visible {
        return Err(ErreurProposition::Invalide(
            "un champ masqué ne peut pas être ouvert à la famille",
        ));
    }
    if r.source_qui_fait_foi != SourceFoi::Scolia && !def.sources_foi.contains(&r.source_qui_fait_foi)
    {
        return Err(ErreurProposition::Invalide(
            "cette source ne peut pas faire foi pour ce champ",
        ));
    }
    // Un champ dont l'établissement fait foi et que personne n'a de raison de refuser
    // localement : « interdite » n'a de sens que si une autorité externe écrit à sa place.
    if r.modification_locale == ModificationLocale::Interdite && !r.source_qui_fait_foi.externe() {
        return Err(ErreurProposition::Invalide(
            "la modification locale ne peut être interdite que si une source externe fait foi",
        ));
    }
    let mut vus = std::collections::HashSet::new();
    for m in &r.obligatoire_a {
        if !MOMENTS_OBLIGATOIRE.contains(&m.as_str()) {
            return Err(ErreurProposition::Invalide("moment d'obligation inconnu"));
        }
        if !vus.insert(m.as_str()) {
            return Err(ErreurProposition::Invalide("moment d'obligation en double"));
        }
    }
    Ok(())
}

/// Une règle automatique n'a de sens que pour un flux machine, un champ non sensible et une
/// autorité externe qui fait foi (sinon la règle ne s'appliquerait jamais : on la refuse).
pub fn valider_regle_auto(
    def: &DefChamp,
    source: SourceProposition,
    r: &ReglageChamp,
) -> Result<(), ErreurProposition> {
    if !source.automatisable() {
        return Err(ErreurProposition::Invalide(
            "une règle automatique ne s'applique qu'à un flux machine (import)",
        ));
    }
    if def.sensible {
        return Err(ErreurProposition::Invalide(
            "un champ sensible n'est jamais appliqué automatiquement",
        ));
    }
    if !r.source_qui_fait_foi.externe() {
        return Err(ErreurProposition::Invalide(
            "il faut d'abord désigner une source externe qui fait foi pour ce champ",
        ));
    }
    Ok(())
}

// ───────────────────────── Décisions ─────────────────────────

/// Qui dépose pour la famille.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deposant {
    /// Responsable légal (lien actif), ou élève majeur ou émancipé (soi).
    Famille,
    /// Personnel qui signale une erreur vue sur le terrain (ELV-40).
    Personnel,
}

/// Le dépôt est-il permis ? (le réglage fait foi : on ne propose que ce qui est autorisé)
pub fn verifier_depot(
    def: &DefChamp,
    r: &ReglageChamp,
    deposant: Deposant,
) -> Result<(), ErreurProposition> {
    match deposant {
        Deposant::Famille => {
            if def.famille_max == SaisieFamille::Non {
                return Err(ErreurProposition::Refus(
                    "ce champ n'est jamais modifiable par la famille",
                ));
            }
            if !r.visible || r.saisie_famille == SaisieFamille::Non {
                return Err(ErreurProposition::Refus(
                    "l'établissement n'ouvre pas ce champ à la famille",
                ));
            }
            Ok(())
        }
        // Un personnel signale ce qu'il voit : tout champ visible du catalogue.
        Deposant::Personnel => {
            if !r.visible {
                return Err(ErreurProposition::Refus("ce champ n'est pas utilisé"));
            }
            Ok(())
        }
    }
}

/// Une modification **directe** par le secrétariat est-elle permise sur ce champ ?
/// (les commandes ELV-03/04/05/07 appellent cette garde avant d'écrire)
pub fn verifier_modification_directe(r: &ReglageChamp) -> Result<(), ErreurProposition> {
    if r.verrouille {
        return Err(ErreurProposition::Refus(
            "champ verrouillé : aucune modification tant qu'il l'est",
        ));
    }
    match r.modification_locale {
        ModificationLocale::Libre => Ok(()),
        ModificationLocale::ParProposition => Err(ErreurProposition::Refus(
            "champ réglé « par proposition » : déposer une proposition à décider",
        )),
        ModificationLocale::Interdite => Err(ErreurProposition::Refus(
            "champ géré par une source externe : à corriger dans la source",
        )),
    }
}

/// L'acceptation d'une proposition (par une personne) est-elle permise sur ce champ ?
pub fn verifier_acceptation(
    r: &ReglageChamp,
    source: SourceProposition,
    origine: Option<OrigineProposition>,
) -> Result<(), ErreurProposition> {
    if r.verrouille {
        return Err(ErreurProposition::Refus(
            "champ verrouillé : aucune proposition ne peut être acceptée tant qu'il l'est",
        ));
    }
    if r.modification_locale == ModificationLocale::Interdite
        && !vient_de_la_source_qui_fait_foi(r, source, origine)
    {
        return Err(ErreurProposition::Refus(
            "champ géré par une source externe : refuser avec le motif « à corriger dans la source »",
        ));
    }
    Ok(())
}

/// La proposition vient-elle de l'autorité qui fait foi pour ce champ ?
pub fn vient_de_la_source_qui_fait_foi(
    r: &ReglageChamp,
    source: SourceProposition,
    origine: Option<OrigineProposition>,
) -> bool {
    match source {
        SourceProposition::Import | SourceProposition::Educonnect | SourceProposition::Systeme => {
            origine
                .and_then(OrigineProposition::fait_foi_pour)
                .is_some_and(|f| f == r.source_qui_fait_foi)
        }
        SourceProposition::Famille | SourceProposition::Eleve => {
            r.source_qui_fait_foi == SourceFoi::Famille
        }
        _ => false,
    }
}

/// Pourquoi une règle automatique ne s'est pas appliquée (affiché à l'écran des propositions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaisonNonAuto {
    AucuneRegle,
    SourceNonAutomatisable,
    ChampSensible,
    ChampVerrouille,
    SourceNeFaitPasFoi,
    ConflitAvecLaFiche,
    PieceRequise,
}

impl RaisonNonAuto {
    pub fn code(self) -> &'static str {
        match self {
            Self::AucuneRegle => "aucune_regle",
            Self::SourceNonAutomatisable => "source_non_automatisable",
            Self::ChampSensible => "champ_sensible",
            Self::ChampVerrouille => "champ_verrouille",
            Self::SourceNeFaitPasFoi => "source_ne_fait_pas_foi",
            Self::ConflitAvecLaFiche => "conflit_avec_la_fiche",
            Self::PieceRequise => "piece_requise",
        }
    }
}

/// Contexte d'évaluation d'une règle automatique pour UN champ d'une proposition.
#[derive(Debug, Clone, Copy)]
pub struct ContexteAuto<'a> {
    pub def: &'a DefChamp,
    pub reglage: &'a ReglageChamp,
    pub source: SourceProposition,
    pub origine: Option<OrigineProposition>,
    /// Une règle active existe pour (champ, source).
    pub regle_active: bool,
    /// La valeur actuelle de la fiche n'est plus celle vue au dépôt (modification locale
    /// plus récente) : jamais d'application automatique, un conflit visible à la place.
    pub conflit: bool,
}

/// Une règle automatique s'applique-t-elle ? `Ok(())` ou la raison du refus. Toujours
/// explicite : sans règle active, rien ne s'applique seul.
pub fn evaluer_auto(c: ContexteAuto<'_>) -> Result<(), RaisonNonAuto> {
    if !c.regle_active {
        return Err(RaisonNonAuto::AucuneRegle);
    }
    if !c.source.automatisable() {
        return Err(RaisonNonAuto::SourceNonAutomatisable);
    }
    if c.def.sensible {
        return Err(RaisonNonAuto::ChampSensible);
    }
    if c.reglage.verrouille {
        return Err(RaisonNonAuto::ChampVerrouille);
    }
    if !vient_de_la_source_qui_fait_foi(c.reglage, c.source, c.origine) {
        return Err(RaisonNonAuto::SourceNeFaitPasFoi);
    }
    if c.conflit {
        return Err(RaisonNonAuto::ConflitAvecLaFiche);
    }
    Ok(())
}

/// Nom de la règle tracée sur la proposition appliquée automatiquement.
pub fn nom_regle_auto(source: SourceProposition) -> String {
    format!("regle_auto:{}", source.code())
}

/// Statut final d'une proposition selon la décision de chacun de ses champs.
pub fn statut_final(acceptes: usize, total: usize) -> &'static str {
    if acceptes == 0 {
        "refusee"
    } else if acceptes == total {
        "acceptee"
    } else {
        "acceptee_partiellement"
    }
}

/// Motifs de refus proposés (code du logiciel ; le texte libre est facultatif).
pub const MOTIFS_REFUS: &[&str] = &[
    "a_corriger_dans_la_source",
    "piece_insuffisante",
    "valeur_incorrecte",
    "deja_traite",
    "hors_perimetre",
    "autre",
];

// ───────────────────────── Profils d'établissement ─────────────────────────

/// Préréglage du paramétrage (§9.1). `Personnalise` = l'établissement a réglé à la main.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfilEtablissement {
    /// Public, second degré : SIECLE (via l'AAF) fait foi.
    Public,
    /// Privé sous contrat : ScolIA fait foi et remonte vers BEE.
    PriveSousContrat,
    /// Privé sous contrat qui cohabite avec un logiciel agréé : ce logiciel fait foi
    /// (par imports, source « reprise » en attendant un code dédié).
    PriveSousContratLogicielAgree,
    HorsContrat,
    /// Premier degré : ONDE fait foi.
    PremierDegre,
    Personnalise,
}

impl ProfilEtablissement {
    /// Profils proposés à l'application (`Personnalise` ne se choisit pas : il constate).
    pub const PROPOSES: [Self; 5] = [
        Self::Public,
        Self::PriveSousContrat,
        Self::PriveSousContratLogicielAgree,
        Self::HorsContrat,
        Self::PremierDegre,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::PriveSousContrat => "prive_sous_contrat",
            Self::PriveSousContratLogicielAgree => "prive_sous_contrat_logiciel_agree",
            Self::HorsContrat => "hors_contrat",
            Self::PremierDegre => "premier_degre",
            Self::Personnalise => "personnalise",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::PROPOSES
            .into_iter()
            .chain([Self::Personnalise])
            .find(|c| c.code() == s)
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Public => "Public, second degré (SIECLE fait foi)",
            Self::PriveSousContrat => "Privé sous contrat (ScolIA fait foi, remontée vers BEE)",
            Self::PriveSousContratLogicielAgree => {
                "Privé sous contrat avec un logiciel agréé (ce logiciel fait foi)"
            }
            Self::HorsContrat => "Hors contrat (ScolIA fait foi)",
            Self::PremierDegre => "Premier degré (ONDE fait foi)",
            Self::Personnalise => "Personnalisé",
        }
    }

    /// Autorité qui fait foi pour l'état civil et l'INE.
    fn foi(self) -> SourceFoi {
        match self {
            Self::Public => SourceFoi::Siecle,
            Self::PremierDegre => SourceFoi::Onde,
            Self::PriveSousContratLogicielAgree => SourceFoi::Reprise,
            Self::PriveSousContrat | Self::HorsContrat | Self::Personnalise => SourceFoi::Scolia,
        }
    }

    /// Modification locale de l'état civil (§9.1) : le public interdit (Q-E7, réglable par
    /// champ), le premier degré et le logiciel agréé passent par proposition, les autres
    /// profils laissent libre.
    fn modification_locale(self) -> ModificationLocale {
        match self {
            Self::Public => ModificationLocale::Interdite,
            Self::PremierDegre | Self::PriveSousContratLogicielAgree => {
                ModificationLocale::ParProposition
            }
            Self::PriveSousContrat | Self::HorsContrat | Self::Personnalise => {
                ModificationLocale::Libre
            }
        }
    }

    /// Les imports d'une autorité externe s'appliquent seuls (règle explicite, champ non
    /// sensible, sans conflit) quand l'autorité fait foi **et** écrit à la place de
    /// l'établissement (public, premier degré). Un logiciel agréé qui cohabite reste
    /// décidé à la main (aucune règle automatique par défaut).
    fn regles_auto(self) -> bool {
        matches!(self, Self::Public | Self::PremierDegre)
    }
}

/// Réglage d'un champ dans un profil. Retourne `None` pour un champ hors catalogue.
pub fn reglage_profil(profil: ProfilEtablissement, code: &str) -> Option<ReglageChamp> {
    let def = champ(code)?;
    let mut r = ReglageChamp::defaut();
    // Le prénom et le nom d'usage sont locaux : fermés à la famille tant que
    // l'établissement n'a pas défini sa procédure (accord des parents, entretien).
    if def.usage {
        return Some(r);
    }
    // Les champs de l'état civil et les identifiants suivent l'autorité du profil si elle
    // peut faire foi pour eux.
    let foi = profil.foi();
    if foi == SourceFoi::Scolia || def.sources_foi.contains(&foi) {
        r.source_qui_fait_foi = foi;
    }
    if r.source_qui_fait_foi.externe() {
        r.modification_locale = profil.modification_locale();
    }
    // La famille peut signaler une erreur d'état civil (avec pièce), jamais l'INE.
    r.saisie_famille = def.famille_max.min(SaisieFamille::Proposer);
    // Moments d'obligation raisonnables par défaut (à valider : [NV] côté BEE).
    r.obligatoire_a = match code {
        "nom_naissance" | "prenoms" => vec!["creation".into()],
        "date_naissance" => vec!["inscription".into()],
        "ine" if matches!(
            profil,
            ProfilEtablissement::Public
                | ProfilEtablissement::PriveSousContrat
                | ProfilEtablissement::PriveSousContratLogicielAgree
        ) =>
        {
            vec!["export_bee".into()]
        }
        _ => Vec::new(),
    };
    Some(r)
}

/// Règles automatiques d'un profil : `(champ, source)` — import d'une autorité externe qui
/// fait foi, champs non sensibles seulement.
pub fn regles_auto_profil(profil: ProfilEtablissement) -> Vec<(&'static str, SourceProposition)> {
    if !profil.regles_auto() {
        return Vec::new();
    }
    CATALOGUE
        .iter()
        .filter(|d| !d.sensible && !d.usage)
        .filter(|d| {
            reglage_profil(profil, d.code).is_some_and(|r| r.source_qui_fait_foi.externe())
        })
        .map(|d| (d.code, SourceProposition::Import))
        .collect()
}

// ───────────────────────── Tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn reglage_public(code: &str) -> ReglageChamp {
        reglage_profil(ProfilEtablissement::Public, code).unwrap()
    }

    #[test]
    fn catalogue_codes_uniques_et_valides() {
        let mut vus = std::collections::HashSet::new();
        for d in CATALOGUE {
            assert!(vus.insert(d.code), "doublon {}", d.code);
            assert!(champ(d.code).is_some());
            // Le nom respecte le CHECK SQL ^[a-z][a-z0-9_]{0,59}$.
            assert!(d.code.len() <= 60 && d.code.chars().next().unwrap().is_ascii_lowercase());
            assert!(d
                .code
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'));
        }
    }

    #[test]
    fn ine_jamais_proposable_par_la_famille() {
        let ine = champ("ine").unwrap();
        assert_eq!(ine.famille_max, SaisieFamille::Non);
        let mut r = ReglageChamp::defaut();
        r.saisie_famille = SaisieFamille::Proposer;
        assert!(valider_reglage(ine, &r).is_err());
        let r = ReglageChamp::defaut();
        assert!(verifier_depot(ine, &r, Deposant::Famille).is_err());
        // Même règle quel que soit le profil.
        for p in ProfilEtablissement::PROPOSES {
            assert_eq!(
                reglage_profil(p, "ine").unwrap().saisie_famille,
                SaisieFamille::Non
            );
        }
    }

    #[test]
    fn famille_ne_depose_que_ce_que_l_etablissement_ouvre() {
        let nom = champ("nom_naissance").unwrap();
        let mut r = ReglageChamp::defaut();
        // Par défaut (rien de réglé) : fermé.
        assert!(verifier_depot(nom, &r, Deposant::Famille).is_err());
        r.saisie_famille = SaisieFamille::Proposer;
        assert!(verifier_depot(nom, &r, Deposant::Famille).is_ok());
        // Champ masqué : ni la famille ni le personnel.
        r.visible = false;
        assert!(valider_reglage(nom, &r).is_err());
        assert!(verifier_depot(nom, &r, Deposant::Famille).is_err());
        assert!(verifier_depot(nom, &r, Deposant::Personnel).is_err());
    }

    #[test]
    fn profils_tous_valides_et_distincts() {
        for p in ProfilEtablissement::PROPOSES {
            for d in CATALOGUE {
                let r = reglage_profil(p, d.code).unwrap();
                valider_reglage(d, &r).unwrap_or_else(|e| panic!("{} / {} : {e}", p.code(), d.code));
            }
            for (champ_code, source) in regles_auto_profil(p) {
                let d = champ(champ_code).unwrap();
                let r = reglage_profil(p, champ_code).unwrap();
                valider_regle_auto(d, source, &r).unwrap();
            }
            assert_eq!(ProfilEtablissement::parse(p.code()), Some(p));
        }
    }

    #[test]
    fn public_interdit_la_modification_locale_de_l_etat_civil() {
        // Dans le public, quand SIECLE fait foi : état civil interdit localement (Q-E7).
        for code in ["nom_naissance", "date_naissance", "prenoms", "sexe", "ine"] {
            let r = reglage_public(code);
            assert_eq!(r.source_qui_fait_foi, SourceFoi::Siecle, "{code}");
            assert_eq!(r.modification_locale, ModificationLocale::Interdite, "{code}");
            assert!(verifier_modification_directe(&r).is_err());
        }
        // Le nom et le prénom d'usage restent locaux et libres.
        for code in ["nom_usage", "prenom_usage"] {
            let r = reglage_public(code);
            assert_eq!(r.source_qui_fait_foi, SourceFoi::Scolia);
            assert!(verifier_modification_directe(&r).is_ok());
            assert_eq!(r.saisie_famille, SaisieFamille::Non);
        }
        // Les autres profils laissent le secrétariat corriger.
        for p in [ProfilEtablissement::PriveSousContrat, ProfilEtablissement::HorsContrat] {
            let r = reglage_profil(p, "nom_naissance").unwrap();
            assert_eq!(r.source_qui_fait_foi, SourceFoi::Scolia);
            assert!(verifier_modification_directe(&r).is_ok());
        }
        // Premier degré : ONDE fait foi, par proposition.
        let r = reglage_profil(ProfilEtablissement::PremierDegre, "date_naissance").unwrap();
        assert_eq!(r.source_qui_fait_foi, SourceFoi::Onde);
        assert_eq!(r.modification_locale, ModificationLocale::ParProposition);
    }

    #[test]
    fn champ_verrouille_gele_tout() {
        let mut r = reglage_public("nom_naissance");
        r.verrouille = true;
        assert!(verifier_modification_directe(&r).is_err());
        assert!(verifier_acceptation(&r, SourceProposition::Import, Some(OrigineProposition::Siecle)).is_err());
        let d = champ("nom_naissance").unwrap();
        let c = ContexteAuto {
            def: d,
            reglage: &r,
            source: SourceProposition::Import,
            origine: Some(OrigineProposition::Siecle),
            regle_active: true,
            conflit: false,
        };
        assert_eq!(evaluer_auto(c), Err(RaisonNonAuto::ChampVerrouille));
    }

    #[test]
    fn acceptation_champ_interdit_seule_la_source_qui_fait_foi() {
        let r = reglage_public("nom_naissance");
        // La demande d'une famille ou d'un personnel ne peut qu'être refusée.
        for s in [SourceProposition::Famille, SourceProposition::Personnel, SourceProposition::Eleve] {
            assert!(verifier_acceptation(&r, s, None).is_err(), "{s:?}");
        }
        // Un import SIECLE (ou AAF, BEE) est accepté ; un import d'une autre origine non.
        for o in [OrigineProposition::Siecle, OrigineProposition::Aaf, OrigineProposition::Bee] {
            assert!(verifier_acceptation(&r, SourceProposition::Import, Some(o)).is_ok());
        }
        assert!(verifier_acceptation(&r, SourceProposition::Import, Some(OrigineProposition::Reprise)).is_err());
        assert!(verifier_acceptation(&r, SourceProposition::Import, None).is_err());
        // Champ libre : tout est acceptable par une personne.
        let libre = ReglageChamp::defaut();
        assert!(verifier_acceptation(&libre, SourceProposition::Famille, None).is_ok());
    }

    fn contexte<'a>(
        def: &'a DefChamp,
        r: &'a ReglageChamp,
        source: SourceProposition,
        origine: Option<OrigineProposition>,
    ) -> ContexteAuto<'a> {
        ContexteAuto {
            def,
            reglage: r,
            source,
            origine,
            regle_active: true,
            conflit: false,
        }
    }

    #[test]
    fn regle_auto_explicite_sans_conflit_et_source_qui_fait_foi() {
        let d = champ("nom_naissance").unwrap();
        let r = reglage_public("nom_naissance");
        let ok = contexte(d, &r, SourceProposition::Import, Some(OrigineProposition::Siecle));
        assert_eq!(evaluer_auto(ok), Ok(()));
        // Sans règle active : rien ne s'applique seul.
        let mut c = ok;
        c.regle_active = false;
        assert_eq!(evaluer_auto(c), Err(RaisonNonAuto::AucuneRegle));
        // Modification locale plus récente : conflit visible, pas d'application.
        let mut c = ok;
        c.conflit = true;
        assert_eq!(evaluer_auto(c), Err(RaisonNonAuto::ConflitAvecLaFiche));
        // Un import qui n'est pas de l'autorité qui fait foi.
        let c = contexte(d, &r, SourceProposition::Import, Some(OrigineProposition::Reprise));
        assert_eq!(evaluer_auto(c), Err(RaisonNonAuto::SourceNeFaitPasFoi));
        let c = contexte(d, &r, SourceProposition::Import, None);
        assert_eq!(evaluer_auto(c), Err(RaisonNonAuto::SourceNeFaitPasFoi));
        // Une proposition de la famille ne s'applique jamais seule.
        let c = contexte(d, &r, SourceProposition::Famille, None);
        assert_eq!(evaluer_auto(c), Err(RaisonNonAuto::SourceNonAutomatisable));
        // Champ sensible : jamais.
        let sensible = champ("prenom_usage").unwrap();
        let rs = reglage_public("prenom_usage");
        let c = contexte(sensible, &rs, SourceProposition::Import, Some(OrigineProposition::Siecle));
        assert_eq!(evaluer_auto(c), Err(RaisonNonAuto::ChampSensible));
        let prenoms = champ("prenoms").unwrap();
        let rp = reglage_public("prenoms");
        let c = contexte(prenoms, &rp, SourceProposition::Import, Some(OrigineProposition::Siecle));
        assert_eq!(evaluer_auto(c), Err(RaisonNonAuto::ChampSensible));
    }

    #[test]
    fn regles_auto_des_profils() {
        // Public et premier degré : imports de l'autorité, champs non sensibles.
        let public = regles_auto_profil(ProfilEtablissement::Public);
        assert!(public.contains(&("nom_naissance", SourceProposition::Import)));
        assert!(public.contains(&("ine", SourceProposition::Import)));
        assert!(!public.iter().any(|(c, _)| ["prenoms", "sexe", "prenom_usage"].contains(c)));
        assert!(regles_auto_profil(ProfilEtablissement::PremierDegre)
            .contains(&("date_naissance", SourceProposition::Import)));
        // Les autres profils : aucune règle automatique par défaut.
        for p in [
            ProfilEtablissement::PriveSousContrat,
            ProfilEtablissement::PriveSousContratLogicielAgree,
            ProfilEtablissement::HorsContrat,
        ] {
            assert!(regles_auto_profil(p).is_empty(), "{}", p.code());
        }
    }

    #[test]
    fn bornes_du_reglage() {
        let nom = champ("nom_naissance").unwrap();
        let mut r = ReglageChamp::defaut();
        // « interdite » sans autorité externe : refusé.
        r.modification_locale = ModificationLocale::Interdite;
        assert!(valider_reglage(nom, &r).is_err());
        r.source_qui_fait_foi = SourceFoi::Siecle;
        assert!(valider_reglage(nom, &r).is_ok());
        // Une source qui ne peut pas faire foi pour ce champ.
        let usage = champ("nom_usage").unwrap();
        let mut r = ReglageChamp::defaut();
        r.source_qui_fait_foi = SourceFoi::Siecle;
        assert!(valider_reglage(usage, &r).is_err());
        // « modifier » directement : au-delà du maximum du catalogue.
        let mut r = ReglageChamp::defaut();
        r.saisie_famille = SaisieFamille::Modifier;
        assert!(valider_reglage(nom, &r).is_err());
        // Moment inconnu ou en double.
        let mut r = ReglageChamp::defaut();
        r.obligatoire_a = vec!["demain".into()];
        assert!(valider_reglage(nom, &r).is_err());
        r.obligatoire_a = vec!["creation".into(), "creation".into()];
        assert!(valider_reglage(nom, &r).is_err());
        r.obligatoire_a = vec!["creation".into(), "export_bee".into()];
        assert!(valider_reglage(nom, &r).is_ok());
    }

    #[test]
    fn statut_final_selon_les_champs() {
        assert_eq!(statut_final(0, 3), "refusee");
        assert_eq!(statut_final(3, 3), "acceptee");
        assert_eq!(statut_final(1, 3), "acceptee_partiellement");
    }

    #[test]
    fn correspondances_champs_identite_et_systemes() {
        assert_eq!(champ_catalogue_de(ChampIdentite::RangPrenomUsuel), Some("prenoms"));
        assert_eq!(champ_catalogue_de(ChampIdentite::DateNaissanceStatut), Some("date_naissance"));
        assert_eq!(champ_catalogue_de(ChampIdentite::NomEcritureOrigine), None);
        for def in CATALOGUE.iter().filter(|d| d.cible == CibleProposition::Identite) {
            // Chaque champ d'identité du catalogue est atteignable depuis un ChampIdentite.
            assert!(
                [
                    ChampIdentite::NomNaissance,
                    ChampIdentite::NomUsage,
                    ChampIdentite::Prenoms,
                    ChampIdentite::PrenomUsage,
                    ChampIdentite::Sexe,
                    ChampIdentite::DateNaissance,
                    ChampIdentite::NaissanceCommune,
                    ChampIdentite::NaissanceCommuneCog,
                    ChampIdentite::NaissancePaysCog,
                    ChampIdentite::NaissanceLieuEtranger,
                ]
                .iter()
                .any(|c| champ_catalogue_de(*c) == Some(def.code)),
                "{}",
                def.code
            );
        }
        assert_eq!(champ_catalogue_du_systeme(SystemeIdentifiant::Ine), Some("ine"));
        assert_eq!(champ_catalogue_du_systeme(SystemeIdentifiant::Badge), None);
    }

    #[test]
    fn vocabulaires_aller_retour() {
        for s in SourceProposition::TOUTES {
            assert_eq!(SourceProposition::parse(s.code()), Some(s));
        }
        for c in [
            CibleProposition::Identite,
            CibleProposition::Identifiant,
            CibleProposition::NouvelEleve,
        ] {
            assert_eq!(CibleProposition::parse(c.code()), Some(c));
        }
        assert!(CibleProposition::Identite.applicable());
        assert!(!CibleProposition::NouvelEleve.applicable());
        assert!(!CibleProposition::Photo.applicable());
        assert_eq!(ModificationLocale::parse("interdite"), Some(ModificationLocale::Interdite));
        assert_eq!(SourceFoi::parse("famille"), Some(SourceFoi::Famille));
        assert_eq!(OrigineProposition::Aaf.provenance_identifiant(), "import_aaf");
        assert_eq!(SourceProposition::Famille.source_identite(), "famille");
    }
}
