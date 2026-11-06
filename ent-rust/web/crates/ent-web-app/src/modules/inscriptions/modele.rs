//! Logique d'écran pure des inscriptions (S2-3) : libellés, étapes du parcours famille,
//! contrôles de confort du formulaire (le serveur reste l'autorité), lecture du jeton
//! d'invitation dans l'adresse. Aucun appel réseau, aucune dépendance Leptos.

use ent_web_api::types_inscriptions::{
    DossierDto, EleveFormulaireDto, NiveauCampagneDto, PieceCampagneDto, PieceDossierDto,
    PreinscriptionBody, ProfilDto, ResponsableFormulaireDto,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

/// Étapes du parcours famille. Rien n'est affiché avant `Espace` (e-mail vérifié).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Etape {
    Email,
    Code { demande_id: Uuid },
    Espace,
    Formulaire { campagne_id: Uuid },
    Dossier { id: Uuid },
}

pub fn libelle_statut(statut: &str) -> &'static str {
    match statut {
        "brouillon" => "À confirmer",
        "deposee" => "Déposé",
        "en_instruction" => "En cours d'instruction",
        "incomplet" => "Incomplet",
        "liste_attente" => "Liste d'attente",
        "admis" => "Admis",
        "refuse" => "Refusé",
        "validee" => "Inscription validée",
        "desistement" => "Départ annoncé",
        "attendue" => "Attendue",
        "acceptee" => "Acceptée",
        "refusee" => "Refusée",
        "ouverte" => "Ouverte",
        "fermee" => "Fermée",
        _ => "Autre",
    }
}

pub fn ton_statut(statut: &str) -> TonEtat {
    match statut {
        "validee" | "admis" | "acceptee" | "ouverte" => TonEtat::Ok,
        "refuse" | "refusee" | "desistement" => TonEtat::Mal,
        "incomplet" | "liste_attente" | "attendue" => TonEtat::Avertissement,
        "deposee" | "en_instruction" => TonEtat::Info,
        _ => TonEtat::Neutre,
    }
}

pub fn libelle_nature(nature: &str) -> &'static str {
    if nature == "reinscription" {
        "Réinscription"
    } else {
        "Admission"
    }
}

pub fn libelle_regime(r: &str) -> &'static str {
    match r {
        "externe" => "Externe",
        "demi_pension" => "Demi-pensionnaire",
        "interne" => "Interne",
        _ => "Non précisé",
    }
}

pub const REGIMES: [&str; 3] = ["externe", "demi_pension", "interne"];

/// Pièces obligatoires qui attendent encore une action de la famille.
pub fn pieces_a_fournir(pieces: &[PieceDossierDto]) -> Vec<&PieceDossierDto> {
    pieces
        .iter()
        .filter(|p| p.obligatoire && matches!(p.statut.as_str(), "attendue" | "refusee"))
        .collect()
}

pub fn peut(d: &DossierDto, action: &str) -> bool {
    d.actions.iter().any(|a| a == action)
}

/// Code à 6 chiffres saisi (espaces tolérés) ; `None` si le format est faux.
pub fn code_saisi(s: &str) -> Option<String> {
    let c: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (c.len() == 6 && c.chars().all(|c| c.is_ascii_digit())).then_some(c)
}

/// Contrôle de confort d'une adresse (le serveur valide).
pub fn email_plausible(e: &str) -> bool {
    let e = e.trim();
    e.len() <= 254
        && e.split_once('@')
            .is_some_and(|(a, d)| !a.is_empty() && d.contains('.') && !d.ends_with('.'))
}

/// Champs saisis du formulaire de préinscription (valeurs brutes des champs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieFormulaire {
    pub eleve_nom: String,
    pub eleve_prenom: String,
    pub eleve_naissance: String,
    pub niveau: String,
    pub regime: String,
    pub resp_nom: String,
    pub resp_prenom: String,
    pub resp_telephone: String,
    pub resp_adresse: String,
    pub resp_lien: String,
    pub second: bool,
    pub second_nom: String,
    pub second_prenom: String,
    pub second_email: String,
    pub second_meme_foyer: bool,
    pub utiliser_mes_infos: bool,
    pub enregistrer_profil: bool,
}

fn opt(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// « Utiliser mes informations ? » oui : pré-remplissage modifiable avec le profil.
pub fn pre_remplir(s: &mut SaisieFormulaire, p: &ProfilDto) {
    if let Some(n) = &p.nom {
        s.resp_nom = n.clone();
    }
    if let Some(n) = &p.prenom {
        s.resp_prenom = n.clone();
    }
    if let Some(t) = &p.telephone {
        s.resp_telephone = t.clone();
    }
    if let Some(a) = &p.adresse {
        s.resp_adresse = a.clone();
    }
    s.utiliser_mes_infos = true;
}

/// Erreurs de confort (champ, message) ; vide si l'envoi peut être tenté.
pub fn erreurs_formulaire(s: &SaisieFormulaire, niveaux: &[String]) -> Vec<(String, String)> {
    let mut e = Vec::new();
    if s.eleve_nom.trim().is_empty() {
        e.push(("pi-eleve-nom".into(), "Nom de l'élève obligatoire".into()));
    }
    if s.eleve_prenom.trim().is_empty() {
        e.push((
            "pi-eleve-prenom".into(),
            "Prénom de l'élève obligatoire".into(),
        ));
    }
    if !s.eleve_naissance.trim().is_empty()
        && chrono::NaiveDate::parse_from_str(s.eleve_naissance.trim(), "%Y-%m-%d").is_err()
    {
        e.push((
            "pi-eleve-naissance".into(),
            "Date de naissance invalide".into(),
        ));
    }
    if !niveaux.is_empty() && !niveaux.iter().any(|n| n == &s.niveau) {
        e.push(("pi-niveau".into(), "Choisissez un niveau".into()));
    }
    if s.resp_nom.trim().is_empty() {
        e.push(("pi-resp-nom".into(), "Votre nom est obligatoire".into()));
    }
    if s.second {
        if s.second_nom.trim().is_empty() {
            e.push((
                "pi-second-nom".into(),
                "Nom du second parent obligatoire".into(),
            ));
        }
        let email = s.second_email.trim();
        if !s.second_meme_foyer && email.is_empty() {
            e.push((
                "pi-second-email".into(),
                "Parents séparés : l'e-mail du second parent est obligatoire".into(),
            ));
        } else if !email.is_empty() && !email_plausible(email) {
            e.push(("pi-second-email".into(), "Adresse e-mail invalide".into()));
        }
    }
    e
}

/// Corps envoyé au serveur (le responsable 1 reçoit l'adresse vérifiée côté serveur).
pub fn corps_preinscription(campagne_id: Uuid, s: &SaisieFormulaire) -> PreinscriptionBody {
    PreinscriptionBody {
        campagne_id,
        utiliser_mes_infos: s.utiliser_mes_infos,
        enregistrer_profil: s.enregistrer_profil,
        eleve: EleveFormulaireDto {
            nom: s.eleve_nom.trim().into(),
            prenom: s.eleve_prenom.trim().into(),
            date_naissance: chrono::NaiveDate::parse_from_str(s.eleve_naissance.trim(), "%Y-%m-%d")
                .ok(),
            niveau_demande: opt(&s.niveau),
            regime_demande: opt(&s.regime),
            ..Default::default()
        },
        responsable: ResponsableFormulaireDto {
            nom: s.resp_nom.trim().into(),
            prenom: opt(&s.resp_prenom),
            telephone: opt(&s.resp_telephone),
            adresse: opt(&s.resp_adresse),
            lien: opt(&s.resp_lien),
            meme_foyer: true,
            autorite_parentale: true,
            ..Default::default()
        },
        second_parent: s.second.then(|| ResponsableFormulaireDto {
            nom: s.second_nom.trim().into(),
            prenom: opt(&s.second_prenom),
            email: opt(&s.second_email),
            meme_foyer: s.second_meme_foyer,
            autorite_parentale: true,
            ..Default::default()
        }),
    }
}

/// Niveaux saisis « 6EME:120, 5EME » (capacité facultative après « : »).
pub fn niveaux_depuis_texte(t: &str) -> Result<Vec<NiveauCampagneDto>, String> {
    let mut v = Vec::new();
    for morceau in t
        .split([',', '\n'])
        .map(str::trim)
        .filter(|m| !m.is_empty())
    {
        let (code, cap) = match morceau.split_once(':') {
            Some((c, n)) => {
                let n = n
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| format!("capacité illisible : « {morceau} »"))?;
                (c.trim(), Some(n))
            }
            None => (morceau, None),
        };
        v.push(NiveauCampagneDto {
            code: code.to_uppercase(),
            capacite: cap,
        });
    }
    Ok(v)
}

/// Pièces saisies une par ligne : « code ; libellé ; facultative » (obligatoire par défaut).
pub fn pieces_depuis_texte(t: &str) -> Result<Vec<PieceCampagneDto>, String> {
    let mut v = Vec::new();
    for ligne in t.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let mut champs = ligne.split(';').map(str::trim);
        let code = champs.next().unwrap_or_default().to_lowercase();
        let libelle = champs.next().unwrap_or_default().to_string();
        if code.is_empty() || libelle.is_empty() {
            return Err(format!("pièce « {ligne} » : code et libellé attendus"));
        }
        let facultative = champs
            .next()
            .is_some_and(|f| f.eq_ignore_ascii_case("facultative"));
        v.push(PieceCampagneDto {
            code,
            libelle,
            obligatoire: !facultative,
        });
    }
    Ok(v)
}

/// Jeton d'invitation lu dans la chaîne de requête (`?jeton=…`), borné.
pub fn jeton_de_requete(requete: &str) -> Option<String> {
    requete
        .trim_start_matches('?')
        .split('&')
        .find_map(|kv| kv.strip_prefix("jeton="))
        .map(str::to_string)
        .filter(|j| !j.is_empty() && j.len() <= 256 && j.chars().all(|c| c.is_ascii_alphanumeric()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_et_email() {
        assert_eq!(code_saisi(" 123 456 "), Some("123456".into()));
        assert_eq!(code_saisi("12345"), None);
        assert_eq!(code_saisi("12345a"), None);
        assert!(email_plausible("a@b.fr"));
        assert!(!email_plausible("a@b"));
        assert!(!email_plausible("@b.fr"));
    }

    #[test]
    fn formulaire_parents_separes() {
        let mut s = SaisieFormulaire {
            eleve_nom: "Lovegood".into(),
            eleve_prenom: "Luna".into(),
            niveau: "6EME".into(),
            resp_nom: "Lovegood".into(),
            second: true,
            second_nom: "Lovegood".into(),
            second_meme_foyer: false,
            ..Default::default()
        };
        let e = erreurs_formulaire(&s, &["6EME".into()]);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].0, "pi-second-email");
        s.second_email = "mere@exemple.fr".into();
        assert!(erreurs_formulaire(&s, &["6EME".into()]).is_empty());
        assert!(!erreurs_formulaire(&s, &["5EME".into()]).is_empty());
        let b = corps_preinscription(Uuid::nil(), &s);
        assert_eq!(
            b.second_parent.as_ref().unwrap().email.as_deref(),
            Some("mere@exemple.fr")
        );
        assert!(!b.second_parent.unwrap().meme_foyer);
        assert_eq!(
            b.responsable.email, None,
            "l'adresse vérifiée est posée par le serveur"
        );
    }

    #[test]
    fn pre_remplissage_modifiable() {
        let mut s = SaisieFormulaire::default();
        pre_remplir(
            &mut s,
            &ProfilDto {
                nom: Some("Potter".into()),
                prenom: Some("Lily".into()),
                ..Default::default()
            },
        );
        assert_eq!(
            (s.resp_nom.as_str(), s.resp_prenom.as_str()),
            ("Potter", "Lily")
        );
        assert!(s.utiliser_mes_infos);
        s.resp_prenom = "James".into();
        assert_eq!(
            corps_preinscription(Uuid::nil(), &s)
                .responsable
                .prenom
                .as_deref(),
            Some("James")
        );
    }

    #[test]
    fn saisie_de_campagne() {
        let n = niveaux_depuis_texte("6eme:120, 5EME").unwrap();
        assert_eq!(n.len(), 2);
        assert_eq!((n[0].code.as_str(), n[0].capacite), ("6EME", Some(120)));
        assert_eq!(n[1].capacite, None);
        assert!(niveaux_depuis_texte("6EME:beaucoup").is_err());
        let p = pieces_depuis_texte("livret ; Livret scolaire\nphoto;Photo;facultative\n").unwrap();
        assert_eq!(p.len(), 2);
        assert!(p[0].obligatoire && !p[1].obligatoire);
        assert!(pieces_depuis_texte("seulcode").is_err());
    }

    #[test]
    fn jeton_invitation() {
        assert_eq!(jeton_de_requete("?jeton=abc123"), Some("abc123".into()));
        assert_eq!(jeton_de_requete("?x=1&jeton=abc"), Some("abc".into()));
        assert_eq!(jeton_de_requete("?jeton=<script>"), None);
        assert_eq!(jeton_de_requete(""), None);
    }

    #[test]
    fn libelles_et_tons() {
        assert_eq!(libelle_statut("validee"), "Inscription validée");
        assert_eq!(libelle_statut("inconnu"), "Autre");
        assert!(matches!(ton_statut("refuse"), TonEtat::Mal));
        let pieces = vec![
            PieceDossierDto {
                code: "a".into(),
                libelle: "A".into(),
                obligatoire: true,
                statut: "refusee".into(),
                nom_fichier: None,
                motif_refus: Some("x".into()),
            },
            PieceDossierDto {
                code: "b".into(),
                libelle: "B".into(),
                obligatoire: false,
                statut: "attendue".into(),
                nom_fichier: None,
                motif_refus: None,
            },
        ];
        assert_eq!(pieces_a_fournir(&pieces).len(), 1);
    }
}
