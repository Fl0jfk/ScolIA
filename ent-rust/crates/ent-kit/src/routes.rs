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
        // `/x/{a}` et `/x/{b}` sont la même route pour axum (panique au démarrage).
        if !vues.insert((r.methode, forme(r.chemin))) {
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

/// Forme d'un chemin sans le nom de ses paramètres (`/x/{a}` → `/x/{}`).
fn forme(chemin: &str) -> String {
    let mut sortie = String::with_capacity(chemin.len());
    let mut dans_parametre = false;
    for c in chemin.chars() {
        match c {
            '{' => {
                dans_parametre = true;
                sortie.push_str("{}");
            }
            '}' => dans_parametre = false,
            _ if dans_parametre => {}
            _ => sortie.push(c),
        }
    }
    sortie
}

/// Cas d'isolation générés pour une route (joués par le harnais HTTP de
/// l'étape 4, qui fournit les acteurs et les identifiants du labo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasIsolation {
    /// Sans jeton ou jeton inventé → 401.
    SansSession,
    /// Personnel qui n'a **aucune** des actions de la route → refus (403/404),
    /// aucune donnée (sémantique « au moins une des actions »).
    PersonnelSansAucuneAction,
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
    if !actions.is_empty() {
        cas.push(CasIsolation::PersonnelSansAucuneAction);
    }
    for a in actions {
        if r.cible != Cible::Aucune {
            cas.push(CasIsolation::HorsPortee(*a));
        }
    }
    // Une liste famille (sans cible) doit aussi rester bornée au lien : le cas
    // « famille non liée » vérifie qu'aucun élève d'une autre famille n'y paraît.
    if matches!(r.acces, Acces::Famille | Acces::Mixte { .. }) {
        cas.push(CasIsolation::FamilleNonLiee);
    }
    if r.cible != Cible::Aucune {
        cas.push(CasIsolation::AutreEtablissement);
    }
    cas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;

    const LIRE: Action = Action::new("infirmerie.passages.lire");

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
                CasIsolation::PersonnelSansAucuneAction,
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
        // Même route aux noms de paramètres près.
        let jumelles = [
            RouteDecl {
                methode: Methode::Get,
                chemin: "/infirmerie/{a}",
                acces: Acces::Famille,
                cible: Cible::Aucune,
            },
            RouteDecl {
                methode: Methode::Get,
                chemin: "/infirmerie/{b}",
                acces: Acces::Famille,
                cible: Cible::Aucune,
            },
        ];
        assert_eq!(verifier("infirmerie", &["/infirmerie"], &jumelles).len(), 1);
        assert_eq!(
            cas_isolation(&jumelles[0]),
            [CasIsolation::SansSession, CasIsolation::FamilleNonLiee]
        );
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
