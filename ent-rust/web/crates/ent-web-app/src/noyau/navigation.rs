//! Registre de navigation : les modules déclarent leurs entrées, filtrées par les droits.
//! La même liste sert à la barre latérale, au rail et à la barre d'onglets.

use ent_web_ui::{EntreeNav, NomIcone};

use super::droits::Droits;
use super::droits_moteur::DroitsMoteur;

pub const ACCUEIL: &str = "/web";
pub const EMPLOI_DU_TEMPS: &str = "/web/emploi-du-temps";
/// EDT de la semaine (personnel, classe, salle, élève) : S2-5.
pub const EDT_SEMAINE: &str = "/web/edt/semaine";
/// Absences de personnel et remplacements (`edt.gerer`) : S2-5.
pub const REMPLACEMENTS: &str = "/web/edt/remplacements";
/// Changements d'emploi du temps vus par l'élève et sa famille : S2-5.
pub const CHANGEMENTS_EDT: &str = "/web/changements-edt";
/// Délégations de droits (« mes délégations » pour le personnel, gestion avec `delegations.gerer`).
pub const DELEGATIONS: &str = "/web/delegations";
pub const APPEL: &str = "/web/appel";
pub const CLASSES: &str = "/web/classes";
pub const MES_CLASSES: &str = "/web/mes-classes";
pub const VIE_SCOLAIRE: &str = "/web/vie-scolaire";
pub const JOURNEE: &str = "/web/journee";
pub const REFERENTIEL: &str = "/web/referentiel";
pub const DISCIPLINE: &str = "/web/discipline";
pub const SIGNALEMENTS: &str = "/web/signalements";
pub const ABSENCES: &str = "/web/absences";
pub const CAHIER: &str = "/web/cahier";
pub const AGENDA: &str = "/web/agenda";
/// Évaluations, conseils, bulletins (S2-7).
pub const EVALUATIONS: &str = "/web/evaluations";
pub const MESSAGES: &str = "/web/messages";
pub const SECURITE: &str = "/web/securite";
pub const POLITIQUE_SECURITE: &str = "/web/direction/securite";
pub const ROLES: &str = "/web/direction/roles";
pub const ROLES_NOUVEAU: &str = "/web/direction/roles/nouveau";
pub const ROLES_GROUPES: &str = "/web/direction/roles/groupes";
/// Inscriptions (S2-3) : personnel, réinscription famille, préinscription publique.
pub const INSCRIPTIONS: &str = "/web/inscriptions";
pub const REINSCRIPTION: &str = "/web/reinscription";
pub const PREINSCRIPTION: &str = "/web/preinscription";
/// Fiche élève 360° : `/web/eleves/{id}` (S2-1).
pub const ELEVES: &str = "/web/eleves";

fn entree(libelle: &str, href: &str, icone: NomIcone) -> EntreeNav {
    EntreeNav {
        libelle: libelle.into(),
        href: href.into(),
        icone,
        badge: None,
    }
}

/// Entrées visibles pour le profil de base seul (avant la réponse du moteur de droits).
pub fn entrees(droits: Droits) -> Vec<EntreeNav> {
    entrees_avec_moteur(droits, &DroitsMoteur::default())
}

/// Entrées visibles pour le profil de base **et** les droits effectifs du moteur (rôles
/// paramétrés compris), dans l'ordre d'affichage. Le serveur revérifie chaque route.
pub fn entrees_avec_moteur(droits: Droits, moteur: &DroitsMoteur) -> Vec<EntreeNav> {
    let mut v = vec![entree("Aujourd'hui", ACCUEIL, NomIcone::Accueil)];
    if droits.emploi_du_temps {
        v.push(entree("Emploi du temps", EMPLOI_DU_TEMPS, NomIcone::Agenda));
    }
    if droits.personnel {
        v.push(entree("EDT de la semaine", EDT_SEMAINE, NomIcone::Agenda));
    }
    if droits.portail_famille || droits.mes_absences {
        v.push(entree(
            "Changements d'EDT",
            CHANGEMENTS_EDT,
            NomIcone::Agenda,
        ));
    }
    if droits.faire_appel {
        v.push(entree("Appel", APPEL, NomIcone::Appel));
    }
    if droits.cahier_ecrire {
        v.push(entree("Cahier de textes", CAHIER, NomIcone::Annonce));
    }
    if droits.agenda {
        v.push(entree("Travail à faire", AGENDA, NomIcone::Agenda));
    }
    if droits.mes_classes {
        v.push(entree("Mes classes", MES_CLASSES, NomIcone::Ecole));
    }
    if droits.voir_classes {
        v.push(entree("Classes", CLASSES, NomIcone::Ecole));
    }
    // Écrans d'administration : proposés à qui gère le référentiel. La lecture (classes, effectifs)
    // reste accessible à tout le personnel par « Classes » ; l'adresse directe consulte sans écrire.
    if moteur.peut_gerer_referentiel() {
        v.push(entree("Référentiel", REFERENTIEL, NomIcone::Salle));
    }
    if droits.trancher_justificatif {
        v.push(entree("Vie scolaire", VIE_SCOLAIRE, NomIcone::Personne));
    }
    if droits.trancher_justificatif
        || moteur.peut_verifier_appels()
        || moteur.peut_declarer_absence()
    {
        v.push(entree("Journée", JOURNEE, NomIcone::Agenda));
    }
    if moteur.peut_donner_punition() || moteur.peut_gerer_discipline() {
        v.push(entree("Discipline", DISCIPLINE, NomIcone::Personne));
    }
    if moteur.peut_signaler_assiduite() {
        v.push(entree("Signalements", SIGNALEMENTS, NomIcone::Annonce));
    }
    // Évaluations : personnel qui lit ou saisit des notes (moteur), familles, élèves.
    if (droits.personnel && (moteur.a("notes.saisir") || moteur.a("notes.lire")))
        || droits.portail_famille
        || droits.mes_absences
    {
        v.push(entree("Évaluations", EVALUATIONS, NomIcone::Ecole));
    }
    if droits.portail_famille || droits.mes_absences {
        v.push(entree("Absences", ABSENCES, NomIcone::Personne));
    }
    if moteur.peut_gerer_edt() {
        v.push(entree("Remplacements", REMPLACEMENTS, NomIcone::Personne));
    }
    if droits.personnel {
        v.push(entree("Délégations", DELEGATIONS, NomIcone::Passkey));
    }
    if droits.messagerie {
        v.push(entree("Messages", MESSAGES, NomIcone::Messages));
    }
    // S2-3 : mêmes profils que la gestion du référentiel (modèle par défaut
    // d'`inscriptions.gerer`), ou rôle paramétré qui l'accorde ; le serveur reste juge.
    if droits.gerer_classes || moteur.a_etablissement("inscriptions.gerer") {
        v.push(entree("Inscriptions", INSCRIPTIONS, NomIcone::Ecole));
    }
    if droits.portail_famille {
        v.push(entree("Réinscription", REINSCRIPTION, NomIcone::Ecole));
    }
    if droits.gerer_roles {
        v.push(entree("Rôles et droits", ROLES, NomIcone::Passkey));
    }
    v
}

/// Liens du pied de navigation, communs à tous les profils : le compte de la personne.
pub fn liens_compte() -> Vec<EntreeNav> {
    vec![entree("Compte", SECURITE, NomIcone::Passkey)]
}

/// Texte du compteur de messages non lus (rien à zéro, « 99+ » au-delà de 99).
pub fn libelle_compteur(n: i64) -> Option<String> {
    match n {
        n if n <= 0 => None,
        n if n > 99 => Some("99+".into()),
        n => Some(n.to_string()),
    }
}

/// Pose le compteur de messages non lus sur l'entrée « Messages ».
pub fn avec_compteur_messages(mut entrees: Vec<EntreeNav>, non_lus: i64) -> Vec<EntreeNav> {
    if let Some(e) = entrees.iter_mut().find(|e| e.href == MESSAGES) {
        e.badge = libelle_compteur(non_lus);
    }
    entrees
}

/// Écrans de saisie : navigation réduite au rail d'icônes pour laisser la place au tableau.
pub fn ecran_de_saisie(chemin: &str) -> bool {
    chemin == APPEL || chemin.starts_with(&format!("{APPEL}/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noyau::droits::droits_du_role;

    #[test]
    fn entrees_selon_le_profil() {
        let prof: Vec<String> = entrees(droits_du_role("prof"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(
            prof,
            [
                ACCUEIL,
                EMPLOI_DU_TEMPS,
                EDT_SEMAINE,
                APPEL,
                CAHIER,
                MES_CLASSES,
                DELEGATIONS,
                MESSAGES
            ]
        );
        let secretariat: Vec<String> = entrees(droits_du_role("administratif"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(
            secretariat,
            [
                ACCUEIL,
                EMPLOI_DU_TEMPS,
                EDT_SEMAINE,
                APPEL,
                CLASSES,
                VIE_SCOLAIRE,
                JOURNEE,
                DELEGATIONS,
                MESSAGES,
                INSCRIPTIONS
            ]
        );
        let compta: Vec<String> = entrees(droits_du_role("compta"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(
            compta,
            [ACCUEIL, EDT_SEMAINE, CLASSES, DELEGATIONS],
            "ni vie scolaire, ni absences, pas de messagerie"
        );
        let parent: Vec<String> = entrees(droits_du_role("parent"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(
            parent,
            [
                ACCUEIL,
                CHANGEMENTS_EDT,
                AGENDA,
                EVALUATIONS,
                ABSENCES,
                MESSAGES,
                REINSCRIPTION
            ],
            "une famille ne voit pas l'appel"
        );
        let cpe: Vec<String> = entrees(droits_du_role("cpe"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(
            cpe,
            [
                ACCUEIL,
                EMPLOI_DU_TEMPS,
                EDT_SEMAINE,
                APPEL,
                CLASSES,
                VIE_SCOLAIRE,
                JOURNEE,
                DELEGATIONS,
                MESSAGES
            ]
        );
        // Un enseignant lit les absences mais ne tranche pas : pas d'entrée « Vie scolaire ».
        assert!(!prof.contains(&VIE_SCOLAIRE.to_string()));
        let eleve: Vec<String> = entrees(droits_du_role("eleve"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(
            eleve,
            [
                ACCUEIL,
                EMPLOI_DU_TEMPS,
                CHANGEMENTS_EDT,
                AGENDA,
                EVALUATIONS,
                ABSENCES,
                MESSAGES
            ]
        );
    }

    fn droit(action: &str, portee: &str) -> ent_web_api::DroitEffectif {
        ent_web_api::DroitEffectif {
            action: action.into(),
            portee: portee.into(),
            cibles: Vec::new(),
        }
    }

    fn adresses(droits: Droits, moteur: &DroitsMoteur) -> Vec<String> {
        entrees_avec_moteur(droits, moteur)
            .into_iter()
            .map(|e| e.href)
            .collect()
    }

    #[test]
    fn le_moteur_ajoute_les_ecrans_selon_les_droits_effectifs() {
        // Secrétariat : référentiel, journée et signalements selon ce que le moteur accorde.
        let moteur = DroitsMoteur::new(vec![
            droit("referentiel.gerer", "etablissement"),
            droit("appel.verifier", "etablissement"),
            droit("punitions.donner", "etablissement"),
            droit("assiduite.signaler", "etablissement"),
        ]);
        let v = adresses(droits_du_role("administratif"), &moteur);
        for attendu in [REFERENTIEL, JOURNEE, DISCIPLINE, SIGNALEMENTS] {
            assert!(
                v.contains(&attendu.to_string()),
                "{attendu} manquant : {v:?}"
            );
        }
        // Lecture seule du référentiel (tout le personnel) : pas d'entrée d'administration.
        let lecture = DroitsMoteur::new(vec![droit("referentiel.lire", "etablissement")]);
        assert!(!adresses(droits_du_role("cpe"), &lecture).contains(&REFERENTIEL.to_string()));
        // Sans réponse du moteur, aucune entrée d'administration n'apparaît.
        let seul = adresses(droits_du_role("administratif"), &DroitsMoteur::default());
        for absent in [REFERENTIEL, DISCIPLINE, SIGNALEMENTS] {
            assert!(!seul.contains(&absent.to_string()));
        }
    }

    #[test]
    fn le_moteur_ouvre_les_remplacements_et_la_semaine_reste_au_personnel() {
        let edt = DroitsMoteur::new(vec![droit("edt.gerer", "etablissement")]);
        let v = adresses(droits_du_role("cpe"), &edt);
        assert!(v.contains(&REMPLACEMENTS.to_string()));
        // Sans le droit, pas d'entrée (le serveur refuserait de toute façon).
        let sans = adresses(droits_du_role("cpe"), &DroitsMoteur::default());
        assert!(!sans.contains(&REMPLACEMENTS.to_string()));
        // La semaine et les délégations reçues sont ouvertes à tout le personnel.
        for role in ["prof", "cpe", "administratif", "compta"] {
            let v = adresses(droits_du_role(role), &DroitsMoteur::default());
            assert!(v.contains(&EDT_SEMAINE.to_string()), "{role}");
            assert!(v.contains(&DELEGATIONS.to_string()), "{role}");
        }
        // Famille et élève : changements d'emploi du temps, jamais l'administration.
        for role in ["parent", "eleve"] {
            let v = adresses(droits_du_role(role), &edt);
            assert!(v.contains(&CHANGEMENTS_EDT.to_string()), "{role}");
            assert!(!v.contains(&DELEGATIONS.to_string()), "{role}");
            assert!(!v.contains(&EDT_SEMAINE.to_string()), "{role}");
        }
        // Les pages restent distinctes pour l'élément de menu actif.
        assert!(!ent_web_ui::lien_actif(EDT_SEMAINE, REMPLACEMENTS));
        assert!(!ent_web_ui::lien_actif(EMPLOI_DU_TEMPS, EDT_SEMAINE));
    }

    #[test]
    fn un_role_parametre_ouvre_la_journee_sans_le_profil() {
        // Un professeur qui reçoit « absences.declarer » voit « Journée » (l'accueil), pas « Vie scolaire ».
        let moteur = DroitsMoteur::new(vec![droit("absences.declarer", "etablissement")]);
        let v = adresses(droits_du_role("prof"), &moteur);
        assert!(v.contains(&JOURNEE.to_string()));
        assert!(!v.contains(&VIE_SCOLAIRE.to_string()));
        // Un enseignant avec ses punitions « sur ses classes » voit « Discipline » mais pas « Référentiel ».
        let p = DroitsMoteur::new(vec![droit("punitions.donner", "mes_classes")]);
        let v = adresses(droits_du_role("prof"), &p);
        assert!(v.contains(&DISCIPLINE.to_string()) && !v.contains(&REFERENTIEL.to_string()));
    }

    #[test]
    fn famille_et_eleve_ne_voient_aucun_ecran_d_administration() {
        let v = adresses(droits_du_role("parent"), &DroitsMoteur::default());
        assert_eq!(
            v,
            [
                ACCUEIL,
                CHANGEMENTS_EDT,
                AGENDA,
                EVALUATIONS,
                ABSENCES,
                MESSAGES,
                REINSCRIPTION
            ]
        );
    }

    #[test]
    fn evaluations_selon_les_droits_sur_les_notes() {
        // Personnel : seulement avec « notes.lire » ou « notes.saisir » (quelle que soit la portée).
        let sans = adresses(droits_du_role("compta"), &DroitsMoteur::default());
        assert!(!sans.contains(&EVALUATIONS.to_string()));
        let prof = DroitsMoteur::new(vec![droit("notes.saisir", "mes_classes")]);
        assert!(adresses(droits_du_role("prof"), &prof).contains(&EVALUATIONS.to_string()));
        let eleve = adresses(droits_du_role("eleve"), &DroitsMoteur::default());
        assert!(eleve.contains(&EVALUATIONS.to_string()));
    }

    #[test]
    fn roles_et_droits_pour_l_administration_seulement() {
        for role in ["admin", "direction"] {
            let v: Vec<String> = entrees(droits_du_role(role))
                .into_iter()
                .map(|e| e.href)
                .collect();
            assert_eq!(v.last().map(String::as_str), Some(ROLES), "{role}");
        }
        for role in ["prof", "cpe", "administratif", "compta", "parent", "eleve"] {
            assert!(
                entrees(droits_du_role(role))
                    .iter()
                    .all(|e| e.href != ROLES),
                "{role}"
            );
        }
        // « Rôles et droits » est actif sur ses sous-pages (édition, responsables des groupes).
        assert!(ent_web_ui::lien_actif(ROLES, ROLES_GROUPES));
        assert!(ent_web_ui::lien_actif(
            ROLES,
            "/web/direction/roles/6f1c3c80"
        ));
        assert!(!ent_web_ui::lien_actif(ROLES, POLITIQUE_SECURITE));
    }

    #[test]
    fn compteur_de_messages_non_lus() {
        assert_eq!(libelle_compteur(0), None);
        assert_eq!(libelle_compteur(-3), None);
        assert_eq!(libelle_compteur(7).as_deref(), Some("7"));
        assert_eq!(libelle_compteur(250).as_deref(), Some("99+"));
        let e = avec_compteur_messages(entrees(droits_du_role("prof")), 3);
        let badges: Vec<_> = e.iter().map(|x| x.badge.clone()).collect();
        assert_eq!(badges.last().unwrap().as_deref(), Some("3"));
        assert!(badges[..badges.len() - 1].iter().all(Option::is_none));
        let sans = avec_compteur_messages(entrees(droits_du_role("compta")), 3);
        assert!(sans.iter().all(|x| x.badge.is_none()));
    }

    #[test]
    fn le_compte_est_accessible_a_tous_les_profils() {
        let liens = liens_compte();
        assert_eq!(liens.len(), 1);
        assert_eq!(liens[0].href, SECURITE);
        // Le lien « Compte » est actif sur ses sous-pages, pas sur la politique de l'établissement.
        assert!(ent_web_ui::lien_actif(SECURITE, "/web/securite"));
        assert!(!ent_web_ui::lien_actif(SECURITE, POLITIQUE_SECURITE));
    }

    #[test]
    fn rail_sur_les_ecrans_de_saisie() {
        assert!(ecran_de_saisie("/web/appel"));
        assert!(!ecran_de_saisie("/web"));
        assert!(!ecran_de_saisie("/web/emploi-du-temps"));
    }
}
