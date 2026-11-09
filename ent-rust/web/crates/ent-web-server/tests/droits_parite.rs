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
        Auth::Accueil,
        Auth::Personnel,
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
            | Auth::Accueil
            | Auth::Personnel
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
        Auth::Accueil => Ui::Accueil,
        Auth::Personnel => Ui::Personnel,
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
        Ui::Accueil => Auth::Accueil,
        Ui::Personnel => Auth::Personnel,
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
    ("accueil", Auth::Accueil),
    ("personnel", Auth::Personnel),
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

/// Messagerie : le personnel autorisé est exactement `PgEnt::role_can_messagerie_staff`
/// (ent-db) ; la famille et l'élève lisent leurs propres fils ; les autres rôles
/// (comptabilité, infirmerie, psychologue) n'y ont pas accès (`list_fils_for_actor`).
#[test]
fn droits_messagerie_conformes_a_ent_db() {
    for r in tous_auth() {
        let d = droits_du_role(r.as_str());
        let personnel = ent_db::PgEnt::role_can_messagerie_staff(r.as_str());
        assert_eq!(
            d.messagerie_personnel, personnel,
            "`messagerie_personnel` diverge de role_can_messagerie_staff pour {r:?}"
        );
        let attendu = personnel || matches!(r, Auth::Parent | Auth::Eleve);
        assert_eq!(
            d.messagerie, attendu,
            "`messagerie` diverge de la règle ent-db (personnel, parent, élève) pour {r:?}"
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
            d == ent_web_app::noyau::droits::Droits::default(),
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

/// Vie scolaire : `lire_vs_absences` = `Role::can_lire_vs_absences` (route `/vs/absences`,
/// `/vs/signaux`, `/eleves/{id}/absences` pour le personnel).
#[test]
fn droits_lire_vs_absences_identique_a_ent_auth() {
    for r in tous_auth() {
        assert_eq!(
            droits_du_role(r.as_str()).lire_vs_absences,
            r.can_lire_vs_absences(),
            "`lire_vs_absences` diverge de ent_auth::Role::can_lire_vs_absences pour {r:?}"
        );
    }
}

/// `trancher_justificatif` = `Role::can_trancher_justificatif` (accepter, refuser,
/// signaux d'absence répétée).
#[test]
fn droits_trancher_justificatif_identique_a_ent_auth() {
    for r in tous_auth() {
        assert_eq!(
            droits_du_role(r.as_str()).trancher_justificatif,
            r.can_trancher_justificatif(),
            "`trancher_justificatif` diverge de ent_auth::Role::can_trancher_justificatif pour {r:?}"
        );
    }
}

/// `annuler_absence_present` = `Role::can_annuler_absence_vers_present`.
#[test]
fn droits_annuler_absence_present_identique_a_ent_auth() {
    for r in tous_auth() {
        assert_eq!(
            droits_du_role(r.as_str()).annuler_absence_present,
            r.can_annuler_absence_vers_present(),
            "`annuler_absence_present` diverge de ent_auth::Role::can_annuler_absence_vers_present pour {r:?}"
        );
    }
}

/// `portail_famille` : `GET /famille/enfants` (`mes_enfants_for_parent`) exige le rôle
/// responsable légal. `mes_absences` : seul le rôle élève consulte « ses » absences.
/// Ni l'un ni l'autre n'ont de méthode dans `ent-auth` : la règle est celle des routes.
#[test]
fn droits_portail_famille_et_eleve_conformes_aux_routes() {
    for r in tous_auth() {
        let d = droits_du_role(r.as_str());
        assert_eq!(
            d.portail_famille,
            r == Auth::Parent,
            "portail_famille pour {r:?}"
        );
        assert_eq!(d.mes_absences, r == Auth::Eleve, "mes_absences pour {r:?}");
        // Cohérence : jamais à la fois personnel de vie scolaire et famille/élève.
        assert!(
            !(d.portail_famille && d.lire_vs_absences),
            "{r:?} cumule portail famille et lecture VS"
        );
        assert!(
            !(d.mes_absences && d.trancher_justificatif),
            "{r:?} cumule consultation élève et décision VS"
        );
    }
}

/// Référentiel scolaire : gestion = `role_peut_gerer_referentiel`, lecture du personnel =
/// `role_peut_lire_referentiel`, « Mes classes » = enseignant (liste filtrée par le serveur).
#[test]
fn droits_classes_conformes_a_ent_db() {
    for r in tous_auth() {
        let d = droits_du_role(r.as_str());
        assert_eq!(
            d.gerer_classes,
            ent_db::role_peut_gerer_referentiel(r.as_str()),
            "`gerer_classes` diverge d'ent-db pour {r:?}"
        );
        let peut_lire = ent_db::role_peut_lire_referentiel(r.as_str());
        assert_eq!(
            d.voir_classes || d.mes_classes,
            peut_lire,
            "l'interface propose (ou cache) les classes à tort pour {r:?}"
        );
        assert_eq!(d.mes_classes, r == Auth::Prof, "{r:?}");
        assert!(!(d.voir_classes && d.mes_classes), "{r:?}");
    }
}

/// Politique de sécurité de l'établissement (`GET`/`PUT /etablissement/securite`) : la route
/// n'accepte que Admin et Direction (`routes.rs::get_politique_securite`, et
/// `definir_politique_securite_for_actor` dans ent-db). L'interface ne propose la page qu'à ces rôles.
#[test]
fn droit_gerer_securite_reserve_a_admin_et_direction() {
    for r in tous_auth() {
        assert_eq!(
            droits_du_role(r.as_str()).gerer_securite,
            matches!(r, Auth::Admin | Auth::Direction),
            "`gerer_securite` diverge de la route pour {r:?}"
        );
    }
}

/// Administration des rôles (D-017) : l'action `roles.gerer` fait partie des modèles par défaut
/// d'Admin et de Direction seulement (`ent_auth::modele_par_defaut`). L'interface propose
/// l'entrée « Rôles et droits » à ces profils ; la page revérifie `/roles/mes-droits`.
#[test]
fn droit_gerer_roles_suit_le_modele_par_defaut() {
    use ent_auth::{modele_par_defaut, Action, Portee, Role};
    for r in tous_auth() {
        let role = Role::parse(r.as_str()).expect("rôle connu d'ent-auth");
        let a_le_droit =
            modele_par_defaut(role).contains(&(Action::RolesGerer, Portee::Etablissement));
        assert_eq!(
            droits_du_role(r.as_str()).gerer_roles,
            a_le_droit,
            "`gerer_roles` diverge du modèle par défaut pour {r:?}"
        );
    }
}

/// Cahier de textes (S2-6) : l'interface ne propose « Cahier de textes » qu'aux rôles dont le
/// modèle par défaut contient `cahier.ecrire`, et « Travail à faire » qu'aux familles et élèves
/// (qui n'ont jamais de rôle paramétrable : leurs droits viennent de l'identité et du lien).
#[test]
fn droits_cahier_conformes_au_modele_par_defaut() {
    use ent_auth::{modele_par_defaut, Action};
    for r in tous_auth() {
        let ecrit = modele_par_defaut(r)
            .iter()
            .any(|(a, _)| *a == Action::CahierEcrire);
        let d = droits_du_role(r.as_str());
        assert_eq!(
            d.cahier_ecrire, ecrit,
            "`cahier_ecrire` diverge de cahier.ecrire (modèle par défaut) pour {r:?}"
        );
        assert_eq!(
            d.agenda,
            matches!(r, Auth::Parent | Auth::Eleve),
            "`agenda` diverge pour {r:?}"
        );
        assert!(
            !(d.agenda
                && modele_par_defaut(r)
                    .iter()
                    .any(|(a, _)| *a == Action::CahierLire)),
            "famille et élève n'ont aucun rôle paramétrable : pas de cahier.lire de personnel"
        );
    }
}
