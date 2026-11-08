//! Logique d'écran pure des inscriptions (S2-3) : libellés, étapes du parcours famille,
//! contrôles de confort du formulaire (le serveur reste l'autorité), lecture du jeton
//! d'invitation dans l'adresse. Aucun appel réseau, aucune dépendance Leptos.

use ent_web_api::types_inscriptions::{
    ChoixEleve, ChoixFoyer, ChoixPersonne, CorrespondancesDto, DossierDto, EleveFormulaireDto,
    NiveauCampagneDto, PersonneCandidateDto, PieceCampagneDto, PieceDossierDto, PreinscriptionBody,
    ProfilDto, ReserveResponsableDto, ResolutionResponsable, ResponsableCorrespondancesDto,
    ResponsableFormulaireDto, ValidationBody,
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
    /// Le déposant demande que son adresse soit protégée (jamais écrite en clair, ouverte à
    /// la direction seule).
    pub resp_adresse_protegee: bool,
    pub second: bool,
    pub second_nom: String,
    pub second_prenom: String,
    pub second_email: String,
    pub second_lien: String,
    pub second_meme_foyer: bool,
    /// Second parent à ne pas solliciter (décision de justice, mesure de protection…) : aucune
    /// invitation ni relance ne lui est adressée ; la direction confirme la mention.
    pub second_ne_pas_solliciter: bool,
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
            reserve: s.resp_adresse_protegee.then(|| ReserveResponsableDto {
                adresse_a_proteger: true,
                ..Default::default()
            }),
            ..Default::default()
        },
        second_parent: s.second.then(|| ResponsableFormulaireDto {
            nom: s.second_nom.trim().into(),
            prenom: opt(&s.second_prenom),
            email: opt(&s.second_email),
            lien: opt(&s.second_lien),
            meme_foyer: s.second_meme_foyer,
            autorite_parentale: true,
            reserve: s.second_ne_pas_solliciter.then(|| ReserveResponsableDto {
                ne_pas_solliciter: true,
                ..Default::default()
            }),
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

// ---------------------------------------------------------------------------
// Validation : rapprochement humain (I0) des responsables et de l'élève
// ---------------------------------------------------------------------------

/// Valeurs des listes de choix du rapprochement.
pub const CHOIX_NOUVEAU: &str = "nouveau";

/// Libellé d'une personne candidate : jamais d'identifiant technique, e-mail déjà masqué par
/// le serveur.
pub fn libelle_candidat(c: &PersonneCandidateDto) -> String {
    let nom = format!("{} {}", c.prenom.clone().unwrap_or_default(), c.nom)
        .trim()
        .to_string();
    let raison = if c.raison == "email" {
        "même adresse e-mail"
    } else {
        "même nom et prénom"
    };
    let mut morceaux = vec![nom];
    if let Some(e) = c.email_masque.as_deref().filter(|e| !e.is_empty()) {
        morceaux.push(e.to_string());
    }
    morceaux.push(raison.to_string());
    if c.a_un_compte {
        morceaux.push("a déjà un compte".to_string());
    }
    morceaux.join(" · ")
}

/// Choix d'une fiche pour un responsable : aucune par défaut (le choix est humain).
pub fn options_personne(r: &ResponsableCorrespondancesDto) -> Vec<(String, String)> {
    std::iter::once((String::new(), "— Choisir —".to_string()))
        .chain(r.candidats.iter().map(|c| {
            (
                c.personne_id.to_string(),
                format!("Rattacher à : {}", libelle_candidat(c)),
            )
        }))
        .chain(std::iter::once((
            CHOIX_NOUVEAU.to_string(),
            "Créer une nouvelle fiche (ce n'est pas la même personne)".to_string(),
        )))
        .collect()
}

/// Foyers proposés pour la personne choisie ; vide si elle n'en a pas (nouveau foyer).
pub fn options_foyer(c: &PersonneCandidateDto) -> Vec<(String, String)> {
    std::iter::once((String::new(), "— Choisir —".to_string()))
        .chain(c.foyers.iter().map(|f| {
            (
                f.foyer_id.to_string(),
                format!("Foyer existant : {}", f.libelle),
            )
        }))
        .chain(std::iter::once((
            CHOIX_NOUVEAU.to_string(),
            "Nouveau foyer".to_string(),
        )))
        .collect()
}

/// Ce que la personne a choisi à l'écran pour un responsable (valeurs des listes ci-dessus).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChoixSaisi {
    pub personne: String,
    pub foyer: String,
}

/// Construit le corps de validation depuis les choix saisis, ou dit ce qui manque (le serveur
/// reste l'autorité : cette fonction évite seulement un aller-retour inutile).
/// `responsables` : un `ChoixSaisi` par entrée de `corr.responsables`, dans le même ordre ;
/// `eleve` : `""` (non choisi), `"nouveau"` ou l'identifiant de la fiche existante.
pub fn construire_validation(
    corr: &CorrespondancesDto,
    responsables: &[ChoixSaisi],
    eleve: &str,
    confirme_responsables_existants: bool,
) -> Result<ValidationBody, String> {
    let mut resolutions = Vec::new();
    for (r, saisi) in corr.responsables.iter().zip(responsables) {
        if r.candidats.is_empty() {
            continue;
        }
        let personne = match saisi.personne.as_str() {
            "" => {
                return Err(format!(
                    "Responsable {} : choisissez la fiche existante ou « nouvelle fiche ».",
                    r.rang
                ))
            }
            CHOIX_NOUVEAU => ChoixPersonne::Nouvelle,
            id => ChoixPersonne::Existante {
                personne_id: id.parse().map_err(|_| "Fiche inconnue.".to_string())?,
            },
        };
        let foyer = match &personne {
            ChoixPersonne::Nouvelle => None,
            ChoixPersonne::Existante { personne_id } => {
                let a_des_foyers = r
                    .candidats
                    .iter()
                    .find(|c| c.personne_id == *personne_id)
                    .is_some_and(|c| !c.foyers.is_empty());
                if !a_des_foyers {
                    Some(ChoixFoyer::Nouveau)
                } else {
                    match saisi.foyer.as_str() {
                        "" => {
                            return Err(format!(
                            "Responsable {} : choisissez un foyer existant ou « nouveau foyer ».",
                            r.rang
                        ))
                        }
                        CHOIX_NOUVEAU => Some(ChoixFoyer::Nouveau),
                        id => Some(ChoixFoyer::Existant {
                            foyer_id: id.parse().map_err(|_| "Foyer inconnu.".to_string())?,
                        }),
                    }
                }
            }
        };
        resolutions.push(ResolutionResponsable {
            responsable_id: r.responsable_id,
            personne,
            foyer,
        });
    }
    let eleve = if corr.eleves.is_empty() {
        None
    } else {
        Some(match eleve {
            "" => return Err("Élève : une fiche porte déjà ce nom ou cet INE, choisissez.".into()),
            CHOIX_NOUVEAU => ChoixEleve::Nouveau,
            id => {
                let eleve_id = id
                    .parse()
                    .map_err(|_| "Fiche élève inconnue.".to_string())?;
                let lies = corr
                    .eleves
                    .iter()
                    .find(|e| e.eleve_id == eleve_id)
                    .map_or(0, |e| e.responsables_lies);
                if lies > 0 && !confirme_responsables_existants {
                    return Err(
                        "Cette fiche a déjà des responsables : confirmez que le dossier peut leur être rattaché."
                            .into(),
                    );
                }
                ChoixEleve::Existant {
                    eleve_id,
                    confirme_responsables_existants,
                }
            }
        })
    };
    Ok(ValidationBody {
        responsables: resolutions,
        eleve,
        ..Default::default()
    })
}

/// Statut d'une campagne (distinct du statut d'un dossier : « brouillon » y est un brouillon).
pub fn libelle_statut_campagne(statut: &str) -> &'static str {
    match statut {
        "brouillon" => "Brouillon",
        autre => libelle_statut(autre),
    }
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
    fn reserves_declarees_a_la_preinscription() {
        let mut s = SaisieFormulaire {
            eleve_nom: "Martin".into(),
            eleve_prenom: "Rose".into(),
            niveau: "4EME".into(),
            resp_nom: "Martin".into(),
            second: true,
            second_nom: "Martin".into(),
            second_prenom: "Marc".into(),
            second_lien: "pere".into(),
            second_meme_foyer: false,
            second_email: "marc@exemple.fr".into(),
            ..Default::default()
        };
        // Sans case cochée : aucune réserve n'est envoyée (le champ reste absent).
        let b = corps_preinscription(Uuid::nil(), &s);
        assert!(b.responsable.reserve.is_none());
        assert!(b.second_parent.as_ref().unwrap().reserve.is_none());
        assert_eq!(
            b.second_parent.as_ref().unwrap().lien.as_deref(),
            Some("pere")
        );
        // Cases cochées : une seule réserve par personne, et seulement celle demandée.
        s.resp_adresse_protegee = true;
        s.second_ne_pas_solliciter = true;
        let b = corps_preinscription(Uuid::nil(), &s);
        let r1 = b.responsable.reserve.expect("réserve du déposant");
        assert!(r1.adresse_a_proteger && !r1.ne_pas_solliciter);
        let r2 = b
            .second_parent
            .unwrap()
            .reserve
            .expect("réserve du second parent");
        assert!(r2.ne_pas_solliciter && !r2.adresse_a_proteger);
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

    fn candidat(nom: &str, foyers: usize) -> PersonneCandidateDto {
        use ent_web_api::types_inscriptions::FoyerCandidatDto;
        PersonneCandidateDto {
            personne_id: Uuid::from_u128(7),
            nom: nom.into(),
            prenom: Some("Ines".into()),
            email_masque: Some("i***@exemple.lab".into()),
            raison: "email".into(),
            a_un_compte: true,
            foyers: (0..foyers)
                .map(|i| FoyerCandidatDto {
                    foyer_id: Uuid::from_u128(100 + i as u128),
                    libelle: "Foyer Martin".into(),
                })
                .collect(),
        }
    }

    #[test]
    fn validation_exige_un_choix_humain_par_responsable() {
        let corr = CorrespondancesDto {
            eleves: vec![],
            responsables: vec![ResponsableCorrespondancesDto {
                responsable_id: Uuid::from_u128(1),
                rang: 1,
                candidats: vec![candidat("Martin", 1)],
            }],
        };
        // Rien choisi : refus local, jamais de rapprochement implicite.
        let vide = [ChoixSaisi::default()];
        assert!(construire_validation(&corr, &vide, "", false).is_err());
        // Fiche existante avec foyer : le foyer doit aussi être choisi.
        let sans_foyer = [ChoixSaisi {
            personne: Uuid::from_u128(7).to_string(),
            foyer: String::new(),
        }];
        assert!(construire_validation(&corr, &sans_foyer, "", false).is_err());
        let complet = [ChoixSaisi {
            personne: Uuid::from_u128(7).to_string(),
            foyer: Uuid::from_u128(100).to_string(),
        }];
        let b = construire_validation(&corr, &complet, "", false).unwrap();
        assert!(matches!(
            b.responsables[0].personne,
            ChoixPersonne::Existante { .. }
        ));
        assert!(matches!(
            b.responsables[0].foyer,
            Some(ChoixFoyer::Existant { .. })
        ));
        // « Nouvelle fiche » : pas de foyer à choisir.
        let nouvelle = [ChoixSaisi {
            personne: CHOIX_NOUVEAU.into(),
            foyer: String::new(),
        }];
        let b = construire_validation(&corr, &nouvelle, "", false).unwrap();
        assert!(matches!(
            b.responsables[0].personne,
            ChoixPersonne::Nouvelle
        ));
        assert!(b.responsables[0].foyer.is_none());
        // Personne sans foyer : nouveau foyer d'office.
        let corr0 = CorrespondancesDto {
            eleves: vec![],
            responsables: vec![ResponsableCorrespondancesDto {
                responsable_id: Uuid::from_u128(1),
                rang: 1,
                candidats: vec![candidat("Martin", 0)],
            }],
        };
        let choix = [ChoixSaisi {
            personne: Uuid::from_u128(7).to_string(),
            foyer: String::new(),
        }];
        let b = construire_validation(&corr0, &choix, "", false).unwrap();
        assert!(matches!(b.responsables[0].foyer, Some(ChoixFoyer::Nouveau)));
    }

    #[test]
    fn validation_ancien_eleve_demande_la_confirmation_des_responsables() {
        use ent_web_api::types_inscriptions::EleveCandidatDto;
        let eleve_id = Uuid::from_u128(9);
        let corr = CorrespondancesDto {
            eleves: vec![EleveCandidatDto {
                eleve_id,
                nom: "Martin".into(),
                prenom: "Rose".into(),
                date_naissance: None,
                statut: "sorti".into(),
                ine_identique: false,
                responsables_lies: 2,
            }],
            responsables: vec![],
        };
        assert!(construire_validation(&corr, &[], "", false).is_err());
        let id = eleve_id.to_string();
        assert!(construire_validation(&corr, &[], &id, false).is_err());
        let b = construire_validation(&corr, &[], &id, true).unwrap();
        assert!(matches!(
            b.eleve,
            Some(ChoixEleve::Existant {
                confirme_responsables_existants: true,
                ..
            })
        ));
        let b = construire_validation(&corr, &[], CHOIX_NOUVEAU, false).unwrap();
        assert!(matches!(b.eleve, Some(ChoixEleve::Nouveau)));
        // Aucune correspondance : corps vide, rien à choisir.
        let rien = CorrespondancesDto {
            eleves: vec![],
            responsables: vec![],
        };
        let b = construire_validation(&rien, &[], "", false).unwrap();
        assert!(b.responsables.is_empty() && b.eleve.is_none());
    }

    #[test]
    fn libelles_de_rapprochement_sans_identifiant_technique() {
        let l = libelle_candidat(&candidat("Martin", 1));
        assert!(
            l.contains("Ines Martin")
                && l.contains("même adresse e-mail")
                && l.contains("a déjà un compte")
        );
        assert!(!l.contains("0000"));
        assert_eq!(libelle_statut_campagne("brouillon"), "Brouillon");
        assert_eq!(libelle_statut_campagne("ouverte"), "Ouverte");
    }
}
