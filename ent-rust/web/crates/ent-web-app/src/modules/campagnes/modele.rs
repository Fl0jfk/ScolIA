//! Logique d'écran des campagnes d'information (pure, sans Leptos ni réseau).
//!
//! Règle d'interface : **aucun envoi sans confirmation humaine**. L'aperçu est calculé par le
//! serveur (droits de l'auteur, liens actifs des responsables) ; la confirmation renvoie son
//! empreinte, que le serveur recalcule. Le bouton de confirmation n'est proposé que pour un
//! aperçu non vide, sans dépassement de plafond SMS et, s'il y a des SMS, avec la case de
//! seconde confirmation cochée.

use ent_types::communication::{
    ApercuCampagneDto, CampagneDto, ConfirmationLancementBody, NouvelleCampagneBody,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

/// Saisie du formulaire de campagne.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaisieCampagne {
    pub titre: String,
    pub corps: String,
    pub email: bool,
    pub sms: bool,
    pub push: bool,
    /// `etablissement` ou `classe`.
    pub audience: String,
    pub classe_id: String,
}

pub fn composer(s: &SaisieCampagne) -> Result<NouvelleCampagneBody, String> {
    let titre = s.titre.trim();
    let corps = s.corps.trim();
    if titre.is_empty() || titre.chars().count() > 200 {
        return Err("Le titre est obligatoire (200 caractères au plus).".into());
    }
    if corps.is_empty() || corps.chars().count() > 5000 {
        return Err("Le message est obligatoire (5 000 caractères au plus).".into());
    }
    let mut canaux = Vec::new();
    if s.email {
        canaux.push("email".to_string());
    }
    if s.sms {
        canaux.push("sms".to_string());
    }
    if s.push {
        canaux.push("push".to_string());
    }
    if canaux.is_empty() {
        return Err("Choisissez au moins un canal d'envoi.".into());
    }
    let cible_ids = match s.audience.as_str() {
        "etablissement" => Vec::new(),
        "classe" => vec![Uuid::parse_str(s.classe_id.trim())
            .map_err(|_| "Choisissez une classe.".to_string())?],
        _ => return Err("Audience inconnue.".into()),
    };
    Ok(NouvelleCampagneBody {
        titre: titre.to_string(),
        corps: corps.to_string(),
        canaux,
        audience: s.audience.clone(),
        cible_ids,
    })
}

pub fn libelle_canal(canal: &str) -> &'static str {
    match canal {
        "email" => "E-mail",
        "sms" => "SMS",
        "push" => "Mobile",
        _ => "Canal",
    }
}

/// Statut d'un envoi tel que rapporté dans le bilan d'une campagne.
pub fn libelle_statut_envoi(statut: &str) -> &'static str {
    match statut {
        "en_attente" => "programmés",
        "envoi" => "en cours",
        "envoye" => "envoyés",
        "simule" => "simulés",
        "echec" => "en échec",
        "ignore" => "non envoyés",
        "plafonne" => "plafonnés",
        "regroupe" => "dans un résumé",
        _ => "autres",
    }
}

pub fn libelle_audience(a: &str) -> &'static str {
    match a {
        "etablissement" => "Tout l'établissement",
        "niveau" => "Niveaux",
        "classe" => "Classes",
        "groupe" => "Groupes",
        _ => "Audience",
    }
}

pub fn libelle_statut(s: &str) -> &'static str {
    match s {
        "brouillon" => "Brouillon",
        "apercu" => "Aperçu prêt",
        "lancee" => "Lancée",
        "annulee" => "Annulée",
        _ => "Inconnu",
    }
}

pub fn ton_statut(s: &str) -> TonEtat {
    match s {
        "lancee" => TonEtat::Ok,
        "apercu" => TonEtat::Info,
        "annulee" => TonEtat::Neutre,
        _ => TonEtat::Avertissement,
    }
}

pub fn canaux_texte(c: &CampagneDto) -> String {
    c.canaux
        .iter()
        .map(|k| libelle_canal(k))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Une seconde confirmation est exigée dès que des SMS vont partir.
pub fn confirmation_sms_requise(a: &ApercuCampagneDto) -> bool {
    a.sms.as_ref().is_some_and(|s| s.destinataires > 0)
}

/// Le lancement peut-il être proposé ?
pub fn peut_confirmer(a: &ApercuCampagneDto, sms_coche: bool) -> bool {
    a.nb_destinataires > 0
        && !a.sms.as_ref().is_some_and(|s| s.depasse)
        && (!confirmation_sms_requise(a) || sms_coche)
}

/// Pourquoi le lancement n'est pas (encore) possible.
pub fn obstacle(a: &ApercuCampagneDto, sms_coche: bool) -> Option<&'static str> {
    if a.nb_destinataires == 0 {
        Some("Aucun destinataire : élargissez l'audience ou vérifiez vos droits.")
    } else if a.sms.as_ref().is_some_and(|s| s.depasse) {
        Some("Le nombre de SMS dépasse un plafond : retirez le canal SMS ou réduisez l'audience.")
    } else if confirmation_sms_requise(a) && !sms_coche {
        Some("Cochez la confirmation des SMS pour pouvoir lancer la campagne.")
    } else {
        None
    }
}

/// Résumé lisible de l'aperçu (une ligne par fait important).
pub fn lignes_apercu(a: &ApercuCampagneDto) -> Vec<String> {
    let mut v = vec![format!(
        "{} responsable(s) pour {} élève(s).",
        a.nb_destinataires, a.nb_eleves
    )];
    for (canal, n) in &a.par_canal {
        v.push(format!("{} : {n} destinataire(s).", libelle_canal(canal)));
    }
    if a.sans_canal_externe > 0 {
        v.push(format!(
            "{} responsable(s) ne seront prévenus que dans l'ENT (aucun canal choisi joignable).",
            a.sans_canal_externe
        ));
    }
    if a.exclus_hors_droits > 0 {
        v.push(format!(
            "{} élève(s) de l'audience sont hors de vos droits et ne sont pas concernés.",
            a.exclus_hors_droits
        ));
    }
    if let Some(s) = &a.sms {
        v.push(format!(
            "SMS : {} message(s) de {} segment(s), soit environ {} crédit(s) ; plafond par campagne {}, reste aujourd'hui {}.",
            s.destinataires, s.segments_par_message, s.credits_estimes, s.plafond_campagne, s.plafond_jour_restant
        ));
        if s.tronque {
            v.push("Le message est trop long pour un SMS : il sera tronqué.".into());
        }
    }
    v
}

/// Texte de la fenêtre de confirmation finale.
pub fn message_confirmation(a: &ApercuCampagneDto) -> String {
    format!(
        "Vous allez envoyer ce message à {} responsable(s). L'envoi ne peut pas être repris.",
        a.nb_destinataires
    )
}

pub fn corps_lancement(a: &ApercuCampagneDto, sms_coche: bool) -> ConfirmationLancementBody {
    ConfirmationLancementBody {
        empreinte: a.empreinte.clone(),
        nb_destinataires: a.nb_destinataires,
        confirme_sms: sms_coche,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_types::communication::EstimationSmsDto;
    use std::collections::BTreeMap;

    fn saisie() -> SaisieCampagne {
        SaisieCampagne {
            titre: "Réunion".into(),
            corps: "Jeudi 18 h".into(),
            email: true,
            sms: false,
            push: false,
            audience: "etablissement".into(),
            classe_id: String::new(),
        }
    }

    fn apercu(nb: usize, sms: Option<EstimationSmsDto>) -> ApercuCampagneDto {
        ApercuCampagneDto {
            campagne_id: Uuid::nil(),
            nb_destinataires: nb,
            nb_eleves: nb,
            par_canal: BTreeMap::from([("email".to_string(), nb)]),
            sans_canal_externe: 0,
            exclus_hors_droits: 0,
            sms,
            empreinte: "abc".into(),
            destinataires: vec![],
            tronque: false,
        }
    }

    fn sms(n: usize, depasse: bool) -> EstimationSmsDto {
        EstimationSmsDto {
            destinataires: n,
            segments_par_message: 1,
            credits_estimes: n,
            plafond_campagne: 300,
            plafond_jour_restant: 200,
            depasse,
            tronque: false,
        }
    }

    #[test]
    fn composition_et_validation() {
        let c = composer(&saisie()).unwrap();
        assert_eq!(c.canaux, ["email"]);
        assert!(c.cible_ids.is_empty());
        let mut s = saisie();
        s.titre = "  ".into();
        assert!(composer(&s).is_err());
        let mut s = saisie();
        s.email = false;
        assert!(composer(&s).is_err(), "au moins un canal");
        let mut s = saisie();
        s.audience = "classe".into();
        assert!(composer(&s).is_err(), "classe obligatoire");
        s.classe_id = Uuid::from_u128(7).to_string();
        assert_eq!(composer(&s).unwrap().cible_ids, [Uuid::from_u128(7)]);
    }

    #[test]
    fn confirmation_obligatoire_et_seconde_pour_les_sms() {
        let sans_sms = apercu(3, None);
        assert!(peut_confirmer(&sans_sms, false));
        assert!(
            !peut_confirmer(&apercu(0, None), false),
            "aucun destinataire"
        );
        let avec = apercu(3, Some(sms(2, false)));
        assert!(confirmation_sms_requise(&avec));
        assert!(
            !peut_confirmer(&avec, false),
            "seconde confirmation requise"
        );
        assert!(peut_confirmer(&avec, true));
        assert!(obstacle(&avec, false).is_some() && obstacle(&avec, true).is_none());
        let depasse = apercu(3, Some(sms(2, true)));
        assert!(!peut_confirmer(&depasse, true), "plafond dépassé");
        // Aucun SMS réellement envoyé : pas de seconde confirmation.
        assert!(!confirmation_sms_requise(&apercu(3, Some(sms(0, false)))));
    }

    #[test]
    fn corps_de_lancement_reprend_l_empreinte_de_l_apercu() {
        let a = apercu(4, Some(sms(1, false)));
        let c = corps_lancement(&a, true);
        assert_eq!(
            (c.empreinte.as_str(), c.nb_destinataires, c.confirme_sms),
            ("abc", 4, true)
        );
    }

    #[test]
    fn apercu_lisible() {
        let mut a = apercu(4, Some(sms(2, false)));
        a.exclus_hors_droits = 5;
        a.sans_canal_externe = 1;
        let l = lignes_apercu(&a).join("\n");
        assert!(l.contains("4 responsable(s)") && l.contains("hors de vos droits"));
        assert!(l.contains("SMS : 2 message(s)") && l.contains("ENT"));
        assert!(message_confirmation(&a).contains('4'));
    }
}
