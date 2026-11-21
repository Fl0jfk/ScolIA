//! Logique d'écran de l'infirmerie (socle 2, lot S2-11) : composition des corps envoyés à
//! l'API et contrôles de forme. Pure : ni Leptos ni réseau. Les droits et les règles métier
//! (version du PAI, signatures, règle d'information de la famille) restent côté serveur.

use chrono::NaiveDate;
use ent_types::sante::{
    CloturePassageBody, EnregistrerPaiBody, NouveauPassageBody, PublierContrainteBody,
};
use uuid::Uuid;

use crate::composants_metier::sante::{
    date_facultative, date_obligatoire, heure_facultative, texte_facultatif,
};

pub const MAX_TEXTE: usize = 4000;

/// Erreur de saisie : (identifiant du champ, message).
pub type ErreurChamp = (String, String);

fn err(champ: &str, message: &str) -> Vec<ErreurChamp> {
    vec![(champ.to_string(), message.to_string())]
}

#[derive(Clone, Debug, Default)]
pub struct SaisiePassage {
    pub eleve: Option<Uuid>,
    pub jour: String,
    pub heure: String,
    pub motif: String,
    pub note_soins: String,
    pub soins: String,
    pub issue: String,
    pub visible_famille: bool,
    pub informer_famille: bool,
}

pub fn composer_passage(s: &SaisiePassage) -> Result<NouveauPassageBody, Vec<ErreurChamp>> {
    let eleve = s
        .eleve
        .ok_or_else(|| err("pas-eleve", "Choisissez l'élève."))?;
    let jour = date_obligatoire(&s.jour, "Jour").map_err(|m| err("pas-jour", &m))?;
    let heure = heure_facultative(&s.heure, "Heure d'arrivée").map_err(|m| err("pas-heure", &m))?;
    if s.motif.trim().is_empty() {
        return Err(err("pas-motif", "Choisissez le motif."));
    }
    let (note, soins) = (s.note_soins.trim(), s.soins.trim());
    if note.is_empty() && soins.is_empty() {
        return Err(err("pas-note", "Décrivez la prise en charge ou les soins."));
    }
    if note.chars().count() > MAX_TEXTE || soins.chars().count() > MAX_TEXTE {
        return Err(err(
            "pas-note",
            "Texte trop long (4 000 caractères au plus).",
        ));
    }
    Ok(NouveauPassageBody {
        eleve_id: eleve,
        jour,
        heure_arrivee: heure,
        motif: s.motif.trim().to_string(),
        note_soins: note.to_string(),
        soins: soins.to_string(),
        visible_famille: s.visible_famille,
        issue: if s.issue.trim().is_empty() {
            "non_renseignee".into()
        } else {
            s.issue.trim().to_string()
        },
        informer_famille: s.informer_famille,
    })
}

#[derive(Clone, Debug, Default)]
pub struct SaisieCloture {
    pub heure_sortie: String,
    pub issue: String,
    pub soins: String,
    /// Renvoi au domicile : personne qui récupère l'élève (texte vide : aucune).
    pub personne: String,
}

pub fn composer_cloture(s: &SaisieCloture) -> Result<CloturePassageBody, Vec<ErreurChamp>> {
    let heure =
        heure_facultative(&s.heure_sortie, "Heure de sortie").map_err(|m| err("clo-heure", &m))?;
    if s.issue.trim().is_empty() {
        return Err(err("clo-issue", "Choisissez l'issue."));
    }
    let personne = if s.issue == "renvoi_domicile" {
        let id = Uuid::parse_str(s.personne.trim()).map_err(|_| {
            err(
                "clo-personne",
                "Choisissez la personne qui récupère l'élève.",
            )
        })?;
        Some(id)
    } else {
        None
    };
    Ok(CloturePassageBody {
        heure_sortie: heure,
        issue: s.issue.trim().to_string(),
        soins: texte_facultatif(&s.soins),
        recupere_par_personne_id: personne,
    })
}

#[derive(Clone, Debug, Default)]
pub struct SaisiePai {
    pub motif: String,
    pub mesures: String,
    pub protocole: String,
    pub note_interne: String,
    pub debut: String,
    pub fin: String,
    pub actif: bool,
    pub revision: String,
    pub trousse: bool,
    pub trousse_lieu: String,
    pub trousse_peremption: String,
}

pub fn composer_pai(s: &SaisiePai) -> Result<EnregistrerPaiBody, Vec<ErreurChamp>> {
    let mut erreurs = Vec::new();
    if s.motif.trim().is_empty() {
        erreurs.push((
            "pai-motif".to_string(),
            "Le motif est obligatoire.".to_string(),
        ));
    }
    if s.mesures.trim().is_empty() {
        erreurs.push((
            "pai-mesures".to_string(),
            "Les mesures d'accueil sont obligatoires.".to_string(),
        ));
    }
    let debut = date_obligatoire(&s.debut, "Début");
    let fin = date_facultative(&s.fin, "Fin");
    let revision = date_facultative(&s.revision, "Révision");
    let peremption = date_facultative(&s.trousse_peremption, "Péremption");
    for (champ, r) in [
        ("pai-debut", debut.as_ref().err()),
        ("pai-fin", fin.as_ref().err()),
        ("pai-revision", revision.as_ref().err()),
        ("pai-peremption", peremption.as_ref().err()),
    ] {
        if let Some(m) = r {
            erreurs.push((champ.to_string(), m.clone()));
        }
    }
    if s.trousse && s.trousse_lieu.trim().is_empty() {
        erreurs.push((
            "pai-lieu".to_string(),
            "Indiquez où est rangée la trousse d'urgence.".to_string(),
        ));
    }
    if let (Ok(d), Ok(Some(f))) = (&debut, &fin) {
        if f < d {
            erreurs.push((
                "pai-fin".to_string(),
                "La fin précède le début.".to_string(),
            ));
        }
    }
    if let (Ok(d), Ok(Some(r))) = (&debut, &revision) {
        if r < d {
            erreurs.push((
                "pai-revision".to_string(),
                "La révision précède le début.".to_string(),
            ));
        }
    }
    if !erreurs.is_empty() {
        return Err(erreurs);
    }
    Ok(EnregistrerPaiBody {
        motif: s.motif.trim().to_string(),
        mesures_accueil: s.mesures.trim().to_string(),
        protocole_urgence: s.protocole.trim().to_string(),
        note_interne: s.note_interne.trim().to_string(),
        date_debut: debut.unwrap_or(NaiveDate::MIN),
        date_fin: fin.unwrap_or(None),
        actif: s.actif,
        date_revision: revision.unwrap_or(None),
        trousse_urgence: s.trousse,
        trousse_lieu: if s.trousse {
            s.trousse_lieu.trim().to_string()
        } else {
            String::new()
        },
        trousse_peremption: peremption.unwrap_or(None),
    })
}

#[derive(Clone, Debug, Default)]
pub struct SaisieContrainte {
    pub code: String,
    pub restauration: bool,
    pub enseignants: bool,
    pub sorties: bool,
    pub fin: String,
}

/// Audiences cochées, restreintes à celles que le catalogue prévoit pour ce code.
pub fn composer_contrainte(
    s: &SaisieContrainte,
    permises: &[String],
) -> Result<PublierContrainteBody, Vec<ErreurChamp>> {
    if s.code.trim().is_empty() {
        return Err(err("con-code", "Choisissez la contrainte."));
    }
    let mut audiences = Vec::new();
    for (coche, nom) in [
        (s.restauration, "restauration"),
        (s.enseignants, "enseignants"),
        (s.sorties, "sorties"),
    ] {
        if coche {
            if !permises.iter().any(|p| p == nom) {
                return Err(err(
                    "con-audiences",
                    "Cette contrainte ne se publie pas pour tous les destinataires cochés.",
                ));
            }
            audiences.push(nom.to_string());
        }
    }
    if audiences.is_empty() {
        return Err(err("con-audiences", "Cochez au moins un destinataire."));
    }
    let fin = date_facultative(&s.fin, "Fin").map_err(|m| err("con-fin", &m))?;
    Ok(PublierContrainteBody {
        code: s.code.trim().to_string(),
        audiences,
        date_fin: fin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passage() -> SaisiePassage {
        SaisiePassage {
            eleve: Some(Uuid::nil()),
            jour: "2026-11-05".into(),
            heure: "09:15".into(),
            motif: "soins".into(),
            note_soins: "pansement".into(),
            ..Default::default()
        }
    }

    #[test]
    fn passage_valide_et_erreurs() {
        let b = composer_passage(&passage()).unwrap();
        assert_eq!(b.issue, "non_renseignee");
        assert_eq!(
            b.heure_arrivee.map(|h| h.to_string()),
            Some("09:15:00".into())
        );
        for (modif, champ) in [
            (
                SaisiePassage {
                    eleve: None,
                    ..passage()
                },
                "pas-eleve",
            ),
            (
                SaisiePassage {
                    jour: "".into(),
                    ..passage()
                },
                "pas-jour",
            ),
            (
                SaisiePassage {
                    heure: "99:99".into(),
                    ..passage()
                },
                "pas-heure",
            ),
            (
                SaisiePassage {
                    motif: " ".into(),
                    ..passage()
                },
                "pas-motif",
            ),
            (
                SaisiePassage {
                    note_soins: "".into(),
                    soins: " ".into(),
                    ..passage()
                },
                "pas-note",
            ),
            (
                SaisiePassage {
                    note_soins: "x".repeat(MAX_TEXTE + 1),
                    ..passage()
                },
                "pas-note",
            ),
        ] {
            let e = composer_passage(&modif).unwrap_err();
            assert_eq!(e[0].0, champ, "{e:?}");
        }
    }

    #[test]
    fn cloture_exige_la_personne_pour_le_renvoi() {
        let base = SaisieCloture {
            issue: "retour_classe".into(),
            ..Default::default()
        };
        assert!(composer_cloture(&base)
            .unwrap()
            .recupere_par_personne_id
            .is_none());
        let renvoi = SaisieCloture {
            issue: "renvoi_domicile".into(),
            ..Default::default()
        };
        assert_eq!(composer_cloture(&renvoi).unwrap_err()[0].0, "clo-personne");
        let id = Uuid::from_u128(7);
        let ok = SaisieCloture {
            issue: "renvoi_domicile".into(),
            personne: id.to_string(),
            ..Default::default()
        };
        assert_eq!(
            composer_cloture(&ok).unwrap().recupere_par_personne_id,
            Some(id)
        );
        // Une personne choisie pour une autre issue est ignorée (jamais envoyée).
        let autre = SaisieCloture {
            issue: "retour_classe".into(),
            personne: id.to_string(),
            ..Default::default()
        };
        assert!(composer_cloture(&autre)
            .unwrap()
            .recupere_par_personne_id
            .is_none());
        assert_eq!(
            composer_cloture(&SaisieCloture::default()).unwrap_err()[0].0,
            "clo-issue"
        );
    }

    fn pai() -> SaisiePai {
        SaisiePai {
            motif: "m".into(),
            mesures: "mesures".into(),
            debut: "2026-09-01".into(),
            actif: true,
            ..Default::default()
        }
    }

    #[test]
    fn pai_controles_de_forme() {
        let b = composer_pai(&pai()).unwrap();
        assert!(b.actif && !b.trousse_urgence && b.trousse_lieu.is_empty());
        let e = composer_pai(&SaisiePai {
            motif: "".into(),
            mesures: "".into(),
            debut: "x".into(),
            ..pai()
        })
        .unwrap_err();
        let champs: Vec<_> = e.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(champs, ["pai-motif", "pai-mesures", "pai-debut"]);
        let e = composer_pai(&SaisiePai {
            trousse: true,
            ..pai()
        })
        .unwrap_err();
        assert_eq!(e[0].0, "pai-lieu");
        let ok = composer_pai(&SaisiePai {
            trousse: true,
            trousse_lieu: " infirmerie ".into(),
            ..pai()
        })
        .unwrap();
        assert_eq!(ok.trousse_lieu, "infirmerie");
        // Le lieu d'une trousse décochée n'est pas envoyé.
        let ok = composer_pai(&SaisiePai {
            trousse_lieu: "x".into(),
            ..pai()
        })
        .unwrap();
        assert!(ok.trousse_lieu.is_empty());
        let e = composer_pai(&SaisiePai {
            fin: "2026-08-01".into(),
            revision: "2026-08-02".into(),
            ..pai()
        })
        .unwrap_err();
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn contrainte_limitee_aux_audiences_du_catalogue() {
        let permises = vec!["enseignants".to_string(), "sorties".to_string()];
        let base = SaisieContrainte {
            code: "ne_pas_courir".into(),
            enseignants: true,
            ..Default::default()
        };
        let b = composer_contrainte(&base, &permises).unwrap();
        assert_eq!(b.audiences, ["enseignants"]);
        let e = composer_contrainte(
            &SaisieContrainte {
                restauration: true,
                ..base.clone()
            },
            &permises,
        )
        .unwrap_err();
        assert_eq!(e[0].0, "con-audiences");
        let e = composer_contrainte(
            &SaisieContrainte {
                enseignants: false,
                ..base.clone()
            },
            &permises,
        )
        .unwrap_err();
        assert_eq!(e[0].0, "con-audiences");
        let e = composer_contrainte(
            &SaisieContrainte {
                code: "".into(),
                ..base
            },
            &permises,
        )
        .unwrap_err();
        assert_eq!(e[0].0, "con-code");
    }
}
