//! Socle 2 (l'élève), lot E7 — règles **pures** de la confidentialité renforcée (ELV-15), des
//! photos (ELV-16, ELV-42), des coordonnées propres de l'élève (ELV-17) et du scellement des
//! versions d'identité sensibles (§10.4).
//!
//! Rien ici ne touche la base : les listes fermées, les validations de saisie et les transitions
//! d'états. Les commandes (transaction unique, journal, événement) sont dans `ent-db`.

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

/// Refus d'une règle de ce module (messages sans donnée personnelle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErreurProtection {
    #[error("mesure de confidentialité inconnue")]
    MesureInconnue,
    #[error("le motif est obligatoire")]
    MotifVide,
    #[error("le motif est trop long")]
    MotifTropLong,
    #[error("le motif contient un caractère interdit")]
    MotifCaractere,
    #[error("motif de levée inconnu")]
    MotifLeveeInconnu,
    #[error("période de la mesure invalide")]
    PeriodeInvalide,
    #[error("motif de photo inconnu")]
    MotifPhotoInconnu,
    #[error("usage de coordonnée inconnu")]
    UsageInconnu,
    #[error("cet usage n'est pas possible pour ce type de coordonnée")]
    UsageIncompatible,
    #[error("type de coordonnée inconnu")]
    TypeInconnu,
    #[error("motif de coordonnée inconnu")]
    MotifCoordonneeInconnu,
}

// ─────────────────────────── Confidentialité renforcée (ELV-15) ───────────────────────────

/// Mesure d'une confidentialité renforcée (§7.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MesureConfidentialite {
    MasquerPhoto,
    HorsAnnuaire,
    HorsTrombinoscopeExporte,
    AdresseProtegee,
    AlerteConsultation,
}

impl MesureConfidentialite {
    pub const TOUTES: [Self; 5] = [
        Self::MasquerPhoto,
        Self::HorsAnnuaire,
        Self::HorsTrombinoscopeExporte,
        Self::AdresseProtegee,
        Self::AlerteConsultation,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::MasquerPhoto => "masquer_photo",
            Self::HorsAnnuaire => "hors_annuaire",
            Self::HorsTrombinoscopeExporte => "hors_trombinoscope_exporte",
            Self::AdresseProtegee => "adresse_protegee",
            Self::AlerteConsultation => "alerte_consultation",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUTES.into_iter().find(|m| m.code() == s)
    }

    /// Effets **minimaux non désactivables** (§9.2) : toute mesure retire l'élève de l'annuaire
    /// et masque sa photo pour qui n'a pas de lien direct avec lui. Ces deux mesures sont donc
    /// toujours posées, même si la demande ne les nomme pas.
    pub fn est_implicite(self) -> bool {
        matches!(self, Self::MasquerPhoto | Self::HorsAnnuaire)
    }
}

/// Mesures retenues pour une demande : celles qui sont demandées, plus les effets minimaux,
/// sans doublon, dans l'ordre du catalogue. Une mesure inconnue refuse toute la demande.
pub fn normaliser_mesures(demandees: &[String]) -> Result<Vec<&'static str>, ErreurProtection> {
    let mut retenues = [false; 5];
    for (i, m) in MesureConfidentialite::TOUTES.iter().enumerate() {
        retenues[i] = m.est_implicite();
    }
    for d in demandees {
        let m = MesureConfidentialite::parse(d.trim()).ok_or(ErreurProtection::MesureInconnue)?;
        let i = MesureConfidentialite::TOUTES
            .iter()
            .position(|x| *x == m)
            .ok_or(ErreurProtection::MesureInconnue)?;
        retenues[i] = true;
    }
    Ok(MesureConfidentialite::TOUTES
        .iter()
        .zip(retenues)
        .filter_map(|(m, r)| r.then_some(m.code()))
        .collect())
}

/// Longueur maximale du motif en clair (avant scellement).
pub const MOTIF_CONFIDENTIALITE_MAX: usize = 1000;

/// Motif d'une mesure : texte libre obligatoire (scellé ensuite), sans caractère de contrôle.
pub fn valider_motif_confidentialite(brut: &str) -> Result<String, ErreurProtection> {
    let m = brut.trim();
    if m.is_empty() {
        return Err(ErreurProtection::MotifVide);
    }
    if m.chars().count() > MOTIF_CONFIDENTIALITE_MAX {
        return Err(ErreurProtection::MotifTropLong);
    }
    if m.chars().any(|c| c == '\0' || (c.is_control() && c != '\n' && c != '\t')) {
        return Err(ErreurProtection::MotifCaractere);
    }
    Ok(m.to_string())
}

/// Motifs codés d'une levée (liste du logiciel ; le code n'est pas sensible).
pub const MOTIFS_LEVEE: [&str; 5] =
    ["fin_de_mesure", "decision_levee", "erreur_de_saisie", "changement_de_mesure", "autre"];

pub fn valider_motif_levee(code: &str) -> Result<&'static str, ErreurProtection> {
    MOTIFS_LEVEE
        .iter()
        .copied()
        .find(|m| *m == code)
        .ok_or(ErreurProtection::MotifLeveeInconnu)
}

/// Fenêtre d'une mesure : début au plus un an avant ou un an après aujourd'hui (une mesure
/// judiciaire peut être antérieure à son enregistrement, ou annoncée), fin postérieure au début
/// et à aujourd'hui (une mesure déjà échue n'a pas de sens : on ne la pose pas).
pub fn valider_periode_mesure(
    du: NaiveDate,
    au: Option<NaiveDate>,
    jour: NaiveDate,
) -> Result<(), ErreurProtection> {
    if du < jour - Duration::days(366) || du > jour + Duration::days(366) {
        return Err(ErreurProtection::PeriodeInvalide);
    }
    if let Some(fin) = au {
        if fin <= du || fin <= jour {
            return Err(ErreurProtection::PeriodeInvalide);
        }
    }
    Ok(())
}

// ─────────────────────────── Photos (ELV-16, ELV-42) ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatutPhoto {
    Proposee,
    Validee,
    Retiree,
    Rejetee,
}

impl StatutPhoto {
    pub fn code(self) -> &'static str {
        match self {
            Self::Proposee => "proposee",
            Self::Validee => "validee",
            Self::Retiree => "retiree",
            Self::Rejetee => "rejetee",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [Self::Proposee, Self::Validee, Self::Retiree, Self::Rejetee].into_iter().find(|x| x.code() == s)
    }

    /// Transitions permises (miroir du déclencheur SQL) : proposée → validée | rejetée,
    /// validée → retirée.
    pub fn peut_passer_a(self, cible: Self) -> bool {
        matches!(
            (self, cible),
            (Self::Proposee, Self::Validee) | (Self::Proposee, Self::Rejetee) | (Self::Validee, Self::Retiree)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePhoto {
    Etablissement,
    Photographe,
    Famille,
    Eleve,
    Import,
}

impl SourcePhoto {
    pub fn code(self) -> &'static str {
        match self {
            Self::Etablissement => "etablissement",
            Self::Photographe => "photographe",
            Self::Famille => "famille",
            Self::Eleve => "eleve",
            Self::Import => "import",
        }
    }

    /// Une photo venue de la famille, de l'élève ou d'un import attend toujours une validation
    /// humaine ; celle du personnel qui détient `eleves.photo.gerer` peut être validée du même
    /// geste.
    pub fn exige_validation(self) -> bool {
        matches!(self, Self::Famille | Self::Eleve | Self::Import)
    }
}

/// Au plus ce nombre de photos proposées en attente par élève (anti-inondation).
pub const PHOTOS_EN_ATTENTE_MAX: i64 = 3;

/// Motifs codés d'un rejet ou d'un retrait de photo.
pub const MOTIFS_PHOTO: [&str; 8] = [
    "non_conforme",
    "illisible",
    "autre_personne",
    "refus_famille",
    "remplacee",
    "erreur_de_saisie",
    "demande_famille",
    "autre",
];

pub fn valider_motif_photo(code: &str) -> Result<&'static str, ErreurProtection> {
    MOTIFS_PHOTO
        .iter()
        .copied()
        .find(|m| *m == code)
        .ok_or(ErreurProtection::MotifPhotoInconnu)
}

// ─────────────────────────── Coordonnées propres (ELV-17) ───────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeCoordonneeEleve {
    Email,
    Mobile,
}

impl TypeCoordonneeEleve {
    pub fn code(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Mobile => "mobile",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ErreurProtection> {
        match s {
            "email" => Ok(Self::Email),
            "mobile" => Ok(Self::Mobile),
            _ => Err(ErreurProtection::TypeInconnu),
        }
    }

    /// Canal de vérification par code.
    pub fn canal(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Mobile => "sms",
        }
    }
}

pub const USAGES_COORDONNEE_ELEVE: [&str; 3] = ["notifications", "compte", "urgence"];

/// Au plus ce nombre de coordonnées vivantes (proposées ou actives) par type et par élève.
pub const COORDONNEES_VIVANTES_MAX_PAR_TYPE: i64 = 2;

/// Usages d'une coordonnée : sans doublon, dans l'ordre du catalogue. L'identifiant de compte
/// (`compte`) est une adresse électronique, jamais un mobile.
pub fn normaliser_usages(
    type_: TypeCoordonneeEleve,
    demandes: &[String],
) -> Result<Vec<&'static str>, ErreurProtection> {
    let mut retenus = [false; 3];
    for d in demandes {
        let i = USAGES_COORDONNEE_ELEVE
            .iter()
            .position(|u| *u == d.trim())
            .ok_or(ErreurProtection::UsageInconnu)?;
        retenus[i] = true;
    }
    if retenus[1] && type_ != TypeCoordonneeEleve::Email {
        return Err(ErreurProtection::UsageIncompatible);
    }
    Ok(USAGES_COORDONNEE_ELEVE.iter().zip(retenus).filter_map(|(u, r)| r.then_some(*u)).collect())
}

pub const MOTIFS_COORDONNEE: [&str; 8] = [
    "non_conforme",
    "doublon",
    "refus_etablissement",
    "remplacee",
    "erreur_de_saisie",
    "demande_eleve",
    "demande_famille",
    "autre",
];

pub fn valider_motif_coordonnee(code: &str) -> Result<&'static str, ErreurProtection> {
    MOTIFS_COORDONNEE
        .iter()
        .copied()
        .find(|m| *m == code)
        .ok_or(ErreurProtection::MotifCoordonneeInconnu)
}

/// Une famille peut proposer une coordonnée **de l'élève** tant qu'il est mineur et non
/// émancipé : ensuite ses coordonnées propres ne regardent que lui et l'établissement.
pub fn famille_peut_proposer_coordonnee(majeur: bool, capacite_propre: bool) -> bool {
    !majeur && !capacite_propre
}

// ─────────────────────────── Scellement des versions sensibles (§10.4) ───────────────────────────

/// Valeur de remplacement des colonnes en clair d'une version scellée.
pub const MARQUEUR_SCELLE: &str = "[scellé]";

/// Séparateur d'une liste de prénoms dans une valeur scellée (caractère de contrôle : jamais
/// présent dans un prénom saisi).
const SEPARATEUR_PRENOMS: char = '\u{1f}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChampScelle {
    NomNaissance,
    NomUsage,
    Prenoms,
    RangPrenomUsuel,
    PrenomUsage,
    Sexe,
    NomEcritureOrigine,
    PrenomsEcritureOrigine,
}

impl ChampScelle {
    pub const TOUS: [Self; 8] = [
        Self::NomNaissance,
        Self::NomUsage,
        Self::Prenoms,
        Self::RangPrenomUsuel,
        Self::PrenomUsage,
        Self::Sexe,
        Self::NomEcritureOrigine,
        Self::PrenomsEcritureOrigine,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::NomNaissance => "nom_naissance",
            Self::NomUsage => "nom_usage",
            Self::Prenoms => "prenoms",
            Self::RangPrenomUsuel => "rang_prenom_usuel",
            Self::PrenomUsage => "prenom_usage",
            Self::Sexe => "sexe",
            Self::NomEcritureOrigine => "nom_ecriture_origine",
            Self::PrenomsEcritureOrigine => "prenoms_ecriture_origine",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|c| c.code() == s)
    }
}

/// Liste de prénoms → valeur à sceller.
pub fn encoder_prenoms(prenoms: &[String]) -> String {
    prenoms.join(&SEPARATEUR_PRENOMS.to_string())
}

/// Valeur descellée → liste de prénoms.
pub fn decoder_prenoms(valeur: &str) -> Vec<String> {
    valeur.split(SEPARATEUR_PRENOMS).map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn les_effets_minimaux_sont_toujours_poses_et_dans_l_ordre_du_catalogue() {
        let m = normaliser_mesures(&[]).unwrap();
        assert_eq!(m, ["masquer_photo", "hors_annuaire"]);
        let m = normaliser_mesures(&s(&["alerte_consultation", "adresse_protegee", "alerte_consultation"])).unwrap();
        assert_eq!(m, ["masquer_photo", "hors_annuaire", "adresse_protegee", "alerte_consultation"]);
    }

    #[test]
    fn une_mesure_inconnue_refuse_toute_la_demande() {
        assert_eq!(
            normaliser_mesures(&s(&["hors_annuaire", "invisible"])),
            Err(ErreurProtection::MesureInconnue)
        );
        assert_eq!(normaliser_mesures(&s(&[""])), Err(ErreurProtection::MesureInconnue));
    }

    #[test]
    fn le_motif_est_obligatoire_borne_et_sans_caractere_de_controle() {
        assert_eq!(valider_motif_confidentialite("   "), Err(ErreurProtection::MotifVide));
        assert_eq!(valider_motif_confidentialite(&"x".repeat(1001)), Err(ErreurProtection::MotifTropLong));
        assert_eq!(valider_motif_confidentialite("a\0b"), Err(ErreurProtection::MotifCaractere));
        assert_eq!(valider_motif_confidentialite("a\u{7}b"), Err(ErreurProtection::MotifCaractere));
        assert_eq!(valider_motif_confidentialite("  jugement du 3 mai \n suite ").unwrap(), "jugement du 3 mai \n suite");
        assert!(valider_motif_confidentialite(&"é".repeat(1000)).is_ok());
    }

    #[test]
    fn la_levee_exige_un_motif_du_catalogue() {
        assert_eq!(valider_motif_levee("fin_de_mesure"), Ok("fin_de_mesure"));
        assert_eq!(valider_motif_levee("parce_que"), Err(ErreurProtection::MotifLeveeInconnu));
    }

    #[test]
    fn la_fenetre_d_une_mesure_est_bornee() {
        let jour = NaiveDate::from_ymd_opt(2026, 11, 17).unwrap();
        let d = |a, m, j| NaiveDate::from_ymd_opt(a, m, j).unwrap();
        assert!(valider_periode_mesure(jour, None, jour).is_ok());
        assert!(valider_periode_mesure(d(2026, 6, 1), Some(d(2027, 6, 30)), jour).is_ok());
        // Début trop ancien ou trop lointain.
        assert_eq!(valider_periode_mesure(d(2024, 1, 1), None, jour), Err(ErreurProtection::PeriodeInvalide));
        assert_eq!(valider_periode_mesure(d(2028, 1, 1), None, jour), Err(ErreurProtection::PeriodeInvalide));
        // Fin avant le début, égale au début, ou déjà échue.
        assert_eq!(valider_periode_mesure(jour, Some(d(2026, 11, 1)), jour), Err(ErreurProtection::PeriodeInvalide));
        assert_eq!(valider_periode_mesure(jour, Some(jour), jour), Err(ErreurProtection::PeriodeInvalide));
        assert_eq!(valider_periode_mesure(d(2026, 1, 1), Some(d(2026, 11, 17)), jour), Err(ErreurProtection::PeriodeInvalide));
    }

    #[test]
    fn transitions_de_photo() {
        use StatutPhoto::*;
        assert!(Proposee.peut_passer_a(Validee));
        assert!(Proposee.peut_passer_a(Rejetee));
        assert!(Validee.peut_passer_a(Retiree));
        for (a, b) in [(Validee, Proposee), (Retiree, Validee), (Rejetee, Validee), (Proposee, Retiree), (Validee, Validee)] {
            assert!(!a.peut_passer_a(b), "{a:?} → {b:?}");
        }
        assert_eq!(StatutPhoto::parse("validee"), Some(Validee));
        assert_eq!(StatutPhoto::parse("autre"), None);
    }

    #[test]
    fn la_famille_et_l_eleve_attendent_toujours_une_validation() {
        assert!(SourcePhoto::Famille.exige_validation());
        assert!(SourcePhoto::Eleve.exige_validation());
        assert!(SourcePhoto::Import.exige_validation());
        assert!(!SourcePhoto::Etablissement.exige_validation());
        assert!(!SourcePhoto::Photographe.exige_validation());
        assert!(valider_motif_photo("refus_famille").is_ok());
        assert!(valider_motif_photo("n_importe_quoi").is_err());
    }

    #[test]
    fn usages_de_coordonnee() {
        use TypeCoordonneeEleve::*;
        assert_eq!(normaliser_usages(Email, &s(&["urgence", "notifications", "notifications"])).unwrap(), ["notifications", "urgence"]);
        assert_eq!(normaliser_usages(Email, &[]).unwrap(), Vec::<&str>::new());
        assert_eq!(normaliser_usages(Email, &s(&["compte"])).unwrap(), ["compte"]);
        assert_eq!(normaliser_usages(Mobile, &s(&["compte"])), Err(ErreurProtection::UsageIncompatible));
        assert_eq!(normaliser_usages(Mobile, &s(&["facturation"])), Err(ErreurProtection::UsageInconnu));
        assert_eq!(TypeCoordonneeEleve::parse("fixe"), Err(ErreurProtection::TypeInconnu));
        assert_eq!(Email.canal(), "email");
        assert_eq!(Mobile.canal(), "sms");
    }

    #[test]
    fn la_famille_ne_propose_plus_une_fois_majeur_ou_emancipe() {
        assert!(famille_peut_proposer_coordonnee(false, false));
        assert!(!famille_peut_proposer_coordonnee(true, false));
        assert!(!famille_peut_proposer_coordonnee(false, true));
    }

    #[test]
    fn prenoms_scelles_aller_retour() {
        let p = s(&["Jean", "Pierre Marie", "Éloïse"]);
        assert_eq!(decoder_prenoms(&encoder_prenoms(&p)), p);
        assert_eq!(decoder_prenoms(&encoder_prenoms(&s(&["Seul"]))), s(&["Seul"]));
        for c in ChampScelle::TOUS {
            assert_eq!(ChampScelle::parse(c.code()), Some(c));
        }
        assert_eq!(ChampScelle::parse("date_naissance"), None);
    }
}
