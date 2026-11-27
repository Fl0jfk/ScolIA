//! Logique d'écran pure de la sécurité du compte : libellés, validation, tri des appareils,
//! édition de la politique par rôle. Sans Leptos ni réseau, testée en natif.
//!
//! Aucune règle de sécurité ici : le serveur décide (mot de passe, plafond de passkeys,
//! droits). Ces fonctions préparent l'affichage et évitent les envois inutiles.

use std::collections::BTreeSet;

use chrono::{DateTime, Datelike, NaiveDateTime, Timelike, Utc};
use ent_web_api::{ApiError, AppareilSession, PasskeyCompte};

use crate::noyau::droits::Role;
use crate::noyau::portee::Portee;
use crate::noyau::temps::nom_mois;

// ---------------------------------------------------------------------------
// Niveau de la session
// ---------------------------------------------------------------------------

/// Comment la session a été ouverte (`assurance` de `/auth/me`).
pub fn libelle_assurance(assurance: &str) -> &'static str {
    match assurance {
        "passkey" => "Passkey",
        "sso" => "Connexion unique (SSO)",
        "mot_de_passe" => "Mot de passe",
        _ => "Inconnu",
    }
}

/// Explication courte du niveau, pour la personne (jamais seulement la couleur).
pub fn explication_assurance(assurance: &str) -> &'static str {
    match assurance {
        "passkey" => "Vous vous êtes identifié avec une passkey de votre appareil.",
        "sso" => "Vous vous êtes identifié par le service de connexion de votre organisation.",
        "mot_de_passe" => {
            "Vous vous êtes identifié avec votre mot de passe seul. Une passkey renforce la protection de votre compte."
        }
        _ => "Le niveau de cette session n'est pas connu.",
    }
}

/// Vrai si le niveau est le plus faible (mot de passe seul).
pub fn assurance_faible(assurance: &str) -> bool {
    assurance == "mot_de_passe"
}

/// Type de client de la session.
pub fn libelle_client(client: &str) -> &'static str {
    match client {
        "mobile" => "Application mobile",
        "web" => "Navigateur web",
        _ => "Autre appareil",
    }
}

/// Phrase décrivant l'obligation de passkey de la personne.
pub fn libelle_obligation(obligatoire: bool, nb_passkeys: i64) -> &'static str {
    match (obligatoire, nb_passkeys > 0) {
        (true, true) => {
            "Votre établissement impose la passkey pour votre rôle : elle est en place."
        }
        (true, false) => {
            "Votre établissement impose la passkey pour votre rôle : vous devez en enregistrer une."
        }
        (false, true) => "Une passkey est enregistrée sur votre compte.",
        (false, false) => {
            "Aucune passkey n'est enregistrée sur votre compte (facultative pour votre rôle)."
        }
    }
}

// ---------------------------------------------------------------------------
// Passkeys
// ---------------------------------------------------------------------------

/// Longueur maximale (en caractères) d'un nom repris à l'écran.
const NOM_MAX: usize = 80;

fn borne(texte: &str) -> String {
    let une_ligne = texte.split_whitespace().collect::<Vec<_>>().join(" ");
    if une_ligne.chars().count() > NOM_MAX {
        let coupe: String = une_ligne.chars().take(NOM_MAX).collect();
        format!("{}…", coupe.trim_end())
    } else {
        une_ligne
    }
}

/// Nom affiché d'une passkey (le nom est donné par la personne ; borné à l'affichage).
pub fn nom_passkey(p: &PasskeyCompte) -> String {
    match p.name.as_deref().map(borne).filter(|n| !n.is_empty()) {
        Some(n) => n,
        None => "Passkey sans nom".to_string(),
    }
}

/// Le mot de passe actuel est exigé sauf en session « enrôlement » (le serveur l'accepte vide).
pub fn mot_de_passe_requis(portee: Portee) -> bool {
    portee != Portee::Enrolement
}

/// Contrôle de confort avant l'envoi ; le serveur reste l'autorité.
pub fn valider_mot_de_passe(saisie: &str, requis: bool) -> Result<(), &'static str> {
    if requis && saisie.is_empty() {
        Err("Saisissez votre mot de passe actuel.")
    } else {
        Ok(())
    }
}

/// Texte d'introduction de l'écran d'enrôlement obligatoire.
pub fn explication_obligation_enrolement(role: &str) -> String {
    format!(
        "Votre établissement exige une passkey pour le rôle « {role} ». Elle doit être enregistrée avant d'utiliser l'ENT : c'est une seule étape, d'environ une minute."
    )
}

/// Avertissement avant de supprimer une passkey.
pub fn avertissement_suppression(nb_passkeys: i64, obligatoire: bool) -> Option<&'static str> {
    if nb_passkeys <= 1 && obligatoire {
        Some("C'est votre dernière passkey et votre rôle l'impose : à la prochaine connexion, vous devrez en enregistrer une nouvelle avant tout autre usage.")
    } else if nb_passkeys <= 1 {
        Some("C'est votre dernière passkey : la connexion se fera par mot de passe seul.")
    } else {
        None
    }
}

/// Message pour une action sensible (ajout ou suppression de passkey) : distingue le
/// mot de passe refusé des autres refus (le serveur répond 403 dans les deux cas).
pub fn message_action_sensible(e: &ApiError) -> String {
    match e {
        ApiError::Interdit(corps) if corps.to_lowercase().contains("mot de passe actuel") => {
            "Mot de passe actuel incorrect.".to_string()
        }
        ApiError::Conflit(_) => {
            "Le nombre maximal de passkeys est atteint : supprimez-en une avant d'en ajouter."
                .to_string()
        }
        ApiError::TropDeRequetes => {
            "Trop de tentatives. Patientez quelques minutes avant de réessayer.".to_string()
        }
        autre => autre.message_utilisateur(),
    }
}

// ---------------------------------------------------------------------------
// Appareils connectés
// ---------------------------------------------------------------------------

/// Appareil courant d'abord, puis le plus récemment actif.
pub fn trier_appareils(mut liste: Vec<AppareilSession>) -> Vec<AppareilSession> {
    liste.sort_by(|a, b| {
        b.courante
            .cmp(&a.courante)
            .then(b.last_seen_at.cmp(&a.last_seen_at))
            .then(a.id.cmp(&b.id))
    });
    liste
}

/// Nom affiché d'un appareil (texte venant du navigateur ou de l'appli : borné, une ligne).
pub fn nom_appareil(a: &AppareilSession) -> String {
    match a.appareil.as_deref().map(borne).filter(|n| !n.is_empty()) {
        Some(n) => n,
        None => libelle_client(&a.client).to_string(),
    }
}

/// Date et heure en français (« 5 novembre 2026 à 09:07 »), sur un instant local.
pub fn horodatage_fr(local: NaiveDateTime) -> String {
    format!(
        "{} {} {} à {:02}:{:02}",
        local.day(),
        nom_mois(local.month()),
        local.year(),
        local.hour(),
        local.minute()
    )
}

/// Instant UTC du serveur, affiché à l'heure locale du navigateur.
pub fn horodatage_local(instant: DateTime<Utc>) -> String {
    horodatage_fr(instant.with_timezone(&chrono::Local).naive_local())
}

// ---------------------------------------------------------------------------
// Politique de sécurité de l'établissement
// ---------------------------------------------------------------------------

/// Rôles proposés dans la politique : (identifiant du serveur, libellé au pluriel).
pub const ROLES_POLITIQUE: [(&str, &str); 11] = [
    ("admin", "Administration"),
    ("direction", "Direction"),
    ("administratif", "Secrétariat"),
    ("cpe", "Vie scolaire (CPE)"),
    ("compta", "Comptabilité"),
    ("prof", "Enseignants"),
    ("infirmerie", "Infirmerie"),
    ("psy", "Psychologues"),
    ("accueil", "Accueil"),
    ("parent", "Familles (responsables légaux)"),
    ("eleve", "Élèves"),
];

/// Libellé d'un rôle de la politique (l'identifiant brut si le rôle est inconnu de l'interface).
pub fn libelle_role_politique(role: &str) -> String {
    ROLES_POLITIQUE
        .iter()
        .find(|(id, _)| *id == role)
        .map(|(_, l)| (*l).to_string())
        .unwrap_or_else(|| role.to_string())
}

/// Rôle de la personne connectée, ramené à son identifiant canonique de politique.
fn role_canonique(role: &str) -> Option<&'static str> {
    let r = Role::depuis(role)?;
    Some(match r {
        Role::Admin => "admin",
        Role::Direction => "direction",
        Role::Administratif => "administratif",
        Role::Cpe => "cpe",
        Role::Compta => "compta",
        Role::Prof => "prof",
        Role::Infirmerie => "infirmerie",
        Role::Psy => "psy",
        Role::Accueil => "accueil",
        Role::Personnel => "personnel",
        Role::Parent => "parent",
        Role::Eleve => "eleve",
    })
}

/// Édition de la politique : ensemble des rôles cochés, comparé à l'état enregistré.
/// Les rôles que l'interface ne connaît pas sont conservés tels quels à l'enregistrement
/// (on ne retire jamais en silence une règle posée par ailleurs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditionPolitique {
    enregistre: BTreeSet<String>,
    courant: BTreeSet<String>,
}

impl EditionPolitique {
    pub fn nouvelle(roles: &[String]) -> Self {
        let ensemble: BTreeSet<String> = roles.iter().map(|r| r.trim().to_lowercase()).collect();
        Self {
            enregistre: ensemble.clone(),
            courant: ensemble,
        }
    }

    pub fn est_coche(&self, role: &str) -> bool {
        self.courant.contains(role)
    }

    pub fn definir(&mut self, role: &str, coche: bool) {
        if coche {
            self.courant.insert(role.to_string());
        } else {
            self.courant.remove(role);
        }
    }

    pub fn est_modifiee(&self) -> bool {
        self.enregistre != self.courant
    }

    /// Rôles nouvellement soumis à l'obligation (ordre de la liste affichée).
    pub fn ajoutes(&self) -> Vec<String> {
        self.ordonner(self.courant.difference(&self.enregistre))
    }

    /// Rôles dispensés de l'obligation.
    pub fn retires(&self) -> Vec<String> {
        self.ordonner(self.enregistre.difference(&self.courant))
    }

    /// Liste à envoyer au serveur (le serveur la trie et la valide).
    pub fn a_envoyer(&self) -> Vec<String> {
        self.ordonner(self.courant.iter())
    }

    /// Après un enregistrement réussi : l'état renvoyé par le serveur fait foi.
    pub fn apres_enregistrement(&mut self, roles_serveur: &[String]) {
        *self = Self::nouvelle(roles_serveur);
    }

    /// Rôles connus dans l'ordre d'affichage, puis les inconnus par ordre alphabétique.
    fn ordonner<'a>(&self, roles: impl Iterator<Item = &'a String>) -> Vec<String> {
        let a_garder: BTreeSet<&String> = roles.collect();
        let mut sortie: Vec<String> = ROLES_POLITIQUE
            .iter()
            .filter(|(id, _)| a_garder.iter().any(|r| r.as_str() == *id))
            .map(|(id, _)| (*id).to_string())
            .collect();
        for r in a_garder {
            if !ROLES_POLITIQUE.iter().any(|(id, _)| *id == r.as_str()) {
                sortie.push(r.clone());
            }
        }
        sortie
    }

    /// Phrase de confirmation : ce qui va changer, en français.
    pub fn resume_changements(&self) -> String {
        let noms = |roles: Vec<String>| {
            roles
                .iter()
                .map(|r| libelle_role_politique(r))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut morceaux = Vec::new();
        let ajoutes = self.ajoutes();
        if !ajoutes.is_empty() {
            morceaux.push(format!(
                "Passkey rendue obligatoire pour : {}.",
                noms(ajoutes)
            ));
        }
        let retires = self.retires();
        if !retires.is_empty() {
            morceaux.push(format!(
                "Passkey redevenue facultative pour : {}.",
                noms(retires)
            ));
        }
        if morceaux.is_empty() {
            "Aucun changement.".to_string()
        } else {
            morceaux.join(" ")
        }
    }

    /// Avertissement si la modification verrouille la personne qui l'enregistre : son propre
    /// rôle devient obligatoire alors qu'elle n'a pas de passkey.
    pub fn avertissement_auto_verrouillage(
        &self,
        mon_role: &str,
        mes_passkeys: i64,
    ) -> Option<&'static str> {
        let moi = role_canonique(mon_role)?;
        (mes_passkeys <= 0 && self.ajoutes().iter().any(|r| r == moi)).then_some(
            "Votre propre rôle devient soumis à l'obligation et vous n'avez pas de passkey : vous devrez en enregistrer une dès votre prochaine action.",
        )
    }

    /// Avertissement général sur l'effet pour les personnes concernées.
    pub fn avertissement_effet(&self) -> Option<&'static str> {
        (!self.ajoutes().is_empty()).then_some(
            "Les personnes de ces rôles qui n'ont pas encore de passkey devront en enregistrer une avant de continuer à utiliser l'ENT.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};
    use uuid::Uuid;

    fn appareil(n: u128, courante: bool, vu: i64, nom: Option<&str>) -> AppareilSession {
        let base = Utc.with_ymd_and_hms(2026, 11, 5, 8, 0, 0).unwrap();
        AppareilSession {
            id: Uuid::from_u128(n),
            client: "web".into(),
            assurance: "passkey".into(),
            appareil: nom.map(String::from),
            created_at: base,
            last_seen_at: base + chrono::Duration::minutes(vu),
            expires_at: base + chrono::Duration::hours(12),
            courante,
        }
    }

    #[test]
    fn libelles_du_niveau_de_session() {
        assert_eq!(libelle_assurance("passkey"), "Passkey");
        assert_eq!(libelle_assurance("sso"), "Connexion unique (SSO)");
        assert_eq!(libelle_assurance("mot_de_passe"), "Mot de passe");
        assert_eq!(libelle_assurance("autre"), "Inconnu");
        assert!(assurance_faible("mot_de_passe"));
        assert!(!assurance_faible("passkey") && !assurance_faible("sso"));
        assert_eq!(libelle_client("mobile"), "Application mobile");
        assert_eq!(libelle_client("web"), "Navigateur web");
        assert_eq!(libelle_client("x"), "Autre appareil");
        for a in ["passkey", "sso", "mot_de_passe", "?"] {
            assert!(!explication_assurance(a).is_empty());
        }
    }

    #[test]
    fn obligation_selon_le_role_et_les_passkeys() {
        assert!(libelle_obligation(true, 0).contains("vous devez"));
        assert!(libelle_obligation(true, 2).contains("en place"));
        assert!(libelle_obligation(false, 1).contains("enregistrée"));
        assert!(libelle_obligation(false, 0).contains("facultative"));
    }

    #[test]
    fn mot_de_passe_exige_sauf_en_enrolement() {
        assert!(mot_de_passe_requis(Portee::Complete));
        assert!(!mot_de_passe_requis(Portee::Enrolement));
        assert!(valider_mot_de_passe("", true).is_err());
        assert!(valider_mot_de_passe("x", true).is_ok());
        assert!(
            valider_mot_de_passe("", false).is_ok(),
            "vide accepté en enrôlement"
        );
    }

    #[test]
    fn avertissement_avant_de_supprimer_la_derniere_passkey() {
        assert!(avertissement_suppression(1, true)
            .unwrap()
            .contains("vous devrez en enregistrer"));
        assert!(avertissement_suppression(1, false)
            .unwrap()
            .contains("mot de passe seul"));
        assert_eq!(avertissement_suppression(2, true), None);
        assert_eq!(avertissement_suppression(3, false), None);
    }

    #[test]
    fn messages_des_actions_sensibles() {
        let faux = ApiError::depuis_statut(403, "mot de passe actuel requis ou incorrect".into());
        assert_eq!(
            message_action_sensible(&faux),
            "Mot de passe actuel incorrect."
        );
        let plafond = ApiError::depuis_statut(409, "plafond de 10 passkeys".into());
        assert!(message_action_sensible(&plafond).contains("maximal"));
        assert!(message_action_sensible(&ApiError::TropDeRequetes).contains("Trop de tentatives"));
        // Autre refus : message générique, sans reprendre le texte du serveur.
        let autre = ApiError::depuis_statut(403, "réservé".into());
        assert!(message_action_sensible(&autre).contains("droits"));
    }

    #[test]
    fn noms_bornes_et_sur_une_ligne() {
        let p = PasskeyCompte {
            id: Uuid::nil(),
            name: Some(format!("Téléphone\n<b>{}</b>", "x".repeat(300))),
            created_at: Utc::now(),
            last_used_at: None,
        };
        let n = nom_passkey(&p);
        assert!(!n.contains('\n') && n.chars().count() <= NOM_MAX + 1, "{n}");
        let sans = PasskeyCompte {
            name: Some("   ".into()),
            ..p.clone()
        };
        assert_eq!(nom_passkey(&sans), "Passkey sans nom");
        assert_eq!(
            nom_passkey(&PasskeyCompte { name: None, ..p }),
            "Passkey sans nom"
        );
    }

    #[test]
    fn appareils_courant_d_abord_puis_plus_recent() {
        let liste = vec![
            appareil(1, false, 10, None),
            appareil(2, false, 50, None),
            appareil(3, true, 5, None),
        ];
        let ids: Vec<u128> = trier_appareils(liste)
            .into_iter()
            .map(|a| a.id.as_u128())
            .collect();
        assert_eq!(ids, [3, 2, 1]);
    }

    #[test]
    fn nom_d_appareil_de_repli_et_borne() {
        assert_eq!(nom_appareil(&appareil(1, false, 0, None)), "Navigateur web");
        let mut mobile = appareil(2, false, 0, Some(" "));
        mobile.client = "mobile".into();
        assert_eq!(nom_appareil(&mobile), "Application mobile");
        let long = appareil(3, false, 0, Some(&"Mozilla ".repeat(60)));
        assert!(nom_appareil(&long).chars().count() <= NOM_MAX + 1);
        assert_eq!(
            nom_appareil(&appareil(4, false, 0, Some("Firefox sur Linux"))),
            "Firefox sur Linux"
        );
    }

    #[test]
    fn date_en_francais() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 5)
            .unwrap()
            .and_hms_opt(9, 7, 0)
            .unwrap();
        assert_eq!(horodatage_fr(d), "5 novembre 2026 à 09:07");
    }

    #[test]
    fn roles_de_la_politique_alignes_sur_les_roles_de_l_interface() {
        for (id, libelle) in ROLES_POLITIQUE {
            assert!(Role::depuis(id).is_some(), "{id} inconnu des droits");
            assert_eq!(role_canonique(id), Some(id), "{id} non canonique");
            assert!(!libelle.is_empty());
        }
        let mut ids: Vec<_> = ROLES_POLITIQUE.iter().map(|(i, _)| *i).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 11, "pas de doublon");
        assert_eq!(libelle_role_politique("prof"), "Enseignants");
        assert_eq!(libelle_role_politique("futur_role"), "futur_role");
    }

    fn v(roles: &[&str]) -> Vec<String> {
        roles.iter().map(|r| r.to_string()).collect()
    }

    #[test]
    fn edition_sans_changement() {
        let e = EditionPolitique::nouvelle(&v(&["prof", "cpe"]));
        assert!(!e.est_modifiee());
        assert!(e.est_coche("prof") && !e.est_coche("eleve"));
        assert_eq!(e.resume_changements(), "Aucun changement.");
        assert_eq!(e.a_envoyer(), v(&["cpe", "prof"]), "ordre d'affichage");
    }

    #[test]
    fn edition_ajouts_retraits_et_annulation() {
        let mut e = EditionPolitique::nouvelle(&v(&["prof"]));
        e.definir("parent", true);
        e.definir("prof", false);
        assert!(e.est_modifiee());
        assert_eq!(e.ajoutes(), v(&["parent"]));
        assert_eq!(e.retires(), v(&["prof"]));
        let resume = e.resume_changements();
        assert!(resume.contains("obligatoire pour : Familles"), "{resume}");
        assert!(
            resume.contains("facultative pour : Enseignants"),
            "{resume}"
        );
        // Revenir à l'état de départ = plus de modification.
        e.definir("parent", false);
        e.definir("prof", true);
        assert!(!e.est_modifiee());
    }

    #[test]
    fn edition_conserve_les_roles_inconnus_de_l_interface() {
        let mut e = EditionPolitique::nouvelle(&v(&["prof", "role_futur"]));
        e.definir("eleve", true);
        assert_eq!(e.a_envoyer(), v(&["prof", "eleve", "role_futur"]));
        assert!(e.retires().is_empty());
    }

    #[test]
    fn edition_apres_enregistrement_prend_l_etat_du_serveur() {
        let mut e = EditionPolitique::nouvelle(&[]);
        e.definir("psy", true);
        e.apres_enregistrement(&v(&["psy"]));
        assert!(!e.est_modifiee() && e.est_coche("psy"));
    }

    #[test]
    fn avertissement_si_on_se_verrouille_soi_meme() {
        let mut e = EditionPolitique::nouvelle(&[]);
        e.definir("direction", true);
        assert!(e.avertissement_auto_verrouillage("direction", 0).is_some());
        assert!(e.avertissement_auto_verrouillage("direction", 1).is_none());
        assert!(e.avertissement_auto_verrouillage("admin", 0).is_none());
        assert!(e.avertissement_effet().is_some());
        let sans_ajout = EditionPolitique::nouvelle(&v(&["direction"]));
        assert!(sans_ajout
            .avertissement_auto_verrouillage("direction", 0)
            .is_none());
        assert!(sans_ajout.avertissement_effet().is_none());
    }
}
