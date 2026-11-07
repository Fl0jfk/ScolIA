//! Actions du catalogue D-017 déclarées par chaque brique.
//!
//! Aujourd'hui `ent_auth::Action` est un `enum` fermé : chaque lot y ajoute
//! ses variantes, ce qui fait de `actions.rs` un point de collision. Ici une
//! action est une **valeur constante** : une brique déclare les siennes dans
//! son crate, le catalogue est l'union vérifiée des déclarations.
//!
//! Point vérifié par les tests : une constante d'un type à `PartialEq, Eq`
//! dérivés est utilisable comme motif de `match`. La bascule de l'`enum` vers
//! ce type peut donc garder les noms existants (`Action::NotesLire` devient une
//! constante associée) sans toucher les 463 usages.

use std::collections::BTreeMap;
use std::fmt;

/// Portée d'un droit accordé (D-017, D-022).
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

/// Action contrôlée par le serveur. Le `code` est l'identifiant stable écrit
/// en base (`ent_role_droit.action`) : ne jamais le renommer sans migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Action {
    code: &'static str,
    libelle: &'static str,
    portees: &'static [Portee],
    sensible: bool,
}

impl Action {
    /// Déclare une action (utilisable dans une `const`).
    pub const fn declarer(
        code: &'static str,
        libelle: &'static str,
        portees: &'static [Portee],
    ) -> Self {
        Self {
            code,
            libelle,
            portees,
            sensible: false,
        }
    }
    /// Marque une action donnant accès à des données sensibles (santé, psy…) :
    /// consultation tracée, jamais servie à l'IA sans droit dédié (IA_COEUR §5.5).
    pub const fn sensible(self) -> Self {
        Self {
            sensible: true,
            ..self
        }
    }
    pub const fn code(&self) -> &'static str {
        self.code
    }
    pub const fn libelle(&self) -> &'static str {
        self.libelle
    }
    pub const fn portees_admises(&self) -> &'static [Portee] {
        self.portees
    }
    pub const fn est_sensible(&self) -> bool {
        self.sensible
    }
    /// Domaine : ce qui précède le premier point (`sante` pour `sante.lire`).
    pub fn domaine(&self) -> &'static str {
        self.code.split('.').next().unwrap_or(self.code)
    }
}

/// Contribution d'une brique à un modèle de rôle par défaut (D-017) : la brique
/// dit elle-même quels modèles reçoivent ses actions, au lieu d'une grande
/// fonction `modele_par_defaut` centrale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContributionModele {
    /// Code du modèle (`prof`, `cpe`, `infirmerie`…), aujourd'hui `Role::as_str`.
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
    pub actions: &'static [Action],
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
}

impl fmt::Display for ErreurCatalogue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Catalogue assemblé, figé au démarrage.
#[derive(Debug, Clone)]
pub struct Catalogue {
    par_code: BTreeMap<&'static str, Action>,
    modeles: Vec<ContributionModele>,
}

/// Forme d'un code : `domaine.verbe[.complément]`, minuscules et `_`.
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
        let mut par_code: BTreeMap<&'static str, Action> = BTreeMap::new();
        let mut origine: BTreeMap<&'static str, &'static str> = BTreeMap::new();
        for d in decls {
            for a in d.actions {
                if !code_valide(a.code) {
                    erreurs.push(ErreurCatalogue::CodeInvalide(a.code));
                }
                if a.portees.is_empty() {
                    erreurs.push(ErreurCatalogue::SansPortee(a.code));
                }
                if proprietaire.get(a.domaine()) != Some(&d.brique) {
                    erreurs.push(ErreurCatalogue::DomaineNonPossede {
                        brique: d.brique,
                        code: a.code,
                    });
                }
                if let Some(autre) = origine.insert(a.code, d.brique) {
                    erreurs.push(ErreurCatalogue::CodeEnDouble {
                        code: a.code,
                        a: autre,
                        b: d.brique,
                    });
                }
                par_code.insert(a.code, *a);
            }
        }
        let mut modeles = Vec::new();
        for d in decls {
            for c in d.modeles {
                match par_code.get(c.action.code) {
                    None => erreurs.push(ErreurCatalogue::ModeleSurActionInconnue {
                        brique: d.brique,
                        code: c.action.code,
                    }),
                    Some(a) if !a.portees.contains(&c.portee) => {
                        erreurs.push(ErreurCatalogue::ModelePorteeNonAdmise {
                            modele: c.modele,
                            code: a.code,
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
        self.par_code.get(code).copied()
    }
    pub fn toutes(&self) -> impl Iterator<Item = &Action> {
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
    pub const PASSAGES_LIRE: Action = Action::declarer(
        "infirmerie.passages.lire",
        "Lire les passages à l'infirmerie",
        Portee::TOUTES,
    )
    .sensible();
    pub const PASSAGES_SAISIR: Action = Action::declarer(
        "infirmerie.passages.saisir",
        "Saisir un passage",
        Portee::ETABLISSEMENT_SEULE,
    )
    .sensible();

    /// Bascule sans changement des usages : les noms de l'ancien `enum`
    /// deviennent des constantes associées.
    struct Compat;
    #[allow(non_upper_case_globals)]
    impl Compat {
        const NotesLire: Action = Action::declarer("notes.lire", "Lire les notes", Portee::TOUTES);
    }

    fn infirmerie() -> DeclActions {
        DeclActions {
            brique: "infirmerie",
            domaines: &["infirmerie"],
            actions: &[PASSAGES_LIRE, PASSAGES_SAISIR],
            modeles: &[
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
            ],
        }
    }

    #[test]
    fn une_constante_sert_de_motif_de_match() {
        fn libelle_court(a: Action) -> &'static str {
            match a {
                PASSAGES_LIRE => "lecture",
                PASSAGES_SAISIR => "saisie",
                Compat::NotesLire => "notes",
                _ => "autre",
            }
        }
        assert_eq!(libelle_court(PASSAGES_SAISIR), "saisie");
        assert_eq!(libelle_court(Compat::NotesLire), "notes");
        assert!(PASSAGES_LIRE.est_sensible());
        assert_eq!(PASSAGES_LIRE.domaine(), "infirmerie");
    }

    #[test]
    fn catalogue_assemble_et_modele_par_contributions() {
        let notes = DeclActions {
            brique: "evaluations",
            domaines: &["notes"],
            actions: &[Compat::NotesLire],
            modeles: &[ContributionModele {
                modele: "prof",
                action: Compat::NotesLire,
                portee: Portee::MesClasses,
            }],
        };
        let cat = Catalogue::assembler(&[infirmerie(), notes]).expect("catalogue valide");
        assert_eq!(cat.len(), 3);
        assert_eq!(cat.parse("infirmerie.passages.lire"), Some(PASSAGES_LIRE));
        assert_eq!(cat.parse("infirmerie.inconnu"), None);
        assert_eq!(cat.modele("infirmerie").len(), 2);
        assert_eq!(
            cat.modele("prof"),
            [(Compat::NotesLire, Portee::MesClasses)]
        );
    }

    #[test]
    fn refuse_les_declarations_fautives() {
        const SANTE_DANS_INFIRMERIE: Action = Action::declarer("sante.lire", "x", Portee::TOUTES);
        const MAL_ECRIT: Action = Action::declarer("Infirmerie Lire", "x", Portee::TOUTES);
        const SANS_PORTEE: Action = Action::declarer("infirmerie.vide", "x", &[]);
        let fautive = DeclActions {
            brique: "infirmerie",
            domaines: &["infirmerie"],
            actions: &[
                SANTE_DANS_INFIRMERIE,
                MAL_ECRIT,
                SANS_PORTEE,
                PASSAGES_LIRE,
                PASSAGES_LIRE,
            ],
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
            code: "infirmerie.passages.lire",
            a: "infirmerie",
            b: "infirmerie"
        }));
        assert!(e.contains(&ErreurCatalogue::ModeleSurActionInconnue {
            brique: "infirmerie",
            code: "infirmerie.passages.saisir"
        }));

        let e = Catalogue::assembler(&[infirmerie(), infirmerie()]).unwrap_err();
        assert!(e
            .iter()
            .any(|x| matches!(x, ErreurCatalogue::DomaineRevendiqueDeuxFois { .. })));
    }

    #[test]
    fn un_modele_ne_peut_pas_elargir_la_portee() {
        let d = DeclActions {
            brique: "infirmerie",
            domaines: &["infirmerie"],
            actions: &[PASSAGES_SAISIR],
            modeles: &[ContributionModele {
                modele: "prof",
                action: PASSAGES_SAISIR,
                portee: Portee::MesGroupes,
            }],
        };
        let e = Catalogue::assembler(&[d]).unwrap_err();
        assert_eq!(
            e,
            [ErreurCatalogue::ModelePorteeNonAdmise {
                modele: "prof",
                code: "infirmerie.passages.saisir"
            }]
        );
    }
}
