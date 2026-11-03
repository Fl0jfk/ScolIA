//! Registre de navigation : les modules déclarent leurs entrées, filtrées par les droits.
//! La même liste sert à la barre latérale, au rail et à la barre d'onglets.

use ent_web_ui::{EntreeNav, NomIcone};

use super::droits::Droits;

pub const ACCUEIL: &str = "/web";
pub const EMPLOI_DU_TEMPS: &str = "/web/emploi-du-temps";
pub const APPEL: &str = "/web/appel";

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
    v
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
        assert_eq!(prof, [ACCUEIL, EMPLOI_DU_TEMPS, APPEL]);
        let parent = entrees(droits_du_role("parent"));
        assert_eq!(parent.len(), 1, "une famille ne voit pas l'appel");
        let eleve: Vec<String> = entrees(droits_du_role("eleve"))
            .into_iter()
            .map(|e| e.href)
            .collect();
        assert_eq!(eleve, [ACCUEIL, EMPLOI_DU_TEMPS]);
    }

    #[test]
    fn rail_sur_les_ecrans_de_saisie() {
        assert!(ecran_de_saisie("/web/appel"));
        assert!(!ecran_de_saisie("/web"));
        assert!(!ecran_de_saisie("/web/emploi-du-temps"));
    }
}
