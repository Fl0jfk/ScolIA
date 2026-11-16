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
/// Demi-journée de l'avis d'assiduité (socle 12, AS5) : « (matin) », « (après-midi) » ou vide.
const PERIODE: Variable = Variable { nom: "periode", sensibilite: Sensibilite::C0 };
/// Sens de la décision d'assiduité (socle 12, AS5) : « justifiée » ou « non retenue ».
const DECISION: Variable = Variable { nom: "decision", sensibilite: Sensibilite::C0 };

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
        corps: "{prenom} a été noté(e) absent(e) à l'appel le {jour}{periode}. \
                Vous pouvez justifier cette absence depuis le portail famille.",
        sms: "Vie scolaire : une information concernant {prenom} vous attend sur ScolIA.",
        variables: &[PRENOM, JOUR, PERIODE],
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

/// Le justificatif d'absence est accepté : information (U0), la première à passer au résumé (budget de bruit).
pub const ABSENCE_JUSTIFIEE: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.absence_justifiee",
    version: 1,
    brique: "vie_scolaire",
    categorie: Categorie::VieScolaire,
    sujet: TypeSujet::Eleve,
    eleve_destinataire: EleveDest::SiMajeur,
    droit_requis: DroitRequis::VieScolaire,
    urgence: Urgence::U0,
    sensibilite: Sensibilite::C1,
    plancher: Plancher::InApp,
    canaux_defaut: &[Canal::InApp, Canal::Push, Canal::Email],
    escalade: None,
    delai_confirmation_min: None,
    regroupement: Regroupement::Resume,
    gabarit: Gabarit {
        id: "vie_scolaire.absence_justifiee",
        titre: "Absence de {prenom} justifiée",
        corps: "Le justificatif d'absence de {prenom} a été accepté.",
        sms: "Vie scolaire : une information concernant {prenom} vous attend sur ScolIA.",
        variables: &[PRENOM],
    },
    annulee_par: &[],
    correction_si_remis: false,
    conservation_preuve: true,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: false,
};


// ───────────────────────── Socle 12, lot AS5 : natures de l'assiduité (§14.5) ─────────────────────────

/// Une absence déjà annoncée à la famille est corrigée (élève présent, localisé, séance annulée) : le démenti
/// suit l'avis (R-AS14). Il ne part que si l'avis est effectivement parti ; avant la remise, l'avis est annulé.
pub const ABSENCE_DEMENTI: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.absence_dementi",
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
    escalade: None,
    delai_confirmation_min: Some(0),
    regroupement: Regroupement::Aucun,
    gabarit: Gabarit {
        id: "vie_scolaire.absence_dementi",
        titre: "Correction concernant {prenom}",
        corps: "L'avis d'absence de {prenom} du {jour}{periode} était inexact : {prenom} n'est pas absent(e). \
                Merci de ne pas en tenir compte, et toutes nos excuses pour ce message.",
        sms: "Vie scolaire : une correction concernant {prenom} vous attend sur ScolIA.",
        variables: &[PRENOM, JOUR, PERIODE],
    },
    annulee_par: &[],
    correction_si_remis: false,
    conservation_preuve: true,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: false,
};

/// La décision sur une absence change de sens après une requalification (justifiée ↔ non retenue) : information
/// (U0) à lire au résumé. Les premières décisions restent portées par `absence_justifiee` (ancien circuit).
pub const ABSENCE_DECISION: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.absence_decision",
    version: 1,
    brique: "vie_scolaire",
    categorie: Categorie::VieScolaire,
    sujet: TypeSujet::Eleve,
    eleve_destinataire: EleveDest::SiMajeur,
    droit_requis: DroitRequis::VieScolaire,
    urgence: Urgence::U0,
    sensibilite: Sensibilite::C1,
    plancher: Plancher::InApp,
    canaux_defaut: &[Canal::InApp, Canal::Push, Canal::Email],
    escalade: None,
    delai_confirmation_min: None,
    regroupement: Regroupement::Resume,
    gabarit: Gabarit {
        id: "vie_scolaire.absence_decision",
        titre: "Décision modifiée pour {prenom}",
        corps: "La décision concernant l'absence de {prenom} du {jour} a changé : elle est désormais {decision}. \
                Le détail est dans le portail famille.",
        sms: "Vie scolaire : une information concernant {prenom} vous attend sur ScolIA.",
        variables: &[PRENOM, JOUR, DECISION],
    },
    annulee_par: &[],
    correction_si_remis: false,
    conservation_preuve: true,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: false,
};

/// Relance graduée d'une absence sans motif (§10.5) : U1 au rappel (ENT, push), montée à U2 au troisième jour
/// scolaire pour ouvrir le SMS ou le courriel (`urgence_plus`, bornée par la nature).
pub const RELANCE_JUSTIFICATIF: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.relance_justificatif",
    version: 1,
    brique: "vie_scolaire",
    categorie: Categorie::VieScolaire,
    sujet: TypeSujet::Eleve,
    eleve_destinataire: EleveDest::SiMajeur,
    droit_requis: DroitRequis::VieScolaire,
    urgence: Urgence::U1,
    sensibilite: Sensibilite::C1,
    plancher: Plancher::UnExterne,
    canaux_defaut: &[Canal::InApp, Canal::Push, Canal::Email],
    escalade: None,
    delai_confirmation_min: None,
    regroupement: Regroupement::Aucun,
    gabarit: Gabarit {
        id: "vie_scolaire.relance_justificatif",
        titre: "Motif d'absence attendu pour {prenom}",
        corps: "Nous n'avons pas reçu le motif de l'absence de {prenom} du {jour}. \
                Vous pouvez le donner depuis le portail famille ou joindre la vie scolaire.",
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

/// Absence ou retour d'un élève accompagné (AESH), sans motif (§14.3, AS-206). Destinataire : le personnel qui
/// l'accompagne. **Aucun destinataire n'est résolu tant que le moteur ne sait pas prévenir un personnel**
/// (socle 8, N12) : la nature est déclarée, jamais émise vers une famille.
pub const ABSENCE_ELEVE_ACCOMPAGNE: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.absence_eleve_accompagne",
    version: 1,
    brique: "vie_scolaire",
    categorie: Categorie::VieScolaire,
    sujet: TypeSujet::Personne,
    eleve_destinataire: EleveDest::Jamais,
    droit_requis: DroitRequis::VieScolaire,
    urgence: Urgence::U1,
    sensibilite: Sensibilite::C1,
    plancher: Plancher::InApp,
    canaux_defaut: &[Canal::InApp, Canal::Push],
    escalade: None,
    delai_confirmation_min: None,
    regroupement: Regroupement::Aucun,
    gabarit: Gabarit {
        id: "vie_scolaire.absence_eleve_accompagne",
        titre: "Élève accompagné(e) absent(e)",
        corps: "{prenom} est absent(e) le {jour}.",
        sms: "Vie scolaire : une information vous attend sur ScolIA.",
        variables: &[PRENOM, JOUR],
    },
    annulee_par: &[],
    correction_si_remis: false,
    conservation_preuve: false,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: false,
};

/// Appel à faire (rappel au professeur ou à l'AED) — émetteur : lot AS6, destinataire personnel (N12).
pub const APPEL_A_FAIRE: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.appel_a_faire",
    version: 1,
    brique: "vie_scolaire",
    categorie: Categorie::VieScolaire,
    sujet: TypeSujet::Cours,
    eleve_destinataire: EleveDest::Jamais,
    droit_requis: DroitRequis::VieScolaire,
    urgence: Urgence::U1,
    sensibilite: Sensibilite::C0,
    plancher: Plancher::InApp,
    canaux_defaut: &[Canal::InApp, Canal::Push],
    escalade: None,
    delai_confirmation_min: None,
    regroupement: Regroupement::Aucun,
    gabarit: Gabarit {
        id: "vie_scolaire.appel_a_faire",
        titre: "Appel à faire",
        corps: "L'appel du {jour} n'est pas encore fait.",
        sms: "ScolIA : un appel est à faire.",
        variables: &[JOUR],
    },
    annulee_par: &[],
    correction_si_remis: false,
    conservation_preuve: false,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: false,
};

/// Appel non fait (à la vie scolaire) — émetteur : lot AS6, destinataire personnel (N12).
pub const APPEL_NON_FAIT: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.appel_non_fait",
    version: 1,
    brique: "vie_scolaire",
    categorie: Categorie::VieScolaire,
    sujet: TypeSujet::Cours,
    eleve_destinataire: EleveDest::Jamais,
    droit_requis: DroitRequis::VieScolaire,
    urgence: Urgence::U1,
    sensibilite: Sensibilite::C0,
    plancher: Plancher::InApp,
    canaux_defaut: &[Canal::InApp, Canal::Push],
    escalade: None,
    delai_confirmation_min: None,
    regroupement: Regroupement::Aucun,
    gabarit: Gabarit {
        id: "vie_scolaire.appel_non_fait",
        titre: "Appel non fait",
        corps: "L'appel du {jour} n'a pas été fait à l'heure.",
        sms: "ScolIA : un appel n'a pas été fait.",
        variables: &[JOUR],
    },
    annulee_par: &[],
    correction_si_remis: false,
    conservation_preuve: false,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: false,
};

/// Conflit d'appel après une saisie hors ligne — émetteur : lot AS10, destinataire personnel (N12).
pub const CONFLIT_HORS_LIGNE: NatureAvisDecl = NatureAvisDecl {
    code: "vie_scolaire.conflit_hors_ligne",
    version: 1,
    brique: "vie_scolaire",
    categorie: Categorie::VieScolaire,
    sujet: TypeSujet::Personne,
    eleve_destinataire: EleveDest::Jamais,
    droit_requis: DroitRequis::VieScolaire,
    urgence: Urgence::U1,
    sensibilite: Sensibilite::C0,
    plancher: Plancher::InApp,
    canaux_defaut: &[Canal::InApp, Canal::Push],
    escalade: None,
    delai_confirmation_min: None,
    regroupement: Regroupement::Aucun,
    gabarit: Gabarit {
        id: "vie_scolaire.conflit_hors_ligne",
        titre: "Conflit sur un appel hors ligne",
        corps: "Un appel saisi hors ligne le {jour} est en conflit avec une autre saisie : à trancher.",
        sms: "ScolIA : un appel est à trancher.",
        variables: &[JOUR],
    },
    annulee_par: &[],
    correction_si_remis: false,
    conservation_preuve: false,
    confidentiel_famille: false,
    texte_libre: TexteLibre::Interdit,
    actions_emission: &[],
    brique_u3_autorisee: false,
};

/// Toutes les natures déclarées (l'assemblage les vérifie).
pub const NATURES: &[&NatureAvisDecl] = &[
    &ABSENCE_CONSTATEE,
    &RETARD_CONSTATE,
    &PASSAGE_INFIRMERIE,
    &ABSENCE_JUSTIFIEE,
    // Socle 12, lot AS5 (§14.5).
    &ABSENCE_DEMENTI,
    &ABSENCE_DECISION,
    &RELANCE_JUSTIFICATIF,
    &ABSENCE_ELEVE_ACCOMPAGNE,
    &APPEL_A_FAIRE,
    &APPEL_NON_FAIT,
    &CONFLIT_HORS_LIGNE,
];

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
        // Socle 12 (AS5) : quand l'appel porte `avis_demi_journee`, l'avis est créé par demi-journée par le
        // consommateur `assiduite.familles` (registre `ent_asd_avis_demi_journee`) et non plus par événement.
        filtre: |e| {
            e.payload_str("source") == Some("appel")
                && e.payload_bool("prevenue") != Some(true)
                && e.payload_bool("avis_demi_journee") != Some(true)
        },
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
        evenement: types::ELEVE_ABSENCE_JUSTIFIED,
        nature: &ABSENCE_JUSTIFIEE,
        filtre: |_| true,
        cle: |e| cle_fait(e, "justifiee"),
        parametres: |_| Vec::new(),
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
        "absence_justifiee" => Some(ABSENCE_JUSTIFIEE.code),
        "infirmerie_passage" => Some(PASSAGE_INFIRMERIE.code),
        _ => None,
    }
}

/// Nature déclarée par son code.
pub fn nature(code: &str) -> Option<&'static NatureAvisDecl> {
    NATURES.iter().copied().find(|n| n.code == code)
}
