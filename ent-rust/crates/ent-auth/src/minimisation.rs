//! Groupes de lecture du dossier administratif (socle 4, lot A1 ; `04_ADMINISTRATIF.md` §12.2).
//!
//! **Fonction pure, un seul point de décision** : à partir des droits de l'acteur (déjà évalués
//! par le moteur de droits, portée comprise) et de sa situation vis-à-vis de l'élève (conflit
//! d'intérêts, ou titulaire), elle dit quels groupes de champs sont ouverts et, pour chaque
//! groupe fermé, **pourquoi**. Le service n'applique aucune autre règle de minimisation : il
//! filtre les champs d'après cette vue, et renvoie les raisons à l'écran.
//!
//! | Groupe | Contenu | Ouvert par |
//! |---|---|---|
//! | F0 | noms, nature du lien, « joignable » | `responsables.lire` |
//! | F1 | e-mails, téléphones | `responsables.lire` |
//! | F2 | adresses ordinaires, foyer | `responsables.adresses.lire` |
//! | F3 | qualité, autorité, droits du lien, décisions sans motif | `responsables.juridique.lire` |
//! | F4 | adresses protégées, motifs scellés | `responsables.protege.lire` |
//! | R | remise, interdictions (consigne), contacts d'urgence | `remise.lire` |
//!
//! Les payeurs ne sont pas dans F3 : ils suivent `payeurs.definir` (la comptabilité les voit sans
//! la situation juridique ; la vie scolaire voit la situation juridique sans les payeurs).

use crate::Action;

/// Groupe de champs (minimisation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Groupe {
    F0,
    F1,
    F2,
    F3,
    F4,
    R,
}

impl Groupe {
    pub const TOUS: [Groupe; 6] = [
        Groupe::F0,
        Groupe::F1,
        Groupe::F2,
        Groupe::F3,
        Groupe::F4,
        Groupe::R,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::F0 => "F0",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::R => "R",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::F0 => "Désignation des responsables",
            Self::F1 => "Moyens de joindre les responsables",
            Self::F2 => "Adresses des responsables",
            Self::F3 => "Situation juridique",
            Self::F4 => "Données protégées",
            Self::R => "Remise, interdictions et contacts d'urgence",
        }
    }

    /// Action qui ouvre le groupe pour le personnel.
    pub fn action(self) -> Action {
        match self {
            Self::F0 | Self::F1 => Action::ResponsablesLire,
            Self::F2 => Action::ResponsablesAdressesLire,
            Self::F3 => Action::ResponsablesJuridiqueLire,
            Self::F4 => Action::ResponsablesProtegeLire,
            Self::R => Action::RemiseLire,
        }
    }
}

/// Pourquoi un groupe est fermé (renvoyé à l'écran, jamais un simple champ vide).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Raison {
    /// Il manque le droit (ou la portée) qui ouvre ce groupe ; l'action à demander est donnée.
    DroitManquant(Action),
    /// Conflit d'intérêts : l'agent est lui-même lié à l'élève (R-A18) ; ses droits
    /// professionnels n'ouvrent pas les données des autres personnes.
    ConflitInteret,
    /// Famille : donnée d'une autre personne que soi (droit d'accès sans les données de l'autre).
    AutreResponsable,
    /// Famille : donnée réservée à l'établissement (jamais montrée à une famille).
    ReserveEtablissement,
}

impl Raison {
    pub fn code(self) -> &'static str {
        match self {
            Self::DroitManquant(_) => "droit_manquant",
            Self::ConflitInteret => "conflit_interet",
            Self::AutreResponsable => "autre_responsable",
            Self::ReserveEtablissement => "reserve_etablissement",
        }
    }

    /// Droit à demander, quand la raison est un droit manquant.
    pub fn action_a_demander(self) -> Option<Action> {
        match self {
            Self::DroitManquant(a) => Some(a),
            _ => None,
        }
    }

    /// Message affichable (français simple, sans détail sensible : ne dit pas *qui* est lié).
    pub fn message(self) -> &'static str {
        match self {
            Self::DroitManquant(_) => "Votre rôle n'ouvre pas cette information.",
            Self::ConflitInteret => {
                "Vous êtes vous-même lié à cet élève : cette information est réservée à un collègue."
            }
            Self::AutreResponsable => "Cette information concerne un autre responsable.",
            Self::ReserveEtablissement => "Cette information est réservée à l'établissement.",
        }
    }
}

/// Un groupe fermé et sa raison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Masquage {
    pub groupe: Groupe,
    pub raison: Raison,
}

/// Droits du personnel sur **cet élève** (portées déjà résolues par le moteur).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DroitsPersonnel {
    pub responsables_lire: bool,
    pub adresses_lire: bool,
    pub juridique_lire: bool,
    pub protege_lire: bool,
    pub remise_lire: bool,
    pub payeurs_definir: bool,
}

/// Vue calculée : groupes ouverts, payeurs, raisons des groupes fermés.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VueGroupes {
    pub f0: bool,
    pub f1: bool,
    pub f2: bool,
    pub f3: bool,
    pub f4: bool,
    pub r: bool,
    /// Parts de paiement visibles (hors F3).
    pub payeurs: bool,
    /// Conflit d'intérêts : tout accès ouvert doit être tracé comme accès sensible.
    pub conflit_interet: bool,
    pub masquages: Vec<Masquage>,
}

impl VueGroupes {
    pub fn ouvert(&self, g: Groupe) -> bool {
        match g {
            Groupe::F0 => self.f0,
            Groupe::F1 => self.f1,
            Groupe::F2 => self.f2,
            Groupe::F3 => self.f3,
            Groupe::F4 => self.f4,
            Groupe::R => self.r,
        }
    }

    pub fn raison(&self, g: Groupe) -> Option<Raison> {
        self.masquages.iter().find(|m| m.groupe == g).map(|m| m.raison)
    }

    /// Au moins un groupe lisible ?
    pub fn quelque_chose(&self) -> bool {
        self.f0 || self.f1 || self.f2 || self.f3 || self.f4 || self.r
    }

    /// Un accès à tracer (conflit d'intérêts et au moins un groupe ouvert).
    pub fn acces_a_tracer(&self) -> bool {
        self.conflit_interet && (self.f0 || self.f1 || self.r)
    }

    fn completer(mut self, raison_defaut: impl Fn(Groupe) -> Raison) -> Self {
        self.masquages = Groupe::TOUS
            .into_iter()
            .filter(|g| !self.ouvert(*g))
            .map(|g| Masquage {
                groupe: g,
                raison: raison_defaut(g),
            })
            .collect();
        self
    }
}

/// Personnel : droits résolus + éventuel conflit d'intérêts (R-A18).
///
/// Avec conflit : F2, F3, F4 et les payeurs sont fermés **quels que soient les droits** ; F0, F1
/// et R restent ouverts s'ils le sont (avec trace d'accès sensible, côté service).
pub fn vue_personnel(d: DroitsPersonnel, conflit_interet: bool) -> VueGroupes {
    let ferme = |ouvert: bool| ouvert && !conflit_interet;
    let v = VueGroupes {
        f0: d.responsables_lire,
        f1: d.responsables_lire,
        f2: ferme(d.adresses_lire),
        f3: ferme(d.juridique_lire),
        f4: ferme(d.protege_lire),
        r: d.remise_lire,
        payeurs: ferme(d.payeurs_definir),
        conflit_interet,
        masquages: Vec::new(),
    };
    v.completer(|g| {
        if conflit_interet && matches!(g, Groupe::F2 | Groupe::F3 | Groupe::F4) {
            Raison::ConflitInteret
        } else {
            Raison::DroitManquant(g.action())
        }
    })
}

/// Famille (titulaire du lien actif). `est_moi` : la donnée est celle du titulaire lui-même.
/// Un titulaire voit **ses** données (F0 à F3, sans F4) et, des autres responsables, la
/// désignation seulement (nom, nature du lien) : ni coordonnées, ni adresse, ni situation
/// juridique, ni payeurs, jamais F4.
pub fn vue_titulaire(est_moi: bool) -> VueGroupes {
    let v = VueGroupes {
        f0: true,
        f1: est_moi,
        f2: est_moi,
        f3: est_moi,
        f4: false,
        r: true,
        payeurs: est_moi,
        conflit_interet: false,
        masquages: Vec::new(),
    };
    v.completer(|g| {
        if g == Groupe::F4 {
            Raison::ReserveEtablissement
        } else {
            Raison::AutreResponsable
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{modele_par_defaut, Portee, Role};

    fn droits_modele(r: Role) -> DroitsPersonnel {
        let m = modele_par_defaut(r);
        let a = |x: Action| {
            m.iter().any(|(y, p)| {
                (*y == x || x.accordee_par().contains(y))
                    && matches!(p, Portee::Etablissement | Portee::MesClasses)
            })
        };
        DroitsPersonnel {
            responsables_lire: a(Action::ResponsablesLire),
            adresses_lire: a(Action::ResponsablesAdressesLire),
            juridique_lire: a(Action::ResponsablesJuridiqueLire),
            protege_lire: a(Action::ResponsablesProtegeLire),
            remise_lire: a(Action::RemiseLire),
            payeurs_definir: a(Action::PayeursDefinir),
        }
    }

    /// Oracle : profil × groupe, par défaut (O = ouvert, dans l'ordre F0 F1 F2 F3 F4 R payeurs).
    #[test]
    fn oracle_profils_par_groupe() {
        let attendu: [(Role, &str); 9] = [
            (Role::Admin, "OOOOOOO"),
            (Role::Direction, "OOOOOOO"),
            (Role::Administratif, "OOOO.OO"),
            (Role::Cpe, "OO.O.O."),
            (Role::Compta, "OOO...O"),
            (Role::Prof, "OO...O."),
            (Role::Infirmerie, "OO...O."),
            (Role::Psy, "......."),
            (Role::Accueil, "OO...O."),
        ];
        for (role, motif) in attendu {
            let v = vue_personnel(droits_modele(role), false);
            let obtenu: String = [v.f0, v.f1, v.f2, v.f3, v.f4, v.r, v.payeurs]
                .iter()
                .map(|b| if *b { 'O' } else { '.' })
                .collect();
            assert_eq!(obtenu, motif, "{role:?}");
        }
        for r in [Role::Parent, Role::Eleve] {
            assert_eq!(droits_modele(r), DroitsPersonnel::default(), "{r:?}");
        }
    }

    #[test]
    fn toute_fermeture_a_une_raison() {
        for role in [Role::Cpe, Role::Prof, Role::Psy, Role::Accueil] {
            let v = vue_personnel(droits_modele(role), false);
            for g in Groupe::TOUS {
                assert_eq!(v.ouvert(g), v.raison(g).is_none(), "{role:?} {g:?}");
                if let Some(Raison::DroitManquant(a)) = v.raison(g) {
                    assert_eq!(a, g.action());
                }
            }
        }
    }

    /// R-A18 : le conflit ferme F2, F3, F4 et les payeurs quels que soient les droits ; F0, F1, R
    /// restent ouverts et l'accès doit être tracé.
    #[test]
    fn conflit_d_interet_ferme_f2_f3_f4() {
        let v = vue_personnel(droits_modele(Role::Direction), true);
        assert!(v.f0 && v.f1 && v.r);
        assert!(!v.f2 && !v.f3 && !v.f4 && !v.payeurs);
        assert_eq!(v.raison(Groupe::F2), Some(Raison::ConflitInteret));
        assert_eq!(v.raison(Groupe::F4), Some(Raison::ConflitInteret));
        assert!(v.acces_a_tracer());
        let sans = vue_personnel(droits_modele(Role::Direction), false);
        assert!(!sans.acces_a_tracer());
    }

    #[test]
    fn titulaire_ne_voit_pas_l_autre_responsable() {
        let soi = vue_titulaire(true);
        assert!(soi.f0 && soi.f1 && soi.f2 && soi.f3 && soi.payeurs && !soi.f4);
        let autre = vue_titulaire(false);
        assert!(autre.f0 && !autre.f1 && !autre.f2 && !autre.f3 && !autre.f4 && !autre.payeurs);
        assert_eq!(autre.raison(Groupe::F1), Some(Raison::AutreResponsable));
        assert_eq!(autre.raison(Groupe::F4), Some(Raison::ReserveEtablissement));
        assert_eq!(soi.raison(Groupe::F4), Some(Raison::ReserveEtablissement));
    }
}
