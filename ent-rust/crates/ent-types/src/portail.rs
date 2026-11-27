//! Socle 29 (portails), lot P1 : la page « Aujourd'hui » composée côté serveur
//! (`GET /portail/aujourdhui`, `docs/socles/29_PORTAILS.md` §9).
//!
//! Le portail ne possède aucune donnée métier : chaque tuile est une **vue typée** rendue par
//! le serveur sous les droits de l'acteur. Une source pas encore codée n'a **pas** de tuile
//! (jamais de donnée inventée) ; une source en erreur ou hors délai donne une tuile
//! `indisponible` sans faire tomber la page (R-PO12).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Profil d'accueil (détermine l'ordre et le titre de la page, pas les droits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfilPortail {
    Famille,
    Eleve,
    Professeur,
    VieScolaire,
    Accueil,
    Secretariat,
    Direction,
    Comptabilite,
    Personnel,
}

/// État d'une tuile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EtatTuile {
    /// Contenu à montrer.
    Ok,
    /// Rien à montrer : l'état vide est signifiant (message dans `principal`).
    Vide,
    /// Source en erreur ou trop lente : « indisponible, réessayer ».
    Indisponible,
}

/// Ton d'une ligne (jamais la seule porteuse du sens : un libellé l'accompagne).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TonLigne {
    Neutre,
    Attention,
    Alerte,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LignePortail {
    pub libelle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub ton: TonLigne,
    /// Chemin de l'écran propriétaire (relatif, même origine).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lien: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPortail {
    pub libelle: String,
    pub lien: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuilePortail {
    /// Code stable `{domaine}.{tuile}` (ex. `edt.journee`).
    pub code: String,
    pub titre: String,
    /// Largeur Bento proposée : 4, 6, 8 ou 12 colonnes.
    pub largeur: u8,
    pub etat: EtatTuile,
    /// Une information principale (ou le message de l'état vide).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    /// Compteur réellement actionnable par la personne.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nombre: Option<i64>,
    #[serde(default)]
    pub lignes: Vec<LignePortail>,
    /// Une action principale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionPortail>,
    /// Contenu non montré d'emblée (notes) : le navigateur doit demander par un geste.
    #[serde(default)]
    pub sur_geste: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortailAujourdhui {
    pub profil: ProfilPortail,
    pub jour: NaiveDate,
    /// Prénom d'usage pour « Bonjour … » (jamais le nom de naissance).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prenom: Option<String>,
    pub tuiles: Vec<TuilePortail>,
}

impl TuilePortail {
    pub fn nouvelle(code: &str, titre: &str, largeur: u8) -> Self {
        Self {
            code: code.to_string(),
            titre: titre.to_string(),
            largeur,
            etat: EtatTuile::Vide,
            principal: None,
            nombre: None,
            lignes: Vec::new(),
            action: None,
            sur_geste: false,
        }
    }

    /// Tuile « indisponible » : rendue à la place d'une source en erreur.
    pub fn indisponible(code: &str, titre: &str, largeur: u8) -> Self {
        let mut t = Self::nouvelle(code, titre, largeur);
        t.etat = EtatTuile::Indisponible;
        t
    }
}

impl LignePortail {
    pub fn simple(libelle: impl Into<String>) -> Self {
        Self {
            libelle: libelle.into(),
            detail: None,
            ton: TonLigne::Neutre,
            lien: None,
        }
    }
}
