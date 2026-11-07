//! Actions du catalogue D-017 déclarées par chaque brique.
//!
//! Aujourd'hui `ent_auth::Action` est un `enum` fermé : chaque lot y ajoute
//! ses variantes en six endroits, ce qui fait de `actions.rs` un point de
//! collision. Ici :
//! - une [`Action`] n'est **que son identifiant stable** (le code écrit en base
//!   dans `ent_role_droit.action`) : égalité, hachage et ordre portent sur le
//!   code seul, et une constante sert de motif de `match` ;
//! - sa **définition** ([`ActionDef`] : libellé, portées admises, sensibilité,
//!   export) vit dans le catalogue ;
//! - la macro [`declarer_actions!`] produit en une fois les constantes et la
//!   liste des définitions : une action ne peut pas exister sans définition
//!   (ce que le test actuel de `TOUTES` ne garantit pas).
//!
//! Bascule de l'existant (étape E2) : ce type remplace l'`enum` ; les 68
//! variantes deviennent des **constantes associées du même nom**
//! (`Action::NotesLire`), déclarées ici dans un bloc `impl Action` réservé à
//! l'historique, et `ent-auth` ré-exporte le type. Les usages `Action::X`
//! compilent sans changement ; les `use Action::*` (4 sites dans `ent-auth`)
//! sont à remplacer, un glob ne s'appliquant pas aux constantes associées.

use std::collections::BTreeMap;
use std::fmt;

/// Portée d'un droit accordé (D-017, D-022). Copie des quatre portées du code
/// actuel ; la portée « moi-même » de D-017 et d'IA_COEUR §5.5 n'existe pas
/// encore dans `ent-auth` (question Q-B16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Portee {
    Etablissement,
    MesClasses,
    MesGroupes,
    Designes,
}

impl Portee {
    pub const TOUTES: &'static [Portee] = &[
        Portee::Etablissement,
        Portee::MesClasses,
        Portee::MesGroupes,
        Portee::Designes,
    ];
    pub const ETABLISSEMENT_SEULE: &'static [Portee] = &[Portee::Etablissement];
}

/// Identifiant d'une action contrôlée par le serveur. Ne jamais renommer un
/// code publié sans migration de `ent_role_droit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Action(&'static str);

impl Action {
    pub const fn new(code: &'static str) -> Self {
        Self(code)
    }
    pub const fn code(&self) -> &'static str {
        self.0
    }
    /// Domaine : ce qui précède le premier point (`infirmerie` pour `infirmerie.passages.lire`).
    pub fn domaine(&self) -> &'static str {
        self.0.split('.').next().unwrap_or(self.0)
    }
}

/// Définition d'une action, tenue par le catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDef {
    pub action: Action,
    pub libelle: &'static str,
    pub portees: &'static [Portee],
    /// Données sensibles (santé, psy…) : consultation tracée, jamais servie à
    /// l'IA sans droit dédié (IA_COEUR §5.5).
    pub sensible: bool,
    /// Export officiel (LSU, LSL, Notanet, ONDE…) : réservé aux modèles
    /// administratifs (D-022), contrôlé par l'assemblage.
    pub export: bool,
}

impl ActionDef {
    pub const fn new(action: Action, libelle: &'static str, portees: &'static [Portee]) -> Self {
        Self {
            action,
            libelle,
            portees,
            sensible: false,
            export: false,
        }
    }
    pub const fn sensible(self) -> Self {
        Self {
            sensible: true,
            ..self
        }
    }
    pub const fn export(self) -> Self {
        Self {
            export: true,
            ..self
        }
    }
}

/// Déclare des actions : une constante par action et `DEFINITIONS`, la liste
/// de leurs définitions (impossible d'en oublier une).
///
/// ```
/// mod mes_actions {
///     use ent_kit::actions::Portee;
///     ent_kit::declarer_actions! {
///         PASSAGES_LIRE = "infirmerie.passages.lire", "Lire les passages", Portee::TOUTES, sensible;
///         PASSAGES_SAISIR = "infirmerie.passages.saisir", "Saisir un passage", Portee::ETABLISSEMENT_SEULE;
///     }
/// }
/// assert_eq!(mes_actions::DEFINITIONS.len(), 2);
/// assert!(mes_actions::DEFINITIONS[0].sensible);
/// ```
#[macro_export]
macro_rules! declarer_actions {
    ( $( $(#[$m:meta])* $nom:ident = $code:literal, $libelle:literal, $portees:expr $(, $drapeau:ident)* ; )* ) => {
        $( $(#[$m])* pub const $nom: $crate::actions::Action = $crate::actions::Action::new($code); )*
        pub const DEFINITIONS: &[$crate::actions::ActionDef] = &[
            $( $crate::actions::ActionDef::new($nom, $libelle, $portees) $( .$drapeau() )* ),*
        ];
    };
}

/// Modèles de rôle qui peuvent recevoir une action d'export (D-022 : direction
/// et secrétariat ; codes de `Role::as_str` aujourd'hui).
pub const MODELES_EXPORT_AUTORISES: &[&str] = &["admin", "direction", "administratif"];

/// Contribution d'une brique à un modèle de rôle par défaut (D-017) : la brique
/// dit elle-même quels modèles reçoivent ses actions, au lieu d'une grande
/// fonction `modele_par_defaut` centrale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContributionModele {
    pub modele: &'static str,
    pub action: Action,
    pub portee: Portee,
}

/// Ce qu'une brique déclare pour le catalogue.
#[derive(Debug, Clone, Copy)]
pub struct DeclActions {
    pub brique: &'static str,
    /// Domaines (préfixes de code) possédés par la brique.
    pub domaines: &'static [&'static str],
    pub definitions: &'static [ActionDef],
    pub modeles: &'static [ContributionModele],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurCatalogue {
    CodeInvalide(&'static str),
    CodeEnDouble {
        code: &'static str,
        a: &'static str,
        b: &'static str,
    },
    DomaineNonPossede {
        brique: &'static str,
        code: &'static str,
    },
    DomaineRevendiqueDeuxFois {
        domaine: &'static str,
        a: &'static str,
        b: &'static str,
    },
    SansPortee(&'static str),
    ModeleSurActionInconnue {
        brique: &'static str,
        code: &'static str,
    },
    ModelePorteeNonAdmise {
        modele: &'static str,
        code: &'static str,
    },
    ExportHorsAdministratifs {
        modele: &'static str,
        code: &'static str,
    },
}

impl fmt::Display for ErreurCatalogue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Catalogue assemblé, figé au démarrage.
#[derive(Debug, Clone)]
pub struct Catalogue {
    par_code: BTreeMap<&'static str, ActionDef>,
    modeles: Vec<ContributionModele>,
}

/// Forme d'un code : `domaine.verbe[.complément]`, minuscules, chiffres et `_`.
pub fn code_valide(code: &str) -> bool {
    let morceaux: Vec<&str> = code.split('.').collect();
    morceaux.len() >= 2
        && morceaux.iter().all(|m| {
            !m.is_empty()
                && m.bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        })
}

impl Catalogue {
    pub fn assembler(decls: &[DeclActions]) -> Result<Self, Vec<ErreurCatalogue>> {
        let mut erreurs = Vec::new();
        let mut proprietaire: BTreeMap<&'static str, &'static str> = BTreeMap::new();
        for d in decls {
            for dom in d.domaines {
                if let Some(autre) = proprietaire.insert(dom, d.brique) {
                    erreurs.push(ErreurCatalogue::DomaineRevendiqueDeuxFois {
                        domaine: dom,
                        a: autre,
                        b: d.brique,
                    });
                }
            }
        }
        let mut par_code: BTreeMap<&'static str, ActionDef> = BTreeMap::new();
        let mut origine: BTreeMap<&'static str, &'static str> = BTreeMap::new();
        for d in decls {
            for def in d.definitions {
                let code = def.action.code();
                if !code_valide(code) {
                    erreurs.push(ErreurCatalogue::CodeInvalide(code));
                }
                if def.portees.is_empty() {
                    erreurs.push(ErreurCatalogue::SansPortee(code));
                }
                if proprietaire.get(def.action.domaine()) != Some(&d.brique) {
                    erreurs.push(ErreurCatalogue::DomaineNonPossede {
                        brique: d.brique,
                        code,
                    });
                }
                if let Some(autre) = origine.insert(code, d.brique) {
                    erreurs.push(ErreurCatalogue::CodeEnDouble {
                        code,
                        a: autre,
                        b: d.brique,
                    });
                }
                par_code.insert(code, *def);
            }
        }
        let mut modeles = Vec::new();
        for d in decls {
            for c in d.modeles {
                let code = c.action.code();
                match par_code.get(code) {
                    None => erreurs.push(ErreurCatalogue::ModeleSurActionInconnue {
                        brique: d.brique,
                        code,
                    }),
                    Some(def) if !def.portees.contains(&c.portee) => {
                        erreurs.push(ErreurCatalogue::ModelePorteeNonAdmise {
                            modele: c.modele,
                            code,
                        })
                    }
                    Some(def) if def.export && !MODELES_EXPORT_AUTORISES.contains(&c.modele) => {
                        erreurs.push(ErreurCatalogue::ExportHorsAdministratifs {
                            modele: c.modele,
                            code,
                        })
                    }
                    Some(_) => modeles.push(*c),
                }
            }
        }
        if erreurs.is_empty() {
            Ok(Self { par_code, modeles })
        } else {
            Err(erreurs)
        }
    }

    /// Relit un code stocké en base ; un code inconnu n'élargit jamais rien.
    pub fn parse(&self, code: &str) -> Option<Action> {
        self.par_code.get(code).map(|d| d.action)
    }
    pub fn definition(&self, a: Action) -> Option<&ActionDef> {
        self.par_code.get(a.code())
    }
    pub fn toutes(&self) -> impl Iterator<Item = &ActionDef> {
        self.par_code.values()
    }
    pub fn len(&self) -> usize {
        self.par_code.len()
    }
    pub fn is_empty(&self) -> bool {
        self.par_code.is_empty()
    }
    /// Droits d'un modèle par défaut : union des contributions des briques.
    pub fn modele(&self, modele: &str) -> Vec<(Action, Portee)> {
        let mut v: Vec<_> = self
            .modeles
            .iter()
            .filter(|c| c.modele == modele)
            .map(|c| (c.action, c.portee))
            .collect();
        v.sort();
        v.dedup();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ce que la brique « infirmerie » écrirait dans son crate.
    mod infirmerie {
        use super::Portee;
        crate::declarer_actions! {
            /// Lire les passages d'un élève.
            PASSAGES_LIRE = "infirmerie.passages.lire", "Lire les passages à l'infirmerie", Portee::TOUTES, sensible;
            PASSAGES_SAISIR = "infirmerie.passages.saisir", "Saisir un passage", Portee::ETABLISSEMENT_SEULE, sensible;
            REGISTRE_EXPORTER = "infirmerie.registre.exporter", "Exporter le registre", Portee::ETABLISSEMENT_SEULE, sensible, export;
        }
    }
    use infirmerie::{PASSAGES_LIRE, PASSAGES_SAISIR, REGISTRE_EXPORTER};

    // Bascule de l'existant : les noms de l'ancien `enum` deviennent des
    // constantes associées (bloc réservé à l'historique, dans ce crate).
    #[allow(non_upper_case_globals)]
    impl Action {
        pub const NotesLire: Action = Action::new("notes.lire");
    }

    fn decl_infirmerie(modeles: &'static [ContributionModele]) -> DeclActions {
        DeclActions {
            brique: "infirmerie",
            domaines: &["infirmerie"],
            definitions: infirmerie::DEFINITIONS,
            modeles,
        }
    }

    #[test]
    fn une_constante_sert_de_motif_de_match() {
        fn libelle_court(a: Action) -> &'static str {
            match a {
                PASSAGES_LIRE => "lecture",
                PASSAGES_SAISIR => "saisie",
                Action::NotesLire => "notes",
                _ => "autre",
            }
        }
        assert_eq!(libelle_court(PASSAGES_SAISIR), "saisie");
        assert_eq!(libelle_court(Action::NotesLire), "notes");
        assert_eq!(
            libelle_court(Action::new("notes.lire")),
            "notes",
            "égalité par code seul"
        );
        assert_eq!(PASSAGES_LIRE.domaine(), "infirmerie");
    }

    #[test]
    fn la_macro_produit_constantes_et_definitions() {
        assert_eq!(infirmerie::DEFINITIONS.len(), 3);
        assert_eq!(infirmerie::DEFINITIONS[0].action, PASSAGES_LIRE);
        assert!(infirmerie::DEFINITIONS.iter().all(|d| d.sensible));
        assert!(infirmerie::DEFINITIONS[2].export && !infirmerie::DEFINITIONS[1].export);
    }

    #[test]
    fn catalogue_assemble_et_modele_par_contributions() {
        const NOTES: &[ActionDef] = &[ActionDef::new(
            Action::NotesLire,
            "Lire les notes",
            Portee::TOUTES,
        )];
        let notes = DeclActions {
            brique: "evaluations",
            domaines: &["notes"],
            definitions: NOTES,
            modeles: &[ContributionModele {
                modele: "prof",
                action: Action::NotesLire,
                portee: Portee::MesClasses,
            }],
        };
        let infi = decl_infirmerie(&[
            ContributionModele {
                modele: "infirmerie",
                action: PASSAGES_LIRE,
                portee: Portee::Etablissement,
            },
            ContributionModele {
                modele: "infirmerie",
                action: PASSAGES_SAISIR,
                portee: Portee::Etablissement,
            },
            ContributionModele {
                modele: "direction",
                action: REGISTRE_EXPORTER,
                portee: Portee::Etablissement,
            },
        ]);
        let cat = Catalogue::assembler(&[infi, notes]).expect("catalogue valide");
        assert_eq!(cat.len(), 4);
        assert_eq!(cat.parse("infirmerie.passages.lire"), Some(PASSAGES_LIRE));
        assert_eq!(cat.parse("infirmerie.inconnu"), None);
        assert_eq!(
            cat.definition(PASSAGES_SAISIR).map(|d| d.libelle),
            Some("Saisir un passage")
        );
        assert_eq!(cat.modele("infirmerie").len(), 2);
        assert_eq!(
            cat.modele("prof"),
            [(Action::NotesLire, Portee::MesClasses)]
        );
    }

    #[test]
    fn refuse_les_declarations_fautives() {
        const FAUTIVES: &[ActionDef] = &[
            ActionDef::new(Action::new("sante.lire"), "x", Portee::TOUTES),
            ActionDef::new(Action::new("Infirmerie Lire"), "x", Portee::TOUTES),
            ActionDef::new(Action::new("infirmerie.vide"), "x", &[]),
            ActionDef::new(Action::new("infirmerie.double"), "x", Portee::TOUTES),
            ActionDef::new(Action::new("infirmerie.double"), "y", Portee::TOUTES),
        ];
        let fautive = DeclActions {
            brique: "infirmerie",
            domaines: &["infirmerie"],
            definitions: FAUTIVES,
            modeles: &[ContributionModele {
                modele: "prof",
                action: PASSAGES_SAISIR,
                portee: Portee::MesClasses,
            }],
        };
        let e = Catalogue::assembler(&[fautive]).unwrap_err();
        assert!(e.contains(&ErreurCatalogue::DomaineNonPossede {
            brique: "infirmerie",
            code: "sante.lire"
        }));
        assert!(e.contains(&ErreurCatalogue::CodeInvalide("Infirmerie Lire")));
        assert!(e.contains(&ErreurCatalogue::SansPortee("infirmerie.vide")));
        assert!(e.contains(&ErreurCatalogue::CodeEnDouble {
            code: "infirmerie.double",
            a: "infirmerie",
            b: "infirmerie"
        }));
        assert!(e.contains(&ErreurCatalogue::ModeleSurActionInconnue {
            brique: "infirmerie",
            code: "infirmerie.passages.saisir"
        }));

        let e = Catalogue::assembler(&[decl_infirmerie(&[]), decl_infirmerie(&[])]).unwrap_err();
        assert!(e
            .iter()
            .any(|x| matches!(x, ErreurCatalogue::DomaineRevendiqueDeuxFois { .. })));
    }

    #[test]
    fn un_modele_ne_peut_pas_elargir_la_portee() {
        let d = decl_infirmerie(&[ContributionModele {
            modele: "infirmerie",
            action: PASSAGES_SAISIR,
            portee: Portee::MesGroupes,
        }]);
        let e = Catalogue::assembler(&[d]).unwrap_err();
        assert_eq!(
            e,
            [ErreurCatalogue::ModelePorteeNonAdmise {
                modele: "infirmerie",
                code: "infirmerie.passages.saisir"
            }]
        );
    }

    #[test]
    fn un_professeur_ne_recoit_jamais_un_export() {
        // D-022 : contrôlé par l'assemblage, pas laissé au test de chaque brique.
        let d = decl_infirmerie(&[ContributionModele {
            modele: "prof",
            action: REGISTRE_EXPORTER,
            portee: Portee::Etablissement,
        }]);
        let e = Catalogue::assembler(&[d]).unwrap_err();
        assert_eq!(
            e,
            [ErreurCatalogue::ExportHorsAdministratifs {
                modele: "prof",
                code: "infirmerie.registre.exporter"
            }]
        );
    }
}
