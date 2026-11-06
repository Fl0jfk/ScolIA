//! Logique d'écran de l'accompagnement (socle 2, lot S2-11) : composition des corps envoyés à
//! l'API et contrôles de forme. Pure : ni Leptos ni réseau. Les droits et les règles (catalogue
//! fermé des aménagements, clôture qui retire les aménagements liés) restent côté serveur.

use ent_types::sante::{
    AffecterAeshBody, DefinirAmenagementBody, OuvrirDispositifBody, PlanifierReunionBody,
};
use uuid::Uuid;

use crate::composants_metier::sante::{date_facultative, date_obligatoire, texte_facultatif};

/// Types de dispositifs proposés (le PAI a son propre parcours à l'infirmerie).
pub const KINDS: [(&str, &str); 6] = [
    ("pap", "PAP — plan d'accompagnement personnalisé"),
    ("pps", "PPS — projet personnalisé de scolarisation"),
    (
        "ppre",
        "PPRE — programme personnalisé de réussite éducative",
    ),
    ("ulis", "ULIS"),
    ("gevasco", "GEVA-Sco"),
    ("autre", "Autre dispositif"),
];

pub const PORTEES: [(&str, &str); 3] = [
    ("les_deux", "En cours et aux examens"),
    ("cours", "En cours"),
    ("examen", "Aux examens"),
];

pub const REUNIONS: [(&str, &str); 2] = [
    ("equipe_suivi", "Équipe de suivi"),
    ("equipe_educative", "Équipe éducative"),
];

pub type ErreurChamp = (String, String);

fn err(champ: &str, message: &str) -> Vec<ErreurChamp> {
    vec![(champ.to_string(), message.to_string())]
}

#[derive(Clone, Debug, Default)]
pub struct SaisieDispositif {
    pub kind: String,
    pub libelle: String,
    pub debut: String,
    pub revision: String,
}

pub fn composer_dispositif(s: &SaisieDispositif) -> Result<OuvrirDispositifBody, Vec<ErreurChamp>> {
    if !KINDS.iter().any(|(k, _)| *k == s.kind) {
        return Err(err("dis-kind", "Choisissez le type de dispositif."));
    }
    if s.libelle.chars().count() > 120 {
        return Err(err("dis-libelle", "120 caractères au plus."));
    }
    let debut = date_facultative(&s.debut, "Début").map_err(|m| err("dis-debut", &m))?;
    let revision =
        date_facultative(&s.revision, "Révision").map_err(|m| err("dis-revision", &m))?;
    if let (Some(d), Some(r)) = (debut, revision) {
        if r < d {
            return Err(err("dis-revision", "La révision précède le début."));
        }
    }
    Ok(OuvrirDispositifBody {
        kind: s.kind.clone(),
        libelle: texte_facultatif(&s.libelle),
        document_id: None,
        date_debut: debut,
        date_revision: revision,
    })
}

#[derive(Clone, Debug, Default)]
pub struct SaisieAmenagement {
    pub code: String,
    pub portee: String,
    /// Identifiant du dispositif lié (vide : aucun).
    pub dispositif: String,
    pub fin: String,
}

pub fn composer_amenagement(
    s: &SaisieAmenagement,
) -> Result<DefinirAmenagementBody, Vec<ErreurChamp>> {
    if s.code.trim().is_empty() {
        return Err(err("ame-code", "Choisissez l'aménagement."));
    }
    if !PORTEES.iter().any(|(p, _)| *p == s.portee) {
        return Err(err("ame-portee", "Choisissez quand il s'applique."));
    }
    let dispositif = match s.dispositif.trim() {
        "" => None,
        t => Some(Uuid::parse_str(t).map_err(|_| err("ame-dispositif", "Dispositif invalide."))?),
    };
    let fin = date_facultative(&s.fin, "Fin").map_err(|m| err("ame-fin", &m))?;
    Ok(DefinirAmenagementBody {
        code: s.code.trim().to_string(),
        portee: s.portee.clone(),
        dispositif_id: dispositif,
        date_fin: fin,
    })
}

#[derive(Clone, Debug, Default)]
pub struct SaisieAesh {
    pub nom: String,
    pub heures: String,
    pub debut: String,
    pub fin: String,
}

pub fn composer_aesh(s: &SaisieAesh) -> Result<AffecterAeshBody, Vec<ErreurChamp>> {
    let nom = s.nom.trim();
    if nom.is_empty() || nom.chars().count() > 120 {
        return Err(err(
            "aes-nom",
            "Nom de l'AESH obligatoire (120 caractères au plus).",
        ));
    }
    let heures = match s.heures.trim() {
        "" => None,
        t => {
            let h: i16 = t
                .parse()
                .map_err(|_| err("aes-heures", "Nombre d'heures invalide."))?;
            if !(1..=60).contains(&h) {
                return Err(err("aes-heures", "Entre 1 et 60 heures par semaine."));
            }
            Some(h)
        }
    };
    let debut = date_obligatoire(&s.debut, "Début").map_err(|m| err("aes-debut", &m))?;
    let fin = date_facultative(&s.fin, "Fin").map_err(|m| err("aes-fin", &m))?;
    if fin.is_some_and(|f| f < debut) {
        return Err(err("aes-fin", "La fin précède le début."));
    }
    Ok(AffecterAeshBody {
        aesh_nom: nom.to_string(),
        heures_hebdo: heures,
        dispositif_id: None,
        date_debut: debut,
        date_fin: fin,
    })
}

pub fn composer_reunion(kind: &str, date: &str) -> Result<PlanifierReunionBody, Vec<ErreurChamp>> {
    if !REUNIONS.iter().any(|(k, _)| *k == kind) {
        return Err(err("reu-kind", "Choisissez le type de réunion."));
    }
    let d = date_obligatoire(date, "Date").map_err(|m| err("reu-date", &m))?;
    Ok(PlanifierReunionBody {
        kind: kind.to_string(),
        date_reunion: d,
        dispositif_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispositif() {
        let ok = composer_dispositif(&SaisieDispositif {
            kind: "pap".into(),
            libelle: " Suivi ".into(),
            debut: "2026-09-01".into(),
            revision: "2027-09-01".into(),
        })
        .unwrap();
        assert_eq!(ok.libelle.as_deref(), Some("Suivi"));
        // Le PAI n'est pas un dispositif d'accompagnement : refusé avant l'envoi.
        let e = composer_dispositif(&SaisieDispositif {
            kind: "pai".into(),
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(e[0].0, "dis-kind");
        let e = composer_dispositif(&SaisieDispositif {
            kind: "pps".into(),
            debut: "2026-09-01".into(),
            revision: "2026-08-01".into(),
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(e[0].0, "dis-revision");
        let e = composer_dispositif(&SaisieDispositif {
            kind: "pps".into(),
            libelle: "x".repeat(121),
            ..Default::default()
        })
        .unwrap_err();
        assert_eq!(e[0].0, "dis-libelle");
    }

    #[test]
    fn amenagement() {
        let ok = composer_amenagement(&SaisieAmenagement {
            code: "tiers_temps".into(),
            portee: "examen".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(ok.portee, "examen");
        assert!(ok.dispositif_id.is_none());
        for (s, champ) in [
            (
                SaisieAmenagement {
                    portee: "cours".into(),
                    ..Default::default()
                },
                "ame-code",
            ),
            (
                SaisieAmenagement {
                    code: "x".into(),
                    portee: "partout".into(),
                    ..Default::default()
                },
                "ame-portee",
            ),
            (
                SaisieAmenagement {
                    code: "x".into(),
                    portee: "cours".into(),
                    dispositif: "pas-un-uuid".into(),
                    ..Default::default()
                },
                "ame-dispositif",
            ),
            (
                SaisieAmenagement {
                    code: "x".into(),
                    portee: "cours".into(),
                    fin: "31/12".into(),
                    ..Default::default()
                },
                "ame-fin",
            ),
        ] {
            assert_eq!(composer_amenagement(&s).unwrap_err()[0].0, champ);
        }
        let lie = composer_amenagement(&SaisieAmenagement {
            code: "x".into(),
            portee: "cours".into(),
            dispositif: Uuid::from_u128(3).to_string(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(lie.dispositif_id, Some(Uuid::from_u128(3)));
    }

    #[test]
    fn aesh_et_reunion() {
        let ok = composer_aesh(&SaisieAesh {
            nom: " Mme Lupin ".into(),
            heures: "12".into(),
            debut: "2026-09-01".into(),
            fin: "".into(),
        })
        .unwrap();
        assert_eq!(
            (ok.aesh_nom.as_str(), ok.heures_hebdo),
            ("Mme Lupin", Some(12))
        );
        for (s, champ) in [
            (
                SaisieAesh {
                    debut: "2026-09-01".into(),
                    ..Default::default()
                },
                "aes-nom",
            ),
            (
                SaisieAesh {
                    nom: "A".into(),
                    heures: "61".into(),
                    debut: "2026-09-01".into(),
                    ..Default::default()
                },
                "aes-heures",
            ),
            (
                SaisieAesh {
                    nom: "A".into(),
                    heures: "abc".into(),
                    debut: "2026-09-01".into(),
                    ..Default::default()
                },
                "aes-heures",
            ),
            (
                SaisieAesh {
                    nom: "A".into(),
                    ..Default::default()
                },
                "aes-debut",
            ),
            (
                SaisieAesh {
                    nom: "A".into(),
                    debut: "2026-09-02".into(),
                    fin: "2026-09-01".into(),
                    ..Default::default()
                },
                "aes-fin",
            ),
        ] {
            assert_eq!(composer_aesh(&s).unwrap_err()[0].0, champ);
        }
        assert!(composer_reunion("equipe_suivi", "2026-11-12").is_ok());
        assert_eq!(
            composer_reunion("autre", "2026-11-12").unwrap_err()[0].0,
            "reu-kind"
        );
        assert_eq!(
            composer_reunion("equipe_suivi", "").unwrap_err()[0].0,
            "reu-date"
        );
    }
}
