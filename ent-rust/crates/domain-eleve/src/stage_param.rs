//! Stages, lot ST1 (a) : règles pures des paramètres, des organismes (état, sites, contacts) et
//! des périodes (`docs/socles/39_STAGES.md` §9, §10, §19.1 à §19.3). Aucune base ni réseau :
//! `ent-db` applique ces règles dans la transaction, `ent-api` ne fait que décoder.

use crate::stage::StageErreur;
use chrono::NaiveDate;

pub const TYPES_ORGANISME: [&str; 8] = [
    "entreprise",
    "association",
    "administration",
    "collectivite",
    "etablissement_scolaire",
    "sante",
    "etranger",
    "autre",
];
pub const ETATS_ORGANISME: [&str; 5] = ["actif", "a_verifier", "ferme", "deconseille", "interdit"];
/// Motifs codés d'un organisme déconseillé : jamais une personne nommée (§10.1).
pub const MOTIFS_ETAT_ORGANISME: [&str; 5] = [
    "securite",
    "encadrement_insuffisant",
    "refus_discriminatoire_signale",
    "non_respect_horaires",
    "autre",
];

fn borne(s: &str, max: usize) -> Option<String> {
    let t = s.trim();
    (!t.is_empty() && t.chars().count() <= max).then(|| t.to_string())
}

fn borne_opt(s: Option<&str>, max: usize) -> Result<Option<String>, ()> {
    match s.map(str::trim).filter(|t| !t.is_empty()) {
        None => Ok(None),
        Some(t) if t.chars().count() <= max => Ok(Some(t.to_string())),
        Some(_) => Err(()),
    }
}

// ───────────── Paramètres de l'établissement (STG-100 à STG-109) ─────────────

/// Modification des paramètres : chaque champ `Some` remplace la valeur (`None` = inchangé).
/// Les champs à valeur facultative (`assurance`, quotas…) se vident avec `Some(None)`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModifierParametres {
    /// `(assureur, n° de contrat)` : les deux ou aucun (`Some(None)` efface).
    pub assurance: Option<Option<(String, String)>>,
    pub deliberation: Option<Option<(String, NaiveDate)>>,
    pub max_stagiaires_referent: Option<Option<i16>>,
    pub max_stagiaires_organisme: Option<Option<i16>>,
    pub relance_ouverture_j: Option<i16>,
    pub relance_sans_lieu_j: Option<i16>,
    pub conservation_contacts_annees: Option<i16>,
    pub etablissement_signe_en_dernier: Option<bool>,
    pub accord_titulaires: Option<String>,
    pub stage_famille: Option<String>,
}

/// Contrôle les bornes des réglages (S-3901 à S-3914) ; renvoie la commande normalisée.
pub fn valider_parametres(mut p: ModifierParametres) -> Result<ModifierParametres, StageErreur> {
    if let Some(Some((assureur, contrat))) = p.assurance.as_mut() {
        let (a, c) = (borne(assureur, 200), borne(contrat, 100));
        match (a, c) {
            (Some(a), Some(c)) => {
                *assureur = a;
                *contrat = c;
            }
            _ => return Err(StageErreur::AssuranceIncomplete),
        }
    }
    if let Some(Some((reference, _))) = p.deliberation.as_mut() {
        *reference = borne(reference, 200).ok_or(StageErreur::ParametreInvalide)?;
    }
    for q in [p.max_stagiaires_referent, p.max_stagiaires_organisme].into_iter().flatten().flatten() {
        if !(1..=200).contains(&q) {
            return Err(StageErreur::ParametreInvalide);
        }
    }
    let dans = |v: Option<i16>, min: i16, max: i16| v.is_none_or(|v| (min..=max).contains(&v));
    if !dans(p.relance_ouverture_j, 15, 90)
        || !dans(p.relance_sans_lieu_j, 7, 90)
        || !dans(p.conservation_contacts_annees, 1, 5)
    {
        return Err(StageErreur::ParametreInvalide);
    }
    if p.accord_titulaires.as_deref().is_some_and(|v| !matches!(v, "un" | "tous"))
        || p.stage_famille.as_deref().is_some_and(|v| !matches!(v, "autorise_avec_alerte" | "interdit"))
    {
        return Err(StageErreur::ParametreInvalide);
    }
    Ok(p)
}

// ───────────── État d'un organisme (STG-117) ─────────────

/// Valide un changement d'état. « Interdit » est une décision de la direction ; « déconseillé »
/// exige un motif codé ; le texte libre est court et ne vise jamais une personne nommément
/// (contrôle humain : le champ est réservé au DDFPT et à la direction).
pub fn valider_etat_organisme(
    etat: &str,
    motif_code: Option<&str>,
    motif_texte: Option<&str>,
    direction: bool,
) -> Result<(String, Option<String>, Option<String>), StageErreur> {
    if !ETATS_ORGANISME.contains(&etat) {
        return Err(StageErreur::EtatOrganismeInvalide);
    }
    if etat == "interdit" && !direction {
        return Err(StageErreur::InterditReserveDirection);
    }
    let motif_code = motif_code.map(str::trim).filter(|m| !m.is_empty());
    if let Some(m) = motif_code {
        if !MOTIFS_ETAT_ORGANISME.contains(&m) {
            return Err(StageErreur::MotifEtatRequis);
        }
    }
    if etat == "deconseille" && motif_code.is_none() {
        return Err(StageErreur::MotifEtatRequis);
    }
    let texte = borne_opt(motif_texte, 300).map_err(|_| StageErreur::MotifEtatRequis)?;
    // Un état « actif » efface le motif : on ne garde pas un motif sans état qui le justifie.
    if etat == "actif" {
        return Ok((etat.to_string(), None, None));
    }
    Ok((etat.to_string(), motif_code.map(str::to_string), texte))
}

// ───────────── Adresse d'un organisme (STG-110) ─────────────

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModifierOrganisme {
    pub type_organisme: Option<String>,
    pub enseigne: Option<Option<String>>,
    pub adresse: Option<String>,
    pub code_postal: Option<Option<String>>,
    pub commune: Option<Option<String>>,
    pub pays: Option<String>,
}

pub fn valider_organisme(mut m: ModifierOrganisme) -> Result<ModifierOrganisme, StageErreur> {
    if m.type_organisme.as_deref().is_some_and(|t| !TYPES_ORGANISME.contains(&t)) {
        return Err(StageErreur::ParametreInvalide);
    }
    let texte = |v: &mut Option<Option<String>>, max| -> Result<(), StageErreur> {
        if let Some(inner) = v {
            *inner = borne_opt(inner.as_deref(), max).map_err(|_| StageErreur::AdresseTropLongue)?;
        }
        Ok(())
    };
    texte(&mut m.enseigne, 200)?;
    texte(&mut m.code_postal, 10)?;
    texte(&mut m.commune, 120)?;
    if let Some(a) = m.adresse.as_mut() {
        *a = a.trim().to_string();
        if a.chars().count() > 400 {
            return Err(StageErreur::AdresseTropLongue);
        }
    }
    if let Some(p) = m.pays.as_mut() {
        *p = p.trim().to_ascii_uppercase();
        if p.len() != 2 || !p.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(StageErreur::ParametreInvalide);
        }
    }
    Ok(m)
}

// ───────────── Sites (STG-114) ─────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteSaisie {
    pub libelle: String,
    pub adresse_ligne1: String,
    pub adresse_ligne2: Option<String>,
    pub code_postal: String,
    pub commune: String,
    pub pays: String,
}

pub fn valider_site(s: SiteSaisie) -> Result<SiteSaisie, StageErreur> {
    let pays = s.pays.trim().to_ascii_uppercase();
    let pays = if pays.is_empty() { "FR".to_string() } else { pays };
    if pays.len() != 2 || !pays.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(StageErreur::SiteInvalide);
    }
    Ok(SiteSaisie {
        libelle: borne(&s.libelle, 120).ok_or(StageErreur::SiteInvalide)?,
        adresse_ligne1: borne(&s.adresse_ligne1, 200).ok_or(StageErreur::SiteInvalide)?,
        adresse_ligne2: borne_opt(s.adresse_ligne2.as_deref(), 200).map_err(|_| StageErreur::SiteInvalide)?,
        code_postal: borne(&s.code_postal, 10).ok_or(StageErreur::SiteInvalide)?,
        commune: borne(&s.commune, 120).ok_or(StageErreur::SiteInvalide)?,
        pays,
    })
}

// ───────────── Contacts (STG-115) ─────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactSaisie {
    pub nom: String,
    pub prenom: String,
    pub fonction: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub est_tuteur: bool,
    pub est_signataire: bool,
    pub est_rh: bool,
}

fn courriel_plausible(e: &str) -> bool {
    let mut p = e.split('@');
    match (p.next(), p.next(), p.next()) {
        (Some(l), Some(d), None) => {
            !l.is_empty() && d.contains('.') && !e.chars().any(char::is_whitespace) && e.len() <= 254
        }
        _ => false,
    }
}

/// Un contact n'a jamais de compte ; pour signer, il lui faut un canal (adresse électronique).
pub fn valider_contact(c: ContactSaisie) -> Result<ContactSaisie, StageErreur> {
    let email = borne_opt(c.email.as_deref(), 254).map_err(|_| StageErreur::ContactInvalide)?;
    if email.as_deref().is_some_and(|e| !courriel_plausible(e)) {
        return Err(StageErreur::ContactInvalide);
    }
    let telephone = borne_opt(c.telephone.as_deref(), 30).map_err(|_| StageErreur::ContactInvalide)?;
    if telephone.as_deref().is_some_and(|t| t.chars().count() < 3) {
        return Err(StageErreur::ContactInvalide);
    }
    if c.est_signataire && email.is_none() {
        return Err(StageErreur::SignataireSansCourriel);
    }
    Ok(ContactSaisie {
        nom: borne(&c.nom, 80).ok_or(StageErreur::ContactInvalide)?,
        prenom: borne(&c.prenom, 80).ok_or(StageErreur::ContactInvalide)?,
        fonction: borne_opt(c.fonction.as_deref(), 120).map_err(|_| StageErreur::ContactInvalide)?,
        email,
        telephone,
        ..c
    })
}

// ───────────── Périodes (STG-120 à STG-124) ─────────────

pub mod periode {
    /// États d'une période : `prevue` → `publiee` → `close` (jamais de retour).
    pub const PREVUE: &str = "prevue";
    pub const PUBLIEE: &str = "publiee";
    pub const CLOSE: &str = "close";

    pub fn transition_permise(de: &str, vers: &str) -> bool {
        matches!((de, vers), (PREVUE, PUBLIEE) | (PREVUE, CLOSE) | (PUBLIEE, CLOSE))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodeSaisie {
    pub libelle: String,
    pub du: NaiveDate,
    pub au: NaiveDate,
    pub individuelle: bool,
    pub motif: Option<String>,
}

/// Valide une période : libellé, ordre des dates, motif d'une période individuelle (STG-123).
/// L'appartenance à l'année scolaire et l'activation du dispositif sont contrôlées avec la base.
pub fn valider_periode(p: PeriodeSaisie) -> Result<PeriodeSaisie, StageErreur> {
    let libelle = borne(&p.libelle, 120).ok_or(StageErreur::PeriodeInvalide)?;
    if p.au < p.du {
        return Err(StageErreur::PeriodeInvalide);
    }
    let motif = borne_opt(p.motif.as_deref(), 300).map_err(|_| StageErreur::PeriodeMotifRequis)?;
    if p.individuelle && motif.is_none() {
        return Err(StageErreur::PeriodeMotifRequis);
    }
    Ok(PeriodeSaisie { libelle, motif, ..p })
}

/// Cible d'une période : exactement une classe, un groupe ou un élève.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicPeriode {
    Classe(uuid::Uuid),
    Groupe(uuid::Uuid),
    Eleve(uuid::Uuid),
}

/// Une période individuelle ne vise que des élèves ; les publics sont dédoublonnés.
pub fn valider_publics(
    individuelle: bool,
    publics: &[PublicPeriode],
) -> Result<Vec<PublicPeriode>, StageErreur> {
    let mut vus = Vec::with_capacity(publics.len());
    for p in publics {
        let id = match p {
            PublicPeriode::Classe(i) | PublicPeriode::Groupe(i) | PublicPeriode::Eleve(i) => i,
        };
        if id.is_nil() || (individuelle && !matches!(p, PublicPeriode::Eleve(_))) {
            return Err(StageErreur::PeriodePublicInvalide);
        }
        if !vus.contains(p) {
            vus.push(*p);
        }
    }
    Ok(vus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn d(m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, j).unwrap()
    }

    #[test]
    fn parametres_bornes() {
        let ok = ModifierParametres {
            relance_ouverture_j: Some(60),
            relance_sans_lieu_j: Some(30),
            conservation_contacts_annees: Some(3),
            max_stagiaires_referent: Some(Some(6)),
            accord_titulaires: Some("tous".into()),
            ..Default::default()
        };
        assert!(valider_parametres(ok).is_ok());
        for mauvais in [
            ModifierParametres { relance_ouverture_j: Some(10), ..Default::default() },
            ModifierParametres { relance_ouverture_j: Some(91), ..Default::default() },
            ModifierParametres { conservation_contacts_annees: Some(6), ..Default::default() },
            ModifierParametres { max_stagiaires_organisme: Some(Some(0)), ..Default::default() },
            ModifierParametres { accord_titulaires: Some("aucun".into()), ..Default::default() },
            ModifierParametres { stage_famille: Some("oui".into()), ..Default::default() },
        ] {
            assert_eq!(valider_parametres(mauvais), Err(StageErreur::ParametreInvalide));
        }
    }

    /// DN-18 : l'assurance n'est jamais codée en dur ; assureur et contrat vont ensemble.
    #[test]
    fn assurance_complete_ou_absente() {
        let p = |a: &str, c: &str| ModifierParametres {
            assurance: Some(Some((a.into(), c.into()))),
            ..Default::default()
        };
        assert!(valider_parametres(p("MAIF", "123-45")).is_ok());
        assert_eq!(valider_parametres(p("MAIF", " ")), Err(StageErreur::AssuranceIncomplete));
        assert_eq!(valider_parametres(p("", "123")), Err(StageErreur::AssuranceIncomplete));
        assert!(valider_parametres(ModifierParametres { assurance: Some(None), ..Default::default() }).is_ok());
    }

    #[test]
    fn etat_organisme_regles() {
        assert_eq!(
            valider_etat_organisme("deconseille", None, None, false),
            Err(StageErreur::MotifEtatRequis)
        );
        assert_eq!(
            valider_etat_organisme("deconseille", Some("pas_un_motif"), None, false),
            Err(StageErreur::MotifEtatRequis)
        );
        assert!(valider_etat_organisme("deconseille", Some("securite"), Some(" court "), false).is_ok());
        assert_eq!(
            valider_etat_organisme("interdit", Some("securite"), None, false),
            Err(StageErreur::InterditReserveDirection)
        );
        assert!(valider_etat_organisme("interdit", Some("securite"), None, true).is_ok());
        assert_eq!(valider_etat_organisme("inconnu", None, None, true), Err(StageErreur::EtatOrganismeInvalide));
        // Texte libre trop long refusé ; un organisme réactivé perd son motif.
        let long = "x".repeat(301);
        assert!(valider_etat_organisme("ferme", None, Some(&long), true).is_err());
        assert_eq!(
            valider_etat_organisme("actif", Some("securite"), Some("ancien"), false).unwrap(),
            ("actif".to_string(), None, None)
        );
    }

    #[test]
    fn organisme_adresse_et_type() {
        let m = valider_organisme(ModifierOrganisme {
            type_organisme: Some("association".into()),
            pays: Some(" fr ".into()),
            commune: Some(Some("  Poudlard ".into())),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(m.pays.as_deref(), Some("FR"));
        assert_eq!(m.commune, Some(Some("Poudlard".into())));
        assert!(valider_organisme(ModifierOrganisme { type_organisme: Some("x".into()), ..Default::default() }).is_err());
        assert!(valider_organisme(ModifierOrganisme { pays: Some("FRA".into()), ..Default::default() }).is_err());
    }

    #[test]
    fn site_complet() {
        let s = SiteSaisie {
            libelle: " Atelier ".into(),
            adresse_ligne1: "1 rue des Potions".into(),
            adresse_ligne2: None,
            code_postal: "75005".into(),
            commune: "Paris".into(),
            pays: "".into(),
        };
        let v = valider_site(s.clone()).unwrap();
        assert_eq!((v.libelle.as_str(), v.pays.as_str()), ("Atelier", "FR"));
        assert_eq!(valider_site(SiteSaisie { commune: " ".into(), ..s }), Err(StageErreur::SiteInvalide));
    }

    #[test]
    fn contact_canal_pour_signer() {
        let c = ContactSaisie {
            nom: "Weasley".into(),
            prenom: "Fred".into(),
            fonction: None,
            email: None,
            telephone: Some("0102030405".into()),
            est_tuteur: true,
            est_signataire: true,
            est_rh: false,
        };
        assert_eq!(valider_contact(c.clone()), Err(StageErreur::SignataireSansCourriel));
        let ok = ContactSaisie { email: Some("fred@wwws.example".into()), ..c.clone() };
        assert!(valider_contact(ok).is_ok());
        for mauvais in ["fred", "fred@", "@x.fr", "a b@x.fr", "a@b"] {
            let r = valider_contact(ContactSaisie { email: Some(mauvais.into()), est_signataire: false, ..c.clone() });
            assert_eq!(r, Err(StageErreur::ContactInvalide), "{mauvais}");
        }
        assert_eq!(valider_contact(ContactSaisie { nom: " ".into(), est_signataire: false, ..c }), Err(StageErreur::ContactInvalide));
    }

    #[test]
    fn periodes_regles() {
        let base = PeriodeSaisie { libelle: "Stage 3e".into(), du: d(12, 7), au: d(12, 11), individuelle: false, motif: None };
        assert!(valider_periode(base.clone()).is_ok());
        assert_eq!(valider_periode(PeriodeSaisie { au: d(12, 1), ..base.clone() }), Err(StageErreur::PeriodeInvalide));
        assert_eq!(valider_periode(PeriodeSaisie { libelle: " ".into(), ..base.clone() }), Err(StageErreur::PeriodeInvalide));
        // STG-123 : période individuelle = motif obligatoire.
        assert_eq!(valider_periode(PeriodeSaisie { individuelle: true, ..base.clone() }), Err(StageErreur::PeriodeMotifRequis));
        assert!(valider_periode(PeriodeSaisie { individuelle: true, motif: Some("maladie".into()), ..base }).is_ok());
        assert!(periode::transition_permise("prevue", "publiee"));
        assert!(periode::transition_permise("publiee", "close"));
        assert!(!periode::transition_permise("close", "publiee"));
        assert!(!periode::transition_permise("publiee", "prevue"));
    }

    #[test]
    fn publics_de_periode() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let v = valider_publics(false, &[PublicPeriode::Classe(a), PublicPeriode::Classe(a), PublicPeriode::Eleve(b)]).unwrap();
        assert_eq!(v.len(), 2);
        assert!(valider_publics(true, &[PublicPeriode::Classe(a)]).is_err());
        assert!(valider_publics(true, &[PublicPeriode::Eleve(b)]).is_ok());
        assert!(valider_publics(false, &[PublicPeriode::Eleve(Uuid::nil())]).is_err());
    }
}
