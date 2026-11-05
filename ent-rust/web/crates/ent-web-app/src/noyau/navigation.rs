//! Registre de navigation : les modules déclarent leurs entrées, filtrées par les droits.
//! La même liste sert à la barre latérale, au rail et à la barre d'onglets.

use ent_web_ui::{EntreeNav, NomIcone};

use super::droits::Droits;

pub const ACCUEIL: &str = "/web";
pub const EMPLOI_DU_TEMPS: &str = "/web/emploi-du-temps";
pub const APPEL: &str = "/web/appel";
pub const CLASSES: &str = "/web/classes";
pub const MES_CLASSES: &str = "/web/mes-classes";
pub const VIE_SCOLAIRE: &str = "/web/vie-scolaire";
pub const JOURNEE: &str = "/web/journee";
pub const ABSENCES: &str = "/web/absences";
pub const MESSAGES: &str = "/web/messages";
pub const SECURITE: &str = "/web/securite";
pub const POLITIQUE_SECURITE: &str = "/web/direction/securite";

fn entree(libelle: &str, href: &str, icone: NomIcone) -> EntreeNav {
    EntreeNav {
        libelle: libelle.into(),
        href: href.into(),
        icone,
        badge: None,
    }
}

/// Entrées visibles pour ces droits, dans l'ordre d'affichage.
pub fn entrees(droits: Droits) -> Vec<EntreeNav> {
    let mut v = vec![entree("Aujourd'hui", ACCUEIL, NomIcone::Accueil)];
    if droits.emploi_du_temps {
        v.push(entree("Emploi du temps", EMPLOI_DU_TEMPS, NomIcone::Agenda));
    }
    if droits.faire_appel {
        v.push(entree("Appel", APPEL, NomIcone::Appel));
    }
    if droits.mes_classes {
        v.push(entree("Mes classes", MES_CLASSES, NomIcone::Ecole));
    }
    if droits.voir_classes {
        v.push(entree("Classes", CLASSES, NomIcone::Ecole));
    }
    if droits.trancher_justificatif {
        v.push(entree("Vie scolaire", VIE_SCOLAIRE, NomIcone::Personne));
        v.push(entree("Journée", JOURNEE, NomIcone::Agenda));
    }
    if droits.portail_famille || droits.mes_absences {
        v.push(entree("Absences", ABSENCES, NomIcone::Personne));
    }
    if droits.messagerie {
        v.push(entree("Messages", MESSAGES, NomIcone::Messages));
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
            [ACCUEIL, EMPLOI_DU_TEMPS, APPEL, MES_CLASSES, MESSAGES]
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
                APPEL,
                CLASSES,
                VIE_SCOLAIRE,
                JOURNEE,
                MESSAGES
            ]
        );
        let compta: Vec<String> = entrees(droits_du_role("compta"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(
            compta,
            [ACCUEIL, CLASSES],
            "ni vie scolaire, ni absences, pas de messagerie"
        );
        let parent: Vec<String> = entrees(droits_du_role("parent"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(
            parent,
            [ACCUEIL, ABSENCES, MESSAGES],
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
                APPEL,
                CLASSES,
                VIE_SCOLAIRE,
                JOURNEE,
                MESSAGES
            ]
        );
        // Un enseignant lit les absences mais ne tranche pas : pas d'entrée « Vie scolaire ».
        assert!(!prof.contains(&VIE_SCOLAIRE.to_string()));
        let eleve: Vec<String> = entrees(droits_du_role("eleve"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(eleve, [ACCUEIL, EMPLOI_DU_TEMPS, ABSENCES, MESSAGES]);
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
