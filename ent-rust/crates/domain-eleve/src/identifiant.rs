//! Socle 2 (l'élève), lot E3 — identifiants externes de l'élève (INE et autres) :
//! systèmes, statuts, provenances, normalisation et transitions. Logique **pure**, sans
//! accès base.
//!
//! **Format de l'INE : règle large, volontairement.** Aucune source officielle lue n'a pu
//! confirmer ici la longueur, la composition par famille (second degré, premier degré,
//! apprentissage, agriculture…) ni l'algorithme de la clé de contrôle ; les extraits
//! trouvés se contredisent (cf. `docs/socles/02_ELEVE.md` §7.3 [NV], D-019). On garde donc
//! la règle existante (9 à 16 caractères alphanumériques) et on ne code **aucune** clé de
//! contrôle de mémoire. La validation par famille se branchera ici, dans ce module isolé,
//! quand un texte officiel aura été vérifié.

use crate::ine::normalize_ine;

/// Erreur de règle sur un identifiant (messages sans donnée personnelle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErreurIdentifiant {
    #[error("{0}")]
    Invalide(&'static str),
    #[error("{0}")]
    Conflit(&'static str),
}

/// Système qui attribue l'identifiant (liste du logiciel, `ent_systeme_identifiant`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemeIdentifiant {
    Ine,
    SiecleEleve,
    Onde,
    AafJointure,
    Badge,
    AncienLogiciel,
}

impl SystemeIdentifiant {
    pub const TOUS: [Self; 6] = [
        Self::Ine,
        Self::SiecleEleve,
        Self::Onde,
        Self::AafJointure,
        Self::Badge,
        Self::AncienLogiciel,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Ine => "INE",
            Self::SiecleEleve => "SIECLE_ELEVE",
            Self::Onde => "ONDE",
            Self::AafJointure => "AAF_JOINTURE",
            Self::Badge => "BADGE",
            Self::AncienLogiciel => "ANCIEN_LOGICIEL",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::TOUS.into_iter().find(|x| x.code() == s)
    }

    /// Unicité de la valeur dans l'établissement (jamais entre établissements).
    pub fn unique_par_etablissement(self) -> bool {
        !matches!(self, Self::AncienLogiciel)
    }

    /// Le badge est géré par la vie scolaire (`eleves.badge.gerer`), pas par le secrétariat
    /// d'identité (ELV-41).
    pub fn est_badge(self) -> bool {
        matches!(self, Self::Badge)
    }
}

/// Statut d'un identifiant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatutIdentifiant {
    /// Recopié d'un bulletin, donné par la famille.
    Declare,
    /// Venu d'un dossier ou d'une reprise, à confirmer.
    AVerifier,
    /// Confirmé par un flux officiel ou une vérification.
    Confirme,
    /// Collision ou doute : n'est plus utilisé, alerte de qualité.
    Conteste,
    /// Retiré (fin de validité, remplacé).
    Revoque,
}

impl StatutIdentifiant {
    pub fn code(self) -> &'static str {
        match self {
            Self::Declare => "declare",
            Self::AVerifier => "a_verifier",
            Self::Confirme => "confirme",
            Self::Conteste => "conteste",
            Self::Revoque => "revoque",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::Declare,
            Self::AVerifier,
            Self::Confirme,
            Self::Conteste,
            Self::Revoque,
        ]
        .into_iter()
        .find(|x| x.code() == s)
    }

    /// Statut « en vigueur » (compte pour l'unicité).
    pub fn est_actif(self) -> bool {
        matches!(self, Self::Declare | Self::AVerifier | Self::Confirme)
    }
}

/// D'où vient la valeur (§8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceIdentifiant {
    Saisie,
    DocumentFamille,
    AncienEtablissement,
    ImportAaf,
    ImportBee,
    RetourBee,
    ImportOnde,
    Educonnect,
    RepriseLogiciel,
    Badge,
}

impl ProvenanceIdentifiant {
    pub fn code(self) -> &'static str {
        match self {
            Self::Saisie => "saisie",
            Self::DocumentFamille => "document_famille",
            Self::AncienEtablissement => "ancien_etablissement",
            Self::ImportAaf => "import_aaf",
            Self::ImportBee => "import_bee",
            Self::RetourBee => "retour_bee",
            Self::ImportOnde => "import_onde",
            Self::Educonnect => "educonnect",
            Self::RepriseLogiciel => "reprise_logiciel",
            Self::Badge => "badge",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::Saisie,
            Self::DocumentFamille,
            Self::AncienEtablissement,
            Self::ImportAaf,
            Self::ImportBee,
            Self::RetourBee,
            Self::ImportOnde,
            Self::Educonnect,
            Self::RepriseLogiciel,
            Self::Badge,
        ]
        .into_iter()
        .find(|x| x.code() == s)
    }

    /// Une provenance « officielle » (flux national, badge de l'établissement) entre
    /// `confirme` ; une saisie ou un document de la famille reste `declare` ; un dossier
    /// d'ancien établissement ou une reprise entre `a_verifier` (§8.1).
    pub fn statut_initial(self) -> StatutIdentifiant {
        match self {
            Self::ImportAaf
            | Self::ImportBee
            | Self::RetourBee
            | Self::ImportOnde
            | Self::Badge => StatutIdentifiant::Confirme,
            Self::AncienEtablissement | Self::RepriseLogiciel => StatutIdentifiant::AVerifier,
            Self::Saisie | Self::DocumentFamille | Self::Educonnect => StatutIdentifiant::Declare,
        }
    }

    /// Provenances qu'un opérateur peut déclarer à la main (les flux officiels et le
    /// badge passent par leur propre chemin).
    pub fn saisissable_a_la_main(self) -> bool {
        matches!(
            self,
            Self::Saisie | Self::DocumentFamille | Self::AncienEtablissement
        )
    }
}

/// Normalise la valeur selon le système. L'INE garde la règle large (9–16
/// alphanumériques, majuscules) ; les autres : 1 à 64 caractères imprimables, sans
/// espace de bord, ni caractère de contrôle.
pub fn normaliser_valeur(
    systeme: SystemeIdentifiant,
    brut: &str,
) -> Result<String, ErreurIdentifiant> {
    match systeme {
        SystemeIdentifiant::Ine => normalize_ine(brut)
            .map_err(|_| ErreurIdentifiant::Invalide("INE invalide (9 à 16 caractères alphanumériques)")),
        _ => {
            let v = brut.trim();
            if v.is_empty() || v.chars().count() > 64 {
                return Err(ErreurIdentifiant::Invalide(
                    "identifiant invalide (1 à 64 caractères)",
                ));
            }
            if v.chars().any(|c| c.is_control()) {
                return Err(ErreurIdentifiant::Invalide(
                    "identifiant invalide (caractère non imprimable)",
                ));
            }
            Ok(v.to_string())
        }
    }
}

/// Motif libre : obligatoire pour contester ou révoquer, 500 caractères au plus.
pub fn motif_valide(motif: Option<&str>) -> Result<String, ErreurIdentifiant> {
    let m = motif.map(str::trim).unwrap_or("");
    if m.is_empty() {
        return Err(ErreurIdentifiant::Invalide("un motif est obligatoire"));
    }
    if m.chars().count() > 500 {
        return Err(ErreurIdentifiant::Invalide("motif trop long (500 caractères)"));
    }
    Ok(m.to_string())
}

/// Transitions autorisées par une commande (ELV-07).
///
/// * confirmer : `declare` / `a_verifier` → `confirme` ; `conteste` → `confirme` seulement
///   après vérification qu'aucune collision ne subsiste (faite par l'appelant) ;
/// * contester : `declare` / `a_verifier` / `confirme` → `conteste` (motif) ;
/// * révoquer : tout statut sauf `revoque` → `revoque` (motif).
pub fn transition_permise(
    de: StatutIdentifiant,
    vers: StatutIdentifiant,
) -> Result<(), ErreurIdentifiant> {
    use StatutIdentifiant::*;
    let ok = match vers {
        Confirme => matches!(de, Declare | AVerifier | Conteste),
        Conteste => matches!(de, Declare | AVerifier | Confirme),
        Revoque => !matches!(de, Revoque),
        Declare | AVerifier => false,
    };
    if ok {
        Ok(())
    } else {
        Err(ErreurIdentifiant::Conflit(
            "transition impossible depuis le statut actuel de l'identifiant",
        ))
    }
}

/// Les exports nationaux (BEE, LSU, LSL) n'utilisent qu'un identifiant confirmé : un
/// identifiant qui ne l'est pas est signalé à l'écran de qualité.
pub fn utilisable_pour_export(statut: StatutIdentifiant) -> bool {
    matches!(statut, StatutIdentifiant::Confirme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statut_initial_suit_la_provenance() {
        use ProvenanceIdentifiant::*;
        assert_eq!(ImportAaf.statut_initial(), StatutIdentifiant::Confirme);
        assert_eq!(RetourBee.statut_initial(), StatutIdentifiant::Confirme);
        assert_eq!(Badge.statut_initial(), StatutIdentifiant::Confirme);
        assert_eq!(AncienEtablissement.statut_initial(), StatutIdentifiant::AVerifier);
        assert_eq!(RepriseLogiciel.statut_initial(), StatutIdentifiant::AVerifier);
        assert_eq!(DocumentFamille.statut_initial(), StatutIdentifiant::Declare);
        assert_eq!(Saisie.statut_initial(), StatutIdentifiant::Declare);
    }

    #[test]
    fn codes_aller_retour() {
        for s in SystemeIdentifiant::TOUS {
            assert_eq!(SystemeIdentifiant::parse(s.code()), Some(s));
        }
        assert_eq!(SystemeIdentifiant::parse("inconnu"), None);
        for st in ["declare", "a_verifier", "confirme", "conteste", "revoque"] {
            assert_eq!(StatutIdentifiant::parse(st).unwrap().code(), st);
        }
        for p in ["saisie", "import_aaf", "retour_bee", "reprise_logiciel", "badge"] {
            assert_eq!(ProvenanceIdentifiant::parse(p).unwrap().code(), p);
        }
    }

    #[test]
    fn ine_garde_la_regle_large_sans_cle_de_controle() {
        // Règle large documentée : 9 à 16 alphanumériques, majuscules, espaces retirés.
        assert_eq!(
            normaliser_valeur(SystemeIdentifiant::Ine, " 1234 5678 9ab ").unwrap(),
            "123456789AB"
        );
        assert!(normaliser_valeur(SystemeIdentifiant::Ine, "court").is_err());
        assert!(normaliser_valeur(SystemeIdentifiant::Ine, "12345678-90A").is_err());
        assert!(normaliser_valeur(SystemeIdentifiant::Ine, &"A".repeat(17)).is_err());
    }

    #[test]
    fn autres_systemes_bornes_et_imprimables() {
        let b = normaliser_valeur(SystemeIdentifiant::Badge, "  bdg-001 ").unwrap();
        assert_eq!(b, "bdg-001");
        assert!(normaliser_valeur(SystemeIdentifiant::Badge, "  ").is_err());
        assert!(normaliser_valeur(SystemeIdentifiant::Badge, "a\u{0}b").is_err());
        assert!(normaliser_valeur(SystemeIdentifiant::AncienLogiciel, &"x".repeat(65)).is_err());
    }

    #[test]
    fn transitions() {
        use StatutIdentifiant::*;
        assert!(transition_permise(Declare, Confirme).is_ok());
        assert!(transition_permise(AVerifier, Confirme).is_ok());
        assert!(transition_permise(Conteste, Confirme).is_ok());
        assert!(transition_permise(Revoque, Confirme).is_err());
        assert!(transition_permise(Confirme, Conteste).is_ok());
        assert!(transition_permise(Conteste, Conteste).is_err());
        assert!(transition_permise(Confirme, Revoque).is_ok());
        assert!(transition_permise(Revoque, Revoque).is_err());
        assert!(transition_permise(Confirme, Declare).is_err());
    }

    #[test]
    fn motif_obligatoire_et_borne() {
        assert!(motif_valide(None).is_err());
        assert!(motif_valide(Some("  ")).is_err());
        assert!(motif_valide(Some(&"x".repeat(501))).is_err());
        assert_eq!(motif_valide(Some(" doute ")).unwrap(), "doute");
    }

    #[test]
    fn seul_un_identifiant_confirme_sert_aux_exports() {
        assert!(utilisable_pour_export(StatutIdentifiant::Confirme));
        assert!(!utilisable_pour_export(StatutIdentifiant::Declare));
        assert!(!utilisable_pour_export(StatutIdentifiant::Conteste));
    }
}
