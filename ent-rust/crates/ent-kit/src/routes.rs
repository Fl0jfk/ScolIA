//! Déclaration des routes d'une brique, et cas d'isolation qui en découlent.
//!
//! Aujourd'hui l'inventaire de `ent-api/tests/separation` relit le texte des
//! fichiers source (`routes_src.rs`) et ignore silencieusement un chemin non
//! littéral. Ici la déclaration est une donnée : le routeur axum sera construit
//! à partir d'elle (étape 3 du plan) et la matrice d'isolation en est déduite.

use crate::actions::Action;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Methode {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Qui peut appeler la route (contrôle refait par le service, jamais seulement ici).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acces {
    /// Sans session ; le motif est obligatoire (revue de sécurité).
    Public { motif: &'static str },
    /// Personnel : au moins une des actions, avec sa portée.
    Personnel { actions: &'static [Action] },
    /// Famille ou élève : lien actif avec l'élève ciblé (droits du lien, socle 4).
    Famille,
    /// Personnel ou famille selon les droits ; le service tranche.
    Mixte { actions: &'static [Action] },
}

/// Ressource visée, pour générer les cas d'isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cible {
    /// Pas d'objet désigné dans le chemin (listes bornées par la portée).
    Aucune,
    /// Un élève désigné par un paramètre du chemin.
    Eleve { parametre: &'static str },
    /// Un objet de la brique (identifiant du chemin), rattaché à un élève.
    Objet { parametre: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteDecl {
    pub methode: Methode,
    pub chemin: &'static str,
    pub acces: Acces,
    pub cible: Cible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurRoute {
    HorsPrefixe {
        brique: &'static str,
        chemin: &'static str,
    },
    EnDouble {
        methode: Methode,
        chemin: &'static str,
    },
    PublicSansMotif(&'static str),
    SansAction(&'static str),
    ParametreAbsent {
        chemin: &'static str,
        parametre: &'static str,
    },
}

impl fmt::Display for ErreurRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Contrôle les routes d'une brique : chemins sous ses préfixes, pas de
/// doublon, accès explicite, paramètre de cible présent dans le chemin.
pub fn verifier(brique: &'static str, prefixes: &[&str], routes: &[RouteDecl]) -> Vec<ErreurRoute> {
    let mut erreurs = Vec::new();
    let mut vues = BTreeSet::new();
    for r in routes {
        if !prefixes
            .iter()
            .any(|p| r.chemin == *p || r.chemin.starts_with(&format!("{p}/")))
        {
            erreurs.push(ErreurRoute::HorsPrefixe {
                brique,
                chemin: r.chemin,
            });
        }
        if !vues.insert((r.methode, r.chemin)) {
            erreurs.push(ErreurRoute::EnDouble {
                methode: r.methode,
                chemin: r.chemin,
            });
        }
        match r.acces {
            Acces::Public { motif } if motif.trim().is_empty() => {
                erreurs.push(ErreurRoute::PublicSansMotif(r.chemin))
            }
            Acces::Personnel { actions } | Acces::Mixte { actions } if actions.is_empty() => {
                erreurs.push(ErreurRoute::SansAction(r.chemin))
            }
            _ => {}
        }
        if let Cible::Eleve { parametre } | Cible::Objet { parametre } = r.cible {
            if !r.chemin.contains(&format!("{{{parametre}}}")) {
                erreurs.push(ErreurRoute::ParametreAbsent {
                    chemin: r.chemin,
                    parametre,
                });
            }
        }
    }
    erreurs
}

/// Cas d'isolation générés pour une route (joués par le harnais HTTP de
/// l'étape 4, qui fournit les acteurs et les identifiants du labo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasIsolation {
    /// Sans jeton ou jeton inventé → 401.
    SansSession,
    /// Personnel sans aucune des actions → refus (403/404), aucune donnée.
    PersonnelSansAction(Action),
    /// Action accordée avec une portée qui n'inclut pas l'élève → refus.
    HorsPortee(Action),
    /// Famille d'un autre élève → refus ou réponse indistincte.
    FamilleNonLiee,
    /// Identifiant d'un autre établissement avec un jeton de A → indistinct.
    AutreEtablissement,
}

pub fn cas_isolation(r: &RouteDecl) -> Vec<CasIsolation> {
    let mut cas = Vec::new();
    let actions = match r.acces {
        Acces::Public { .. } => return cas,
        Acces::Personnel { actions } | Acces::Mixte { actions } => actions,
        Acces::Famille => &[],
    };
    cas.push(CasIsolation::SansSession);
    for a in actions {
        cas.push(CasIsolation::PersonnelSansAction(*a));
        if r.cible != Cible::Aucune {
            cas.push(CasIsolation::HorsPortee(*a));
        }
    }
    if r.cible != Cible::Aucune {
        if matches!(r.acces, Acces::Famille | Acces::Mixte { .. }) {
            cas.push(CasIsolation::FamilleNonLiee);
        }
        cas.push(CasIsolation::AutreEtablissement);
    }
    cas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, Portee};

    const LIRE: Action = Action::declarer("infirmerie.passages.lire", "Lire", Portee::TOUTES);

    #[test]
    fn cas_generes_pour_une_route_eleve() {
        let r = RouteDecl {
            methode: Methode::Get,
            chemin: "/infirmerie/eleves/{eleve_id}/passages",
            acces: Acces::Personnel { actions: &[LIRE] },
            cible: Cible::Eleve {
                parametre: "eleve_id",
            },
        };
        assert!(verifier("infirmerie", &["/infirmerie"], &[r]).is_empty());
        assert_eq!(
            cas_isolation(&r),
            [
                CasIsolation::SansSession,
                CasIsolation::PersonnelSansAction(LIRE),
                CasIsolation::HorsPortee(LIRE),
                CasIsolation::AutreEtablissement
            ]
        );
    }

    #[test]
    fn refuse_les_routes_mal_declarees() {
        let routes = [
            RouteDecl {
                methode: Methode::Get,
                chemin: "/sante/x",
                acces: Acces::Famille,
                cible: Cible::Aucune,
            },
            RouteDecl {
                methode: Methode::Get,
                chemin: "/infirmerie/p",
                acces: Acces::Public { motif: " " },
                cible: Cible::Aucune,
            },
            RouteDecl {
                methode: Methode::Get,
                chemin: "/infirmerie/p",
                acces: Acces::Personnel { actions: &[] },
                cible: Cible::Aucune,
            },
            RouteDecl {
                methode: Methode::Post,
                chemin: "/infirmerie/eleves/{id}",
                acces: Acces::Famille,
                cible: Cible::Eleve {
                    parametre: "eleve_id",
                },
            },
        ];
        let e = verifier("infirmerie", &["/infirmerie"], &routes);
        assert_eq!(e.len(), 5, "{e:?}");
        assert!(e.contains(&ErreurRoute::HorsPrefixe {
            brique: "infirmerie",
            chemin: "/sante/x"
        }));
        assert!(e.contains(&ErreurRoute::EnDouble {
            methode: Methode::Get,
            chemin: "/infirmerie/p"
        }));
        // « /infirmerie-bis » n'est pas sous « /infirmerie ».
        let bis = [RouteDecl {
            methode: Methode::Get,
            chemin: "/infirmerie-bis",
            acces: Acces::Famille,
            cible: Cible::Aucune,
        }];
        assert_eq!(verifier("infirmerie", &["/infirmerie"], &bis).len(), 1);
    }
}
