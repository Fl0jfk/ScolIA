//! **Fournisseurs d'extrait** (socle 27, §6, R-RG14) : squelette du lot G0.
//!
//! Le droit d'accès (art. 15) assemble, pour une personne et une période, des **sections typées**
//! fournies par chaque socle. Ce fichier déclare le port et **qui couvre quelles tables** ;
//! l'assemblage, le masquage des tiers (R-RG15) et la remise viennent avec le lot G3. Rien ici ne
//! lit de donnée : le test ne vérifie que la couverture déclarée.
//!
//! Remplace l'ancien drapeau `export_portabilite` du registre (RG-D05), qui n'avait aucun lecteur.

use crate::registre_donnees::{ClasseTable, Portee};

/// Une section d'extrait renvoyée par un fournisseur (forme figée pour le lot G3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionExtrait {
    pub titre: String,
    pub lignes: Vec<String>,
    /// Références de pièces (documents du socle 7), jamais leur contenu.
    pub pieces: Vec<String>,
    pub destinataires_habituels: Vec<String>,
    pub sources: Vec<String>,
    /// Mentions de tiers à masquer avant remise (R-RG15) : rien ne part sans relecture.
    pub mentions_tiers: Vec<String>,
}

/// État d'un fournisseur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatFournisseur {
    /// Déclaré, pas encore écrit (lot G3).
    Declare,
    Implemente,
}

/// Un fournisseur déclaré : un code et les domaines du registre des tables qu'il couvre.
#[derive(Clone, Copy, Debug)]
pub struct Fournisseur {
    pub code: &'static str,
    pub socle: &'static str,
    pub domaines: &'static [&'static str],
    pub etat: EtatFournisseur,
}

const fn f(
    code: &'static str,
    socle: &'static str,
    domaines: &'static [&'static str],
) -> Fournisseur {
    Fournisseur {
        code,
        socle,
        domaines,
        etat: EtatFournisseur::Declare,
    }
}

/// Fournisseurs déclarés, par socle propriétaire.
pub const FOURNISSEURS: &[Fournisseur] = &[
    f("eleve", "2", &["dossier_eleve", "structure_scolaire"]),
    f("inscriptions", "3", &["inscriptions"]),
    f("administratif", "4", &["dossier_personnes"]),
    f("personnels", "6", &["personnels"]),
    f("assiduite", "12", &["assiduite"]),
    f("vie_scolaire", "13", &["vie_scolaire"]),
    f("cahier_de_textes", "14", &["cahier_textes"]),
    f("absences_personnels", "15", &["personnels_absences"]),
    f("messagerie", "16", &["messagerie"]),
    f("evaluations", "21", &["evaluations_pedagogie"]),
    f("finances", "24", &["finances"]),
    f(
        "emploi_du_temps",
        "10",
        &["emploi_du_temps_structure", "portails"],
    ),
    f("communication", "8", &["communication_technique"]),
    f("demandes", "43", &["demandes"]),
    f("sorties_voyages", "42", &["sorties_voyages"]),
    f("stages", "39", &["stages"]),
    f("vie_pratique", "41", &["vie_pratique"]),
    f("sante", "34", &["sante"]),
    f("comptes", "1", &["identite_acces"]),
    f("conformite", "27", &["conformite"]),
];

/// Tables « données personnelles » qui n'entrent pas dans un extrait, avec la raison.
/// Règle générale : l'état technique éphémère (sessions, défis, codes) n'est pas une donnée de l'extrait.
pub fn hors_extrait(t: &ClasseTable) -> Option<&'static str> {
    if matches!(t.portee, Portee::Technique) {
        return Some("état technique éphémère (sessions, défis, codes à usage unique) : sans valeur pour la personne");
    }
    None
}

/// Fournisseurs qui couvrent une table.
pub fn fournisseurs_de(t: &ClasseTable) -> impl Iterator<Item = &'static Fournisseur> + '_ {
    FOURNISSEURS
        .iter()
        .filter(move |p| p.domaines.contains(&t.domaine))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registre_donnees::REGISTRE;
    use std::collections::BTreeSet;

    /// R-RG14 : chaque table « données personnelles » est couverte par au moins un fournisseur
    /// ou listée « hors extrait, raison ».
    #[test]
    fn chaque_table_de_donnees_personnelles_est_couverte() {
        let non_couvertes: Vec<_> = REGISTRE
            .iter()
            .filter(|t| t.donnees_personnelles)
            .filter(|t| hors_extrait(t).is_none() && fournisseurs_de(t).next().is_none())
            .map(|t| format!("{} ({})", t.nom, t.domaine))
            .collect();
        assert!(
            non_couvertes.is_empty(),
            "tables sans fournisseur d'extrait ni mention « hors extrait » (R-RG14) : {non_couvertes:?}"
        );
    }

    #[test]
    fn un_fournisseur_ne_declare_que_des_domaines_existants() {
        let domaines: BTreeSet<_> = REGISTRE.iter().map(|t| t.domaine).collect();
        let codes: BTreeSet<_> = FOURNISSEURS.iter().map(|p| p.code).collect();
        assert_eq!(
            codes.len(),
            FOURNISSEURS.len(),
            "code de fournisseur en double"
        );
        for p in FOURNISSEURS {
            for d in p.domaines {
                assert!(domaines.contains(d), "{} : domaine inconnu « {d} »", p.code);
            }
        }
    }
}
