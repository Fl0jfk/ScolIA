//! Socle 2 (l'élève), lot E1 — groupes de champs et minimisation en UN seul point.
//!
//! Les champs de l'élève sont rangés en groupes (`docs/socles/02_ELEVE.md` §10.2).
//! Chaque groupe s'ouvre par une action du catalogue `ent_auth::Action`, avec une
//! portée ; ce module ne connaît que les **codes** d'actions (pas la crate des
//! droits) : l'appelant (`ent-db`) traduit les droits effectifs en
//! [`ProfilChamps`], puis [`ProfilChamps::pour`] donne les groupes ouverts pour un
//! élève précis, avec la **raison** de chaque masquage (renvoyée à l'écran).
//!
//! Correspondance champ → groupe (les trois niveaux historiques en sont des cas
//! particuliers : « complet » = G1+G2+G3, « santé » = G2, « réduit » = rien de plus
//! que G0) :
//!
//! | Groupe | Champs | Action |
//! |---|---|---|
//! | G0 désignation | nom, prénom, classe, statut, site, photo | `eleves.annuaire` |
//! | G1 scolarité | régime, MEF, dates d'entrée et de sortie, motif de sortie, redoublant | `eleves.lire` |
//! | G2 état civil | sexe, date de naissance (et âge) | `eleves.etat_civil.lire` |
//! | G3 identifiants | INE, badge | `eleves.identifiants.lire` |
//! | G5 historique | versions d'identité non sensibles | `eleves.historique.lire` |
//! | G6 sensible | versions sensibles, motif de confidentialité | `eleves.confidentiel.gerer` |

use std::collections::HashSet;

use serde::Serialize;
use uuid::Uuid;

use crate::{DossierEleve, Eleve, NiveauIdentite};

/// Groupe de champs de l'élève.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupeChamps {
    Designation,
    Scolarite,
    EtatCivil,
    Identifiants,
    Historique,
    Sensible,
}

impl GroupeChamps {
    pub const TOUS: [GroupeChamps; 6] = [
        Self::Designation,
        Self::Scolarite,
        Self::EtatCivil,
        Self::Identifiants,
        Self::Historique,
        Self::Sensible,
    ];

    fn rang(self) -> usize {
        match self {
            Self::Designation => 0,
            Self::Scolarite => 1,
            Self::EtatCivil => 2,
            Self::Identifiants => 3,
            Self::Historique => 4,
            Self::Sensible => 5,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Designation => "designation",
            Self::Scolarite => "scolarite",
            Self::EtatCivil => "etat_civil",
            Self::Identifiants => "identifiants",
            Self::Historique => "historique",
            Self::Sensible => "sensible",
        }
    }

    /// Code de l'action du catalogue qui ouvre ce groupe.
    pub fn action(self) -> &'static str {
        match self {
            Self::Designation => "eleves.annuaire",
            Self::Scolarite => "eleves.lire",
            Self::EtatCivil => "eleves.etat_civil.lire",
            Self::Identifiants => "eleves.identifiants.lire",
            Self::Historique => "eleves.historique.lire",
            Self::Sensible => "eleves.confidentiel.gerer",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Designation => "désignation",
            Self::Scolarite => "scolarité",
            Self::EtatCivil => "état civil",
            Self::Identifiants => "identifiants",
            Self::Historique => "historique d'identité",
            Self::Sensible => "données sensibles",
        }
    }
}

/// Pourquoi un groupe est masqué (affiché à l'écran : jamais de masquage muet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RaisonMasquage {
    /// Le rôle n'a pas l'action qui ouvre ce groupe.
    DroitManquant,
    /// Le rôle a l'action, mais pas sur cet élève (portée).
    HorsPortee,
    /// Réservé au personnel habilité (famille et élève : jamais).
    ReserveAuPersonnel,
}

impl RaisonMasquage {
    pub fn code(self) -> &'static str {
        match self {
            Self::DroitManquant => "droit_manquant",
            Self::HorsPortee => "hors_portee",
            Self::ReserveAuPersonnel => "reserve_au_personnel",
        }
    }
}

/// Ligne « champ masqué » renvoyée aux écrans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MasqueChamps {
    pub groupe: &'static str,
    pub raison: &'static str,
    /// Action qui ouvrirait le groupe (absente pour une réserve absolue).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<&'static str>,
    pub message: String,
}

/// État d'un groupe pour un lecteur et un élève.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatGroupe {
    Ouvert,
    Masque(RaisonMasquage),
}

/// Groupes ouverts (ou masqués, avec raison) pour un lecteur et un élève donnés.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupesAutorises {
    etats: [EtatGroupe; 6],
}

impl GroupesAutorises {
    pub fn ouvert(&self, g: GroupeChamps) -> bool {
        self.etats[g.rang()] == EtatGroupe::Ouvert
    }

    pub fn etat(&self, g: GroupeChamps) -> EtatGroupe {
        self.etats[g.rang()]
    }

    /// Tous les groupes ouverts (usage interne : tâches système, tests).
    pub fn tous() -> Self {
        Self {
            etats: [EtatGroupe::Ouvert; 6],
        }
    }

    /// Famille ou élève lui-même : G0 à G3 (selon le lien, vérifié ailleurs), jamais G5 ni G6.
    pub fn famille() -> Self {
        let mut g = Self::tous();
        g.etats[GroupeChamps::Historique.rang()] =
            EtatGroupe::Masque(RaisonMasquage::ReserveAuPersonnel);
        g.etats[GroupeChamps::Sensible.rang()] =
            EtatGroupe::Masque(RaisonMasquage::ReserveAuPersonnel);
        g
    }

    /// Ancien niveau d'identité exprimé en groupes (compatibilité des appelants).
    pub fn depuis_niveau(niveau: NiveauIdentite) -> Self {
        let manque = EtatGroupe::Masque(RaisonMasquage::DroitManquant);
        let mut g = Self {
            etats: [manque; 6],
        };
        g.etats[GroupeChamps::Designation.rang()] = EtatGroupe::Ouvert;
        match niveau {
            NiveauIdentite::Complet => {
                for x in [
                    GroupeChamps::Scolarite,
                    GroupeChamps::EtatCivil,
                    GroupeChamps::Identifiants,
                ] {
                    g.etats[x.rang()] = EtatGroupe::Ouvert;
                }
            }
            NiveauIdentite::Sante => g.etats[GroupeChamps::EtatCivil.rang()] = EtatGroupe::Ouvert,
            NiveauIdentite::Reduit => {}
        }
        g
    }

    /// Niveau d'identité **dérivé** des groupes (et non plus du profil de base).
    pub fn niveau(&self) -> NiveauIdentite {
        if self.ouvert(GroupeChamps::EtatCivil) {
            if self.ouvert(GroupeChamps::Scolarite) && self.ouvert(GroupeChamps::Identifiants) {
                NiveauIdentite::Complet
            } else {
                NiveauIdentite::Sante
            }
        } else {
            NiveauIdentite::Reduit
        }
    }

    /// Groupes masqués avec leur raison, pour l'écran.
    pub fn masques(&self) -> Vec<MasqueChamps> {
        GroupeChamps::TOUS
            .into_iter()
            .filter_map(|g| match self.etat(g) {
                EtatGroupe::Ouvert => None,
                EtatGroupe::Masque(r) => Some(MasqueChamps {
                    groupe: g.code(),
                    raison: r.code(),
                    action: match r {
                        RaisonMasquage::ReserveAuPersonnel => None,
                        _ => Some(g.action()),
                    },
                    message: match r {
                        RaisonMasquage::DroitManquant => format!(
                            "Votre rôle n'ouvre pas {} de l'élève.",
                            self.article(g)
                        ),
                        RaisonMasquage::HorsPortee => format!(
                            "{} : cet élève est hors de votre périmètre.",
                            capitaliser(g.libelle())
                        ),
                        RaisonMasquage::ReserveAuPersonnel => format!(
                            "{} : réservé au personnel habilité.",
                            capitaliser(g.libelle())
                        ),
                    },
                }),
            })
            .collect()
    }

    fn article(&self, g: GroupeChamps) -> String {
        match g {
            GroupeChamps::EtatCivil | GroupeChamps::Sensible => format!("l'{}", g.libelle()),
            _ => format!("la {}", g.libelle()),
        }
    }
}

fn capitaliser(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Étendue d'une action de groupe pour un lecteur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PorteeGroupe {
    /// Aucun droit.
    Aucune,
    /// Tout l'établissement.
    Tout,
    /// Ces élèves seulement (portées « mes classes », « mes groupes », « désignés »).
    Ensemble(HashSet<Uuid>),
    /// Réservé au personnel (famille et élève).
    Reserve,
}

/// Droits d'un lecteur sur les groupes de champs, calculés **une fois** par requête.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilChamps {
    portees: [PorteeGroupe; 6],
}

impl ProfilChamps {
    pub fn new(portees: [PorteeGroupe; 6]) -> Self {
        Self { portees }
    }

    /// Famille ou élève : G0 à G3 (le lien légal est vérifié par ailleurs).
    pub fn famille() -> Self {
        Self {
            portees: [
                PorteeGroupe::Tout,
                PorteeGroupe::Tout,
                PorteeGroupe::Tout,
                PorteeGroupe::Tout,
                PorteeGroupe::Reserve,
                PorteeGroupe::Reserve,
            ],
        }
    }

    pub fn portee(&self, g: GroupeChamps) -> &PorteeGroupe {
        &self.portees[g.rang()]
    }

    /// Groupes pour cet élève (UN seul calcul, partagé par toutes les routes).
    pub fn pour(&self, eleve_id: Uuid) -> GroupesAutorises {
        let mut etats = [EtatGroupe::Ouvert; 6];
        for g in GroupeChamps::TOUS {
            etats[g.rang()] = match self.portee(g) {
                PorteeGroupe::Tout => EtatGroupe::Ouvert,
                PorteeGroupe::Ensemble(s) if s.contains(&eleve_id) => EtatGroupe::Ouvert,
                PorteeGroupe::Ensemble(_) => EtatGroupe::Masque(RaisonMasquage::HorsPortee),
                PorteeGroupe::Aucune => EtatGroupe::Masque(RaisonMasquage::DroitManquant),
                PorteeGroupe::Reserve => EtatGroupe::Masque(RaisonMasquage::ReserveAuPersonnel),
            };
        }
        GroupesAutorises { etats }
    }

    /// Le lecteur peut-il voir la désignation de cet élève (annuaire) ?
    pub fn voit_designation(&self, eleve_id: Uuid) -> bool {
        self.pour(eleve_id).ouvert(GroupeChamps::Designation)
    }

    /// Le badge est interrogeable (recherche) seulement si G3 est ouvert partout
    /// (pas d'oracle sur le badge d'élèves dont on ne voit pas les identifiants).
    pub fn badge_interrogeable(&self) -> bool {
        *self.portee(GroupeChamps::Identifiants) == PorteeGroupe::Tout
    }
}

impl Eleve {
    /// Retire les champs des groupes masqués (UN seul point de minimisation, E1).
    pub fn minimise_groupes(&mut self, g: &GroupesAutorises) {
        if !g.ouvert(GroupeChamps::Identifiants) {
            self.ine = None;
            self.numero_badge = None;
        }
        if !g.ouvert(GroupeChamps::EtatCivil) {
            self.sexe = None;
            self.date_naissance = None;
        }
        if !g.ouvert(GroupeChamps::Scolarite) {
            self.regime = None;
            self.mef = None;
            self.motif_sortie = None;
            self.redoublant = false;
            self.date_entree = None;
            self.date_sortie = None;
        }
    }
}

impl DossierEleve {
    /// Minimise le dossier selon les groupes ouverts (point unique, après calcul).
    pub fn minimise_groupes(&mut self, g: &GroupesAutorises) {
        self.eleve.minimise_groupes(g);
        if !g.ouvert(GroupeChamps::EtatCivil) {
            self.age_ans = None;
        }
        if !g.ouvert(GroupeChamps::Scolarite) {
            self.periodes.retain(|p| {
                !matches!(
                    p.kind,
                    crate::AttributKind::Regime | crate::AttributKind::Mef
                )
            });
            for a in &mut self.continuum {
                a.mef = None;
                a.regime = None;
                a.redoublement_possible = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profil(cfg: [PorteeGroupe; 6]) -> ProfilChamps {
        ProfilChamps::new(cfg)
    }

    #[test]
    fn niveaux_historiques_sont_des_cas_particuliers() {
        let complet = GroupesAutorises::depuis_niveau(NiveauIdentite::Complet);
        assert_eq!(complet.niveau(), NiveauIdentite::Complet);
        let sante = GroupesAutorises::depuis_niveau(NiveauIdentite::Sante);
        assert_eq!(sante.niveau(), NiveauIdentite::Sante);
        assert!(sante.ouvert(GroupeChamps::EtatCivil) && !sante.ouvert(GroupeChamps::Identifiants));
        let reduit = GroupesAutorises::depuis_niveau(NiveauIdentite::Reduit);
        assert_eq!(reduit.niveau(), NiveauIdentite::Reduit);
        assert!(reduit.ouvert(GroupeChamps::Designation) && !reduit.ouvert(GroupeChamps::Scolarite));
    }

    #[test]
    fn portee_ensemble_distingue_hors_portee_et_droit_manquant() {
        let (dedans, dehors) = (Uuid::new_v4(), Uuid::new_v4());
        let p = profil([
            PorteeGroupe::Tout,
            PorteeGroupe::Ensemble([dedans].into_iter().collect()),
            PorteeGroupe::Aucune,
            PorteeGroupe::Aucune,
            PorteeGroupe::Aucune,
            PorteeGroupe::Aucune,
        ]);
        assert!(p.pour(dedans).ouvert(GroupeChamps::Scolarite));
        assert_eq!(
            p.pour(dehors).etat(GroupeChamps::Scolarite),
            EtatGroupe::Masque(RaisonMasquage::HorsPortee)
        );
        assert_eq!(
            p.pour(dedans).etat(GroupeChamps::EtatCivil),
            EtatGroupe::Masque(RaisonMasquage::DroitManquant)
        );
        let m = p.pour(dehors).masques();
        assert!(m.iter().any(|x| x.groupe == "scolarite" && x.raison == "hors_portee"));
        assert!(m
            .iter()
            .any(|x| x.groupe == "etat_civil" && x.action == Some("eleves.etat_civil.lire")));
        assert!(m.iter().all(|x| !x.message.is_empty()));
    }

    #[test]
    fn famille_voit_g0_a_g3_jamais_g5_g6() {
        let g = ProfilChamps::famille().pour(Uuid::new_v4());
        for ouvert in [
            GroupeChamps::Designation,
            GroupeChamps::Scolarite,
            GroupeChamps::EtatCivil,
            GroupeChamps::Identifiants,
        ] {
            assert!(g.ouvert(ouvert), "{ouvert:?}");
        }
        for ferme in [GroupeChamps::Historique, GroupeChamps::Sensible] {
            assert_eq!(g.etat(ferme), EtatGroupe::Masque(RaisonMasquage::ReserveAuPersonnel));
        }
        // Les réserves absolues n'annoncent pas d'action à demander.
        assert!(g.masques().iter().all(|m| m.action.is_none()));
    }

    #[test]
    fn badge_non_interrogeable_sans_identifiants_partout() {
        let mut cfg = [
            PorteeGroupe::Tout,
            PorteeGroupe::Aucune,
            PorteeGroupe::Aucune,
            PorteeGroupe::Ensemble(HashSet::new()),
            PorteeGroupe::Aucune,
            PorteeGroupe::Aucune,
        ];
        assert!(!profil(cfg.clone()).badge_interrogeable());
        cfg[3] = PorteeGroupe::Tout;
        assert!(profil(cfg).badge_interrogeable());
    }
}
