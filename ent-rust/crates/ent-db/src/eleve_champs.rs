//! Socle 2 (l'élève), lot E1 — traduction des droits effectifs en groupes de champs.
//!
//! UN seul point côté serveur : [`PgEnt::profil_champs`] charge les droits du lecteur
//! (profil de base ∪ rôles attribués, lus en base à chaque requête) et en fait un
//! [`ProfilChamps`]. Les routes n'ont plus qu'à demander `pour(eleve_id)` : groupes
//! ouverts, raisons de masquage, niveau d'identité **dérivé**. Le niveau historique
//! par profil (`niveau_identite`) est lui-même dérivé du modèle par défaut.

use std::collections::HashSet;

use domain_eleve::{Eleve, GroupeChamps, GroupesAutorises, NiveauIdentite, PorteeGroupe, ProfilChamps};
use ent_auth::{modele_par_defaut, Action, Portee, Role};
use uuid::Uuid;

use crate::droits_ctx::accorde;
use crate::service::{EntDbError, PgEnt};

fn action_du_groupe(g: GroupeChamps) -> Action {
    Action::parse(g.action()).expect("action de groupe présente au catalogue")
}

impl PgEnt {
    /// Droits du lecteur sur les groupes de champs de l'élève (calculés une fois).
    /// Familles et élèves : G0 à G3 (le lien légal est vérifié par ailleurs).
    pub async fn profil_champs(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<ProfilChamps, EntDbError> {
        let Some(parsed) = Role::parse(role) else {
            return Err(EntDbError::Forbidden);
        };
        if !parsed.is_staff() {
            return Ok(ProfilChamps::famille());
        }
        let droits = self
            .droits_effectifs(etablissement_id, user_id, role)
            .await?;
        let mut portees: [PorteeGroupe; 6] = std::array::from_fn(|_| PorteeGroupe::Aucune);
        for g in GroupeChamps::TOUS {
            let action = action_du_groupe(g);
            let detenus: Vec<_> = droits
                .iter()
                .filter(|d| accorde(d.action, action))
                .collect();
            let p = if detenus.is_empty() {
                PorteeGroupe::Aucune
            } else if detenus.iter().any(|d| d.portee == Portee::Etablissement) {
                PorteeGroupe::Tout
            } else {
                // Portées restreintes : l'ensemble des élèves couverts (une requête).
                match self
                    .eleves_autorises(etablissement_id, user_id, role, action)
                    .await?
                {
                    None => PorteeGroupe::Tout,
                    Some(ens) => PorteeGroupe::Ensemble(ens),
                }
            };
            portees[match g {
                GroupeChamps::Designation => 0,
                GroupeChamps::Scolarite => 1,
                GroupeChamps::EtatCivil => 2,
                GroupeChamps::Identifiants => 3,
                GroupeChamps::Historique => 4,
                GroupeChamps::Sensible => 5,
            }] = p;
        }
        Ok(ProfilChamps::new(portees))
    }

    /// Profil de champs du **modèle par défaut** d'un profil de base (repli sûr,
    /// sans base de données : jamais plus que les droits historiques du profil).
    pub fn profil_champs_modele(role: Role) -> ProfilChamps {
        if !role.is_staff() {
            return ProfilChamps::famille();
        }
        let modele = modele_par_defaut(role);
        let mut portees: [PorteeGroupe; 6] = std::array::from_fn(|_| PorteeGroupe::Aucune);
        for (i, g) in GroupeChamps::TOUS.into_iter().enumerate() {
            let action = action_du_groupe(g);
            let mut ens: HashSet<Uuid> = HashSet::new();
            let mut tout = false;
            let mut restreint = false;
            for (a, p) in &modele {
                if accorde(*a, action) {
                    match p {
                        Portee::Etablissement => tout = true,
                        _ => restreint = true,
                    }
                }
            }
            portees[i] = if tout {
                PorteeGroupe::Tout
            } else if restreint {
                // Portée par élève non résolvable sans base : aucun élève (jamais plus).
                PorteeGroupe::Ensemble(std::mem::take(&mut ens))
            } else {
                PorteeGroupe::Aucune
            };
        }
        ProfilChamps::new(portees)
    }

    /// Niveau d'identité d'un profil de base, **dérivé** de son modèle par défaut
    /// (point unique de minimisation pour les chemins sans identité d'acteur).
    pub fn niveau_identite(role: &str) -> NiveauIdentite {
        match Role::parse(role) {
            Some(r) => Self::profil_champs_modele(r).pour(Uuid::nil()).niveau(),
            None => NiveauIdentite::Reduit,
        }
    }
}

/// Élève minimisé pour un lecteur, avec ses groupes (et donc les raisons de masquage).
#[derive(Debug, Clone)]
pub struct EleveVue {
    pub eleve: Eleve,
    pub groupes: GroupesAutorises,
}

impl EleveVue {
    /// Minimise `eleve` selon `groupes` (UN seul point : `Eleve::minimise_groupes`).
    pub fn de(mut eleve: Eleve, groupes: GroupesAutorises) -> Self {
        eleve.minimise_groupes(&groupes);
        Self { eleve, groupes }
    }
}

impl PgEnt {
    /// Recherche d'élèves pour un acteur : seuls les élèves dont il voit la
    /// désignation (annuaire, avec sa portée) sont renvoyés ; le badge n'est
    /// interrogeable que si l'acteur voit les identifiants de tous les élèves.
    pub async fn search_eleves_pour_acteur(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        q: &str,
        limit: i64,
    ) -> Result<Vec<EleveVue>, EntDbError> {
        let profil = self.profil_champs(etablissement_id, user_id, role).await?;
        let rows = self
            .search_eleves_brut(etablissement_id, q, limit, profil.badge_interrogeable())
            .await?;
        Ok(rows
            .into_iter()
            .filter(|e| profil.voit_designation(e.id))
            .map(|e| {
                let g = profil.pour(e.id);
                EleveVue::de(e, g)
            })
            .collect())
    }

    /// Un élève pour un acteur, minimisé (annuaire requis, puis groupes).
    pub async fn get_eleve_vue_for_actor(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleve_id: Uuid,
    ) -> Result<Option<EleveVue>, EntDbError> {
        let Some(e) = self
            .get_eleve_for_actor(etablissement_id, user_id, role, eleve_id)
            .await?
        else {
            return Ok(None);
        };
        let g = self
            .profil_champs(etablissement_id, user_id, role)
            .await?
            .pour(e.id);
        Ok(Some(EleveVue::de(e, g)))
    }

    /// Minimise une liste d'élèves (effectif d'une classe…) pour un acteur ; les
    /// élèves dont la désignation est hors portée sont omis.
    pub async fn vues_eleves_pour_acteur(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        eleves: Vec<Eleve>,
    ) -> Result<Vec<EleveVue>, EntDbError> {
        let profil = self.profil_champs(etablissement_id, user_id, role).await?;
        Ok(eleves
            .into_iter()
            .filter(|e| profil.voit_designation(e.id))
            .map(|e| {
                let g = profil.pour(e.id);
                EleveVue::de(e, g)
            })
            .collect())
    }
}
