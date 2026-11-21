//! Logique d'écran des préférences de notification (pure, sans Leptos ni réseau).
//!
//! Le serveur reste l'autorité : un canal n'est utilisé que si la personne l'a choisi
//! **et** qu'elle est joignable ; les heures calmes sont en heure de Paris.

use ent_types::communication::{EnvoiDto, PreferencesDto};
use ent_web_ui::TonEtat;

/// Saisie du formulaire (heures en « HH:MM »).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaisiePreferences {
    pub email: bool,
    pub sms: bool,
    pub push: bool,
    pub quotidien: bool,
    pub calme: bool,
    pub debut: String,
    pub fin: String,
}

impl SaisiePreferences {
    pub fn depuis(p: &PreferencesDto) -> Self {
        Self {
            email: p.email,
            sms: p.sms,
            push: p.push,
            quotidien: p.resume == "quotidien",
            calme: p.calme_debut.is_some() && p.calme_fin.is_some(),
            debut: p.calme_debut.map(hhmm).unwrap_or_else(|| "21:00".into()),
            fin: p.calme_fin.map(hhmm).unwrap_or_else(|| "07:00".into()),
        }
    }
}

/// « 07:30 » → 450.
pub fn minutes(hhmm: &str) -> Option<u16> {
    let (h, m) = hhmm.trim().split_once(':')?;
    let (h, m): (u16, u16) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// 450 → « 07:30 ».
pub fn hhmm(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// Construit le corps à envoyer, ou le message de l'erreur de saisie.
pub fn composer(s: &SaisiePreferences) -> Result<PreferencesDto, String> {
    let (calme_debut, calme_fin) = if s.calme {
        let d = minutes(&s.debut).ok_or("Heure de début des heures calmes invalide.")?;
        let f = minutes(&s.fin).ok_or("Heure de fin des heures calmes invalide.")?;
        if d == f {
            return Err("Le début et la fin des heures calmes doivent différer.".into());
        }
        (Some(d), Some(f))
    } else {
        (None, None)
    };
    Ok(PreferencesDto {
        email: s.email,
        sms: s.sms,
        push: s.push,
        resume: if s.quotidien { "quotidien" } else { "immediat" }.into(),
        calme_debut,
        calme_fin,
    })
}

pub fn libelle_canal(canal: &str) -> &'static str {
    match canal {
        "email" => "E-mail",
        "sms" => "SMS",
        "push" => "Notification sur mobile",
        _ => "Canal",
    }
}

pub fn libelle_statut_envoi(statut: &str) -> &'static str {
    match statut {
        "en_attente" => "Programmé",
        "envoi" => "En cours",
        "envoye" => "Envoyé",
        "simule" => "Simulé",
        "echec" => "Échec",
        "ignore" => "Non envoyé",
        "plafonne" => "Plafond atteint",
        "regroupe" => "Dans un résumé",
        _ => "Inconnu",
    }
}

pub fn ton_statut_envoi(statut: &str) -> TonEtat {
    match statut {
        "envoye" | "regroupe" => TonEtat::Ok,
        "echec" => TonEtat::Mal,
        "plafonne" | "ignore" => TonEtat::Avertissement,
        _ => TonEtat::Neutre,
    }
}

/// Raison lisible d'un envoi non parti.
pub fn libelle_motif(motif: Option<&str>) -> Option<&'static str> {
    Some(match motif? {
        "droit_retire" => "Vous n'avez plus accès à cette information.",
        "sans_adresse" => "Aucune adresse e-mail enregistrée.",
        "sans_telephone" => "Aucun numéro de téléphone enregistré.",
        "sans_appareil" => "Aucun appareil mobile enregistré.",
        "plafond_personne" => "Limite quotidienne de SMS atteinte.",
        "plafond_etablissement" => "Limite quotidienne de SMS de l'établissement atteinte.",
        "canal_non_configure" => "Canal non activé par l'établissement.",
        "interrompu" => "Envoi interrompu, non renouvelé.",
        _ => return None,
    })
}

/// Résumé d'un envoi : « E-mail — Retard de Harry — Envoyé ».
pub fn ligne_envoi(e: &EnvoiDto) -> String {
    format!(
        "{} — {} — {}",
        libelle_canal(&e.canal),
        e.titre,
        libelle_statut_envoi(&e.statut)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaut() -> PreferencesDto {
        PreferencesDto {
            email: true,
            sms: false,
            push: true,
            resume: "immediat".into(),
            calme_debut: None,
            calme_fin: None,
        }
    }

    #[test]
    fn heures() {
        assert_eq!(minutes("07:30"), Some(450));
        assert_eq!(minutes("24:00"), None);
        assert_eq!(minutes("7h"), None);
        assert_eq!(hhmm(1260), "21:00");
    }

    #[test]
    fn aller_retour_des_preferences() {
        let mut p = defaut();
        p.sms = true;
        p.resume = "quotidien".into();
        p.calme_debut = Some(1260);
        p.calme_fin = Some(420);
        let s = SaisiePreferences::depuis(&p);
        assert!(s.sms && s.quotidien && s.calme);
        assert_eq!((s.debut.as_str(), s.fin.as_str()), ("21:00", "07:00"));
        assert_eq!(composer(&s).unwrap(), p);
    }

    #[test]
    fn heures_calmes_desactivees_ou_invalides() {
        let mut s = SaisiePreferences::depuis(&defaut());
        assert!(!s.calme);
        let p = composer(&s).unwrap();
        assert_eq!((p.calme_debut, p.calme_fin), (None, None));
        s.calme = true;
        s.debut = "22:00".into();
        s.fin = "22:00".into();
        assert!(composer(&s).is_err(), "début = fin");
        s.fin = "pas une heure".into();
        assert!(composer(&s).is_err());
    }

    #[test]
    fn le_sms_est_desactive_par_defaut_cote_ecran() {
        assert!(!SaisiePreferences::depuis(&defaut()).sms);
    }

    #[test]
    fn libelles_d_envoi() {
        assert_eq!(libelle_statut_envoi("plafonne"), "Plafond atteint");
        assert_eq!(ton_statut_envoi("echec"), TonEtat::Mal);
        assert!(libelle_motif(Some("droit_retire")).is_some());
        assert!(libelle_motif(Some("inconnu")).is_none() && libelle_motif(None).is_none());
    }
}
