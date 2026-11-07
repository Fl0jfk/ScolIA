//! Déclaration complète d'une brique et vérification de l'assemblage.
//!
//! Une brique est une **valeur** (`pub const BRIQUE: Brique`) exportée par
//! son crate. L'assemblage (crate binaire) liste les briques ; ce module
//! contrôle qu'elles ne se marchent pas dessus : noms, plages de migrations,
//! domaines d'actions, préfixes de routes, d'événements et de consommateurs.
//! La partie exécutable (routeur axum, consommateurs, tâches) sera portée par
//! un trait du kit réel ; le prototype se limite aux déclarations.

use crate::actions::{Catalogue, DeclActions, ErreurCatalogue};
use crate::migrations::{assembler, DeclMigrations, ErreurMigration, Migration};
use crate::routes::{verifier, Acces, ErreurRoute, Methode, RouteDecl};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct Brique {
    /// Nom stable, en minuscules (`infirmerie`).
    pub nom: &'static str,
    pub migrations: DeclMigrations,
    pub actions: DeclActions,
    pub prefixes_routes: &'static [&'static str],
    pub routes: &'static [RouteDecl],
    /// Types d'événements publiés, tous préfixés par le nom de la brique
    /// (`infirmerie.passage_enregistre`) sauf les noms historiques gelés.
    pub evenements: &'static [&'static str],
    /// Noms stables des consommateurs d'outbox (clé d'idempotence en base).
    pub consommateurs: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurAssemblage {
    NomEnDouble(&'static str),
    Migration(ErreurMigration),
    Catalogue(ErreurCatalogue),
    Route(ErreurRoute),
    DeclarationsIncoherentes(&'static str),
    PrefixeRouteEnDouble {
        prefixe: &'static str,
        a: &'static str,
        b: &'static str,
    },
    EvenementHorsPrefixe {
        brique: &'static str,
        evenement: &'static str,
    },
    ConsommateurEnDouble(&'static str),
    /// Une brique ne se déclare pas publique toute seule : la liste des routes
    /// publiques est tenue par l'assemblage et relue en revue de sécurité.
    RoutePubliqueNonAutorisee {
        brique: &'static str,
        chemin: &'static str,
    },
}

/// Résultat d'un assemblage valide : ce que le binaire applique au démarrage.
#[derive(Debug)]
pub struct Assemblage {
    pub migrations: Vec<Migration>,
    pub migration_tag: Option<&'static str>,
    pub catalogue: Catalogue,
    pub routes: Vec<(&'static str, RouteDecl)>,
}

/// Noms d'événements antérieurs au kit, conservés tels quels (ne jamais
/// renommer un type publié : les consommateurs et l'historique en dépendent).
pub fn evenement_admis(brique: &str, evenement: &str, historiques: &[&str]) -> bool {
    historiques.contains(&evenement) || evenement.starts_with(&format!("{brique}."))
}

pub fn assembler_briques(
    briques: &[Brique],
    evenements_historiques: &[&str],
    publiques_autorisees: &[(Methode, &str)],
) -> Result<Assemblage, Vec<ErreurAssemblage>> {
    let mut erreurs = Vec::new();
    let mut noms = BTreeMap::new();
    let mut prefixes: BTreeMap<&'static str, &'static str> = BTreeMap::new();
    let mut consommateurs = BTreeMap::new();
    let mut routes = Vec::new();
    for b in briques {
        if noms.insert(b.nom, ()).is_some() {
            erreurs.push(ErreurAssemblage::NomEnDouble(b.nom));
        }
        if b.migrations.brique != b.nom || b.actions.brique != b.nom {
            erreurs.push(ErreurAssemblage::DeclarationsIncoherentes(b.nom));
        }
        for p in b.prefixes_routes {
            if let Some(autre) = prefixes.insert(p, b.nom) {
                erreurs.push(ErreurAssemblage::PrefixeRouteEnDouble {
                    prefixe: p,
                    a: autre,
                    b: b.nom,
                });
            }
        }
        erreurs.extend(
            verifier(b.nom, b.prefixes_routes, b.routes)
                .into_iter()
                .map(ErreurAssemblage::Route),
        );
        for r in b.routes {
            if matches!(r.acces, Acces::Public { .. })
                && !publiques_autorisees.contains(&(r.methode, r.chemin))
            {
                erreurs.push(ErreurAssemblage::RoutePubliqueNonAutorisee {
                    brique: b.nom,
                    chemin: r.chemin,
                });
            }
        }
        routes.extend(b.routes.iter().map(|r| (b.nom, *r)));
        for e in b.evenements {
            if !evenement_admis(b.nom, e, evenements_historiques) {
                erreurs.push(ErreurAssemblage::EvenementHorsPrefixe {
                    brique: b.nom,
                    evenement: e,
                });
            }
        }
        for c in b.consommateurs {
            if consommateurs.insert(*c, ()).is_some() {
                erreurs.push(ErreurAssemblage::ConsommateurEnDouble(c));
            }
        }
    }
    let decl_m: Vec<_> = briques.iter().map(|b| b.migrations).collect();
    let migrations = assembler(&decl_m)
        .map_err(|e| erreurs.extend(e.into_iter().map(ErreurAssemblage::Migration)));
    let decl_a: Vec<_> = briques.iter().map(|b| b.actions).collect();
    let catalogue = Catalogue::assembler(&decl_a)
        .map_err(|e| erreurs.extend(e.into_iter().map(ErreurAssemblage::Catalogue)));
    match (migrations, catalogue) {
        (Ok(migrations), Ok(catalogue)) if erreurs.is_empty() => {
            let migration_tag = crate::migrations::tag(&migrations);
            Ok(Assemblage {
                migrations,
                migration_tag,
                catalogue,
                routes,
            })
        }
        _ => Err(erreurs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{ContributionModele, Portee};
    use crate::migrations::{Migration, Plage};
    use crate::routes::Cible;

    // ---- La brique « infirmerie » telle que l'écrirait une session Sonnet ----
    mod actions_infirmerie {
        use crate::actions::Portee;
        crate::declarer_actions! {
            PASSAGES_LIRE = "infirmerie.passages.lire", "Lire les passages", Portee::TOUTES, sensible;
            PASSAGES_SAISIR = "infirmerie.passages.saisir", "Saisir un passage", Portee::ETABLISSEMENT_SEULE, sensible;
        }
    }
    use actions_infirmerie::{PASSAGES_LIRE, PASSAGES_SAISIR};

    const INFIRMERIE: Brique = Brique {
        nom: "infirmerie",
        migrations: DeclMigrations {
            brique: "infirmerie",
            plages: const { &[Plage::new(400, 409)] },
            migrations: &[Migration {
                numero: 400,
                nom: "400_infirmerie_passages",
                sql: "",
                brique: "infirmerie",
            }],
        },
        actions: DeclActions {
            brique: "infirmerie",
            domaines: &["infirmerie"],
            definitions: actions_infirmerie::DEFINITIONS,
            modeles: &[ContributionModele {
                modele: "infirmerie",
                action: PASSAGES_SAISIR,
                portee: Portee::Etablissement,
            }],
        },
        prefixes_routes: &["/infirmerie"],
        routes: &[
            RouteDecl {
                methode: Methode::Get,
                chemin: "/infirmerie/eleves/{eleve_id}/passages",
                acces: Acces::Personnel {
                    actions: &[PASSAGES_LIRE],
                },
                cible: Cible::Eleve {
                    parametre: "eleve_id",
                },
            },
            RouteDecl {
                methode: Methode::Post,
                chemin: "/infirmerie/passages",
                acces: Acces::Personnel {
                    actions: &[PASSAGES_SAISIR],
                },
                cible: Cible::Aucune,
            },
        ],
        evenements: &["infirmerie.passage_enregistre"],
        consommateurs: &["infirmerie.alerte_famille"],
    };

    const fn noyau_minimal() -> Brique {
        Brique {
            nom: "noyau",
            migrations: DeclMigrations {
                brique: "noyau",
                plages: const { &[Plage::new(1, 369)] },
                migrations: &[],
            },
            actions: DeclActions {
                brique: "noyau",
                domaines: &["eleves"],
                definitions: &[],
                modeles: &[],
            },
            prefixes_routes: &["/eleves"],
            routes: &[],
            evenements: &["eleve.created"],
            consommateurs: &["d0.notifications"],
        }
    }

    #[test]
    fn une_brique_se_branche_sans_toucher_au_noyau() {
        let a = assembler_briques(&[noyau_minimal(), INFIRMERIE], &["eleve.created"], &[])
            .expect("assemblage valide");
        assert_eq!(a.migration_tag, Some("400_infirmerie_passages"));
        assert_eq!(a.catalogue.len(), 2);
        assert_eq!(a.routes.len(), 2);
        assert_eq!(
            a.catalogue.modele("infirmerie"),
            [(PASSAGES_SAISIR, Portee::Etablissement)]
        );
    }

    #[test]
    fn deux_briques_qui_se_marchent_dessus_sont_refusees() {
        let mut copie = INFIRMERIE;
        copie.nom = "infirmerie2";
        copie.migrations.brique = "infirmerie2";
        copie.actions.brique = "infirmerie2";
        let e = assembler_briques(&[INFIRMERIE, copie], &[], &[]).unwrap_err();
        assert!(e.contains(&ErreurAssemblage::PrefixeRouteEnDouble {
            prefixe: "/infirmerie",
            a: "infirmerie",
            b: "infirmerie2"
        }));
        assert!(e.contains(&ErreurAssemblage::ConsommateurEnDouble(
            "infirmerie.alerte_famille"
        )));
        assert!(e.contains(&ErreurAssemblage::EvenementHorsPrefixe {
            brique: "infirmerie2",
            evenement: "infirmerie.passage_enregistre"
        }));
        assert!(e.iter().any(|x| matches!(
            x,
            ErreurAssemblage::Migration(ErreurMigration::PlagesChevauchees { .. })
        )));
        assert!(e.iter().any(|x| matches!(
            x,
            ErreurAssemblage::Catalogue(ErreurCatalogue::DomaineRevendiqueDeuxFois { .. })
        )));
    }

    #[test]
    fn une_route_publique_doit_etre_autorisee_par_l_assemblage() {
        const OUVERTE: RouteDecl = RouteDecl {
            methode: Methode::Get,
            chemin: "/infirmerie/horaires",
            acces: Acces::Public {
                motif: "horaires d'ouverture affichés à l'accueil",
            },
            cible: Cible::Aucune,
        };
        let mut b = INFIRMERIE;
        b.routes = &[OUVERTE];
        let e = assembler_briques(&[b], &[], &[]).unwrap_err();
        assert_eq!(
            e,
            [ErreurAssemblage::RoutePubliqueNonAutorisee {
                brique: "infirmerie",
                chemin: "/infirmerie/horaires"
            }]
        );
        assert!(assembler_briques(&[b], &[], &[(Methode::Get, "/infirmerie/horaires")]).is_ok());
    }
}
