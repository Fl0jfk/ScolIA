//! Logique d'écran pure des fiches reliées (« individu », migration 279) : libellés des listes
//! fermées, phrases, tri, contrôle local d'une saisie. Sans Leptos ni réseau, testée en natif.
//!
//! Le serveur décide de tout (droits, anti-énumération, « jamais levé par l'individu concerné ») ;
//! ces fonctions ne font qu'expliquer. Les motifs viennent des constantes d'`ent-types`, celles que
//! la base vérifie : une valeur ajoutée côté serveur sans libellé ici fait échouer un test.

use ent_types::dossier::PersonneTrouveeDto;
use ent_types::individu::{
    LienIndividu, MonIndividu, PropositionIndividu, MOTIFS_ECARTEE, MOTIFS_LEVEE, MOTIFS_LIEN,
};
use uuid::Uuid;

/// Ce qu'un lien change, dit à chaque confirmation (principe 1 de la conception).
pub const EFFET_LIEN: &str = "Les deux fiches seront traitées comme une seule personne : cela ajoute des refus \
     (une personne ne décide pas pour elle-même ni pour son enfant, même depuis un autre compte) et n'ouvre aucun \
     accès. Les comptes concernés sont prévenus. Seule la direction peut lever ce lien, jamais la personne concernée.";

/// Réponse affichée après une déclaration, **identique** que l'adresse soit connue ou non.
pub const MESSAGE_DECLARATION: &str = "Demande enregistrée. Si cette adresse est celle d'un compte de \
     l'établissement, ce compte reçoit une demande à confirmer dans les 24 heures. Connectez-vous avec lui pour \
     répondre.";

/// Motif choisi par défaut : aucun (le choix est obligatoire).
pub const AUCUN: &str = "";

/// Libellé d'une origine de lien.
pub fn libelle_origine(code: &str) -> &'static str {
    match code {
        "declaration_direction" => "Relié par l'établissement",
        "declaration_confirmee" => "Déclaré puis confirmé depuis les deux comptes",
        "proposition_acceptee" => "Ressemblance confirmée par l'établissement",
        _ => "Lien enregistré",
    }
}

/// Libellé d'un motif de lien (`MOTIFS_LIEN`).
pub fn libelle_motif_lien(code: &str) -> &'static str {
    match code {
        "deux_comptes" => "Deux comptes de la même personne",
        "fiche_sans_compte" => "Fiche sans compte de la même personne",
        "doublon_agent" => "Deux fiches de personnel de la même personne",
        "autre" => "Autre motif",
        _ => "Motif non listé",
    }
}

/// Libellé d'un motif de levée (`MOTIFS_LEVEE`).
pub fn libelle_motif_levee(code: &str) -> &'static str {
    match code {
        "erreur" => "Lien posé par erreur",
        "homonyme" => "Deux personnes différentes (homonymes)",
        "autre" => "Autre motif",
        _ => "Motif non listé",
    }
}

/// Libellé d'un motif pour écarter une proposition (`MOTIFS_ECARTEE`).
pub fn libelle_motif_ecartee(code: &str) -> &'static str {
    match code {
        "homonyme" => "Deux personnes différentes",
        "coordonnee_partagee" => "Coordonnée partagée (couple, standard, famille)",
        "autre" => "Autre motif",
        _ => "Motif non listé",
    }
}

fn options(codes: &[&str], libelle: fn(&str) -> &'static str) -> Vec<(String, String)> {
    let mut v = vec![(AUCUN.to_string(), "Choisir un motif".to_string())];
    v.extend(
        codes
            .iter()
            .map(|c| (c.to_string(), libelle(c).to_string())),
    );
    v
}

/// Options de la liste « motif du lien ».
pub fn options_motif_lien() -> Vec<(String, String)> {
    options(&MOTIFS_LIEN, libelle_motif_lien)
}

/// Options de la liste « motif de la levée ».
pub fn options_motif_levee() -> Vec<(String, String)> {
    options(&MOTIFS_LEVEE, libelle_motif_levee)
}

/// Options de la liste « motif pour écarter ».
pub fn options_motif_ecartee() -> Vec<(String, String)> {
    options(&MOTIFS_ECARTEE, libelle_motif_ecartee)
}

/// Un motif choisi dans la liste fermée, sinon le message à afficher.
pub fn motif_valide(code: &str, liste: &[&str]) -> Result<String, &'static str> {
    if liste.contains(&code) {
        Ok(code.to_string())
    } else {
        Err("Choisissez un motif dans la liste.")
    }
}

/// « même adresse e-mail et même téléphone ». La coordonnée elle-même n'est jamais transmise.
pub fn libelle_raisons(raisons: &[String]) -> String {
    let mots: Vec<&str> = raisons
        .iter()
        .map(|r| match r.as_str() {
            "email" => "même adresse e-mail",
            "telephone" => "même téléphone",
            _ => "coordonnée commune",
        })
        .collect();
    match mots.as_slice() {
        [] => "coordonnée commune".to_string(),
        [un] => (*un).to_string(),
        [debut @ .., dernier] => format!("{} et {dernier}", debut.join(", ")),
    }
}

/// Phrase de résumé de « Mes autres comptes ».
pub fn resume_moi(m: &MonIndividu) -> String {
    let autres = match m.autres_comptes {
        n if n <= 0 => {
            return "Aucun autre compte n'est reconnu comme le vôtre dans cet établissement.".into()
        }
        1 => "1 autre compte".to_string(),
        n => format!("{n} autres comptes"),
    };
    format!("{autres} de l'établissement vous appartiennent aussi : l'établissement vous reconnaît comme une seule personne.")
}

/// Phrase sur les fiches (comptes et fiches sans compte confondus), quand il y a plus d'une fiche.
pub fn resume_fiches(m: &MonIndividu) -> Option<String> {
    (m.fiches_reliees > 1).then(|| {
        format!(
            "{} fiches de l'établissement vous désignent (fiches de personnel, de responsable ou d'élève).",
            m.fiches_reliees
        )
    })
}

/// Contrôle local d'une adresse avant l'envoi (le serveur refait le même contrôle).
pub fn adresse_a_envoyer(saisie: &str) -> Result<String, &'static str> {
    let e = saisie.trim().to_lowercase();
    if e.is_empty() {
        return Err("Saisissez l'adresse e-mail de votre autre compte.");
    }
    if e.len() > 254
        || !e.contains('@')
        || e.starts_with('@')
        || e.ends_with('@')
        || e.contains(char::is_whitespace)
    {
        return Err("Cette adresse e-mail n'est pas valide.");
    }
    Ok(e)
}

/// Message annoncé après une réponse à une déclaration.
pub fn message_reponse(confirmer: bool) -> &'static str {
    if confirmer {
        "Confirmé : les deux comptes sont reconnus comme une seule personne."
    } else {
        "Demande refusée : rien n'a été relié."
    }
}

/// Liens les plus récents d'abord (ordre stable).
pub fn trier_liens(mut v: Vec<LienIndividu>) -> Vec<LienIndividu> {
    v.sort_by(|a, b| b.pose_le.cmp(&a.pose_le).then_with(|| a.id.cmp(&b.id)));
    v
}

/// Propositions par nom (déjà rangées par le serveur ; ordre rendu stable ici).
pub fn trier_propositions(mut v: Vec<PropositionIndividu>) -> Vec<PropositionIndividu> {
    v.sort_by(|a, b| {
        (
            a.nom_a.to_lowercase(),
            a.nom_b.to_lowercase(),
            a.personne_a,
            a.personne_b,
        )
            .cmp(&(
                b.nom_a.to_lowercase(),
                b.nom_b.to_lowercase(),
                b.personne_a,
                b.personne_b,
            ))
    });
    v
}

/// Désignation d'une paire, lue par le lecteur d'écran sur chaque bouton.
pub fn designation_paire(nom_a: &str, nom_b: &str) -> String {
    format!("{} et {}", nom_ou_defaut(nom_a), nom_ou_defaut(nom_b))
}

fn nom_ou_defaut(n: &str) -> &str {
    let n = n.trim();
    if n.is_empty() {
        "fiche sans nom"
    } else {
        n
    }
}

/// Une fiche choisie pour être reliée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FicheChoisie {
    pub id: Uuid,
    pub libelle: String,
}

/// Désignation d'une fiche trouvée : nom, puis ce qui aide à distinguer deux homonymes sans montrer
/// de coordonnée (compte, fiche d'élève, nombre d'élèves liés).
pub fn libelle_fiche(t: &PersonneTrouveeDto) -> String {
    let p = &t.personne;
    let mut nom = p.nom.trim().to_string();
    if let Some(pr) = p.prenom.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        nom.push(' ');
        nom.push_str(pr);
    }
    let mut details = Vec::new();
    details.push(
        if p.a_compte {
            "a un compte"
        } else {
            "sans compte"
        }
        .to_string(),
    );
    if p.represente_eleve {
        details.push("fiche d'élève".into());
    }
    match t.nb_liens {
        0 => {}
        1 => details.push("responsable d'1 élève".into()),
        n => details.push(format!("responsable de {n} élèves")),
    }
    format!("{} · {}", nom_ou_defaut(&nom), details.join(", "))
}

/// Fiches trouvées encore choisissables (pas déjà choisie de l'autre côté, pas fusionnée).
pub fn fiches_choisissables(
    trouvees: &[PersonneTrouveeDto],
    autre: Option<Uuid>,
) -> Vec<FicheChoisie> {
    trouvees
        .iter()
        .filter(|t| t.personne.fusionnee_dans.is_none() && Some(t.personne.id) != autre)
        .map(|t| FicheChoisie {
            id: t.personne.id,
            libelle: libelle_fiche(t),
        })
        .collect()
}

/// Recherche à envoyer : rien sous deux caractères (le serveur ne répond rien non plus).
pub fn recherche_a_envoyer(saisie: &str) -> Option<String> {
    let q = saisie.trim();
    (q.chars().count() >= 2).then(|| q.to_string())
}

/// Contrôle local d'une demande de lien manuelle : deux fiches distinctes et un motif listé.
pub fn lien_a_envoyer(
    a: Option<&FicheChoisie>,
    b: Option<&FicheChoisie>,
    motif: &str,
) -> Result<(Uuid, Uuid, String), &'static str> {
    let (Some(a), Some(b)) = (a, b) else {
        return Err("Choisissez les deux fiches à relier.");
    };
    if a.id == b.id {
        return Err("Choisissez deux fiches différentes.");
    }
    let motif = motif_valide(motif, &MOTIFS_LIEN)?;
    Ok((a.id, b.id, motif))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use ent_types::dossier::PersonneDto;

    fn t(s: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(s, 0).unwrap()
    }

    fn moi(autres: i64, fiches: i64) -> MonIndividu {
        MonIndividu {
            autres_comptes: autres,
            fiches_reliees: fiches,
            declarations_recues: vec![],
            liens: vec![],
        }
    }

    fn trouvee(
        id: u128,
        nom: &str,
        prenom: Option<&str>,
        compte: bool,
        liens: i64,
    ) -> PersonneTrouveeDto {
        PersonneTrouveeDto {
            personne: PersonneDto {
                id: Uuid::from_u128(id),
                nom: nom.into(),
                prenom: prenom.map(Into::into),
                civilite: None,
                email: Some("secret@exemple.fr".into()),
                telephone: Some("0600000000".into()),
                telephone_2: None,
                adresse: Some("1 rue secrète".into()),
                a_compte: compte,
                represente_eleve: false,
                fusionnee_dans: None,
            },
            nb_liens: liens,
        }
    }

    #[test]
    fn chaque_motif_du_serveur_a_un_libelle() {
        for c in MOTIFS_LIEN {
            assert_ne!(libelle_motif_lien(c), "Motif non listé", "{c}");
        }
        for c in MOTIFS_LEVEE {
            assert_ne!(libelle_motif_levee(c), "Motif non listé", "{c}");
        }
        for c in MOTIFS_ECARTEE {
            assert_ne!(libelle_motif_ecartee(c), "Motif non listé", "{c}");
        }
        for o in [
            "declaration_direction",
            "declaration_confirmee",
            "proposition_acceptee",
        ] {
            assert_ne!(libelle_origine(o), "Lien enregistré", "{o}");
        }
        // Les options commencent par « Choisir », qui n'est pas un motif valide.
        let o = options_motif_lien();
        assert_eq!(o.len(), MOTIFS_LIEN.len() + 1);
        assert_eq!(o[0].0, AUCUN);
        assert!(motif_valide(AUCUN, &MOTIFS_LIEN).is_err());
        assert!(
            motif_valide("erreur", &MOTIFS_LIEN).is_err(),
            "un motif de levée n'est pas un motif de lien"
        );
        assert_eq!(motif_valide("erreur", &MOTIFS_LEVEE).unwrap(), "erreur");
        assert_eq!(options_motif_levee().len(), 4);
        assert_eq!(options_motif_ecartee().len(), 4);
    }

    #[test]
    fn raisons_lisibles_sans_la_coordonnee() {
        assert_eq!(libelle_raisons(&["email".into()]), "même adresse e-mail");
        assert_eq!(
            libelle_raisons(&["email".into(), "telephone".into()]),
            "même adresse e-mail et même téléphone"
        );
        assert_eq!(libelle_raisons(&[]), "coordonnée commune");
    }

    #[test]
    fn resume_selon_les_nombres() {
        assert!(resume_moi(&moi(0, 1)).starts_with("Aucun autre compte"));
        assert!(resume_moi(&moi(1, 2)).starts_with("1 autre compte "));
        assert!(resume_moi(&moi(3, 4)).starts_with("3 autres comptes"));
        assert_eq!(resume_fiches(&moi(0, 1)), None);
        assert!(resume_fiches(&moi(0, 2)).unwrap().starts_with("2 fiches"));
    }

    #[test]
    fn adresse_controlee_avant_l_envoi() {
        assert_eq!(
            adresse_a_envoyer("  Moi@Exemple.FR ").unwrap(),
            "moi@exemple.fr"
        );
        assert!(adresse_a_envoyer("").is_err());
        assert!(adresse_a_envoyer("sans-arobase").is_err());
        assert!(adresse_a_envoyer("@exemple.fr").is_err());
        assert!(adresse_a_envoyer("a b@exemple.fr").is_err());
        assert!(adresse_a_envoyer(&format!("{}@x.fr", "a".repeat(260))).is_err());
        // La réponse ne dépend pas de l'adresse (anti-énumération) : une seule phrase, qui ne dit pas « trouvé ».
        assert!(!MESSAGE_DECLARATION.to_lowercase().contains("trouv"));
    }

    #[test]
    fn messages_selon_la_reponse() {
        assert_ne!(message_reponse(true), message_reponse(false));
        assert!(message_reponse(false).contains("rien n'a été relié"));
    }

    #[test]
    fn tris_stables() {
        let l = |id: u128, s: i64| LienIndividu {
            id: Uuid::from_u128(id),
            personne_a: Uuid::from_u128(100 + id),
            personne_b: Uuid::from_u128(200 + id),
            nom_a: "A".into(),
            nom_b: "B".into(),
            origine: "declaration_direction".into(),
            motif_code: "autre".into(),
            pose_le: t(s),
        };
        let v = trier_liens(vec![l(1, 10), l(2, 30), l(3, 30)]);
        let ids: Vec<u128> = v.iter().map(|x| x.id.as_u128()).collect();
        assert_eq!(ids, [2, 3, 1], "plus récent d'abord, puis par identifiant");
        let p = |a: &str, b: &str| PropositionIndividu {
            personne_a: Uuid::from_u128(1),
            personne_b: Uuid::from_u128(2),
            nom_a: a.into(),
            nom_b: b.into(),
            raisons: vec!["email".into()],
        };
        let v = trier_propositions(vec![
            p("martin", "Zoé"),
            p("Durand", "Léa"),
            p("Martin", "Adam"),
        ]);
        let n: Vec<&str> = v.iter().map(|x| x.nom_b.as_str()).collect();
        assert_eq!(n, ["Léa", "Adam", "Zoé"]);
        assert_eq!(designation_paire(" ", "Martin"), "fiche sans nom et Martin");
    }

    #[test]
    fn fiche_trouvee_sans_coordonnee() {
        let f = libelle_fiche(&trouvee(1, "Martin", Some("Léa"), true, 2));
        assert_eq!(f, "Martin Léa · a un compte, responsable de 2 élèves");
        for secret in ["secret@", "0600", "rue secrète"] {
            assert!(!f.contains(secret), "{f}");
        }
        assert_eq!(
            libelle_fiche(&trouvee(2, "Durand", None, false, 1)),
            "Durand · sans compte, responsable d'1 élève"
        );
    }

    #[test]
    fn choix_des_deux_fiches() {
        let mut fusionnee = trouvee(3, "Ancienne", None, false, 0);
        fusionnee.personne.fusionnee_dans = Some(Uuid::from_u128(9));
        let v = vec![
            trouvee(1, "Martin", None, true, 0),
            trouvee(2, "Martin", None, false, 1),
            fusionnee,
        ];
        let c = fiches_choisissables(&v, Some(Uuid::from_u128(1)));
        assert_eq!(
            c.len(),
            1,
            "ni la fiche déjà choisie, ni une fiche fusionnée : {c:?}"
        );
        assert_eq!(c[0].id, Uuid::from_u128(2));

        let a = FicheChoisie {
            id: Uuid::from_u128(1),
            libelle: "A".into(),
        };
        let b = FicheChoisie {
            id: Uuid::from_u128(2),
            libelle: "B".into(),
        };
        assert!(lien_a_envoyer(Some(&a), None, "deux_comptes").is_err());
        assert!(lien_a_envoyer(Some(&a), Some(&a), "deux_comptes").is_err());
        assert!(lien_a_envoyer(Some(&a), Some(&b), AUCUN).is_err());
        assert_eq!(
            lien_a_envoyer(Some(&a), Some(&b), "deux_comptes").unwrap(),
            (a.id, b.id, "deux_comptes".to_string())
        );
        assert_eq!(recherche_a_envoyer(" m "), None);
        assert_eq!(recherche_a_envoyer(" Ma ").as_deref(), Some("Ma"));
    }
}
