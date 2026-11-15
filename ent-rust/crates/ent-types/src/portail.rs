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

/// Mode de l'accueil (lot P7, `docs/socles/29_PORTAILS.md` §10.1). **Calculé par le serveur** à partir des faits
/// des autres socles ; personne ne « bascule » l'accueil à la main (D-023), sauf la clôture d'une alerte (socle 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModePortail {
    #[default]
    Ordinaire,
    Rentree,
    /// Période des conseils (socle 23) : pas encore calculé.
    VeilleConseil,
    Vacances,
    Examens,
    /// Fermeture, grève, intempéries, journée banalisée (socles 5 et 10).
    JourneeModifiee,
    /// Alerte PPMS (socle 14) : la page entière est la page d'alerte.
    Alerte,
    /// Bascule d'année en cours (socle 5) : pas encore calculé.
    Bascule,
    /// An 1 du pilote, notes dans Charlemagne (D-032) : pas encore calculé.
    Cohabitation,
}

/// Gravité d'un bandeau (§10.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraviteBandeau {
    Info,
    Attention,
    /// Jamais masquable (R-PO15).
    Securite,
}

/// Un bandeau déclaré par son socle : texte, gravité, masquable ou non (§10.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bandeau {
    /// Code stable `{domaine}.{bandeau}` (`structure.fermeture`, `alerte.ppms`).
    pub code: String,
    pub gravite: GraviteBandeau,
    pub texte: String,
    /// Faux pour la sécurité (R-PO15) ; l'écran ne propose alors aucun bouton « Masquer ».
    pub masquable: bool,
    /// Écran propriétaire, s'il y en a un.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lien: Option<String>,
}

/// Un élève dont la personne est responsable maintenant (page d'alerte, personnels seulement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EleveEnCharge {
    /// Libellé minimal (« DUPONT Léa — 4B »).
    pub libelle: String,
    /// Dernier état de présence connu (« présent », « absent », « non pointé »).
    pub presence: String,
}

/// La page d'alerte (mode `alerte`) : la consigne officielle ; la liste des élèves en charge seulement pour
/// les personnels (jamais de détail opérationnel pour les familles, §10.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageAlerte {
    pub consigne: String,
    #[serde(default)]
    pub eleves_en_charge: Vec<EleveEnCharge>,
    /// « Je suis en sécurité avec … » (POR-227) : appartient au socle 14, absent tant qu'il n'existe pas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_securite: Option<ActionPortail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortailAujourdhui {
    pub profil: ProfilPortail,
    pub jour: NaiveDate,
    /// Prénom d'usage pour « Bonjour … » (jamais le nom de naissance).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prenom: Option<String>,
    pub tuiles: Vec<TuilePortail>,
    /// Mode du moment (lot P7).
    #[serde(default)]
    pub mode: ModePortail,
    /// Au plus **deux** bandeaux visibles ; les autres sont repliés (§10.2).
    #[serde(default)]
    pub bandeaux: Vec<Bandeau>,
    #[serde(default)]
    pub bandeaux_replies: Vec<Bandeau>,
    /// Présente seulement en mode `alerte` (alors `tuiles` est vide).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alerte: Option<PageAlerte>,
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

impl Bandeau {
    /// Un bandeau de sécurité n'est **jamais** masquable (R-PO15) : la gravité décide, pas l'appelant.
    pub fn nouveau(
        code: &str,
        gravite: GraviteBandeau,
        texte: &str,
        masquable: bool,
        lien: Option<&str>,
    ) -> Self {
        Self {
            code: code.to_string(),
            gravite,
            texte: texte.to_string(),
            masquable: masquable && gravite != GraviteBandeau::Securite,
            lien: lien.map(str::to_string),
        }
    }
}

/// Nombre de bandeaux visibles d'emblée (§10.2).
pub const BANDEAUX_VISIBLES: usize = 2;

/// Répartit les bandeaux en visibles et repliés : par gravité décroissante (ordre d'arrivée à gravité égale),
/// au plus deux visibles ; un bandeau de sécurité n'est jamais replié.
pub fn repartir_bandeaux(mut tous: Vec<Bandeau>) -> (Vec<Bandeau>, Vec<Bandeau>) {
    tous.sort_by_key(|b| std::cmp::Reverse(b.gravite));
    let mut visibles = Vec::new();
    let mut replies = Vec::new();
    for b in tous {
        if visibles.len() < BANDEAUX_VISIBLES || b.gravite == GraviteBandeau::Securite {
            visibles.push(b);
        } else {
            replies.push(b);
        }
    }
    (visibles, replies)
}

#[cfg(test)]
mod tests_bandeaux {
    use super::*;

    fn b(code: &str, g: GraviteBandeau) -> Bandeau {
        Bandeau::nouveau(code, g, "texte", true, None)
    }

    #[test]
    fn la_securite_n_est_jamais_masquable() {
        assert!(!b("a", GraviteBandeau::Securite).masquable);
        assert!(b("a", GraviteBandeau::Attention).masquable);
        assert!(!Bandeau::nouveau("a", GraviteBandeau::Info, "t", false, None).masquable);
    }

    #[test]
    fn deux_visibles_au_plus_les_plus_graves_d_abord() {
        let (v, r) = repartir_bandeaux(vec![
            b("i1", GraviteBandeau::Info),
            b("a1", GraviteBandeau::Attention),
            b("i2", GraviteBandeau::Info),
            b("s1", GraviteBandeau::Securite),
        ]);
        let codes = |l: &[Bandeau]| l.iter().map(|x| x.code.clone()).collect::<Vec<_>>();
        assert_eq!(codes(&v), ["s1", "a1"]);
        assert_eq!(codes(&r), ["i1", "i2"]);
    }

    #[test]
    fn la_securite_ne_se_replie_jamais() {
        let (v, r) = repartir_bandeaux(vec![
            b("s1", GraviteBandeau::Securite),
            b("s2", GraviteBandeau::Securite),
            b("s3", GraviteBandeau::Securite),
            b("a", GraviteBandeau::Attention),
        ]);
        assert_eq!(v.len(), 3);
        assert!(v.iter().all(|x| x.gravite == GraviteBandeau::Securite));
        assert_eq!(r.len(), 1);
        assert_eq!(repartir_bandeaux(vec![]), (vec![], vec![]));
    }
}
