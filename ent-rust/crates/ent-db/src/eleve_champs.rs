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

/// Plafond de la recherche qui alimente la liste des élèves (E9a) : au-delà, on affine.
pub const LISTE_PLAFOND: i64 = 2000;

/// Filtres de la liste des élèves (E9a).
#[derive(Debug, Clone, Default)]
pub struct FiltreListeEleves<'a> {
    pub q: &'a str,
    pub classe: Option<&'a str>,
    /// `actif`, `preinscrit` ou `sorti`.
    pub statut: Option<&'a str>,
    pub pastille: Option<&'a str>,
    pub limite: i64,
    pub decalage: i64,
}

/// Une page de la liste des élèves.
#[derive(Debug, Clone)]
pub struct ListeEleves {
    pub eleves: Vec<EleveVue>,
    pub total: i64,
    pub decalage: i64,
    pub limite: i64,
    pub tronque: bool,
    /// Le lecteur voit tout l'annuaire de l'établissement (sinon : un périmètre, ex. ses classes).
    pub portee_etendue: bool,
    pub classes: Vec<String>,
}

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
        let mut tx = crate::tenant_tx::TenantTx::begin(self.pool(), etablissement_id)
            .await
            .map_err(EntDbError::from)?;
        self.profil_champs_conn(&mut tx, etablissement_id, user_id, role).await
    }

    /// Même calcul sur la connexion **déjà ouverte** de l'appelant (F7 : aucune connexion supplémentaire).
    pub(crate) async fn profil_champs_conn(
        &self,
        conn: &mut sqlx::PgConnection,
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
            .droits_effectifs_conn(&mut *conn, etablissement_id, user_id, role)
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
                    .eleves_autorises_conn(&mut *conn, etablissement_id, user_id, role, action)
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
    /// Recherche instantanée d'élèves pour un acteur (ELV-20) : tolérante aux accents et aux
    /// fautes, **filtrée par ses droits dans la requête** (portée de l'annuaire, groupes
    /// interrogeables, élèves protégés) puis minimisée par `EleveVue::de` (E1). Le badge, l'INE
    /// et le numéro d'élève ne sont interrogeables que si l'acteur les voit pour tous les élèves ;
    /// l'état civil (nom de naissance, prénoms) de même — aucun oracle sur ce qu'il ne voit pas.
    pub async fn search_eleves_pour_acteur(
        &self,
        etablissement_id: Uuid,
        user_id: Uuid,
        role: &str,
        q: &str,
        limit: i64,
    ) -> Result<Vec<EleveVue>, EntDbError> {
        let profil = self.profil_champs(etablissement_id, user_id, role).await?;
        let designation: Option<Vec<Uuid>> = match profil.portee(GroupeChamps::Designation) {
            PorteeGroupe::Tout => None,
            PorteeGroupe::Ensemble(s) => Some(s.iter().copied().collect()),
            PorteeGroupe::Aucune | PorteeGroupe::Reserve => return Ok(Vec::new()),
        };
        // Élèves protégés : G6 partout, ou lien direct (portée restreinte qui les contient).
        let mut proteges_ids: HashSet<Uuid> = HashSet::new();
        if let PorteeGroupe::Ensemble(s) = profil.portee(GroupeChamps::Designation) {
            proteges_ids.extend(s.iter().copied());
        }
        if let PorteeGroupe::Ensemble(s) = profil.portee(GroupeChamps::Sensible) {
            proteges_ids.extend(s.iter().copied());
        }
        let proteges_ids: Vec<Uuid> = proteges_ids.into_iter().collect();
        let mut tx = crate::tenant_tx::TenantTx::begin(self.pool(), etablissement_id).await?;
        let rows = crate::eleve_repo::PgEleveRepo::rechercher(
            &mut tx,
            &crate::eleve_repo::ParamsRecherche {
                etablissement_id,
                q,
                limit,
                designation: designation.as_deref(),
                civil: profil.ouvert_partout(GroupeChamps::EtatCivil),
                numero: profil.ouvert_partout(GroupeChamps::Scolarite),
                identifiants: profil.ouvert_partout(GroupeChamps::Identifiants),
                proteges_tous: profil.ouvert_partout(GroupeChamps::Sensible),
                proteges_ids: &proteges_ids,
            },
        )
        .await
        .map_err(|e| match e {
            sqlx::Error::Protocol(_) => EntDbError::SaisieInvalide("requête de recherche invalide"),
            e => EntDbError::Sqlx(e),
        })?;
        tx.commit().await?;
        // Défense en profondeur : le filtre SQL et le profil disent la même chose.
        Ok(rows
            .into_iter()
            .filter(|e| profil.voit_designation(e.id))
            .map(|e| {
                let g = profil.pour(e.id);
                EleveVue::de(e, g)
            })
            .collect())
    }

    /// Liste des élèves pour l'écran « Élèves » (lot E9a) : l'annuaire de l'acteur (même garde,
    /// même portée, mêmes protégés écartés que la recherche), filtré puis paginé. Les filtres
    /// ne donnent aucun oracle : ils portent sur ce que l'acteur voit déjà.
    ///
    /// * `pastille` : liste de travail d'une pastille (droits de la pastille exigés, protégés
    ///   écartés avant la limite) ; une pastille inconnue ou non permise refuse la requête.
    /// * La recherche est plafonnée (`LISTE_PLAFOND`) : au-delà, `tronque` invite à affiner.
    pub async fn lister_eleves_pour_acteur(
        &self,
        role: &str,
        etablissement_id: Uuid,
        user_id: Uuid,
        f: &FiltreListeEleves<'_>,
    ) -> Result<ListeEleves, EntDbError> {
        // La liste est réservée au personnel (les familles passent par le portail, lien par lien).
        if !Role::parse(role).is_some_and(Role::is_staff) {
            return Err(EntDbError::Forbidden);
        }
        if let Some(s) = f.statut {
            if !["actif", "preinscrit", "sorti"].contains(&s) {
                return Err(EntDbError::SaisieInvalide("statut inconnu"));
            }
        }
        let profil = self.profil_champs(etablissement_id, user_id, role).await?;
        let etendue = matches!(profil.portee(GroupeChamps::Designation), PorteeGroupe::Tout);
        let tout = self
            .search_eleves_pour_acteur(etablissement_id, user_id, role, f.q, LISTE_PLAFOND)
            .await?;
        let tronque = i64::try_from(tout.len()).unwrap_or(i64::MAX) >= LISTE_PLAFOND;
        let de_la_pastille: Option<HashSet<Uuid>> = match f.pastille {
            Some(code) => Some(
                self.eleves_par_pastille_pour_acteur(role, etablissement_id, user_id, code, 5000)
                    .await?
                    .into_iter()
                    .collect(),
            ),
            None => None,
        };
        let retenus: Vec<EleveVue> = tout
            .into_iter()
            .filter(|v| de_la_pastille.as_ref().is_none_or(|ids| ids.contains(&v.eleve.id)))
            .collect();
        let mut classes: Vec<String> = retenus
            .iter()
            .filter_map(|v| v.eleve.classe.clone())
            .filter(|c| !c.trim().is_empty())
            .collect();
        classes.sort();
        classes.dedup();
        let filtres: Vec<EleveVue> = retenus
            .into_iter()
            .filter(|v| {
                f.classe.is_none_or(|c| v.eleve.classe.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(c)))
                    && f.statut.is_none_or(|s| v.eleve.status.as_str() == s)
            })
            .collect();
        let total = i64::try_from(filtres.len()).unwrap_or(i64::MAX);
        let limite = f.limite.clamp(1, 100);
        let decalage = f.decalage.max(0);
        let page = filtres
            .into_iter()
            .skip(usize::try_from(decalage).unwrap_or(usize::MAX))
            .take(usize::try_from(limite).unwrap_or(100))
            .collect();
        Ok(ListeEleves { eleves: page, total, decalage, limite, tronque, portee_etendue: etendue, classes })
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
        let profil = self.profil_champs(etablissement_id, user_id, role).await?;
        let g = profil.pour(e.id);
        // E7 (§10.5) : toute ouverture de la fiche d'un élève protégé, toute lecture de l'état civil
        // ou des identifiants, est journalisée AVANT d'être servie ; un tiers n'arrive jamais ici
        // (`get_eleve_for_actor` l'a déjà écarté).
        if Role::parse(role).is_some_and(Role::is_staff) {
            let protege: bool = sqlx::query_scalar("SELECT ent_eleve_confidentiel($1, $2)")
                .bind(etablissement_id)
                .bind(e.id)
                .fetch_one(&mut *crate::tenant_tx::TenantTx::begin(self.pool(), etablissement_id).await?)
                .await?;
            let mut lectures: Vec<(&str, &str)> = Vec::new();
            if protege {
                lectures.push(("fiche_protegee", "Ouverture de la fiche d'un élève protégé"));
            }
            if g.ouvert(GroupeChamps::EtatCivil) {
                lectures.push(("etat_civil", "Lecture de l'état civil de l'élève"));
            }
            if g.ouvert(GroupeChamps::Identifiants) {
                lectures.push(("identifiants", "Lecture des identifiants de l'élève"));
            }
            let tiers = protege && !profil.ouvert_partout(GroupeChamps::Sensible);
            for (action, libelle) in lectures {
                self.tracer_consultation(etablissement_id, user_id, role, Some(e.id), action, libelle, tiers && action == "fiche_protegee")
                    .await?;
            }
        }
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
        // E7 : un élève protégé n'apparaît pas dans une liste pour un tiers.
        let ids: Vec<Uuid> = eleves.iter().map(|e| e.id).collect();
        let caches = self.proteges_caches(etablissement_id, role, &profil, &ids).await?;
        Ok(eleves
            .into_iter()
            .filter(|e| !caches.contains(&e.id))
            .filter(|e| profil.voit_designation(e.id))
            .map(|e| {
                let g = profil.pour(e.id);
                EleveVue::de(e, g)
            })
            .collect())
    }
}
