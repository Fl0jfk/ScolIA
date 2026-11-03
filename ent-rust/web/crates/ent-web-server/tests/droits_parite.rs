//! Parité entre les droits d'affichage de l'interface (`ent_web_app::noyau::droits`)
//! et l'autorité `ent_auth::Role`. Le backend reste l'autorité : ce test garantit que
//! l'interface ne propose pas ce que `ent-auth` refuse (ni ne cache ce qu'il autorise).
//! `ent-auth` n'est pas modifié ; il est utilisé en dev-dépendance uniquement.
//!
//! Un rôle ajouté d'un côté fait échouer la compilation : les `match` ci-dessous
//! n'ont volontairement pas de bras `_`.

use ent_auth::Role as Auth;
use ent_web_app::noyau::droits::{droits_du_role, libelle_role, Role as Ui};

/// Tous les rôles côté ent-auth. Le `match` exhaustif force la mise à jour de cette
/// liste quand un rôle est ajouté à `ent_auth::Role`.
fn tous_auth() -> Vec<Auth> {
    let liste = vec![
        Auth::Admin,
        Auth::Direction,
        Auth::Administratif,
        Auth::Cpe,
        Auth::Compta,
        Auth::Prof,
        Auth::Infirmerie,
        Auth::Psy,
        Auth::Parent,
        Auth::Eleve,
    ];
    for r in &liste {
        match r {
            Auth::Admin
            | Auth::Direction
            | Auth::Administratif
            | Auth::Cpe
            | Auth::Compta
            | Auth::Prof
            | Auth::Infirmerie
            | Auth::Psy
            | Auth::Parent
            | Auth::Eleve => {}
        }
    }
    liste
}

/// Correspondance de variantes (exhaustive des deux côtés).
fn vers_ui(r: Auth) -> Ui {
    match r {
        Auth::Admin => Ui::Admin,
        Auth::Direction => Ui::Direction,
        Auth::Administratif => Ui::Administratif,
        Auth::Cpe => Ui::Cpe,
        Auth::Compta => Ui::Compta,
        Auth::Prof => Ui::Prof,
        Auth::Infirmerie => Ui::Infirmerie,
        Auth::Psy => Ui::Psy,
        Auth::Parent => Ui::Parent,
        Auth::Eleve => Ui::Eleve,
    }
}

/// Sens inverse : un rôle ajouté côté interface fait échouer la compilation ici.
fn vers_auth(r: Ui) -> Auth {
    match r {
        Ui::Admin => Auth::Admin,
        Ui::Direction => Auth::Direction,
        Ui::Administratif => Auth::Administratif,
        Ui::Cpe => Auth::Cpe,
        Ui::Compta => Auth::Compta,
        Ui::Prof => Auth::Prof,
        Ui::Infirmerie => Auth::Infirmerie,
        Ui::Psy => Auth::Psy,
        Ui::Parent => Auth::Parent,
        Ui::Eleve => Auth::Eleve,
    }
}

/// Alias textuels connus (union des deux côtés), avec le rôle attendu.
const ALIAS: &[(&str, Auth)] = &[
    ("admin", Auth::Admin),
    ("direction", Auth::Direction),
    ("administratif", Auth::Administratif),
    ("staff", Auth::Administratif),
    ("cpe", Auth::Cpe),
    ("vie_scolaire", Auth::Cpe),
    ("vs", Auth::Cpe),
    ("compta", Auth::Compta),
    ("prof", Auth::Prof),
    ("enseignant", Auth::Prof),
    ("infirmerie", Auth::Infirmerie),
    ("infirmiere", Auth::Infirmerie),
    ("infirmier", Auth::Infirmerie),
    ("psy", Auth::Psy),
    ("psychologue", Auth::Psy),
    ("parent", Auth::Parent),
    ("eleve", Auth::Eleve),
];

#[test]
fn parsing_alias_identiques() {
    for (alias, attendu) in ALIAS {
        assert_eq!(
            Auth::parse(alias),
            Some(*attendu),
            "ent-auth ne reconnaît plus l'alias « {alias} » : mettre la liste de test à jour"
        );
        assert_eq!(
            Ui::depuis(alias).map(vers_auth),
            Some(*attendu),
            "l'interface interprète l'alias « {alias} » autrement que ent-auth"
        );
    }
}

#[test]
fn parsing_casse_et_espaces_identiques() {
    for (alias, attendu) in ALIAS {
        let variantes = [
            alias.to_uppercase(),
            format!("  {alias}  "),
            format!("\t{}\n", alias.to_uppercase()),
            {
                let mut c = alias.chars();
                match c.next() {
                    Some(p) => p.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            },
        ];
        for v in variantes {
            assert_eq!(Auth::parse(&v), Some(*attendu), "ent-auth, entrée {v:?}");
            assert_eq!(
                Ui::depuis(&v).map(vers_auth),
                Some(*attendu),
                "l'interface diverge de ent-auth pour l'entrée {v:?}"
            );
        }
    }
}

#[test]
fn parsing_inconnus_donnent_none_des_deux_cotes() {
    let inconnus = [
        "",
        " ",
        "inconnu",
        "root",
        "superadmin",
        "adm in",
        "admin2",
        "élève",
        "eleves",
        "parents",
        "prof.",
        "ad\0min",
        "admin\u{200b}",
        "ＡＤＭＩＮ",
    ];
    for s in inconnus {
        assert_eq!(
            Auth::parse(s),
            None,
            "ent-auth accepte {s:?} : liste de test à revoir"
        );
        assert_eq!(
            Ui::depuis(s),
            None,
            "l'interface accepte {s:?} alors que ent-auth le rejette"
        );
    }
}

#[test]
fn parsing_noms_canoniques_de_ent_auth() {
    for r in tous_auth() {
        assert_eq!(
            Ui::depuis(r.as_str()).map(vers_auth),
            Some(r),
            "le nom canonique « {} » n'est pas reconnu pareil par l'interface",
            r.as_str()
        );
    }
}

#[test]
fn correspondance_des_variantes() {
    for r in tous_auth() {
        assert_eq!(
            vers_auth(vers_ui(r)),
            r,
            "correspondance de variante non bijective pour {r:?}"
        );
    }
}

#[test]
fn droits_personnel_identique_a_is_staff() {
    for r in tous_auth() {
        assert_eq!(
            droits_du_role(r.as_str()).personnel,
            r.is_staff(),
            "`personnel` diverge de ent_auth::Role::is_staff pour {r:?}"
        );
    }
}

#[test]
fn droits_faire_appel_identique_a_can_faire_appel() {
    for r in tous_auth() {
        assert_eq!(
            droits_du_role(r.as_str()).faire_appel,
            r.can_faire_appel(),
            "`faire_appel` diverge de ent_auth::Role::can_faire_appel pour {r:?}"
        );
    }
}

/// Règle de la route `/edt/mes-creneaux` (ent-api `list_mes_creneaux`) : élève, ou
/// droit d'appel.
#[test]
fn droits_emploi_du_temps_conforme_a_la_route() {
    for r in tous_auth() {
        let attendu = r == Auth::Eleve || r.can_faire_appel();
        assert_eq!(
            droits_du_role(r.as_str()).emploi_du_temps,
            attendu,
            "`emploi_du_temps` diverge de la règle ent-api (élève ou can_faire_appel) pour {r:?}"
        );
    }
}

#[test]
fn droits_via_alias_identiques_au_role_canonique() {
    for (alias, role) in ALIAS {
        assert_eq!(
            droits_du_role(alias),
            droits_du_role(role.as_str()),
            "droits différents entre l'alias « {alias} » et le rôle canonique {role:?}"
        );
    }
}

#[test]
fn role_inconnu_n_obtient_aucun_droit() {
    for s in ["", "inconnu", "root", "élève", "admin2"] {
        let d = droits_du_role(s);
        assert!(
            !d.personnel && !d.faire_appel && !d.emploi_du_temps,
            "un rôle inconnu ({s:?}) obtient des droits : {d:?}"
        );
        assert_eq!(
            libelle_role(s),
            "Utilisateur",
            "libellé de repli pour {s:?}"
        );
    }
}

#[test]
fn libelles_non_vides_et_distincts() {
    // Pas d'équivalent dans ent-auth : on vérifie seulement la cohérence interne.
    let mut vus = std::collections::HashSet::new();
    for r in tous_auth() {
        let l = libelle_role(r.as_str());
        assert!(
            !l.is_empty() && l != "Utilisateur",
            "libellé absent pour {r:?}"
        );
        assert!(vus.insert(l), "libellé « {l} » en double (rôle {r:?})");
        assert_eq!(
            l,
            vers_ui(r).libelle(),
            "libelle_role et Role::libelle divergent pour {r:?}"
        );
    }
}

/// Le bandeau « environnement de test » du frontend et la porte du labo d'ent-api
/// reconnaissent exactement les mêmes environnements non productifs.
#[test]
fn environnements_non_productifs_identiques() {
    let mut api: Vec<&str> = ent_api::ENVS_NON_PROD.to_vec();
    let mut web: Vec<&str> = ent_web_app::modules::connexion::ENVS_NON_PROD.to_vec();
    api.sort_unstable();
    web.sort_unstable();
    assert_eq!(
        api, web,
        "ENVS_NON_PROD diverge entre ent-api et ent-web-app"
    );
}
