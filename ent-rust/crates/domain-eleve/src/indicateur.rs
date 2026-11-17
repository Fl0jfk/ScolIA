//! Socle 2 (l'élève), lot E8 — catalogue des indicateurs (« pastilles ») et contrat des tuiles de la
//! synthèse de la fiche (§7.11, §14).
//!
//! * Une **pastille** est un fait court affiché sur la fiche (interne, PAI, sans INE…). Chaque
//!   domaine **déclare** les siennes (code, libellé, gravité) et **l'action qui les ouvre** ; le
//!   socle 2 ne fait que les projeter et les filtrer par les droits de l'acteur.
//! * Une **tuile** est l'emplacement d'un domaine dans la synthèse : un contrat (domaine, action
//!   qui l'ouvre, pastilles qu'elle porte, lien vers l'écran du domaine). Une tuile sans droit n'est
//!   pas vide : elle est **masquée avec sa raison** (« jamais de masquage muet »).
//!
//! Logique **pure** : rien ici ne lit la base. Les actions sont désignées par leur code
//! (`ent_auth::Action::code`), le crate de domaine ne dépend pas de l'authentification ; un test
//! d'`ent-db` vérifie que chaque code existe.

use serde::Serialize;

/// Gravité d'une pastille (ordre d'affichage : alerte d'abord).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Gravite {
    Alerte,
    Attention,
    Information,
}

impl Gravite {
    pub fn code(self) -> &'static str {
        match self {
            Self::Alerte => "alerte",
            Self::Attention => "attention",
            Self::Information => "information",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [Self::Alerte, Self::Attention, Self::Information].into_iter().find(|g| g.code() == s)
    }
}

/// Déclaration d'une pastille par son domaine propriétaire.
#[derive(Debug, Clone, Copy)]
pub struct DefIndicateur {
    pub code: &'static str,
    /// Domaine propriétaire (code de tuile du contrat ci-dessous).
    pub domaine: &'static str,
    pub libelle: &'static str,
    pub icone: &'static str,
    pub gravite: Gravite,
    /// Codes d'actions dont **l'une suffit** pour voir la pastille (sur cet élève).
    pub actions_requises: &'static [&'static str],
    /// Types d'événements qui déclenchent le recalcul de la pastille d'un élève.
    pub evenements: &'static [&'static str],
    /// La pastille dépend de la date (échéance d'un PAI, majorité…) : recalculée aussi la nuit.
    pub depend_du_temps: bool,
}

/// Événements qui touchent à l'état de scolarité ou à l'existence même de la fiche : toute
/// pastille doit en tenir compte (un élève sorti n'est plus « sans INE » à relancer).
const EV_FICHE: &[&str] = &[
    "eleve.created",
    "eleve.status_changed",
    "eleve.sortie",
    "eleve.reintegre",
    "eleve.fusionne",
    "eleve.defusionne",
    "scolarite.opened",
    "scolarite.closed",
];

/// Catalogue des pastilles du socle 2 et de ses domaines voisins (lot E8, R-E9).
///
/// Les trois premières sont les pastilles de référence du lot (interne, PAI, sans INE) ; les
/// suivantes sont celles que la conception cite (§7.11, R-E9) et que les tables existantes
/// permettent de calculer sans rien inventer.
pub const CATALOGUE_INDICATEURS: &[DefIndicateur] = &[
    DefIndicateur {
        code: "interne",
        domaine: "scolarite",
        libelle: "Interne",
        icone: "lit",
        gravite: Gravite::Information,
        actions_requises: &["eleves.lire", "internat.gerer"],
        evenements: &["eleve.regime_changed"],
        depend_du_temps: true,
    },
    DefIndicateur {
        code: "pai_actif",
        domaine: "sante",
        libelle: "PAI en cours",
        icone: "croix",
        gravite: Gravite::Alerte,
        actions_requises: &["sante.lire"],
        evenements: &["infirmerie.pai", "infirmerie.pai_signe"],
        depend_du_temps: true,
    },
    DefIndicateur {
        code: "sans_ine",
        domaine: "identifiants",
        libelle: "Sans INE",
        icone: "identifiant",
        gravite: Gravite::Attention,
        actions_requises: &["eleves.identifiants.lire"],
        evenements: &["eleve.identifiant_change"],
        depend_du_temps: false,
    },
    DefIndicateur {
        code: "ine_conteste",
        domaine: "identifiants",
        libelle: "INE contesté",
        icone: "identifiant",
        gravite: Gravite::Alerte,
        actions_requises: &["eleves.identifiants.lire"],
        evenements: &["eleve.identifiant_change"],
        depend_du_temps: false,
    },
    DefIndicateur {
        code: "confidentialite_renforcee",
        domaine: "protection",
        libelle: "Confidentialité renforcée",
        icone: "cadenas",
        gravite: Gravite::Alerte,
        actions_requises: &["eleves.confidentiel.gerer"],
        evenements: &["eleve.confidentialite_changee"],
        depend_du_temps: true,
    },
    DefIndicateur {
        code: "majeur",
        domaine: "identite",
        libelle: "Majeur",
        icone: "calendrier",
        gravite: Gravite::Information,
        actions_requises: &["eleves.lire"],
        evenements: &["eleve.majeur", "eleve.identity_changed", "eleve.capacite_changee"],
        depend_du_temps: true,
    },
    DefIndicateur {
        code: "en_mobilite",
        domaine: "scolarite",
        libelle: "En mobilité",
        icone: "avion",
        gravite: Gravite::Information,
        actions_requises: &["eleves.lire"],
        evenements: &["scolarite.mobilite_ouverte", "scolarite.mobilite_fermee"],
        depend_du_temps: true,
    },
    DefIndicateur {
        code: "redoublant",
        domaine: "scolarite",
        libelle: "Redoublant",
        icone: "boucle",
        gravite: Gravite::Information,
        actions_requises: &["eleves.lire"],
        evenements: &["eleve.scolarite_annee_enregistree"],
        depend_du_temps: false,
    },
];

/// Événements écoutés par le consommateur : la **réunion** des événements du catalogue et des
/// événements de fiche (un test garantit que cette liste est exactement cette réunion).
pub const EVENEMENTS_ECOUTES: &[&str] = &[
    "eleve.created",
    "eleve.status_changed",
    "eleve.sortie",
    "eleve.reintegre",
    "eleve.fusionne",
    "eleve.defusionne",
    "scolarite.opened",
    "scolarite.closed",
    "eleve.regime_changed",
    "infirmerie.pai",
    "infirmerie.pai_signe",
    "eleve.identifiant_change",
    "eleve.confidentialite_changee",
    "eleve.majeur",
    "eleve.identity_changed",
    "eleve.capacite_changee",
    "scolarite.mobilite_ouverte",
    "scolarite.mobilite_fermee",
    "eleve.scolarite_annee_enregistree",
];

pub fn indicateur(code: &str) -> Option<&'static DefIndicateur> {
    CATALOGUE_INDICATEURS.iter().find(|d| d.code == code)
}

/// Pastilles qui se recalculent sur ce type d'événement (les événements de fiche valent pour
/// toutes).
pub fn indicateurs_de_l_evenement(type_evenement: &str) -> Vec<&'static DefIndicateur> {
    let fiche = EV_FICHE.contains(&type_evenement);
    CATALOGUE_INDICATEURS
        .iter()
        .filter(|d| fiche || d.evenements.contains(&type_evenement))
        .collect()
}

/// Pourquoi une tuile est masquée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RaisonTuile {
    /// L'acteur n'a aucune des actions qui ouvrent la tuile.
    DroitManquant,
    /// L'acteur a l'action, mais pas sur cet élève (portée).
    HorsPortee,
    /// Réservé au personnel habilité (famille et élève : jamais).
    ReserveAuPersonnel,
}

impl RaisonTuile {
    pub fn code(self) -> &'static str {
        match self {
            Self::DroitManquant => "droit_manquant",
            Self::HorsPortee => "hors_portee",
            Self::ReserveAuPersonnel => "reserve_au_personnel",
        }
    }
}

/// Contrat d'une tuile de synthèse : ce qu'un domaine garantit à la fiche.
#[derive(Debug, Clone, Copy)]
pub struct ContratTuile {
    /// Code du domaine (identique à `DefIndicateur::domaine`).
    pub domaine: &'static str,
    pub libelle: &'static str,
    /// Codes d'actions dont l'une suffit pour ouvrir la tuile.
    pub actions_requises: &'static [&'static str],
    /// Modèle de lien vers l'écran du domaine (`{eleve}` est remplacé par l'identifiant).
    pub lien: &'static str,
    /// Le personnel seul voit la tuile (famille et élève : jamais).
    pub personnel_seul: bool,
    /// Ordre d'affichage dans la grille.
    pub rang: u8,
}

/// Contrats des tuiles du socle 2. Les autres domaines (vie scolaire, évaluations, finances,
/// documents…) y ajoutent le leur en déclarant leurs pastilles dans le catalogue ci-dessus et
/// leur contrat ici : la synthèse ne connaît aucun domaine en dur.
pub const CONTRATS_TUILES: &[ContratTuile] = &[
    ContratTuile {
        domaine: "identite",
        libelle: "Identité",
        actions_requises: &["eleves.etat_civil.lire"],
        lien: "/web/eleves/{eleve}/identite",
        personnel_seul: false,
        rang: 10,
    },
    ContratTuile {
        domaine: "identifiants",
        libelle: "Identifiants",
        actions_requises: &["eleves.identifiants.lire"],
        lien: "/web/eleves/{eleve}/identite#identifiants",
        personnel_seul: false,
        rang: 20,
    },
    ContratTuile {
        domaine: "scolarite",
        libelle: "Scolarité",
        actions_requises: &["eleves.lire", "internat.gerer"],
        lien: "/web/eleves/{eleve}/scolarite",
        personnel_seul: false,
        rang: 30,
    },
    ContratTuile {
        domaine: "sante",
        libelle: "Santé",
        actions_requises: &["sante.lire"],
        lien: "/web/eleves/{eleve}/sante",
        personnel_seul: true,
        rang: 40,
    },
    ContratTuile {
        domaine: "protection",
        libelle: "Confidentialité",
        actions_requises: &["eleves.confidentiel.gerer"],
        lien: "/web/eleves/{eleve}/confidentialite",
        personnel_seul: true,
        rang: 90,
    },
];

pub fn contrat_tuile(domaine: &str) -> Option<&'static ContratTuile> {
    CONTRATS_TUILES.iter().find(|c| c.domaine == domaine)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn les_codes_du_catalogue_sont_uniques_et_leurs_domaines_ont_un_contrat() {
        let codes: BTreeSet<_> = CATALOGUE_INDICATEURS.iter().map(|d| d.code).collect();
        assert_eq!(codes.len(), CATALOGUE_INDICATEURS.len(), "code de pastille en double");
        for d in CATALOGUE_INDICATEURS {
            assert!(contrat_tuile(d.domaine).is_some(), "{} : domaine {} sans contrat de tuile", d.code, d.domaine);
            assert!(!d.actions_requises.is_empty(), "{} : une pastille sans action serait publique", d.code);
            assert!(!d.libelle.is_empty() && !d.icone.is_empty());
        }
    }

    #[test]
    fn les_domaines_de_tuile_sont_uniques_et_leurs_rangs_distincts() {
        let d: BTreeSet<_> = CONTRATS_TUILES.iter().map(|c| c.domaine).collect();
        let r: BTreeSet<_> = CONTRATS_TUILES.iter().map(|c| c.rang).collect();
        assert_eq!(d.len(), CONTRATS_TUILES.len());
        assert_eq!(r.len(), CONTRATS_TUILES.len());
        for c in CONTRATS_TUILES {
            assert!(c.lien.contains("{eleve}"));
            assert!(!c.actions_requises.is_empty());
        }
    }

    #[test]
    fn les_evenements_ecoutes_sont_exactement_ceux_du_catalogue_et_de_la_fiche() {
        let mut attendus: BTreeSet<&str> = EV_FICHE.iter().copied().collect();
        for d in CATALOGUE_INDICATEURS {
            attendus.extend(d.evenements.iter().copied());
        }
        let ecoutes: BTreeSet<&str> = EVENEMENTS_ECOUTES.iter().copied().collect();
        assert_eq!(ecoutes, attendus);
        assert_eq!(EVENEMENTS_ECOUTES.len(), ecoutes.len(), "événement écouté deux fois");
    }

    #[test]
    fn un_evenement_de_fiche_recalcule_les_pastilles_qui_en_dependent() {
        let codes = |t: &str| indicateurs_de_l_evenement(t).into_iter().map(|d| d.code).collect::<Vec<_>>();
        assert!(codes("eleve.sortie").contains(&"sans_ine"));
        assert!(codes("eleve.sortie").contains(&"interne"));
        assert!(codes("eleve.fusionne").contains(&"pai_actif"));
        assert_eq!(codes("infirmerie.pai"), ["pai_actif"]);
        assert_eq!(codes("eleve.regime_changed"), ["interne"]);
        assert!(codes("eleve.inconnu").is_empty());
    }

    #[test]
    fn gravite_ordonnee_alerte_d_abord() {
        let mut v = vec![Gravite::Information, Gravite::Alerte, Gravite::Attention];
        v.sort();
        assert_eq!(v, [Gravite::Alerte, Gravite::Attention, Gravite::Information]);
        assert_eq!(Gravite::parse("attention"), Some(Gravite::Attention));
        assert_eq!(Gravite::parse("x"), None);
    }
}
