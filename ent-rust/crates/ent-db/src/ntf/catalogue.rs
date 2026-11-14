//! Socle 8, lot N1 — catalogue des natures d'avis déclarées et de leurs émissions par événement.
//!
//! En N1 : les trois situations de la vie scolaire que l'ancien chemin (`reactions.rs`) servait déjà, déclarées
//! avec leur gabarit français. Elles sont **inactives par défaut** : tant que `ent_ntf_nature_etab.via_brique`
//! est faux pour un établissement, l'ancien chemin prévient seul ; quand il devient vrai, la brique prévient et
//! l'ancien chemin se tait (bascule progressive, jamais les deux). Les autres briques ajoutent leurs natures
//! ici au fil des lots N11 (bascule des émetteurs).

use super::contrat::*;
use ent_events::types;

const PRENOM: Variable = Variable { nom: "prenom", sensibilite: Sensibilite::C1 };
const JOUR: Variable = Variable { nom: "jour", sensibilite: Sensibilite::C0 };

/// L'élève est noté absent à l'appel (CN-01 : 20 minutes pour corriger avant de prévenir).
pub const ABSENCE_CONSTATEE: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.absence_constatee",
    version: 1,
    brique: "vie_scolaire",
    categorie: Categorie::VieScolaire,
    sujet: TypeSujet::Eleve,
    eleve_destinataire: EleveDest::SiMajeur,
    droit_requis: DroitRequis::VieScolaire,
    urgence: Urgence::U2,
    sensibilite: Sensibilite::C1,
    plancher: Plancher::UnExterne,
    canaux_defaut: &[Canal::InApp, Canal::Push],
    escalade: Some(Escalade { si_non_lu_apres_min: 30, vers: Canal::Sms, sms_consenti: true }),
    delai_confirmation_min: Some(20),
    regroupement: Regroupement::FoyerDemiJournee,
    gabarit: Gabarit {
        id: "vie_scolaire.absence_constatee",
        titre: "Absence de {prenom}",
        corps: "{prenom} a été noté(e) absent(e) à l'appel le {jour}. \
                Vous pouvez justifier cette absence depuis le portail famille.",
        sms: "Vie scolaire : une information concernant {prenom} vous attend sur ScolIA.",
        variables: &[PRENOM, JOUR],
    },
    // Un retard enregistré dans le délai de confirmation remplace l'absence (CN-01).
    annulee_par: &[types::RETARD_DECLARE],
    correction_si_remis: true,
    conservation_preuve: true,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: false,
};

/// L'élève est noté en retard : information à lire, regroupable au résumé.
pub const RETARD_CONSTATE: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.retard_constate",
    version: 1,
    brique: "vie_scolaire",
    categorie: Categorie::VieScolaire,
    sujet: TypeSujet::Eleve,
    eleve_destinataire: EleveDest::SiMajeur,
    droit_requis: DroitRequis::VieScolaire,
    urgence: Urgence::U1,
    sensibilite: Sensibilite::C1,
    plancher: Plancher::InApp,
    canaux_defaut: &[Canal::InApp, Canal::Push, Canal::Email],
    escalade: None,
    delai_confirmation_min: None,
    regroupement: Regroupement::Resume,
    gabarit: Gabarit {
        id: "vie_scolaire.retard_constate",
        titre: "Retard de {prenom}",
        corps: "{prenom} a été noté(e) en retard le {jour}.",
        sms: "Vie scolaire : une information concernant {prenom} vous attend sur ScolIA.",
        variables: &[PRENOM, JOUR],
    },
    annulee_par: &[],
    correction_si_remis: false,
    conservation_preuve: true,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: false,
};

/// Passage à l'infirmerie : la famille doit savoir, sans détail médical (SAN-03). Le droit exigé est le seul
/// lien actif (socle 8 N0b, E6).
pub const PASSAGE_INFIRMERIE: NatureAvisDecl = NatureAvisDecl {
    code: "sante.passage_infirmerie",
    version: 1,
    brique: "sante",
    categorie: Categorie::Sante,
    sujet: TypeSujet::Eleve,
    eleve_destinataire: EleveDest::SiMajeur,
    droit_requis: DroitRequis::Lien,
    urgence: Urgence::U1,
    sensibilite: Sensibilite::C1,
    plancher: Plancher::UnExterne,
    canaux_defaut: &[Canal::InApp, Canal::Push, Canal::Email],
    escalade: None,
    delai_confirmation_min: None,
    regroupement: Regroupement::Aucun,
    gabarit: Gabarit {
        id: "sante.passage_infirmerie",
        titre: "Passage de {prenom} à l'infirmerie",
        corps: "{prenom} s'est présenté(e) à l'infirmerie le {jour}. \
                Pour en savoir plus, contactez l'infirmerie ou la vie scolaire.",
        sms: "ScolIA : une information concernant {prenom} vous attend.",
        variables: &[PRENOM, JOUR],
    },
    annulee_par: &[],
    correction_si_remis: false,
    conservation_preuve: false,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: true,
};

/// Toutes les natures déclarées (l'assemblage les vérifie).
pub const NATURES: &[&NatureAvisDecl] = &[&ABSENCE_CONSTATEE, &RETARD_CONSTATE, &PASSAGE_INFIRMERIE];

fn jour_de(evt: &EvenementVu<'_>, cle: &str) -> Vec<Param> {
    match evt.payload_jour(cle) {
        Some(j) => vec![("jour", ValeurParam::Jour(j))],
        None => Vec::new(),
    }
}

/// Émissions par événement. L'ordre est sans importance ; un événement peut avoir plusieurs natures.
pub const AVIS: &[AvisSurEvenement] = &[
    AvisSurEvenement {
        evenement: types::ELEVE_ABSENCE_DECLARED,
        nature: &ABSENCE_CONSTATEE,
        // Seule l'absence **constatée** à l'appel et pas déjà annoncée prévient la famille.
        filtre: |e| e.payload_str("source") == Some("appel") && e.payload_bool("prevenue") != Some(true),
        cle: |e| cle_fait(e, "absence"),
        parametres: |e| jour_de(e, "debut"),
    },
    AvisSurEvenement {
        evenement: types::RETARD_DECLARE,
        nature: &RETARD_CONSTATE,
        filtre: |_| true,
        cle: |e| cle_fait(e, "retard"),
        parametres: |e| jour_de(e, "jour"),
    },
    AvisSurEvenement {
        evenement: "infirmerie.famille_informee",
        nature: &PASSAGE_INFIRMERIE,
        filtre: |_| true,
        cle: |e| cle_fait(e, "infirmerie"),
        parametres: |e| jour_de(e, "jour"),
    },
];

/// Nature (code) correspondant à un `kind` de l'ancien chemin, pour la bascule progressive.
pub fn nature_de_kind_ancien(kind: &str) -> Option<&'static str> {
    match kind {
        "absence_constatee" => Some(ABSENCE_CONSTATEE.code),
        "retard" => Some(RETARD_CONSTATE.code),
        "infirmerie_passage" => Some(PASSAGE_INFIRMERIE.code),
        _ => None,
    }
}

/// Nature déclarée par son code.
pub fn nature(code: &str) -> Option<&'static NatureAvisDecl> {
    NATURES.iter().copied().find(|n| n.code == code)
}
