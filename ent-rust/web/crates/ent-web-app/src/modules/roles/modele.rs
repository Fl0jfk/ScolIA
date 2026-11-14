//! Logique d'écran pure de l'administration des rôles (D-017) : édition du tableau
//! « action × portée », modèles de départ, validation de confort, messages d'erreur.
//!
//! Aucune règle de sécurité ici : le serveur décide (droit `roles.gerer`, anti-escalade,
//! portées admises, cibles de l'établissement). Cet écran évite seulement d'envoyer ce
//! qui serait manifestement refusé et rend les refus compréhensibles.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use ent_web_api::{ApiError, CatalogueRoles, DroitApi, PersonnelApi, RoleApi, SaisieRole};
use uuid::Uuid;

/// Portée sans cible explicite : les classes ou groupes sont choisis dans le rôle.
pub const PORTEE_DESIGNES: &str = "designes";
/// Action et portée du droit de gérer les rôles.
pub const ACTION_ROLES_GERER: &str = "roles.gerer";
pub const PORTEE_ETABLISSEMENT: &str = "etablissement";

/// Longueur maximale du nom d'un rôle (identique au serveur).
pub const NOM_MAX: usize = 80;
/// Longueur maximale de la description (le serveur tronque au-delà).
pub const DESCRIPTION_MAX: usize = 500;

/// Droit de gérer les rôles, d'après `GET /roles/mes-droits` (affichage seulement).
pub fn peut_gerer_roles(droits: &[DroitApi]) -> bool {
    droits
        .iter()
        .any(|d| d.action == ACTION_ROLES_GERER && d.portee == PORTEE_ETABLISSEMENT)
}

// ---------------------------------------------------------------------------
// Modèles de départ
// ---------------------------------------------------------------------------

/// Point de départ proposé à la création : nom et droits pré-remplis, modifiables.
/// Les droits sont ceux des profils historiques (`ent_auth::modele_par_defaut`) ; ceux que
/// le catalogue du serveur ne connaît pas sont ignorés au pré-remplissage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeleRole {
    pub id: &'static str,
    pub nom: &'static str,
    pub description: &'static str,
    /// Profil historique dont le modèle est issu (champ `modele` du rôle) ; `None` si le
    /// modèle n'a pas d'équivalent historique.
    pub profil: Option<&'static str>,
    /// `(action, portée)`.
    pub droits: &'static [(&'static str, &'static str)],
}

/// Modèles fournis (D-017). Aucun ne donne `roles.gerer` ni `securite.politique`.
pub const MODELES: &[ModeleRole] = &[
    ModeleRole {
        id: "professeur",
        nom: "Professeur",
        description: "Les élèves que la personne enseigne : notes, absences, dossier, messagerie.",
        profil: Some("prof"),
        droits: &[
            ("notes.lire", "mes_classes"),
            ("notes.saisir", "mes_classes"),
            ("competences.evaluer", "mes_classes"),
            ("absences.lire", "mes_classes"),
            ("dossier.lire", "mes_classes"),
            ("journal.lire", "mes_classes"),
            ("messagerie.eleve", "mes_classes"),
            ("suivi.eleve", "mes_classes"),
        ],
    },
    ModeleRole {
        id: "cpe",
        nom: "CPE",
        description: "Vie scolaire de tout l'établissement : absences, justificatifs, suivi.",
        profil: Some("cpe"),
        droits: &[
            ("notes.lire", "etablissement"),
            ("absences.lire", "etablissement"),
            ("absences.justifier", "etablissement"),
            ("dossier.lire", "etablissement"),
            ("journal.lire", "etablissement"),
            ("messagerie.eleve", "etablissement"),
            ("suivi.eleve", "etablissement"),
        ],
    },
    ModeleRole {
        id: "secretariat",
        nom: "Secrétariat",
        description: "Gestion administrative des élèves de tout l'établissement.",
        profil: Some("administratif"),
        droits: &[
            ("notes.lire", "etablissement"),
            ("absences.lire", "etablissement"),
            ("absences.justifier", "etablissement"),
            ("dossier.lire", "etablissement"),
            ("journal.lire", "etablissement"),
            ("messagerie.eleve", "etablissement"),
            ("suivi.eleve", "etablissement"),
        ],
    },
    ModeleRole {
        id: "comptabilite",
        nom: "Comptabilité",
        description: "Consultation des dossiers élèves, sans notes ni vie scolaire.",
        profil: Some("compta"),
        droits: &[
            ("dossier.lire", "etablissement"),
            ("journal.lire", "etablissement"),
            ("suivi.eleve", "etablissement"),
        ],
    },
    ModeleRole {
        id: "accueil",
        nom: "Accueil",
        description: "Trouver un élève, voir sa classe et sa scolarité, savoir s'il est présent : sans notes, état civil, identifiants ni dossier.",
        profil: None,
        droits: &[
            ("eleves.annuaire", "etablissement"),
            ("eleves.lire", "etablissement"),
            ("absences.lire", "etablissement"),
        ],
    },
    ModeleRole {
        id: "direction",
        nom: "Direction (sans administration)",
        description:
            "Tous les droits sur les élèves ; ni gestion des rôles, ni politique de sécurité.",
        profil: Some("direction"),
        droits: &[
            ("notes.lire", "etablissement"),
            ("notes.saisir", "etablissement"),
            ("competences.evaluer", "etablissement"),
            ("absences.lire", "etablissement"),
            ("absences.justifier", "etablissement"),
            ("dossier.lire", "etablissement"),
            ("journal.lire", "etablissement"),
            ("messagerie.eleve", "etablissement"),
            ("suivi.eleve", "etablissement"),
        ],
    },
];

/// Modèle par identifiant.
pub fn modele_par_id(id: &str) -> Option<&'static ModeleRole> {
    MODELES.iter().find(|m| m.id == id)
}

// ---------------------------------------------------------------------------
// Édition d'un rôle
// ---------------------------------------------------------------------------

/// État d'édition d'un rôle : nom, description et droits `(action, portée) → cibles`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditionRole {
    pub nom: String,
    pub description: String,
    /// Profil historique d'origine, conservé tel quel à l'enregistrement.
    pub modele: Option<String>,
    droits: BTreeMap<(String, String), BTreeSet<Uuid>>,
}

impl EditionRole {
    /// Rôle vide (création sans modèle).
    pub fn vide() -> Self {
        Self::default()
    }

    /// Édition d'un rôle existant.
    pub fn depuis_role(role: &RoleApi) -> Self {
        let mut droits = BTreeMap::new();
        for d in &role.droits {
            droits.insert(
                (d.action.clone(), d.portee.clone()),
                d.cibles.iter().copied().collect(),
            );
        }
        Self {
            nom: role.nom.clone(),
            description: role.description.clone().unwrap_or_default(),
            modele: role.modele.clone(),
            droits,
        }
    }

    /// Pré-remplissage depuis un modèle, limité à ce que le catalogue du serveur admet.
    pub fn depuis_modele(modele: &ModeleRole, catalogue: &CatalogueRoles) -> Self {
        let mut e = Self {
            nom: modele.nom.to_string(),
            description: modele.description.to_string(),
            modele: modele.profil.map(str::to_string),
            droits: BTreeMap::new(),
        };
        for (action, portee) in modele.droits {
            e.definir(action, portee, true, catalogue);
        }
        e
    }

    /// Le droit `(action, portée)` est-il coché ?
    pub fn coche(&self, action: &str, portee: &str) -> bool {
        self.droits
            .contains_key(&(action.to_string(), portee.to_string()))
    }

    /// Coche ou décoche un droit. Refuse (sans effet, renvoie `false`) une combinaison que
    /// le catalogue n'admet pas. Décocher « désignés » oublie les cibles choisies.
    pub fn definir(
        &mut self,
        action: &str,
        portee: &str,
        coche: bool,
        catalogue: &CatalogueRoles,
    ) -> bool {
        if coche && !portee_admise(catalogue, action, portee) {
            return false;
        }
        let cle = (action.to_string(), portee.to_string());
        if coche {
            self.droits.entry(cle).or_default();
        } else {
            self.droits.remove(&cle);
        }
        true
    }

    /// Classes et groupes désignés pour cette action.
    pub fn cibles(&self, action: &str) -> BTreeSet<Uuid> {
        self.droits
            .get(&(action.to_string(), PORTEE_DESIGNES.to_string()))
            .cloned()
            .unwrap_or_default()
    }

    /// Coche ou décoche une cible ; sans effet si la portée « désignés » n'est pas cochée.
    pub fn definir_cible(&mut self, action: &str, cible: Uuid, coche: bool) {
        if let Some(c) = self
            .droits
            .get_mut(&(action.to_string(), PORTEE_DESIGNES.to_string()))
        {
            if coche {
                c.insert(cible);
            } else {
                c.remove(&cible);
            }
        }
    }

    /// Nombre de droits (cases cochées).
    pub fn nombre_droits(&self) -> usize {
        self.droits.len()
    }

    /// Droits dans l'ordre stable (action puis portée), prêts à envoyer.
    pub fn droits_a_envoyer(&self) -> Vec<DroitApi> {
        self.droits
            .iter()
            .map(|((action, portee), cibles)| DroitApi {
                action: action.clone(),
                portee: portee.clone(),
                cibles: if portee == PORTEE_DESIGNES {
                    cibles.iter().copied().collect()
                } else {
                    Vec::new()
                },
            })
            .collect()
    }

    /// Corps de `POST /roles` ou `PUT /roles/{id}`.
    pub fn saisie(&self) -> SaisieRole {
        let description = self.description.trim();
        SaisieRole {
            nom: self.nom.trim().to_string(),
            description: (!description.is_empty()).then(|| description.to_string()),
            modele: self.modele.clone(),
            droits: self.droits_a_envoyer(),
        }
    }

    /// Erreurs de saisie (confort ; le serveur revalide) : `(champ, message)`.
    pub fn erreurs(&self) -> Vec<(&'static str, String)> {
        let mut v = Vec::new();
        let nom = self.nom.trim();
        if nom.is_empty() {
            v.push(("nom", "Donnez un nom au rôle.".to_string()));
        } else if nom.chars().count() > NOM_MAX {
            v.push((
                "nom",
                format!("Le nom est trop long ({NOM_MAX} caractères au plus)."),
            ));
        }
        if self.description.trim().chars().count() > DESCRIPTION_MAX {
            v.push((
                "description",
                format!("La description est trop longue ({DESCRIPTION_MAX} caractères au plus)."),
            ));
        }
        let mut sans_cible: Vec<&str> = self
            .droits
            .iter()
            .filter(|((_, p), c)| p == PORTEE_DESIGNES && c.is_empty())
            .map(|((a, _), _)| a.as_str())
            .collect();
        sans_cible.sort_unstable();
        if !sans_cible.is_empty() {
            v.push((
                "droits",
                "Choisissez au moins une classe ou un groupe pour chaque droit « désignés »."
                    .into(),
            ));
        }
        v
    }

    /// Y a-t-il une différence avec l'état initial ?
    pub fn est_modifie(&self, initial: &EditionRole) -> bool {
        self != initial
    }

    /// Résumé des droits ajoutés et retirés par rapport à l'état initial (confirmation).
    pub fn resume_changements(&self, initial: &EditionRole) -> String {
        let ajoutes = self
            .droits
            .keys()
            .filter(|k| !initial.droits.contains_key(*k))
            .count();
        let retires = initial
            .droits
            .keys()
            .filter(|k| !self.droits.contains_key(*k))
            .count();
        let mut parts = Vec::new();
        if ajoutes > 0 {
            parts.push(format!(
                "{ajoutes} droit{} ajouté{}",
                s(ajoutes),
                s(ajoutes)
            ));
        }
        if retires > 0 {
            parts.push(format!(
                "{retires} droit{} retiré{}",
                s(retires),
                s(retires)
            ));
        }
        if self.nom.trim() != initial.nom.trim() {
            parts.push("nom modifié".into());
        }
        if parts.is_empty() {
            "Aucun changement de droits.".into()
        } else {
            format!("{}.", capitaliser(&parts.join(", ")))
        }
    }
}

fn s(n: usize) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

fn capitaliser(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(p) => p.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// La portée est-elle admise pour cette action par le catalogue ?
pub fn portee_admise(catalogue: &CatalogueRoles, action: &str, portee: &str) -> bool {
    catalogue
        .actions
        .iter()
        .find(|a| a.code == action)
        .is_some_and(|a| a.portees.iter().any(|p| p == portee))
}

/// Libellé d'une portée d'après le catalogue (code brut si inconnue).
pub fn libelle_portee<'a>(catalogue: &'a CatalogueRoles, code: &'a str) -> &'a str {
    catalogue
        .portees
        .iter()
        .find(|p| p.code == code)
        .map(|p| p.libelle.as_str())
        .unwrap_or(code)
}

/// Libellé court d'une portée pour l'en-tête du tableau (le catalogue donne des libellés
/// descriptifs, trop longs pour une colonne de 375 px).
pub fn libelle_court_portee(code: &str) -> &'static str {
    match code {
        "etablissement" => "Établissement",
        "mes_classes" => "Mes classes",
        "mes_groupes" => "Mes groupes",
        "designes" => "Désignés",
        _ => "Autre",
    }
}

/// Aide sous l'en-tête d'une portée.
pub fn aide_portee(code: &str) -> &'static str {
    match code {
        "etablissement" => "Tous les élèves de l'établissement.",
        "mes_classes" => "Les élèves que la personne enseigne.",
        "mes_groupes" => "Les élèves des groupes et ateliers dont la personne est responsable.",
        "designes" => "Les élèves des classes ou groupes choisis ci-dessous.",
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// Messages d'erreur
// ---------------------------------------------------------------------------

/// Message pour un refus d'écriture sur les rôles. Un 403 sur une écriture, alors que
/// l'écran n'est proposé qu'aux personnes ayant `roles.gerer`, signale presque toujours une
/// tentative d'accorder un droit que l'on n'a pas.
pub fn message_erreur_role(e: &ApiError) -> String {
    match e {
        ApiError::Interdit(_) => "Vous ne pouvez pas accorder un droit que vous n'avez pas vous-même \
            (même action, portée au moins aussi large). Retirez ce droit du rôle, ou demandez à \
            une personne qui l'a de faire cette modification. Vous n'avez peut-être aussi plus le \
            droit de gérer les rôles."
            .into(),
        ApiError::Invalide(_) => "Les droits demandés ne sont pas acceptés : une portée n'est pas \
            admise pour cette action, un nom est invalide, ou une classe ou un groupe désigné n'existe pas \
            dans l'établissement. Vérifiez le nom et les cases cochées."
            .into(),
        ApiError::Introuvable(_) => "Ce rôle, cette personne ou ce groupe n'existe plus. Rechargez la page.".into(),
        autre => autre.message_utilisateur(),
    }
}

/// Message pour un refus d'attribution ou de retrait de rôle.
pub fn message_erreur_attribution(e: &ApiError) -> String {
    match e {
        ApiError::Interdit(_) => {
            "Vous ne pouvez pas attribuer ce rôle : il contient des droits que \
            vous n'avez pas vous-même, ou cette personne ne peut pas recevoir de rôle \
            (une famille ou un élève n'en reçoit jamais)."
                .into()
        }
        ApiError::Invalide(_) => {
            "Attribution refusée : vérifiez la personne et la date de fin.".into()
        }
        ApiError::Introuvable(_) => "Cette attribution n'existe plus. Rechargez la page.".into(),
        autre => autre.message_utilisateur(),
    }
}

// ---------------------------------------------------------------------------
// Membres
// ---------------------------------------------------------------------------

/// Date de fin saisie (`AAAA-MM-JJ`, champ date) : vide = sans fin ; sinon une date
/// valide, postérieure à `aujourdhui`.
pub fn lire_date_fin(
    saisie: &str,
    aujourdhui: NaiveDate,
) -> Result<Option<NaiveDate>, &'static str> {
    let t = saisie.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let d = NaiveDate::parse_from_str(t, "%Y-%m-%d").map_err(|_| "Date de fin invalide.")?;
    if d <= aujourdhui {
        return Err("La date de fin doit être postérieure à aujourd'hui.");
    }
    Ok(Some(d))
}

/// Nom affiché d'un membre ; compte non listé (au-delà des 100 résultats) → identifiant abrégé.
pub fn etiquette_membre(personnel: &[PersonnelApi], id: Uuid) -> String {
    match personnel.iter().find(|p| p.user_id == id) {
        Some(p) if !p.label.trim().is_empty() => p.label.clone(),
        Some(p) => p.email.clone(),
        None => format!("Compte {}", &id.to_string()[..8]),
    }
}

/// Personnes proposées à l'ajout : personnel qui n'a pas déjà le rôle, filtré par la
/// recherche (nom ou e-mail, sans casse), trié par nom.
pub fn candidats(personnel: &[PersonnelApi], deja: &[Uuid], recherche: &str) -> Vec<PersonnelApi> {
    let q = recherche.trim().to_lowercase();
    let mut v: Vec<PersonnelApi> = personnel
        .iter()
        .filter(|p| !deja.contains(&p.user_id))
        .filter(|p| {
            q.is_empty()
                || p.label.to_lowercase().contains(&q)
                || p.email.to_lowercase().contains(&q)
        })
        .cloned()
        .collect();
    v.sort_by_key(|a| a.label.to_lowercase());
    v
}

// ---------------------------------------------------------------------------
// Liste des rôles
// ---------------------------------------------------------------------------

/// Résumé d'un rôle pour la liste.
pub fn resume_role(r: &RoleApi) -> String {
    let d = r.droits.len();
    let m = r.membres.len();
    format!("{d} droit{}, {m} membre{}", s(d), s(m),)
}

/// Rôles triés par nom (sans casse).
pub fn trier_roles(mut v: Vec<RoleApi>) -> Vec<RoleApi> {
    v.sort_by_key(|a| a.nom.to_lowercase());
    v
}

// ---------------------------------------------------------------------------
// Cibles désignables (classes et groupes)
// ---------------------------------------------------------------------------

/// Classe ou groupe proposé comme cible d'un droit « désignés » ou comme groupe à encadrer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cible {
    pub id: Uuid,
    pub libelle: String,
    pub groupe: bool,
}

/// Libellé « 6e A » / « Atelier théâtre (groupe) » pour une cible.
pub fn libelle_cible(c: &Cible) -> String {
    if c.groupe {
        format!("{} (groupe)", c.libelle)
    } else {
        c.libelle.clone()
    }
}

/// Cibles triées : classes puis groupes, chacun par libellé.
pub fn trier_cibles(mut v: Vec<Cible>) -> Vec<Cible> {
    v.sort_by(|a, b| {
        a.groupe
            .cmp(&b.groupe)
            .then(a.libelle.to_lowercase().cmp(&b.libelle.to_lowercase()))
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_web_api::{ActionCatalogue, PorteeCatalogue};

    fn catalogue() -> CatalogueRoles {
        let toutes = ["etablissement", "mes_classes", "mes_groupes", "designes"];
        let action = |code: &str, portees: &[&str]| ActionCatalogue {
            code: code.into(),
            libelle: code.into(),
            portees: portees.iter().map(|p| p.to_string()).collect(),
        };
        CatalogueRoles {
            actions: vec![
                action("notes.lire", &toutes),
                action("absences.lire", &toutes),
                action("roles.gerer", &["etablissement"]),
            ],
            portees: toutes
                .iter()
                .map(|p| PorteeCatalogue {
                    code: p.to_string(),
                    libelle: format!("libellé {p}"),
                })
                .collect(),
        }
    }

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn cocher_une_portee_non_admise_est_refuse() {
        let cat = catalogue();
        let mut e = EditionRole::vide();
        assert!(!e.definir("roles.gerer", "mes_classes", true, &cat));
        assert!(!e.coche("roles.gerer", "mes_classes"));
        assert!(e.definir("roles.gerer", "etablissement", true, &cat));
        assert!(e.coche("roles.gerer", "etablissement"));
        assert!(!e.definir("action.inconnue", "etablissement", true, &cat));
        assert_eq!(e.nombre_droits(), 1);
    }

    #[test]
    fn decocher_designes_oublie_les_cibles() {
        let cat = catalogue();
        let mut e = EditionRole::vide();
        e.definir("notes.lire", "designes", true, &cat);
        e.definir_cible("notes.lire", id(1), true);
        e.definir_cible("notes.lire", id(2), true);
        assert_eq!(e.cibles("notes.lire").len(), 2);
        e.definir("notes.lire", "designes", false, &cat);
        assert!(e.cibles("notes.lire").is_empty());
        // Sans portée « désignés », cocher une cible n'a aucun effet.
        e.definir_cible("notes.lire", id(1), true);
        assert!(e.cibles("notes.lire").is_empty());
        e.definir("notes.lire", "designes", true, &cat);
        assert!(e.cibles("notes.lire").is_empty());
    }

    #[test]
    fn droits_envoyes_au_format_du_serveur() {
        let cat = catalogue();
        let mut e = EditionRole::vide();
        e.nom = "  Théâtre ".into();
        e.description = "   ".into();
        e.definir("absences.lire", "mes_groupes", true, &cat);
        e.definir("notes.lire", "designes", true, &cat);
        e.definir_cible("notes.lire", id(7), true);
        let s = e.saisie();
        assert_eq!(s.nom, "Théâtre");
        assert_eq!(s.description, None, "description vide = absente");
        assert_eq!(s.droits.len(), 2);
        assert_eq!(s.droits[0].action, "absences.lire");
        assert!(s.droits[0].cibles.is_empty());
        assert_eq!(s.droits[1].cibles, [id(7)]);
    }

    #[test]
    fn cibles_envoyees_seulement_pour_designes() {
        let mut e = EditionRole::vide();
        e.droits
            .insert(("notes.lire".into(), "mes_classes".into()), [id(3)].into());
        assert!(e.droits_a_envoyer()[0].cibles.is_empty());
    }

    #[test]
    fn validation_nom_et_cibles() {
        let cat = catalogue();
        let mut e = EditionRole::vide();
        assert_eq!(e.erreurs()[0].0, "nom");
        e.nom = "x".repeat(81);
        assert!(e.erreurs()[0].1.contains("trop long"));
        e.nom = "Atelier".into();
        assert!(e.erreurs().is_empty());
        e.definir("notes.lire", "designes", true, &cat);
        let err = e.erreurs();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].0, "droits");
        e.definir_cible("notes.lire", id(1), true);
        assert!(e.erreurs().is_empty());
        e.description = "d".repeat(501);
        assert_eq!(e.erreurs()[0].0, "description");
    }

    #[test]
    fn modeles_filtres_par_le_catalogue() {
        let cat = catalogue();
        let prof = EditionRole::depuis_modele(modele_par_id("professeur").unwrap(), &cat);
        assert_eq!(prof.nom, "Professeur");
        assert_eq!(prof.modele.as_deref(), Some("prof"));
        // Le catalogue d'essai ne contient que notes.lire et absences.lire parmi les droits du modèle.
        assert_eq!(prof.nombre_droits(), 2);
        assert!(prof.coche("notes.lire", "mes_classes"));
        let accueil = EditionRole::depuis_modele(modele_par_id("accueil").unwrap(), &cat);
        assert_eq!(accueil.modele, None);
        assert!(accueil.coche("absences.lire", "etablissement"));
    }

    #[test]
    fn aucun_modele_ne_donne_les_droits_d_administration() {
        for m in MODELES {
            assert!(
                m.droits
                    .iter()
                    .all(|(a, _)| *a != "roles.gerer" && *a != "securite.politique"),
                "{}",
                m.id
            );
            let mut vus = BTreeSet::new();
            assert!(m.droits.iter().all(|d| vus.insert(*d)), "doublon {}", m.id);
        }
        assert!(modele_par_id("inconnu").is_none());
    }

    #[test]
    fn modification_et_resume() {
        let cat = catalogue();
        let initial = EditionRole::depuis_modele(modele_par_id("accueil").unwrap(), &cat);
        let mut e = initial.clone();
        assert!(!e.est_modifie(&initial));
        assert_eq!(
            e.resume_changements(&initial),
            "Aucun changement de droits."
        );
        e.definir("notes.lire", "etablissement", true, &cat);
        e.definir("absences.lire", "etablissement", false, &cat);
        e.nom = "Autre".into();
        assert!(e.est_modifie(&initial));
        assert_eq!(
            e.resume_changements(&initial),
            "1 droit ajouté, 1 droit retiré, nom modifié."
        );
    }

    #[test]
    fn edition_d_un_role_existant_conserve_les_cibles() {
        let role = RoleApi {
            id: id(9),
            nom: "Théâtre".into(),
            description: Some("Atelier".into()),
            modele: Some("prof".into()),
            droits: vec![DroitApi {
                action: "notes.lire".into(),
                portee: "designes".into(),
                cibles: vec![id(5), id(6)],
            }],
            membres: vec![],
        };
        let e = EditionRole::depuis_role(&role);
        assert_eq!(e.cibles("notes.lire").len(), 2);
        assert_eq!(e.saisie().droits, role.droits);
        assert_eq!(e.saisie().modele.as_deref(), Some("prof"));
    }

    #[test]
    fn acces_a_la_page_selon_mes_droits() {
        let droit = |a: &str, p: &str| DroitApi {
            action: a.into(),
            portee: p.into(),
            cibles: vec![],
        };
        assert!(peut_gerer_roles(&[droit("roles.gerer", "etablissement")]));
        assert!(!peut_gerer_roles(&[droit("notes.lire", "etablissement")]));
        assert!(!peut_gerer_roles(&[droit("roles.gerer", "mes_classes")]));
        assert!(!peut_gerer_roles(&[]));
    }

    #[test]
    fn escalade_message_comprehensible() {
        let m = message_erreur_role(&ApiError::Interdit("accès refusé".into()));
        assert!(m.contains("accorder un droit que vous n'avez pas"));
        assert!(!m.contains("accès refusé"), "pas de corps brut du serveur");
        assert!(message_erreur_role(&ApiError::Invalide("x".into())).contains("portée"));
        assert!(message_erreur_attribution(&ApiError::Interdit("x".into())).contains("famille"));
        assert_eq!(
            message_erreur_role(&ApiError::TropDeRequetes),
            ApiError::TropDeRequetes.message_utilisateur()
        );
    }

    #[test]
    fn date_de_fin() {
        let auj = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        assert_eq!(lire_date_fin("", auj), Ok(None));
        assert_eq!(lire_date_fin("  ", auj), Ok(None));
        assert_eq!(
            lire_date_fin("2026-12-01", auj),
            Ok(NaiveDate::from_ymd_opt(2026, 12, 1))
        );
        assert!(lire_date_fin("2026-11-05", auj).is_err());
        assert!(lire_date_fin("2026-01-01", auj).is_err());
        assert!(lire_date_fin("pas une date", auj).is_err());
    }

    fn personne(n: u128, label: &str, email: &str) -> PersonnelApi {
        PersonnelApi {
            user_id: id(n),
            email: email.into(),
            role: "prof".into(),
            label: label.into(),
        }
    }

    #[test]
    fn candidats_exclut_les_membres_et_filtre() {
        let p = vec![
            personne(1, "Zoé Martin", "zoe@x.fr"),
            personne(2, "alice Durand", "alice@x.fr"),
            personne(3, "Bob Martin", "bob@x.fr"),
        ];
        let tous = candidats(&p, &[id(3)], "");
        assert_eq!(
            tous.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
            ["alice Durand", "Zoé Martin"],
            "tri sans casse, membres exclus"
        );
        assert_eq!(candidats(&p, &[], " MARTIN ").len(), 2);
        assert_eq!(candidats(&p, &[], "alice@").len(), 1);
        assert!(candidats(&p, &[], "zzz").is_empty());
    }

    #[test]
    fn etiquette_de_membre() {
        let p = vec![
            personne(1, "Zoé Martin", "zoe@x.fr"),
            personne(2, " ", "sans@nom.fr"),
        ];
        assert_eq!(etiquette_membre(&p, id(1)), "Zoé Martin");
        assert_eq!(etiquette_membre(&p, id(2)), "sans@nom.fr");
        assert!(etiquette_membre(&p, id(99)).starts_with("Compte "));
    }

    #[test]
    fn resume_de_role_et_tri() {
        let r = |nom: &str, d: usize, m: usize| RoleApi {
            id: id(1),
            nom: nom.into(),
            description: None,
            modele: None,
            droits: (0..d)
                .map(|i| DroitApi {
                    action: format!("a{i}"),
                    portee: "etablissement".into(),
                    cibles: vec![],
                })
                .collect(),
            membres: (0..m).map(|i| id(i as u128)).collect(),
        };
        assert_eq!(resume_role(&r("A", 1, 0)), "1 droit, 0 membre");
        assert_eq!(resume_role(&r("A", 3, 2)), "3 droits, 2 membres");
        let t = trier_roles(vec![r("zèbre", 0, 0), r("Accueil", 0, 0)]);
        assert_eq!(t[0].nom, "Accueil");
    }

    #[test]
    fn cibles_classes_avant_groupes() {
        let c = |l: &str, g: bool| Cible {
            id: id(1),
            libelle: l.into(),
            groupe: g,
        };
        let t = trier_cibles(vec![c("Atelier", true), c("6e B", false), c("6e A", false)]);
        assert_eq!(
            t.iter().map(libelle_cible).collect::<Vec<_>>(),
            ["6e A", "6e B", "Atelier (groupe)"]
        );
    }

    #[test]
    fn libelles_de_portees() {
        let cat = catalogue();
        assert_eq!(libelle_portee(&cat, "designes"), "libellé designes");
        assert_eq!(libelle_portee(&cat, "xyz"), "xyz");
        assert_eq!(libelle_court_portee("mes_groupes"), "Mes groupes");
        assert!(!aide_portee("designes").is_empty());
    }
}
