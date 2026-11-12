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
/// Infirmerie (S2-11) : tableau du jour ; fiche d'un élève : `/web/infirmerie/eleves/{id}`.
pub const INFIRMERIE: &str = "/web/infirmerie";
/// Accompagnement (S2-11) : PAP, PPS, aménagements ; élève : `/web/accompagnement/eleves/{id}`.
pub const ACCOMPAGNEMENT: &str = "/web/accompagnement";
/// Aménagements des élèves d'un professeur et contraintes publiées (S2-11).
pub const AMENAGEMENTS: &str = "/web/amenagements";
/// Santé de l'enfant côté famille : PAI à signer, passages visibles, aménagements (S2-11).
pub const SANTE_ENFANT: &str = "/web/sante-enfant";
pub const ABSENCES: &str = "/web/absences";
pub const CAHIER: &str = "/web/cahier";
pub const AGENDA: &str = "/web/agenda";
/// Sorties et voyages (socle 42, lot V0a) : personnel selon le moteur de droits, familles et élèves.
pub const VOYAGES: &str = "/web/voyages";
/// Évaluations, conseils, bulletins (S2-7).
pub const EVALUATIONS: &str = "/web/evaluations";
pub const MESSAGES: &str = "/web/messages";
pub const SECURITE: &str = "/web/securite";
pub const POLITIQUE_SECURITE: &str = "/web/direction/securite";
/// Confirmation des liens de responsabilité déclarés par l'établissement (tous les profils connectés).
pub const MES_LIENS: &str = "/web/mes-liens";
/// À-faire unifié (socle 29, lot P3) : familles et élèves majeurs.
pub const A_FAIRE: &str = "/web/a-faire";
/// Protections (adresses à protéger, « ne pas solliciter ») : direction, le serveur reste juge.
pub const PROTECTIONS: &str = "/web/direction/protections";
pub const ROLES: &str = "/web/direction/roles";
pub const ROLES_NOUVEAU: &str = "/web/direction/roles/nouveau";
pub const ROLES_GROUPES: &str = "/web/direction/roles/groupes";
/// Inscriptions (S2-3) : personnel, réinscription famille, préinscription publique.
pub const INSCRIPTIONS: &str = "/web/inscriptions";
pub const REINSCRIPTION: &str = "/web/reinscription";
pub const PREINSCRIPTION: &str = "/web/preinscription";
pub const NOTIFICATIONS: &str = "/web/notifications";
pub const CAMPAGNES: &str = "/web/campagnes";
pub const RENCONTRES: &str = "/web/rencontres";
pub const RENDEZ_VOUS: &str = "/web/rendez-vous";
pub const FORMULAIRES: &str = "/web/formulaires";
/// Finances (S2-9) : comptabilité (personnel) et factures des payeurs (familles).
pub const COMPTABILITE: &str = "/web/comptabilite";
pub const MES_FACTURES: &str = "/web/mes-factures";
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
    // Socle 29 (P3) : la seule liste transverse de ce que la famille (ou l'élève) doit faire.
    if droits.portail_famille || droits.mes_absences {
        v.push(entree("À faire", A_FAIRE, NomIcone::Annonce));
    }
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
    if moteur.peut_lire_sante() {
        v.push(entree("Infirmerie", INFIRMERIE, NomIcone::Personne));
    }
    if moteur.peut_gerer_accompagnement() {
        v.push(entree("Accompagnement", ACCOMPAGNEMENT, NomIcone::Personne));
    }
    if moteur.peut_lire_amenagements() || moteur.peut_lire_contraintes() {
        v.push(entree("Aménagements", AMENAGEMENTS, NomIcone::Ecole));
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
    // Communication (S2-8) : la famille réserve ses rendez-vous et signe ses formulaires (un
    // élève majeur signe lui-même) ; l'enseignant voit ses rendez-vous ; le personnel gère
    // campagnes, rencontres et formulaires selon les droits effectifs du moteur.
    if droits.portail_famille || droits.mes_classes {
        v.push(entree("Rendez-vous", RENDEZ_VOUS, NomIcone::Agenda));
    }
    if droits.portail_famille || droits.mes_absences || moteur.peut_gerer_formulaires() {
        v.push(entree("Formulaires", FORMULAIRES, NomIcone::Annonce));
    }
    if moteur.peut_lancer_campagne() {
        v.push(entree("Campagnes", CAMPAGNES, NomIcone::Annonce));
    }
    if moteur.peut_gerer_rencontres() {
        v.push(entree("Rencontres", RENCONTRES, NomIcone::Agenda));
    }
    if droits.portail_famille {
        v.push(entree("Santé", SANTE_ENFANT, NomIcone::Personne));
    }
    // Socle 42 : la liste minimale des sorties est ouverte au personnel (`voyages.annuaire`) ; les
    // familles et les élèves voient les sorties où l'enfant est inscrit.
    if moteur.a("voyages.annuaire") || droits.portail_famille || droits.mes_absences {
        v.push(entree("Sorties", VOYAGES, NomIcone::Agenda));
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
        v.push(entree("Factures", MES_FACTURES, NomIcone::Annonce));
    }
    // S2-9 : comptabilité selon le moteur de droits (rôles paramétrés compris).
    if moteur.peut_lire_facturation() {
        v.push(entree("Comptabilité", COMPTABILITE, NomIcone::Salle));
    }
    // Administration des rôles : toujours en dernier.
    if droits.gerer_roles {
        v.push(entree("Rôles et droits", ROLES, NomIcone::Passkey));
    }
    v
}

/// Liens du pied de navigation, communs à tous les profils : le compte de la personne.
pub fn liens_compte() -> Vec<EntreeNav> {
    vec![
        entree("Compte", SECURITE, NomIcone::Passkey),
        entree("Notifications", NOTIFICATIONS, NomIcone::Annonce),
    ]
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
                RENDEZ_VOUS,
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
                A_FAIRE,
                CHANGEMENTS_EDT,
                AGENDA,
                EVALUATIONS,
                ABSENCES,
                RENDEZ_VOUS,
                FORMULAIRES,
                SANTE_ENFANT,
                VOYAGES,
                MESSAGES,
                REINSCRIPTION,
                MES_FACTURES
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
                A_FAIRE,
                EMPLOI_DU_TEMPS,
                CHANGEMENTS_EDT,
                AGENDA,
                EVALUATIONS,
                ABSENCES,
                FORMULAIRES,
                VOYAGES,
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
    fn sante_et_accompagnement_selon_les_droits_du_moteur() {
        let sante = DroitsMoteur::new(vec![droit("sante.lire", "etablissement")]);
        let v = adresses(droits_du_role("infirmerie"), &sante);
        assert!(v.contains(&INFIRMERIE.to_string()));
        assert!(!v.contains(&ACCOMPAGNEMENT.to_string()));
        let acc = DroitsMoteur::new(vec![droit("accompagnement.gerer", "etablissement")]);
        assert!(adresses(droits_du_role("direction"), &acc).contains(&ACCOMPAGNEMENT.to_string()));
        // Professeur : « Aménagements » (ses élèves), ni infirmerie ni accompagnement.
        let prof = DroitsMoteur::new(vec![
            droit("amenagements.lire", "mes_classes"),
            droit("contraintes.lire", "mes_classes"),
        ]);
        let v = adresses(droits_du_role("prof"), &prof);
        assert!(v.contains(&AMENAGEMENTS.to_string()));
        assert!(!v.contains(&INFIRMERIE.to_string()) && !v.contains(&ACCOMPAGNEMENT.to_string()));
        // Sans réponse du moteur, aucune de ces entrées n'apparaît.
        let seul = adresses(droits_du_role("infirmerie"), &DroitsMoteur::default());
        for absent in [INFIRMERIE, ACCOMPAGNEMENT, AMENAGEMENTS] {
            assert!(!seul.contains(&absent.to_string()));
        }
    }

    #[test]
    fn famille_et_eleve_ne_voient_aucun_ecran_d_administration() {
        let v = adresses(droits_du_role("parent"), &DroitsMoteur::default());
        assert_eq!(
            v,
            [
                ACCUEIL,
                A_FAIRE,
                CHANGEMENTS_EDT,
                AGENDA,
                EVALUATIONS,
                ABSENCES,
                RENDEZ_VOUS,
                FORMULAIRES,
                SANTE_ENFANT,
                VOYAGES,
                MESSAGES,
                REINSCRIPTION,
                MES_FACTURES
            ]
        );
        // Un parent ne voit jamais la comptabilité, même si le moteur répondait par erreur.
        let v = adresses(droits_du_role("parent"), &DroitsMoteur::default());
        assert!(!v.contains(&COMPTABILITE.to_string()));
    }

    #[test]
    fn sorties_selon_le_moteur_pour_le_personnel_et_pour_les_familles() {
        // Personnel : l'entrée vient du droit `voyages.annuaire` du moteur (jamais du profil seul).
        let sans = adresses(droits_du_role("prof"), &DroitsMoteur::default());
        assert!(!sans.contains(&VOYAGES.to_string()));
        let avec = DroitsMoteur::new(vec![droit("voyages.annuaire", "etablissement")]);
        assert!(adresses(droits_du_role("prof"), &avec).contains(&VOYAGES.to_string()));
        // Familles et élèves : l'écran montre les sorties de leur enfant.
        for role in ["parent", "eleve"] {
            let v = adresses(droits_du_role(role), &DroitsMoteur::default());
            assert!(v.contains(&VOYAGES.to_string()), "{role}");
        }
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
    fn communication_selon_les_droits_effectifs() {
        let moteur = DroitsMoteur::new(vec![
            droit("campagnes.lancer", "etablissement"),
            droit("rencontres.gerer", "etablissement"),
            droit("formulaires.gerer", "etablissement"),
        ]);
        let v = adresses(droits_du_role("direction"), &moteur);
        for attendu in [CAMPAGNES, RENCONTRES, FORMULAIRES] {
            assert!(
                v.contains(&attendu.to_string()),
                "{attendu} manquant : {v:?}"
            );
        }
        // Sans réponse du moteur, aucune entrée de gestion ; la famille garde les siennes.
        let seul = adresses(droits_du_role("direction"), &DroitsMoteur::default());
        for absent in [CAMPAGNES, RENCONTRES, FORMULAIRES] {
            assert!(!seul.contains(&absent.to_string()));
        }
        // Un professeur avec « campagnes.lancer » sur ses classes voit « Campagnes ».
        let p = DroitsMoteur::new(vec![droit("campagnes.lancer", "designes")]);
        assert!(adresses(droits_du_role("prof"), &p).contains(&CAMPAGNES.to_string()));
        // La gestion des rencontres exige la portée établissement.
        let r = DroitsMoteur::new(vec![droit("rencontres.gerer", "mes_classes")]);
        assert!(!adresses(droits_du_role("prof"), &r).contains(&RENCONTRES.to_string()));
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
    fn comptabilite_selon_le_moteur_de_droits() {
        let lire = DroitsMoteur::new(vec![droit("facturation.lire", "etablissement")]);
        let v = adresses(droits_du_role("administratif"), &lire);
        assert!(v.contains(&COMPTABILITE.to_string()));
        assert!(lire.peut_lire_facturation() && !lire.peut_gerer_facturation());
        let gerer = DroitsMoteur::new(vec![droit("facturation.gerer", "etablissement")]);
        assert!(gerer.peut_lire_facturation() && gerer.peut_gerer_facturation());
        let v = adresses(droits_du_role("prof"), &DroitsMoteur::default());
        assert!(!v.contains(&COMPTABILITE.to_string()));
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
        assert_eq!(liens.len(), 2);
        assert_eq!(liens[0].href, SECURITE);
        assert_eq!(
            liens[1].href, NOTIFICATIONS,
            "préférences de notification pour tous"
        );
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
