//! Socle 29, lot P6 : le registre des capacités de la palette contre le catalogue réel D-017 et les modèles
//! de rôles par défaut (sans base : tests purs). POR-244, équivalence palette / écran / IA.

use ent_auth::{modele_par_defaut, Action, Role};
use ent_types::palette::{capacites_ouvertes_ia, capacites_permises, Confirmation, CAPACITES};
use std::collections::HashSet;

const ROLES: [Role; 12] = [
    Role::Admin,
    Role::Direction,
    Role::Administratif,
    Role::Cpe,
    Role::Compta,
    Role::Prof,
    Role::Infirmerie,
    Role::Psy,
    Role::Accueil,
    Role::Personnel,
    Role::Parent,
    Role::Eleve,
];

fn droits_du_role(r: Role) -> HashSet<&'static str> {
    modele_par_defaut(r)
        .into_iter()
        .map(|(a, _)| a.code())
        .collect()
}

/// Aucune capacité ne cite un droit absent du catalogue : une faute de frappe ouvrirait ou fermerait
/// la palette en silence.
#[test]
fn chaque_droit_d_une_capacite_existe_au_catalogue() {
    let catalogue: HashSet<&str> = Action::TOUTES.iter().map(|a| a.code()).collect();
    for c in CAPACITES {
        assert!(!c.droits.is_empty(), "{} sans droit", c.id);
        for d in c.droits {
            assert!(catalogue.contains(d), "{} : droit inconnu {d}", c.id);
        }
    }
}

/// Les familles et les élèves n'ont aucune capacité « Faire » par défaut : la palette ne leur propose
/// que des pages (le personnel seul agit par ce registre).
#[test]
fn familles_et_eleves_n_ont_aucune_capacite() {
    for r in [Role::Parent, Role::Eleve] {
        let d = droits_du_role(r);
        assert!(
            capacites_permises(|x| d.contains(x)).is_empty(),
            "{r:?} voit une capacité"
        );
    }
}

/// Un professeur fait l'appel et écrit au cahier ; il ne gère ni les rôles ni les campagnes.
#[test]
fn le_professeur_par_defaut() {
    let d = droits_du_role(Role::Prof);
    let ids: Vec<_> = capacites_permises(|x| d.contains(x))
        .iter()
        .map(|c| c.id)
        .collect();
    for attendu in ["appel.faire", "cahier.ecrire"] {
        assert!(ids.contains(&attendu), "{attendu} manque : {ids:?}");
    }
    for interdit in [
        "roles.gerer",
        "campagne.lancer",
        "inscription.traiter",
        "delegation.gerer",
    ] {
        assert!(
            !ids.contains(&interdit),
            "{interdit} ne doit pas être proposé : {ids:?}"
        );
    }
}

/// Pour chaque rôle : l'ensemble de l'IA est inclus dans celui de la palette, sans capacité fermée ;
/// « Rôles et droits » n'est proposé qu'à qui détient `roles.gerer`.
#[test]
fn pour_chaque_role_l_ia_ne_depasse_pas_la_palette() {
    for r in ROLES {
        let d = droits_du_role(r);
        let a = |x: &str| d.contains(x);
        let palette: HashSet<_> = capacites_permises(a).iter().map(|c| c.id).collect();
        for c in capacites_ouvertes_ia(a) {
            assert!(palette.contains(c.id), "{r:?} : {} hors palette", c.id);
            assert_ne!(c.confirmation, Confirmation::FermeeIa);
        }
        assert_eq!(
            palette.contains("roles.gerer"),
            d.contains("roles.gerer"),
            "{r:?}"
        );
    }
}
