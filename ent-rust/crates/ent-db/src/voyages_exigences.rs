//! Socle 42, lot V1 : moteur d'exigences par participant — **règles pures** (`docs/socles/42_SORTIES_VOYAGES.md`
//! §9.3, §11.2, §19 ; R-SV6, R-SV9, R-SV40 ; CV-04 à CV-09, CV-25, CV-26). Aucune base ici : les commandes et
//! lectures sont dans `voyages_exigences_repo.rs`.
//!
//! * une sortie **obligatoire** est gratuite et ne demande **aucune** autorisation parentale (§0, point 4a) ;
//!   une sortie **facultative** demande l'autorisation d'**un** titulaire de l'autorité parentale (l'accord de
//!   l'autre est présumé sauf désaccord connu) ; l'élève **majeur** signe lui-même ;
//! * **un seul refus suffit** à bloquer (règle de prudence du socle 2) ; un refus de titulaire n'est **jamais**
//!   dérogeable ; un désaccord déclaré bloque jusqu'à la décision motivée de la direction (CV-04) ;
//! * une signature dont le signataire a **perdu l'autorité** est « à revoir » : l'autre titulaire est sollicité
//!   (CV-06) ;
//! * l'attestation d'assurance **n'est jamais exigée** (guide 2025, §5.7) ; le consentement à l'image et
//!   l'autorisation de soins sont **distincts**, non bloquants et révocables ;
//! * seules des exigences non légales se dérogent, et la dérogation ne vaut **jamais** consentement.

use chrono::NaiveDate;
use uuid::Uuid;

pub const AUTORISATION: &str = "autorisation_participation";
pub const DESACCORD: &str = "desaccord_titulaire";
pub const SOINS: &str = "autorisation_soins";
pub const IMAGE: &str = "droit_image";

pub const CODES: [&str; 4] = [AUTORISATION, DESACCORD, SOINS, IMAGE];

/// Codes qu'un titulaire (ou l'élève majeur) signe : tout sauf le désaccord, qui est un fait de la direction.
pub fn est_signable(code: &str) -> bool {
    matches!(code, AUTORISATION | SOINS | IMAGE)
}

/// Exigences que la direction peut lever : **non légales** seulement. La participation d'une sortie facultative
/// et le désaccord ne se dérogent jamais ; un refus non plus (vérifié sur l'état).
pub fn est_derogeable(code: &str) -> bool {
    matches!(code, SOINS | IMAGE)
}

/// Exigences d'une sortie : `(code, bloquante)`. L'obligatoire n'a pas d'autorisation de participation.
pub fn requises(nature: &str, avec_image: bool, avec_soins: bool) -> Vec<(&'static str, bool)> {
    let mut v = Vec::new();
    if nature == "facultative" {
        v.push((AUTORISATION, true));
    }
    if avec_soins {
        v.push((SOINS, false));
    }
    if avec_image {
        v.push((IMAGE, false));
    }
    v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Accepte,
    Refuse,
    Retire,
}

impl Decision {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "accepte" => Some(Self::Accepte),
            "refuse" => Some(Self::Refuse),
            "retire" => Some(Self::Retire),
            _ => None,
        }
    }
}

/// Dernière décision d'un signataire, avec sa qualité **au moment de la lecture** (autorité encore valable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionSignataire {
    pub signataire: Uuid,
    pub qualifie: bool,
    pub decision: Decision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatSignature {
    Attendue,
    Remplie,
    Refusee,
    /// Une acceptation existe mais son signataire n'est plus qualifié et aucun autre n'a accepté.
    ARevoir,
}

impl EtatSignature {
    pub fn code(self) -> &'static str {
        match self {
            Self::Attendue => "attendue",
            Self::Remplie => "remplie",
            Self::Refusee => "refusee",
            Self::ARevoir => "a_revoir",
        }
    }
}

/// État d'une signature à partir de la **dernière** décision de chaque signataire.
/// `titulaires_qualifies` : nombre de signataires possibles aujourd'hui (titulaires avec autorité, ou 1 pour
/// l'élève majeur). `regle_tous` : tous les titulaires doivent accepter (étranger, §9.3).
pub fn etat_signature(
    dernieres: &[DecisionSignataire],
    titulaires_qualifies: usize,
    regle_tous: bool,
) -> EtatSignature {
    let qualifies: Vec<&DecisionSignataire> = dernieres.iter().filter(|d| d.qualifie).collect();
    if qualifies.iter().any(|d| matches!(d.decision, Decision::Refuse | Decision::Retire)) {
        return EtatSignature::Refusee;
    }
    let acceptations = qualifies.iter().filter(|d| d.decision == Decision::Accepte).count();
    let suffisant = if regle_tous { titulaires_qualifies > 0 && acceptations >= titulaires_qualifies } else { acceptations >= 1 };
    if suffisant {
        EtatSignature::Remplie
    } else if dernieres.iter().any(|d| !d.qualifie && d.decision == Decision::Accepte) && acceptations == 0 {
        EtatSignature::ARevoir
    } else {
        EtatSignature::Attendue
    }
}

/// État final d'une exigence signable après dérogation éventuelle : jamais sur un refus ni un code légal.
pub fn avec_derogation(etat: EtatSignature, code: &str, derogation: bool) -> &'static str {
    if derogation && est_derogeable(code) && etat != EtatSignature::Refusee {
        "derogee"
    } else {
        etat.code()
    }
}

/// Une exigence suffit-elle à départ ? (`remplie`, `derogee`, `levee`).
pub fn est_satisfaite(etat: &str) -> bool {
    matches!(etat, "remplie" | "derogee" | "levee")
}

/// Cause de blocage d'un participant : un refus, un désaccord non levé ou une signature à revoir.
pub fn est_cause_de_blocage(code: &str, etat: &str, bloquante: bool) -> bool {
    bloquante && (matches!(etat, "refusee" | "a_revoir") || (code == DESACCORD && etat == "attendue"))
}

/// Nouvel état d'un participant d'après ses exigences `(code, bloquante, état)` ; `None` si son état actuel
/// n'est pas géré par le moteur (parti, désisté, retiré…). `pret` : tout ce qui est bloquant est satisfait ;
/// `bloque` : une cause de blocage ; sinon `inscrit` (en attente d'une réponse, ce n'est pas un blocage).
pub fn etat_participant(actuel: &str, exigences: &[(&str, bool, &str)]) -> Option<&'static str> {
    if !matches!(actuel, "inscrit" | "pret" | "bloque") {
        return None;
    }
    let bloquantes: Vec<&(&str, bool, &str)> = exigences.iter().filter(|e| e.1).collect();
    if bloquantes.iter().any(|(c, b, e)| est_cause_de_blocage(c, e, *b)) {
        Some("bloque")
    } else if bloquantes.iter().all(|(_, _, e)| est_satisfaite(e)) {
        Some("pret")
    } else {
        Some("inscrit")
    }
}

/// Seuils de relance (jours avant le départ).
pub const SEUILS: [(&str, i64); 3] = [("J-21", 21), ("J-10", 10), ("J-5", 5)];

/// Relances dues un jour donné : `(seuil, omise)`. Un seuil franchi non encore traité est dû ; si plusieurs le
/// sont (reprise après une panne), **seul le plus proche envoie**, les autres sont enregistrés comme omis (aucune
/// rafale). Rien après le départ.
pub fn seuils_dus(jours_avant: i64, deja: &[&str]) -> Vec<(&'static str, bool)> {
    if jours_avant < 0 {
        return Vec::new();
    }
    let dus: Vec<&'static str> = SEUILS
        .iter()
        .filter(|(nom, j)| jours_avant <= *j && !deja.contains(nom))
        .map(|(nom, _)| *nom)
        .collect();
    let n = dus.len();
    dus.into_iter().enumerate().map(|(i, s)| (s, i + 1 < n)).collect()
}

/// Pays de l'Union européenne et de l'Espace économique européen (noms usuels, minuscules sans accents).
const PAYS_UE_EEE: &[&str] = &[
    "allemagne", "autriche", "belgique", "bulgarie", "chypre", "croatie", "danemark", "espagne", "estonie",
    "finlande", "france", "grece", "hongrie", "irlande", "italie", "lettonie", "lituanie", "luxembourg", "malte",
    "pays-bas", "pologne", "portugal", "roumanie", "slovaquie", "slovenie", "suede", "tchequie", "republique tcheque",
    "islande", "liechtenstein", "norvege", "pays bas", "rep. tcheque",
];

fn sans_accents(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' | 'ä' => 'a',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect::<String>()
        .to_lowercase()
}

/// Vrai si le pays du destinataire est **hors** Union européenne / EEE (transfert à signaler, VOY-289).
/// Un pays inconnu est traité comme hors UE : en cas de doute, on signale.
pub fn hors_ue(pays: &str) -> bool {
    let p = sans_accents(pays.trim());
    !PAYS_UE_EEE.contains(&p.as_str())
}

/// Champs qu'une liste nominative peut porter : liste FERMÉE, jamais de contact, d'INE ni de santé.
pub const CHAMPS_LISTE: [&str; 5] = ["nom", "prenom", "classe", "date_naissance", "sexe"];

pub fn champ_liste_valide(c: &str) -> bool {
    CHAMPS_LISTE.contains(&c)
}

/// Jour de signature acceptable pour un coupon papier : ni dans le futur, ni plus ancien qu'un an.
pub fn date_signature_plausible(signee: NaiveDate, aujourdhui: NaiveDate) -> bool {
    signee <= aujourdhui && (aujourdhui - signee).num_days() <= 366
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: u128, qualifie: bool, decision: Decision) -> DecisionSignataire {
        DecisionSignataire { signataire: Uuid::from_u128(s), qualifie, decision }
    }

    #[test]
    fn obligatoire_sans_autorisation_facultative_avec() {
        assert!(requises("obligatoire", false, false).is_empty(), "gratuite et sans autorisation parentale");
        assert_eq!(requises("facultative", false, false), vec![(AUTORISATION, true)]);
        // Image et soins : consentements distincts, jamais bloquants ; l'assurance n'est jamais exigée.
        let v = requises("facultative", true, true);
        assert_eq!(v, vec![(AUTORISATION, true), (SOINS, false), (IMAGE, false)]);
        assert!(v.iter().all(|(c, _)| *c != "assurance"));
        assert_eq!(requises("obligatoire", true, false), vec![(IMAGE, false)]);
    }

    #[test]
    fn un_titulaire_suffit_un_refus_bloque_et_le_majeur_seul_compte() {
        assert_eq!(etat_signature(&[], 2, false), EtatSignature::Attendue);
        // CV-04 / règle de prudence : un accord, un refus de l'autre : refusée.
        let v = [d(1, true, Decision::Accepte), d(2, true, Decision::Refuse)];
        assert_eq!(etat_signature(&v, 2, false), EtatSignature::Refusee);
        assert_eq!(etat_signature(&[d(1, true, Decision::Accepte)], 2, false), EtatSignature::Remplie);
        // Règle « tous » (étranger) : les deux titulaires doivent accepter.
        assert_eq!(etat_signature(&[d(1, true, Decision::Accepte)], 2, true), EtatSignature::Attendue);
        assert_eq!(etat_signature(&[d(1, true, Decision::Accepte), d(2, true, Decision::Accepte)], 2, true), EtatSignature::Remplie);
        // Retrait = refus.
        assert_eq!(etat_signature(&[d(1, true, Decision::Retire)], 2, false), EtatSignature::Refusee);
    }

    #[test]
    fn signataire_sans_autorite_signature_a_revoir_cv_06() {
        // La seule acceptation vient d'un titulaire qui a perdu l'autorité : à revoir, jamais « remplie ».
        assert_eq!(etat_signature(&[d(1, false, Decision::Accepte)], 1, false), EtatSignature::ARevoir);
        // Un autre titulaire a accepté : remplie malgré tout.
        assert_eq!(etat_signature(&[d(1, false, Decision::Accepte), d(2, true, Decision::Accepte)], 1, false), EtatSignature::Remplie);
        // Un refus d'un ancien titulaire n'est plus pris en compte (il n'est plus qualifié).
        assert_eq!(etat_signature(&[d(1, false, Decision::Refuse)], 1, false), EtatSignature::Attendue);
    }

    #[test]
    fn la_derogation_ne_leve_jamais_un_refus_ni_une_exigence_legale() {
        assert_eq!(avec_derogation(EtatSignature::Attendue, IMAGE, true), "derogee");
        assert_eq!(avec_derogation(EtatSignature::Attendue, SOINS, true), "derogee");
        assert_eq!(avec_derogation(EtatSignature::Attendue, AUTORISATION, true), "attendue", "légale : jamais");
        assert_eq!(avec_derogation(EtatSignature::Refusee, IMAGE, true), "refusee", "un refus ne se déroge pas");
        assert_eq!(avec_derogation(EtatSignature::ARevoir, DESACCORD, true), "a_revoir");
        assert!(est_derogeable(SOINS) && est_derogeable(IMAGE));
        assert!(!est_derogeable(AUTORISATION) && !est_derogeable(DESACCORD));
    }

    #[test]
    fn etat_du_participant_pret_bloque_ou_en_attente() {
        let ok = [(AUTORISATION, true, "remplie"), (IMAGE, false, "attendue")];
        assert_eq!(etat_participant("inscrit", &ok), Some("pret"), "le droit à l'image ne bloque pas");
        assert_eq!(etat_participant("inscrit", &[(AUTORISATION, true, "attendue")]), Some("inscrit"), "en attente n'est pas bloqué");
        assert_eq!(etat_participant("inscrit", &[(AUTORISATION, true, "refusee")]), Some("bloque"));
        assert_eq!(etat_participant("pret", &[(AUTORISATION, true, "a_revoir")]), Some("bloque"));
        assert_eq!(etat_participant("pret", &[(AUTORISATION, true, "remplie"), (DESACCORD, true, "attendue")]), Some("bloque"), "CV-04");
        assert_eq!(etat_participant("bloque", &[(AUTORISATION, true, "remplie"), (DESACCORD, true, "levee")]), Some("pret"));
        // Sans exigence bloquante (sortie obligatoire) : prêt.
        assert_eq!(etat_participant("inscrit", &[]), Some("pret"));
        // Les états hors du moteur ne sont jamais touchés.
        for e in ["parti", "desiste", "retire", "rentre", "liste_attente", "propose", "absent_depart"] {
            assert_eq!(etat_participant(e, &ok), None, "{e}");
        }
    }

    #[test]
    fn relances_j21_j10_j5_rattrapees_sans_rafale() {
        assert!(seuils_dus(30, &[]).is_empty());
        assert_eq!(seuils_dus(21, &[]), vec![("J-21", false)]);
        assert_eq!(seuils_dus(15, &["J-21"]), vec![]);
        assert_eq!(seuils_dus(10, &["J-21"]), vec![("J-10", false)]);
        // Panne : on se réveille à J-4, rien n'est parti : seule J-5 envoie, J-21 et J-10 sont omises.
        assert_eq!(seuils_dus(4, &[]), vec![("J-21", true), ("J-10", true), ("J-5", false)]);
        assert_eq!(seuils_dus(4, &["J-21", "J-10", "J-5"]), vec![]);
        assert!(seuils_dus(-1, &[]).is_empty(), "rien après le départ");
        assert_eq!(seuils_dus(0, &["J-21", "J-10"]), vec![("J-5", false)]);
    }

    #[test]
    fn transfert_hors_ue_signale_et_pays_inconnu_traite_comme_hors_ue() {
        for ue in ["France", "Italie", "Allemagne", "Grèce", "Pays-Bas", "Norvège", "ISLANDE", " Espagne "] {
            assert!(!hors_ue(ue), "{ue}");
        }
        for hors in ["Royaume-Uni", "Suisse", "Maroc", "États-Unis", "Narnia", "", "Turquie"] {
            assert!(hors_ue(hors), "{hors}");
        }
    }

    #[test]
    fn champs_de_liste_fermes_sans_contact_ni_ine() {
        for ok in ["nom", "prenom", "classe", "date_naissance", "sexe"] {
            assert!(champ_liste_valide(ok));
        }
        for ko in ["ine", "email", "telephone", "telephone_parent", "email_parent", "adresse", "sante", "pai", "regime", "photo", ""] {
            assert!(!champ_liste_valide(ko), "{ko}");
        }
    }

    #[test]
    fn date_de_signature_papier_plausible() {
        let j = NaiveDate::from_ymd_opt(2027, 3, 10).unwrap();
        assert!(date_signature_plausible(j, j));
        assert!(date_signature_plausible(NaiveDate::from_ymd_opt(2027, 1, 5).unwrap(), j));
        assert!(!date_signature_plausible(NaiveDate::from_ymd_opt(2027, 3, 11).unwrap(), j), "dans le futur");
        assert!(!date_signature_plausible(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), j), "trop ancienne");
    }
}
