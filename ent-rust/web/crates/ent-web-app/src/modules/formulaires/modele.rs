//! Logique d'écran des formulaires, autorisations et consentements (pure, testée en natif).
//!
//! Le serveur reste l'autorité : qui peut signer (responsable avec autorité, élève majeur),
//! décision effective, versions. L'écran ne fait que présenter l'état et proposer les gestes
//! autorisés par l'état courant.

use ent_types::communication::{
    ConsentementEleveDto, FormulaireDto, FormulaireFamilleDto, NouveauFormulaireBody,
    ReponseFormulaireBody,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

/// Objet du consentement qui masque la photo dans le trombinoscope.
pub const OBJET_DROIT_IMAGE: &str = "droit_image";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaisieFormulaire {
    /// `consentement` | `autorisation` | `sondage`.
    pub type_formulaire: String,
    pub objet: String,
    pub titre: String,
    pub texte: String,
    /// `etablissement` ou `classe`.
    pub audience: String,
    pub classe_id: String,
}

pub fn objet_valide(o: &str) -> bool {
    (2..=40).contains(&o.len())
        && o.starts_with(|c: char| c.is_ascii_lowercase())
        && o.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn composer(s: &SaisieFormulaire) -> Result<NouveauFormulaireBody, String> {
    let titre = s.titre.trim();
    let texte = s.texte.trim();
    if titre.is_empty() || titre.chars().count() > 200 {
        return Err("Le titre est obligatoire (200 caractères au plus).".into());
    }
    if texte.is_empty() || texte.chars().count() > 10_000 {
        return Err("Le texte est obligatoire (10 000 caractères au plus).".into());
    }
    let objet = match s.type_formulaire.as_str() {
        "sondage" => None,
        "consentement" | "autorisation" => {
            let o = s.objet.trim();
            if !objet_valide(o) {
                return Err("Objet invalide : minuscules, chiffres et _ (ex. droit_image).".into());
            }
            Some(o.to_string())
        }
        _ => return Err("Type de formulaire inconnu.".into()),
    };
    let cible_ids = match s.audience.as_str() {
        "etablissement" => Vec::new(),
        "classe" => vec![Uuid::parse_str(s.classe_id.trim())
            .map_err(|_| "Choisissez une classe.".to_string())?],
        _ => return Err("Audience inconnue.".into()),
    };
    Ok(NouveauFormulaireBody {
        type_formulaire: s.type_formulaire.clone(),
        objet,
        titre: titre.to_string(),
        texte: texte.to_string(),
        audience: s.audience.clone(),
        cible_ids,
    })
}

pub fn libelle_type(t: &str) -> &'static str {
    match t {
        "consentement" => "Consentement",
        "autorisation" => "Autorisation",
        "sondage" => "Sondage",
        _ => "Formulaire",
    }
}

pub fn libelle_statut(s: &str) -> &'static str {
    match s {
        "brouillon" => "Brouillon",
        "publie" => "Publié",
        "archive" => "Archivé",
        _ => "Inconnu",
    }
}

pub fn ton_statut(s: &str) -> TonEtat {
    match s {
        "publie" => TonEtat::Ok,
        "brouillon" => TonEtat::Info,
        _ => TonEtat::Neutre,
    }
}

/// « Accepté », « Refusé », « Sans réponse » (décision effective ou personnelle).
pub fn libelle_decision(d: &str) -> &'static str {
    match d {
        "accepte" => "Accepté",
        "refuse" => "Refusé",
        "retire" => "Retiré",
        _ => "Sans réponse",
    }
}

pub fn ton_decision(d: &str) -> TonEtat {
    match d {
        "accepte" => TonEtat::Ok,
        "refuse" | "retire" => TonEtat::Mal,
        _ => TonEtat::Neutre,
    }
}

/// Gestes proposés pour un formulaire selon ma dernière décision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gestes {
    pub accepter: bool,
    pub refuser: bool,
    pub retirer: bool,
}

pub fn gestes(f: &FormulaireFamilleDto) -> Gestes {
    let derniere = f.ma_decision.as_deref();
    let consentement = f.type_formulaire != "sondage";
    Gestes {
        accepter: derniere != Some("accepte"),
        refuser: derniere != Some("refuse"),
        // Un consentement donné se retire ; un sondage se modifie en changeant de réponse.
        retirer: consentement && derniere == Some("accepte"),
    }
}

pub fn reponse(eleve_id: Uuid, decision: &str) -> ReponseFormulaireBody {
    ReponseFormulaireBody {
        eleve_id,
        decision: decision.into(),
        commentaire: None,
    }
}

/// Information affichée sous un formulaire de famille.
pub fn rappel_effet(f: &FormulaireFamilleDto) -> Option<&'static str> {
    (f.objet.as_deref() == Some(OBJET_DROIT_IMAGE)).then_some(
        "En cas de refus, la photo de votre enfant n'est plus affichée dans le trombinoscope.",
    )
}

pub fn a_traiter(l: &[FormulaireFamilleDto]) -> usize {
    l.iter().filter(|f| f.a_repondre).count()
}

pub fn nb_refus(l: &[ConsentementEleveDto]) -> usize {
    l.iter().filter(|c| c.decision == "refuse").count()
}

pub fn publiable(f: &FormulaireDto) -> bool {
    f.statut == "brouillon"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saisie() -> SaisieFormulaire {
        SaisieFormulaire {
            type_formulaire: "consentement".into(),
            objet: "droit_image".into(),
            titre: "Droit à l'image".into(),
            texte: "Autorisez-vous… ?".into(),
            audience: "etablissement".into(),
            classe_id: String::new(),
        }
    }

    fn fam(type_formulaire: &str, ma: Option<&str>) -> FormulaireFamilleDto {
        FormulaireFamilleDto {
            formulaire_id: Uuid::nil(),
            type_formulaire: type_formulaire.into(),
            objet: Some("droit_image".into()),
            version: 1,
            titre: "t".into(),
            texte: "x".into(),
            eleve_id: Uuid::nil(),
            eleve_prenom: "Harry".into(),
            ma_decision: ma.map(String::from),
            etat: None,
            a_repondre: ma.is_none(),
        }
    }

    #[test]
    fn composition() {
        assert!(composer(&saisie()).unwrap().objet.as_deref() == Some("droit_image"));
        let mut s = saisie();
        s.objet = "Droit Image".into();
        assert!(composer(&s).is_err());
        s.type_formulaire = "sondage".into();
        assert_eq!(
            composer(&s).unwrap().objet,
            None,
            "un sondage n'a pas d'objet"
        );
        let mut s = saisie();
        s.audience = "classe".into();
        assert!(composer(&s).is_err());
        s.classe_id = Uuid::from_u128(5).to_string();
        assert_eq!(composer(&s).unwrap().cible_ids.len(), 1);
        s.titre.clear();
        assert!(composer(&s).is_err());
    }

    #[test]
    fn gestes_selon_la_derniere_decision() {
        let g = gestes(&fam("consentement", None));
        assert!(g.accepter && g.refuser && !g.retirer);
        let g = gestes(&fam("consentement", Some("accepte")));
        assert!(!g.accepter && g.refuser && g.retirer);
        let g = gestes(&fam("consentement", Some("refuse")));
        assert!(g.accepter && !g.refuser && !g.retirer);
        let g = gestes(&fam("sondage", Some("accepte")));
        assert!(!g.retirer, "un sondage ne se retire pas");
    }

    #[test]
    fn effet_du_droit_a_l_image_annonce() {
        assert!(rappel_effet(&fam("consentement", None))
            .unwrap()
            .contains("trombinoscope"));
        let mut f = fam("autorisation", None);
        f.objet = Some("sortie".into());
        assert!(rappel_effet(&f).is_none());
    }

    #[test]
    fn compteurs_et_libelles() {
        assert_eq!(
            a_traiter(&[fam("sondage", None), fam("sondage", Some("accepte"))]),
            1
        );
        assert_eq!(libelle_decision("aucune"), "Sans réponse");
        assert_eq!(ton_decision("retire"), TonEtat::Mal);
        assert_eq!(libelle_type("consentement"), "Consentement");
        let l = [
            ConsentementEleveDto {
                eleve_id: Uuid::nil(),
                nom: "A".into(),
                prenom: "B".into(),
                decision: "refuse".into(),
            },
            ConsentementEleveDto {
                eleve_id: Uuid::nil(),
                nom: "C".into(),
                prenom: "D".into(),
                decision: "aucune".into(),
            },
        ];
        assert_eq!(nb_refus(&l), 1);
    }
}
