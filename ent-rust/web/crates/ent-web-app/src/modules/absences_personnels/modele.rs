//! Logique d'écran des absences des personnels (socle 15, A11a à A11c) : **pure**, sans Leptos ni
//! réseau, testée en natif (libellés, composition de la déclaration, actions proposées, regroupement du
//! tableau « ce matin », plan de remplacements).
//!
//! Le serveur reste l'autorité (droits §23, circuit, traitements permis, confidentialité) : ici on ne
//! fait que présenter et ne pas proposer ce qui serait refusé à coup sûr. Aucun libellé ne déduit une
//! nature d'un autre champ : la nature n'est affichée que si l'API la fournit (R-AP2).

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use ent_types::absences_personnels::{
    AbsenceDto, DeclarerAbsenceBody, ImpactDto, ImpactTableauDto, LigneTableauDto, NatureDto,
    PlageBody, PropositionDto, TableauJourDto,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::noyau::temps::{a_paris, date_longue};

// ───────────────────────────── Libellés ─────────────────────────────

pub fn libelle_etat(etat: &str) -> &'static str {
    match etat {
        "signalee" => "Signalée, à confirmer",
        "declaree" => "Déclarée",
        "en_instruction" => "En instruction",
        "prise_en_compte" => "Prise en compte",
        "decision_employeur" => "Décision de l'employeur reçue",
        "validee" => "Validée",
        "refusee" => "Refusée",
        "traitee" => "Traitée",
        "close" => "Clôturée",
        "annulee" => "Annulée",
        _ => "État inconnu",
    }
}

pub fn ton_etat(etat: &str) -> TonEtat {
    match etat {
        "signalee" | "en_instruction" => TonEtat::Avertissement,
        "declaree" | "prise_en_compte" => TonEtat::Info,
        "validee" | "traitee" | "decision_employeur" => TonEtat::Ok,
        "refusee" => TonEtat::Mal,
        _ => TonEtat::Neutre,
    }
}

pub fn libelle_etat_impact(etat: &str) -> &'static str {
    match etat {
        "a_traiter" => "À traiter",
        "propose" => "Réponse proposée",
        "sollicite" => "Remplaçant demandé",
        "remplace" => "Remplacée",
        "autre_cours" => "Autre cours",
        "deplace" => "Déplacée",
        "continuite" => "Continuité encadrée",
        "annule" => "Annulée, élèves pris en charge",
        "supplee" => "Suppléance",
        "sans_objet" => "Sans objet",
        "non_assure" => "Non assurée",
        _ => "État inconnu",
    }
}

pub fn ton_etat_impact(etat: &str) -> TonEtat {
    match etat {
        "a_traiter" | "non_assure" => TonEtat::Mal,
        "propose" | "sollicite" => TonEtat::Avertissement,
        "remplace" | "autre_cours" | "deplace" | "continuite" | "supplee" => TonEtat::Ok,
        "annule" => TonEtat::Info,
        _ => TonEtat::Neutre,
    }
}

/// Une réponse peut être levée (réversible, fait daté) quand elle a été posée.
pub fn reponse_levable(etat: &str) -> bool {
    matches!(
        etat,
        "sollicite" | "remplace" | "autre_cours" | "deplace" | "continuite" | "annule" | "supplee"
    )
}

/// Un impact attend une réponse : on peut lui chercher un remplaçant.
pub fn reponse_attendue(etat: &str) -> bool {
    matches!(etat, "a_traiter" | "propose" | "non_assure")
}

/// Traitements des heures (§9.3) : l'union des listes État et OGEC ; le serveur impose celui qui
/// convient à l'engagement et à la nature, et son refus est affiché.
pub const TRAITEMENTS: &[&str] = &[
    "sans_incidence",
    "rattrapage_interne",
    "transmission_rectorat",
    "service_non_fait",
    "rattrapage",
    "retenue_salaire",
    "maintien_salaire",
    "conge_paye",
];

pub fn libelle_traitement(code: &str) -> String {
    match code {
        "sans_incidence" => "Sans incidence sur les heures",
        "rattrapage_interne" => "Rattrapage interne",
        "transmission_rectorat" => "Transmission au rectorat",
        "service_non_fait" => "Service non fait (retenue signalée)",
        "rattrapage" => "Rattrapage",
        "retenue_salaire" => "Retenue de salaire",
        "maintien_salaire" => "Maintien de salaire",
        "conge_paye" => "Décompte sur les congés payés",
        autre => autre,
    }
    .to_string()
}

pub fn libelle_famille(famille: &str) -> &'static str {
    match famille {
        "sante" => "Santé",
        "parentalite" => "Parentalité",
        "famille" => "Famille",
        "autorisation" => "Autorisation d'absence",
        "service" => "Service",
        "representation" => "Représentation",
        "greve" => "Grève",
        "conges" => "Congés",
        _ => "Autre",
    }
}

pub fn libelle_piece_attendue(code: &str) -> &'static str {
    match code {
        "arret" => "un arrêt de travail (volets 2 et 3)",
        "justificatif" => "un justificatif",
        "convocation" => "la convocation",
        "declaration" => "la déclaration d'accident",
        "certificat" => "un certificat",
        _ => "",
    }
}

pub const TYPES_PIECE: &[(&str, &str)] = &[
    ("arret", "Arrêt de travail"),
    ("justificatif", "Justificatif"),
    ("convocation", "Convocation"),
    ("autre", "Autre pièce"),
];

pub fn libelle_type_piece(code: &str) -> &'static str {
    TYPES_PIECE
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| *l)
        .unwrap_or("Pièce")
}

pub fn libelle_etat_piece(etat: &str) -> &'static str {
    match etat {
        "deposee" => "Déposée",
        "retiree" => "Retirée",
        "purgee" => "Supprimée (délai de conservation échu)",
        _ => "",
    }
}

pub fn libelle_echeance(genre: &str) -> &'static str {
    match genre {
        "arret_a_recevoir" => "Arrêt à recevoir sous 48 h",
        "piece_manquante" => "Pièce attendue",
        _ => "Échéance",
    }
}

pub fn libelle_acteur(acteur: &str) -> &'static str {
    match acteur {
        "agent" => "La personne absente",
        "valideur" => "Le valideur",
        "instructeur" => "L'instructeur (secrétariat ou comptabilité)",
        "employeur" => "L'employeur (rectorat, DSDEN)",
        _ => "Personne",
    }
}

pub fn libelle_action_etape(action: &str) -> &'static str {
    match action {
        "confirmer" => "doit confirmer l'absence",
        "valider" => "doit valider ou refuser",
        "traiter" => "doit traiter administrativement",
        "clore" => "doit clôturer",
        "decision_externe" => "doit rendre sa décision, à enregistrer ici",
        _ => "n'a rien à faire",
    }
}

pub fn libelle_role_circuit(role: &str) -> &'static str {
    match role {
        "valideur" => "Valideur",
        "instructeur" => "Instructeur",
        "informe" => "Informé de l'organisation",
        _ => "Intervenant",
    }
}

pub fn libelle_source_circuit(source: &str) -> &'static str {
    match source {
        "regle" => "règle de l'établissement",
        "responsable_fonctionnel" => "responsable fonctionnel",
        "defaut" => "par défaut (chef d'établissement)",
        "delegation" => "délégation",
        _ => "",
    }
}

/// Fait du journal en clair (sans valeur : la valeur n'est montrée qu'à qui la reçoit de l'API).
pub fn libelle_action_journal(action: &str) -> &'static str {
    match action {
        "creee" => "Absence enregistrée",
        "confirmee" => "Confirmée par la personne",
        "validee" => "Validée",
        "refusee" => "Refusée",
        "precision_demandee" => "Précision demandée",
        "traitee" => "Traitée",
        "close" | "closee" => "Clôturée",
        "rouverte" => "Rouverte",
        "annulee" => "Annulée",
        "requalifiee" => "Nature changée",
        "prolongee" => "Prolongée",
        "ecourtee" => "Écourtée (reprise anticipée)",
        "arret_recu" => "Arrêt reçu",
        "piece_deposee" => "Pièce déposée",
        "piece_retiree" => "Pièce retirée",
        "piece_purgee" => "Pièce supprimée",
        "piece_lue" => "Pièce consultée",
        "message" => "Message au fil",
        "decision_externe" => "Décision de l'employeur enregistrée",
        _ => "Modification",
    }
}

pub const REFUS_CODES: &[(&str, &str)] = &[
    ("", "Sans motif"),
    ("absent", "Je suis absent"),
    ("autre_service", "J'ai un autre service à cette heure"),
    ("delai", "Délai de prévenance non respecté"),
    ("autre", "Autre motif légitime"),
];

pub fn libelle_etat_sollicitation(etat: &str) -> &'static str {
    match etat {
        "envoyee" => "En attente de votre réponse",
        "acceptee" => "Acceptée",
        "refusee" => "Refusée",
        "expiree" => "Expirée sans réponse",
        "annulee" => "Annulée",
        "designee" => "Désignée : accusez réception",
        _ => "État inconnu",
    }
}

pub fn ton_etat_sollicitation(etat: &str) -> TonEtat {
    match etat {
        "envoyee" | "designee" => TonEtat::Avertissement,
        "acceptee" => TonEtat::Ok,
        "refusee" => TonEtat::Mal,
        _ => TonEtat::Neutre,
    }
}

/// Une sollicitation attend une action de l'agent (accepter, refuser ou accuser réception).
pub fn sollicitation_ouverte(etat: &str) -> bool {
    matches!(etat, "envoyee" | "designee")
}

/// Un lot ou un plan s'arrête sur 401 (session), 403 (droit) ou 429 (trop de requêtes) au lieu d'enchaîner
/// les écritures (FRONTEND_LEPTOS §11) ; les autres échecs sont listés et le lot continue.
pub fn arret_sur_erreur(e: &ent_web_api::ApiError) -> bool {
    use ent_web_api::ApiError::*;
    matches!(e, NonConnecte | Interdit(_) | TropDeRequetes)
}

// ───────────────────────────── Dates ─────────────────────────────

/// Premier et dernier jour **inclus** (jour de Paris) d'une absence dont la fin est exclusive.
pub fn jours_absence(debut: DateTime<Utc>, fin: DateTime<Utc>) -> (NaiveDate, NaiveDate) {
    let d = a_paris(debut);
    let f = a_paris(fin);
    let dernier = if f.time() == NaiveTime::MIN && f.date() > d.date() {
        f.date() - Duration::days(1)
    } else {
        f.date()
    };
    (d.date(), dernier.max(d.date()))
}

fn jour_court(d: NaiveDate) -> String {
    date_longue(d).to_lowercase()
}

/// « Mardi 3 novembre », « du mardi 3 au jeudi 5 novembre », « mardi 3 novembre, de 08:00 à 12:00 ».
pub fn libelle_periode(debut: DateTime<Utc>, fin: DateTime<Utc>) -> String {
    let (premier, dernier) = jours_absence(debut, fin);
    let (d, f) = (a_paris(debut), a_paris(fin));
    let entiere = d.time() == NaiveTime::MIN && f.time() == NaiveTime::MIN;
    if premier == dernier {
        if entiere {
            date_longue(premier)
        } else {
            let h_fin = if f.time() == NaiveTime::MIN {
                "minuit".to_string()
            } else {
                f.format("%H:%M").to_string()
            };
            format!(
                "{}, de {} à {}",
                date_longue(premier),
                d.format("%H:%M"),
                h_fin
            )
        }
    } else {
        format!("Du {} au {}", jour_court(premier), jour_court(dernier))
    }
}

// ───────────────────────────── Déclarer (A11a) ─────────────────────────────

/// Saisie brute du formulaire de déclaration (champs en texte, comme dans la page).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieDeclaration {
    /// Identifiant de l'agent absent si l'on déclare pour un collègue ; vide : moi.
    pub collegue: String,
    /// Code de nature ; vide : « je préciserai plus tard ».
    pub nature: String,
    /// `aujourdhui`, `demain` ou `dates`.
    pub quand: String,
    pub debut: String,
    pub fin: String,
    /// `journee`, `matin`, `apres_midi` ou `heures`.
    pub partie: String,
    pub heure_debut: String,
    pub heure_fin: String,
    /// Autres périodes (absence discontinue) : (début, fin).
    pub autres_periodes: Vec<(String, String)>,
    pub fin_provisoire: bool,
    pub texte_libre: String,
    /// Préférence de traitement des heures (jamais imposée) ; vide : aucune.
    pub preference: String,
}

/// Erreurs d'un formulaire : (identifiant du champ, message).
pub type ErreursSaisie = Vec<(String, String)>;

pub const MIDI: NaiveTime = match NaiveTime::from_hms_opt(12, 0, 0) {
    Some(h) => h,
    None => NaiveTime::MIN,
};

fn lire_jour(texte: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texte.trim(), "%Y-%m-%d").ok()
}

fn lire_heure_saisie(texte: &str) -> Option<NaiveTime> {
    let t = texte.trim();
    NaiveTime::parse_from_str(t, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(t, "%H:%M:%S"))
        .ok()
}

/// Nombre maximal de caractères d'un texte libre (R-AP1).
pub const TEXTE_LIBRE_MAX: usize = 200;

/// Compose le corps envoyé à `POST /personnels/absences` après contrôle local. Les règles de fond (nature
/// permise à l'engagement, texte libre sans donnée de santé, chevauchement, droits) restent au serveur.
pub fn composer_declaration(
    s: &SaisieDeclaration,
    aujourdhui: NaiveDate,
    nature: Option<&NatureDto>,
) -> Result<DeclarerAbsenceBody, ErreursSaisie> {
    let mut e: ErreursSaisie = Vec::new();

    let agent_id = match s.collegue.trim() {
        "" => None,
        t => match Uuid::parse_str(t) {
            Ok(u) => Some(u),
            Err(_) => {
                e.push((
                    "decl-collegue".into(),
                    "Choisissez la personne absente dans la liste.".into(),
                ));
                None
            }
        },
    };

    if !s.nature.trim().is_empty() && nature.is_none() {
        e.push((
            "decl-nature".into(),
            "Cette nature n'est pas proposée pour votre engagement.".into(),
        ));
    }

    let (debut, fin) = match s.quand.as_str() {
        "aujourdhui" => (Some(aujourdhui), Some(aujourdhui)),
        "demain" => {
            let d = aujourdhui + Duration::days(1);
            (Some(d), Some(d))
        }
        _ => {
            let d = lire_jour(&s.debut);
            let f = lire_jour(&s.fin).or(d);
            if d.is_none() {
                e.push((
                    "decl-debut".into(),
                    "Indiquez le premier jour d'absence.".into(),
                ));
            }
            if !s.fin.trim().is_empty() && lire_jour(&s.fin).is_none() {
                e.push((
                    "decl-fin".into(),
                    "Le dernier jour n'est pas une date valide.".into(),
                ));
            }
            (d, f)
        }
    };
    if let (Some(d), Some(f)) = (debut, fin) {
        if f < d {
            e.push((
                "decl-fin".into(),
                "Le dernier jour ne peut pas précéder le premier.".into(),
            ));
        }
    }

    let mut heure_debut = None;
    let mut heure_fin = None;
    let un_seul_jour = debut.is_some() && debut == fin;
    match s.partie.as_str() {
        "matin" | "apres_midi" | "heures" if !un_seul_jour && debut.is_some() && fin.is_some() => {
            e.push((
                "decl-partie-journee".into(),
                "Une demi-journée ou des heures ne s'appliquent qu'à un seul jour : ajoutez une autre période pour une absence discontinue.".into(),
            ));
        }
        "matin" => heure_fin = Some(MIDI),
        "apres_midi" => heure_debut = Some(MIDI),
        "heures" => {
            let h1 = lire_heure_saisie(&s.heure_debut);
            let h2 = lire_heure_saisie(&s.heure_fin);
            match (h1, h2) {
                (Some(a), Some(b)) if b > a => {
                    heure_debut = Some(a);
                    heure_fin = Some(b);
                }
                (Some(_), Some(_)) => {
                    e.push((
                        "decl-heure-fin".into(),
                        "L'heure de fin doit suivre l'heure de début.".into(),
                    ));
                }
                (None, _) => e.push((
                    "decl-heure-debut".into(),
                    "Indiquez l'heure de début.".into(),
                )),
                (_, None) => e.push(("decl-heure-fin".into(), "Indiquez l'heure de fin.".into())),
            }
        }
        _ => {}
    }

    let texte = s.texte_libre.trim();
    let texte_libre = if texte.is_empty() {
        None
    } else if texte.chars().count() > TEXTE_LIBRE_MAX {
        e.push((
            "decl-texte".into(),
            format!("Le texte est limité à {TEXTE_LIBRE_MAX} caractères."),
        ));
        None
    } else if !nature.map(|n| n.texte_libre_permis).unwrap_or(false) {
        e.push((
            "decl-texte".into(),
            "Un texte libre n'est pas permis pour cette nature : le motif reste confidentiel."
                .into(),
        ));
        None
    } else {
        Some(texte.to_string())
    };

    let preference_traitement = match s.preference.trim() {
        "" => None,
        p if TRAITEMENTS.contains(&p) => Some(p.to_string()),
        _ => {
            e.push((
                "decl-preference".into(),
                "Ce traitement des heures n'existe pas.".into(),
            ));
            None
        }
    };

    // Autres périodes : une absence, N plages (ABP-115).
    let mut plages: Vec<PlageBody> = Vec::new();
    for (i, (d, f)) in s.autres_periodes.iter().enumerate() {
        if d.trim().is_empty() && f.trim().is_empty() {
            continue;
        }
        let id = format!("decl-autre-debut-{i}");
        match (lire_jour(d), lire_jour(f).or_else(|| lire_jour(d))) {
            (Some(d), Some(f)) if f >= d => plages.push(PlageBody {
                debut: d,
                fin: f,
                heure_debut: None,
                heure_fin: None,
            }),
            (Some(_), Some(_)) => e.push((id, "Le dernier jour précède le premier.".into())),
            _ => e.push((id, "Cette période n'est pas une date valide.".into())),
        }
    }

    if !e.is_empty() {
        return Err(e);
    }
    let (Some(debut), Some(fin)) = (debut, fin) else {
        return Err(vec![(
            "decl-debut".into(),
            "Indiquez le premier jour d'absence.".into(),
        )]);
    };

    let (mut env_debut, mut env_fin) = (debut, fin);
    let mut corps_plages = Vec::new();
    let (mut h_debut, mut h_fin) = (heure_debut, heure_fin);
    if !plages.is_empty() {
        corps_plages.push(PlageBody {
            debut,
            fin,
            heure_debut,
            heure_fin,
        });
        for p in &plages {
            env_debut = env_debut.min(p.debut);
            env_fin = env_fin.max(p.fin);
        }
        corps_plages.extend(plages);
        // L'enveloppe porte les dates ; les heures sont dans les plages.
        h_debut = None;
        h_fin = None;
    }

    Ok(DeclarerAbsenceBody {
        agent_id,
        nature_code: (!s.nature.trim().is_empty()).then(|| s.nature.trim().to_string()),
        debut: env_debut,
        fin: env_fin,
        heure_debut: h_debut,
        heure_fin: h_fin,
        plages: corps_plages,
        texte_libre,
        fin_provisoire: s.fin_provisoire,
        preference_traitement,
        engagement_id: None,
    })
}

/// Saisie d'un signalement (téléphone, constat à 8 h 05) : **sans nature** (ABP-102, 103).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieSignalement {
    pub agent: String,
    pub debut: String,
    pub fin: String,
    pub heure_debut: String,
    pub heure_fin: String,
}

/// Compose le corps de `POST /personnels/absences/signalements`. Sans date de fin : la même journée.
pub fn composer_signalement(
    s: &SaisieSignalement,
) -> Result<ent_types::absences_personnels::SignalerAbsenceBody, ErreursSaisie> {
    let mut e: ErreursSaisie = Vec::new();
    let agent = Uuid::parse_str(s.agent.trim()).ok();
    if agent.is_none() {
        e.push((
            "sig-agent".into(),
            "Choisissez la personne absente dans la liste.".into(),
        ));
    }
    let debut = lire_jour(&s.debut);
    if debut.is_none() {
        e.push((
            "sig-debut".into(),
            "Indiquez le premier jour d'absence.".into(),
        ));
    }
    let fin = if s.fin.trim().is_empty() {
        debut
    } else {
        lire_jour(&s.fin)
    };
    if fin.is_none() && !s.fin.trim().is_empty() {
        e.push((
            "sig-fin".into(),
            "Le dernier jour n'est pas une date valide.".into(),
        ));
    }
    if let (Some(d), Some(f)) = (debut, fin) {
        if f < d {
            e.push((
                "sig-fin".into(),
                "Le dernier jour ne peut pas précéder le premier.".into(),
            ));
        }
    }
    let heure = |t: &str, champ: &str, e: &mut ErreursSaisie| {
        if t.trim().is_empty() {
            return None;
        }
        let h = lire_heure_saisie(t);
        if h.is_none() {
            e.push((champ.to_string(), "Cette heure n'est pas valide.".into()));
        }
        h
    };
    let heure_debut = heure(&s.heure_debut, "sig-heure-debut", &mut e);
    let heure_fin = heure(&s.heure_fin, "sig-heure-fin", &mut e);
    if let (Some(a), Some(b)) = (heure_debut, heure_fin) {
        if b <= a && debut == fin {
            e.push((
                "sig-heure-fin".into(),
                "L'heure de fin doit suivre l'heure de début.".into(),
            ));
        }
    }
    if !e.is_empty() {
        return Err(e);
    }
    match (agent, debut, fin) {
        (Some(agent_id), Some(debut), Some(fin)) => {
            Ok(ent_types::absences_personnels::SignalerAbsenceBody {
                agent_id,
                debut,
                fin,
                heure_debut,
                heure_fin,
            })
        }
        _ => Err(vec![(
            "sig-debut".into(),
            "Indiquez le premier jour d'absence.".into(),
        )]),
    }
}

/// Résumé lisible de ce qui va être déclaré, montré avant l'envoi.
pub fn resume_declaration(b: &DeclarerAbsenceBody, nature: Option<&NatureDto>) -> String {
    let jours = if b.debut == b.fin {
        date_longue(b.debut)
    } else {
        format!("du {} au {}", jour_court(b.debut), jour_court(b.fin))
    };
    let quand = match (b.heure_debut, b.heure_fin) {
        (None, Some(h)) if h == MIDI && b.plages.is_empty() => format!("{jours}, le matin"),
        (Some(h), None) if h == MIDI && b.plages.is_empty() => format!("{jours}, l'après-midi"),
        (Some(a), Some(z)) => format!("{jours}, de {} à {}", a.format("%H:%M"), z.format("%H:%M")),
        _ => jours,
    };
    let nature = nature
        .map(|n| n.libelle.clone())
        .unwrap_or_else(|| "nature à préciser plus tard".to_string());
    let mut t = format!("{quand} — {nature}");
    if b.plages.len() > 1 {
        t.push_str(&format!(" ({} périodes)", b.plages.len()));
    }
    if b.fin_provisoire {
        t.push_str(", fin à confirmer");
    }
    t
}

// ───────────────────────────── Mes absences (A11a) ─────────────────────────────

/// Sépare « en cours ou à venir » (les plus proches d'abord) et « passées » (les plus récentes d'abord).
/// Une absence annulée ou refusée n'est jamais « en cours ».
pub fn partager_mes_absences(
    absences: Vec<AbsenceDto>,
    maintenant: DateTime<Utc>,
) -> (Vec<AbsenceDto>, Vec<AbsenceDto>) {
    let (mut a_venir, mut passees): (Vec<_>, Vec<_>) = absences
        .into_iter()
        .partition(|a| a.fin > maintenant && !matches!(a.etat.as_str(), "annulee" | "refusee"));
    a_venir.sort_by_key(|a| a.debut);
    passees.sort_by_key(|a| std::cmp::Reverse(a.debut));
    (a_venir, passees)
}

/// Ce que l'agent peut tenter sur sa propre absence (le serveur reste juge).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionsAgent {
    pub confirmer: bool,
    pub annuler: bool,
    pub prolonger: bool,
    pub ecourter: bool,
    pub deposer_piece: bool,
}

pub fn actions_agent(a: &AbsenceDto, maintenant: DateTime<Utc>) -> ActionsAgent {
    let vivante = !matches!(a.etat.as_str(), "annulee" | "refusee" | "close");
    let en_cours_ou_future = a.fin > maintenant;
    ActionsAgent {
        confirmer: a.etat == "signalee",
        // L'agent n'annule qu'avant le début (ABP-113).
        annuler: vivante && a.debut > maintenant,
        prolonger: matches!(
            a.etat.as_str(),
            "declaree" | "en_instruction" | "validee" | "prise_en_compte"
        ) && a.fin > maintenant - Duration::days(1),
        ecourter: matches!(
            a.etat.as_str(),
            "declaree" | "en_instruction" | "validee" | "prise_en_compte"
        ) && en_cours_ou_future,
        deposer_piece: vivante && a.etat != "traitee",
    }
}

// ───────────────────────────── Fiche et décisions (A11b) ─────────────────────────────

/// Actions proposées sur une fiche selon l'état, les droits d'affichage et la nature de l'absence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionsFiche {
    pub valider: bool,
    pub refuser: bool,
    pub demander_precision: bool,
    pub requalifier: bool,
    pub traiter: bool,
    pub clore: bool,
    pub rouvrir: bool,
    pub annuler: bool,
    pub prolonger: bool,
    pub ecourter: bool,
    pub arret_recu: bool,
    pub decision_externe: bool,
}

impl ActionsFiche {
    pub fn aucune(&self) -> bool {
        *self == Self::default()
    }
}

/// `valideur` / `instructeur` : droits d'affichage du moteur (`personnels.absences.valider` /
/// `instruire`) ; `mienne` : l'absence est celle de la personne connectée (jamais son propre valideur,
/// R-AP10 : aucune décision proposée, le serveur la refuserait de toute façon).
pub fn actions_fiche(
    a: &AbsenceDto,
    valideur: bool,
    instructeur: bool,
    mienne: bool,
    maintenant: DateTime<Utc>,
) -> ActionsFiche {
    if mienne {
        return ActionsFiche::default();
    }
    let etat = a.etat.as_str();
    let externe = a.decision == "externe";
    let ouverte = matches!(etat, "declaree" | "en_instruction");
    let vivante = matches!(
        etat,
        "declaree" | "en_instruction" | "prise_en_compte" | "decision_employeur" | "validee"
    );
    let nature = a.nature_code.as_deref().unwrap_or("");
    ActionsFiche {
        valider: valideur && ouverte,
        refuser: valideur && ouverte && !externe,
        demander_precision: (valideur || instructeur) && etat == "declaree",
        requalifier: (valideur || instructeur) && vivante,
        traiter: instructeur
            && matches!(etat, "validee" | "decision_employeur")
            && !(externe && etat == "validee"),
        clore: (valideur || instructeur) && etat == "traitee",
        rouvrir: valideur && matches!(etat, "validee" | "refusee" | "traitee" | "close"),
        annuler: valideur && (vivante || etat == "signalee"),
        prolonger: instructeur && vivante && a.fin > maintenant - Duration::days(1),
        ecourter: (valideur || instructeur) && vivante && a.fin > maintenant,
        arret_recu: instructeur && vivante && matches!(nature, "maladie" | "accident_service"),
        decision_externe: externe
            && matches!(etat, "prise_en_compte" | "validee")
            && (valideur || instructeur),
    }
}

/// Raisons codées (jamais un diagnostic, R-AP41) proposées pour un refus, une demande de précision, une
/// annulation ou une réouverture.
pub const RAISONS_REFUS: &[(&str, &str)] = &[
    ("non_conforme", "Demande non conforme"),
    ("delai_depasse", "Délai de demande dépassé"),
    ("necessites_service", "Nécessités du service"),
    ("autre", "Autre raison"),
];
pub const RAISONS_PRECISION: &[(&str, &str)] = &[
    ("piece_manquante", "Pièce manquante"),
    ("precision_nature", "Nature à préciser"),
    ("dates_a_verifier", "Dates à vérifier"),
];
pub const RAISONS_ANNULATION: &[(&str, &str)] = &[
    ("erreur_saisie", "Erreur de saisie"),
    ("pas_eu_lieu", "L'absence n'a pas eu lieu"),
    ("doublon", "Doublon"),
    ("autre", "Autre raison"),
];
pub const VOIES_TRANSMISSION: &[(&str, &str)] = &[
    ("rectorat", "Rectorat"),
    ("dsden", "DSDEN"),
    ("paie", "Paie"),
    ("comptabilite", "Comptabilité"),
    ("autre", "Autre voie"),
];
pub const EMPLOYEURS_EXTERNES: &[(&str, &str)] = &[
    ("rectorat", "Rectorat"),
    ("dsden", "DSDEN"),
    ("employeur_mutualisateur", "Employeur mutualisateur"),
    ("autre", "Autre employeur"),
];
pub const DECISIONS_EXTERNES: &[(&str, &str)] = &[
    ("accordee", "Accordée"),
    ("refusee", "Refusée"),
    ("requalifiee", "Requalifiée"),
];

/// Ce que demande la boîte de dialogue d'une décision : titre, bouton, champs, raisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpecDecision {
    pub titre: &'static str,
    pub bouton: &'static str,
    pub message_ok: &'static str,
    pub avec_traitement: bool,
    pub avec_voie: bool,
    pub raisons: &'static [(&'static str, &'static str)],
    pub danger: bool,
}

pub fn spec_decision(a: ent_web_api::ActionDecision) -> SpecDecision {
    use ent_web_api::ActionDecision::*;
    let base = SpecDecision {
        titre: "",
        bouton: "",
        message_ok: "",
        avec_traitement: false,
        avec_voie: false,
        raisons: &[],
        danger: false,
    };
    match a {
        Valider => SpecDecision {
            titre: "Valider l'absence",
            bouton: "Valider",
            message_ok: "Absence validée.",
            avec_traitement: true,
            ..base
        },
        Refuser => SpecDecision {
            titre: "Refuser l'absence",
            bouton: "Refuser",
            message_ok: "Absence refusée : la personne est prévenue.",
            raisons: RAISONS_REFUS,
            danger: true,
            ..base
        },
        Precision => SpecDecision {
            titre: "Demander une précision ou une pièce",
            bouton: "Envoyer la demande",
            message_ok: "Demande envoyée : l'absence passe en instruction.",
            raisons: RAISONS_PRECISION,
            ..base
        },
        Traiter => SpecDecision {
            titre: "Traiter administrativement",
            bouton: "Marquer comme traitée",
            message_ok: "Absence traitée.",
            avec_voie: true,
            ..base
        },
        Clore => SpecDecision {
            titre: "Clôturer l'absence",
            bouton: "Clôturer",
            message_ok: "Absence clôturée.",
            ..base
        },
        Rouvrir => SpecDecision {
            titre: "Rouvrir l'absence",
            bouton: "Rouvrir",
            message_ok: "Absence rouverte : les effets de la décision sont à défaire.",
            raisons: RAISONS_ANNULATION,
            ..base
        },
        Annuler => SpecDecision {
            titre: "Annuler l'absence",
            bouton: "Annuler l'absence",
            message_ok: "Absence annulée : les séances touchées sont à reprendre.",
            raisons: RAISONS_ANNULATION,
            danger: true,
            ..base
        },
    }
}

/// Natures non sensibles où la validation en lot a un sens (ABP-124) : convocations et formations,
/// sans incidence sur les heures. Aucune nature sensible dans un lot ; le serveur revérifie chaque
/// absence une à une.
pub const NATURES_LOT: &[&str] = &["formation", "convocation"];

pub fn eligible_lot(a: &AbsenceDto) -> bool {
    a.etat == "declaree"
        && a.nature_code
            .as_deref()
            .map(|n| NATURES_LOT.contains(&n))
            .unwrap_or(false)
}

/// Trie une file par début d'absence (le plus proche d'abord) ; à égalité, par nom.
pub fn trier_file(mut v: Vec<AbsenceDto>) -> Vec<AbsenceDto> {
    v.sort_by(|a, b| {
        a.debut
            .cmp(&b.debut)
            .then_with(|| a.agent_label.cmp(&b.agent_label))
    });
    v
}

/// Filtre une file sur le nom de l'agent (sans accent ni casse) ; vide : tout.
pub fn filtrer_file(v: &[AbsenceDto], recherche: &str) -> Vec<AbsenceDto> {
    let r = sans_accent(recherche.trim());
    if r.is_empty() {
        return v.to_vec();
    }
    v.iter()
        .filter(|a| {
            a.agent_label
                .as_deref()
                .map(|n| sans_accent(n).contains(&r))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

pub fn sans_accent(t: &str) -> String {
    t.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            autre => autre,
        })
        .collect()
}

/// Nom affiché d'un agent : le libellé fourni par l'API, jamais un identifiant.
pub fn nom_agent(label: &Option<String>) -> String {
    label
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("Personne absente")
        .to_string()
}

// ───────────────────────────── Ce matin (A11c) ─────────────────────────────

/// Fenêtre que le serveur accepte pour le tableau : de la veille à 14 jours (« Jour hors fenêtre »).
pub fn fenetre_tableau(aujourdhui: NaiveDate) -> (NaiveDate, NaiveDate) {
    (
        aujourdhui - Duration::days(1),
        aujourdhui + Duration::days(14),
    )
}

pub fn jour_dans_fenetre(jour: NaiveDate, aujourdhui: NaiveDate) -> bool {
    let (a, b) = fenetre_tableau(aujourdhui);
    (a..=b).contains(&jour)
}

/// Compteurs non nuls du tableau, dans l'ordre d'urgence : (état, libellé, nombre).
pub fn compteurs_non_nuls(t: &TableauJourDto) -> Vec<(&'static str, &'static str, u32)> {
    let c = &t.compteurs;
    [
        ("a_traiter", c.a_traiter),
        ("non_assure", c.non_assure),
        ("propose", c.propose),
        ("sollicite", c.sollicite),
        ("remplace", c.remplace),
        ("autre_cours", c.autre_cours),
        ("deplace", c.deplace),
        ("continuite", c.continuite),
        ("annule", c.annule),
        ("supplee", c.supplee),
        ("sans_objet", c.sans_objet),
    ]
    .into_iter()
    .filter(|(_, n)| *n > 0)
    .map(|(e, n)| (e, libelle_etat_impact(e), n))
    .collect()
}

/// Séances qui attendent encore une réponse (à traiter ou non assurées).
pub fn a_couvrir(t: &TableauJourDto) -> u32 {
    t.compteurs.a_traiter + t.compteurs.non_assure
}

/// Lignes du tableau, avec le nombre d'impacts qui attendent une réponse par heure.
pub fn lignes_avec_urgence(t: &TableauJourDto) -> Vec<(&LigneTableauDto, usize)> {
    t.lignes
        .iter()
        .map(|l| {
            let n = l
                .impacts
                .iter()
                .filter(|i| reponse_attendue(&i.impact.etat))
                .count();
            (l, n)
        })
        .collect()
}

/// Résumé d'une séance touchée : « 4ᵉ B · Mathématiques · salle 12 · 10:00–11:00 ».
pub fn libelle_seance(i: &ImpactDto) -> String {
    match &i.seance {
        Some(s) => {
            let mut v = vec![s.classe.clone(), s.matiere_libelle.clone()];
            if let Some(salle) = s.salle.as_deref().filter(|x| !x.is_empty()) {
                v.push(format!("salle {salle}"));
            }
            v.push(format!(
                "{}–{}",
                s.heure_debut.format("%H:%M"),
                s.heure_fin.format("%H:%M")
            ));
            v.join(" · ")
        }
        None => "Poste de service ou engagement".to_string(),
    }
}

/// Raisons d'une proposition, dans l'ordre où le serveur les classe (§11.2). Jamais une donnée sur la
/// personne : seulement des comptes et la discipline (R-AP37, D-033 B).
pub fn raisons_proposition(p: &PropositionDto) -> Vec<String> {
    let mut r = Vec::new();
    if p.meme_discipline {
        r.push("Même discipline".to_string());
    }
    r.push(match p.seances_ce_jour {
        0 => "Aucune autre séance ce jour".to_string(),
        1 => "1 séance ce jour".to_string(),
        n => format!("{n} séances ce jour"),
    });
    r.push(match p.remplacements_faits {
        0 => "Aucun remplacement fait cette année".to_string(),
        1 => "1 remplacement fait cette année".to_string(),
        n => format!("{n} remplacements faits cette année"),
    });
    // Décision de Florian (2026-11-16) : jamais de compteur ; une mention neutre au-delà du seuil fixé par le serveur.
    if p.repond_rarement {
        r.push("Répond rarement aux sollicitations".to_string());
    }
    r
}

pub fn nom_proposition(p: &PropositionDto) -> String {
    p.nom
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("Collègue")
        .to_string()
}

/// Une ligne du plan de remplacements (ABP-168) : impact, séance en clair, candidat retenu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LignePlan {
    pub impact_id: Uuid,
    pub seance: String,
    pub candidat_id: Option<Uuid>,
    pub candidat_nom: Option<String>,
}

/// Construit l'aperçu du plan : pour chaque impact choisi, la première proposition du serveur.
/// Un impact sans candidat reste dans le plan, sans action (il sera dit « aucun remplaçant libre »).
pub fn composer_plan(
    impacts: &[(Uuid, String)],
    propositions: &[(Uuid, Vec<PropositionDto>)],
) -> Vec<LignePlan> {
    impacts
        .iter()
        .map(|(id, seance)| {
            let premier = propositions
                .iter()
                .find(|(i, _)| i == id)
                .and_then(|(_, v)| v.first());
            LignePlan {
                impact_id: *id,
                seance: seance.clone(),
                candidat_id: premier.map(|p| p.agent_id),
                candidat_nom: premier.map(nom_proposition),
            }
        })
        .collect()
}

/// Lignes du plan qui seront réellement appliquées (un candidat, sans doublon d'agent sur une même heure
/// : le serveur contrôle le chevauchement, mais on ne demande pas deux fois le même agent à la même heure).
pub fn lignes_applicables(plan: &[LignePlan], heures: &[(Uuid, NaiveTime)]) -> Vec<LignePlan> {
    let mut vus: Vec<(Uuid, NaiveTime)> = Vec::new();
    let mut sortie = Vec::new();
    for l in plan {
        let Some(c) = l.candidat_id else { continue };
        let heure = heures
            .iter()
            .find(|(i, _)| *i == l.impact_id)
            .map(|(_, h)| *h);
        if let Some(h) = heure {
            if vus.contains(&(c, h)) {
                continue;
            }
            vus.push((c, h));
        }
        sortie.push(l.clone());
    }
    sortie
}

/// Impacts du tableau qui peuvent entrer dans un plan : ceux qui attendent une réponse.
pub fn impacts_planifiables(t: &TableauJourDto) -> Vec<&ImpactTableauDto> {
    t.lignes
        .iter()
        .flat_map(|l| l.impacts.iter())
        .filter(|i| reponse_attendue(&i.impact.etat))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use ent_types::absences_personnels::{CompteursImpactDto, SeanceImpactDto};

    fn j(a: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, d).unwrap()
    }

    fn utc(a: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(a, m, d, h, mi, 0).unwrap()
    }

    fn absence(etat: &str, decision: &str, nature: Option<&str>) -> AbsenceDto {
        AbsenceDto {
            id: Uuid::nil(),
            agent_id: Uuid::nil(),
            agent_label: Some("Mme Martin".into()),
            etat: etat.into(),
            debut: utc(2026, 11, 16, 7, 0),
            fin: utc(2026, 11, 18, 23, 0),
            fin_provisoire: false,
            nature_code: nature.map(Into::into),
            nature_libelle: None,
            texte_libre: None,
            traitement: None,
            decision: decision.into(),
            confirmee_par: None,
            absence_liee_id: None,
            circuit: vec![],
        }
    }

    fn nature(code: &str, libre: bool) -> NatureDto {
        NatureDto {
            code: code.into(),
            libelle: "Formation".into(),
            famille: "service".into(),
            texte_libre_permis: libre,
            piece_attendue: String::new(),
        }
    }

    #[test]
    fn libelles_et_tons_couvrent_tous_les_etats() {
        for e in [
            "signalee",
            "declaree",
            "en_instruction",
            "prise_en_compte",
            "decision_employeur",
            "validee",
            "refusee",
            "traitee",
            "close",
            "annulee",
        ] {
            assert_ne!(libelle_etat(e), "État inconnu", "{e}");
        }
        assert_eq!(libelle_etat("zzz"), "État inconnu");
        assert_eq!(ton_etat("refusee"), TonEtat::Mal);
        assert_eq!(ton_etat_impact("a_traiter"), TonEtat::Mal);
        assert_eq!(ton_etat_impact("remplace"), TonEtat::Ok);
        for e in [
            "a_traiter",
            "propose",
            "sollicite",
            "remplace",
            "autre_cours",
            "deplace",
            "continuite",
            "annule",
            "supplee",
            "sans_objet",
            "non_assure",
        ] {
            assert_ne!(libelle_etat_impact(e), "État inconnu", "{e}");
        }
    }

    #[test]
    fn un_impact_attend_une_reponse_ou_peut_etre_leve() {
        assert!(reponse_attendue("a_traiter") && reponse_attendue("non_assure"));
        assert!(!reponse_attendue("remplace"));
        assert!(reponse_levable("remplace") && reponse_levable("sollicite"));
        assert!(!reponse_levable("a_traiter") && !reponse_levable("sans_objet"));
    }

    #[test]
    fn la_fin_exclusive_donne_le_dernier_jour_inclus() {
        // 16 novembre 00:00 (Paris) → 19 novembre 00:00 (Paris) = du 16 au 18.
        let (d, f) = jours_absence(utc(2026, 11, 15, 23, 0), utc(2026, 11, 18, 23, 0));
        assert_eq!((d, f), (j(2026, 11, 16), j(2026, 11, 18)));
        assert_eq!(
            libelle_periode(utc(2026, 11, 15, 23, 0), utc(2026, 11, 18, 23, 0)),
            "Du lundi 16 novembre au mercredi 18 novembre"
        );
        assert_eq!(
            libelle_periode(utc(2026, 11, 15, 23, 0), utc(2026, 11, 16, 23, 0)),
            "Lundi 16 novembre"
        );
        assert_eq!(
            libelle_periode(utc(2026, 11, 16, 7, 0), utc(2026, 11, 16, 11, 0)),
            "Lundi 16 novembre, de 08:00 à 12:00"
        );
    }

    #[test]
    fn declarer_aujourdhui_sans_nature_est_un_je_preciserai() {
        let s = SaisieDeclaration {
            quand: "aujourdhui".into(),
            partie: "journee".into(),
            ..Default::default()
        };
        let b = composer_declaration(&s, j(2026, 11, 16), None).unwrap();
        assert_eq!(b.debut, j(2026, 11, 16));
        assert_eq!(b.fin, j(2026, 11, 16));
        assert_eq!(b.nature_code, None);
        assert!(b.agent_id.is_none() && b.plages.is_empty());
        assert!(resume_declaration(&b, None).contains("nature à préciser plus tard"));
    }

    #[test]
    fn declarer_demain_le_matin() {
        let s = SaisieDeclaration {
            quand: "demain".into(),
            partie: "matin".into(),
            nature: "formation".into(),
            ..Default::default()
        };
        let n = nature("formation", true);
        let b = composer_declaration(&s, j(2026, 11, 16), Some(&n)).unwrap();
        assert_eq!(b.debut, j(2026, 11, 17));
        assert_eq!(b.heure_fin, Some(MIDI));
        assert_eq!(b.heure_debut, None);
        assert!(resume_declaration(&b, Some(&n)).contains("le matin"));
    }

    #[test]
    fn une_demi_journee_exige_un_seul_jour() {
        let s = SaisieDeclaration {
            quand: "dates".into(),
            debut: "2026-11-16".into(),
            fin: "2026-11-18".into(),
            partie: "matin".into(),
            ..Default::default()
        };
        let e = composer_declaration(&s, j(2026, 11, 16), None).unwrap_err();
        assert!(e.iter().any(|(c, _)| c == "decl-partie-journee"));
    }

    #[test]
    fn les_dates_incoherentes_sont_signalees_champ_par_champ() {
        let s = SaisieDeclaration {
            quand: "dates".into(),
            debut: "2026-11-18".into(),
            fin: "2026-11-16".into(),
            ..Default::default()
        };
        let e = composer_declaration(&s, j(2026, 11, 16), None).unwrap_err();
        assert_eq!(e[0].0, "decl-fin");
        let vide = SaisieDeclaration {
            quand: "dates".into(),
            ..Default::default()
        };
        let e = composer_declaration(&vide, j(2026, 11, 16), None).unwrap_err();
        assert_eq!(e[0].0, "decl-debut");
    }

    #[test]
    fn des_heures_se_declarent_avec_un_debut_et_une_fin() {
        let mut s = SaisieDeclaration {
            quand: "aujourdhui".into(),
            partie: "heures".into(),
            heure_debut: "10:00".into(),
            heure_fin: "09:00".into(),
            ..Default::default()
        };
        assert_eq!(
            composer_declaration(&s, j(2026, 11, 16), None).unwrap_err()[0].0,
            "decl-heure-fin"
        );
        s.heure_fin = "12:30".into();
        let b = composer_declaration(&s, j(2026, 11, 16), None).unwrap();
        assert_eq!(b.heure_debut, NaiveTime::from_hms_opt(10, 0, 0));
        assert_eq!(b.heure_fin, NaiveTime::from_hms_opt(12, 30, 0));
        s.heure_debut = String::new();
        assert_eq!(
            composer_declaration(&s, j(2026, 11, 16), None).unwrap_err()[0].0,
            "decl-heure-debut"
        );
    }

    #[test]
    fn le_texte_libre_est_refuse_hors_natures_non_sensibles_et_borne() {
        let mut s = SaisieDeclaration {
            quand: "aujourdhui".into(),
            nature: "maladie".into(),
            texte_libre: "rendez-vous".into(),
            ..Default::default()
        };
        let sensible = nature("maladie", false);
        let e = composer_declaration(&s, j(2026, 11, 16), Some(&sensible)).unwrap_err();
        assert_eq!(e[0].0, "decl-texte");
        let libre = nature("formation", true);
        s.nature = "formation".into();
        let b = composer_declaration(&s, j(2026, 11, 16), Some(&libre)).unwrap();
        assert_eq!(b.texte_libre.as_deref(), Some("rendez-vous"));
        s.texte_libre = "x".repeat(201);
        assert_eq!(
            composer_declaration(&s, j(2026, 11, 16), Some(&libre)).unwrap_err()[0].0,
            "decl-texte"
        );
    }

    #[test]
    fn une_nature_absente_de_la_liste_est_refusee_localement() {
        let s = SaisieDeclaration {
            quand: "aujourdhui".into(),
            nature: "greve".into(),
            ..Default::default()
        };
        assert_eq!(
            composer_declaration(&s, j(2026, 11, 16), None).unwrap_err()[0].0,
            "decl-nature"
        );
    }

    #[test]
    fn une_absence_discontinue_compose_les_plages_et_l_enveloppe() {
        let s = SaisieDeclaration {
            quand: "dates".into(),
            debut: "2026-11-23".into(),
            fin: "2026-11-23".into(),
            autres_periodes: vec![
                ("2026-11-30".into(), "2026-11-30".into()),
                ("2026-11-16".into(), String::new()),
            ],
            ..Default::default()
        };
        let b = composer_declaration(&s, j(2026, 11, 16), None).unwrap();
        assert_eq!(b.plages.len(), 3);
        assert_eq!(b.debut, j(2026, 11, 16));
        assert_eq!(b.fin, j(2026, 11, 30));
        assert!(resume_declaration(&b, None).contains("3 périodes"));
    }

    #[test]
    fn une_preference_inconnue_est_refusee_et_un_collegue_invalide_aussi() {
        let s = SaisieDeclaration {
            quand: "aujourdhui".into(),
            preference: "gratuit".into(),
            collegue: "pas-un-uuid".into(),
            ..Default::default()
        };
        let e = composer_declaration(&s, j(2026, 11, 16), None).unwrap_err();
        let champs: Vec<&str> = e.iter().map(|(c, _)| c.as_str()).collect();
        assert!(champs.contains(&"decl-preference") && champs.contains(&"decl-collegue"));
    }

    #[test]
    fn mes_absences_se_partagent_en_cours_et_passees() {
        let maintenant = utc(2026, 11, 17, 9, 0);
        let mut passee = absence("close", "interne", None);
        passee.fin = utc(2026, 11, 10, 23, 0);
        let mut annulee = absence("annulee", "interne", None);
        annulee.fin = utc(2026, 12, 1, 23, 0);
        let en_cours = absence("validee", "interne", None);
        let (a_venir, passees) = partager_mes_absences(vec![passee, annulee, en_cours], maintenant);
        assert_eq!(a_venir.len(), 1);
        assert_eq!(
            passees.len(),
            2,
            "une absence annulée n'est jamais « en cours »"
        );
    }

    #[test]
    fn l_agent_confirme_le_signale_et_n_annule_qu_avant_le_debut() {
        let avant = utc(2026, 11, 15, 9, 0);
        let pendant = utc(2026, 11, 17, 9, 0);
        let s = absence("signalee", "interne", None);
        assert!(actions_agent(&s, avant).confirmer && actions_agent(&s, avant).annuler);
        let v = absence("validee", "interne", None);
        assert!(actions_agent(&v, avant).annuler);
        assert!(
            !actions_agent(&v, pendant).annuler,
            "plus d'annulation une fois commencée"
        );
        assert!(actions_agent(&v, pendant).prolonger && actions_agent(&v, pendant).ecourter);
        let r = absence("refusee", "interne", None);
        assert_eq!(actions_agent(&r, avant), ActionsAgent::default());
    }

    #[test]
    fn jamais_de_decision_sur_sa_propre_absence() {
        let a = absence("declaree", "interne", Some("formation"));
        let maintenant = utc(2026, 11, 16, 9, 0);
        assert!(actions_fiche(&a, true, true, true, maintenant).aucune());
        let f = actions_fiche(&a, true, false, false, maintenant);
        assert!(f.valider && f.refuser && f.demander_precision && f.requalifier && f.annuler);
        assert!(!f.traiter && !f.clore);
    }

    #[test]
    fn la_file_de_l_instructeur_et_la_decision_externe() {
        let maintenant = utc(2026, 11, 16, 9, 0);
        let v = absence("validee", "interne", Some("maladie"));
        let f = actions_fiche(&v, false, true, false, maintenant);
        assert!(f.traiter && f.arret_recu && !f.valider);
        let ext = absence("prise_en_compte", "externe", Some("maladie"));
        let f = actions_fiche(&ext, true, true, false, maintenant);
        assert!(
            f.decision_externe && !f.refuser,
            "l'employeur décide, pas l'établissement"
        );
        let t = absence("traitee", "interne", None);
        let f = actions_fiche(&t, true, false, false, maintenant);
        assert!(f.clore && f.rouvrir && !f.valider);
    }

    #[test]
    fn la_validation_en_lot_exclut_les_natures_sensibles() {
        assert!(eligible_lot(&absence(
            "declaree",
            "interne",
            Some("convocation")
        )));
        assert!(eligible_lot(&absence(
            "declaree",
            "interne",
            Some("formation")
        )));
        assert!(!eligible_lot(&absence(
            "declaree",
            "interne",
            Some("maladie")
        )));
        assert!(!eligible_lot(&absence(
            "declaree",
            "interne",
            Some("greve")
        )));
        assert!(
            !eligible_lot(&absence("declaree", "interne", None)),
            "nature non fournie : jamais en lot"
        );
        assert!(!eligible_lot(&absence(
            "validee",
            "interne",
            Some("formation")
        )));
    }

    #[test]
    fn la_file_se_trie_par_debut_et_se_filtre_sans_accent() {
        let mut a = absence("declaree", "interne", None);
        a.agent_label = Some("Hélène Durand".into());
        a.debut = utc(2026, 11, 20, 7, 0);
        let mut b = absence("declaree", "interne", None);
        b.agent_label = Some("Paul Martin".into());
        let tri = trier_file(vec![a.clone(), b.clone()]);
        assert_eq!(tri[0].agent_label.as_deref(), Some("Paul Martin"));
        assert_eq!(filtrer_file(&tri, "helene").len(), 1);
        assert_eq!(filtrer_file(&tri, "  ").len(), 2);
        assert_eq!(filtrer_file(&tri, "zzz").len(), 0);
        assert_eq!(nom_agent(&None), "Personne absente");
    }

    fn impact(etat: &str, h: u32) -> ImpactTableauDto {
        ImpactTableauDto {
            impact: ImpactDto {
                id: Uuid::from_u128(h as u128),
                absence_id: Uuid::nil(),
                type_impact: "seance".into(),
                seance: Some(SeanceImpactDto {
                    seance_id: Uuid::nil(),
                    creneau_id: Uuid::nil(),
                    jour: j(2026, 11, 16),
                    heure_debut: NaiveTime::from_hms_opt(h, 0, 0).unwrap(),
                    heure_fin: NaiveTime::from_hms_opt(h + 1, 0, 0).unwrap(),
                    classe: "4e B".into(),
                    matiere_libelle: "Mathématiques".into(),
                    salle: Some("12".into()),
                }),
                debut: utc(2026, 11, 16, h, 0),
                fin: utc(2026, 11, 16, h + 1, 0),
                etat: etat.into(),
                a_revoir: false,
            },
            agent_label: Some("Mme Martin".into()),
        }
    }

    fn tableau() -> TableauJourDto {
        let mut c = CompteursImpactDto::default();
        c.ajouter("a_traiter");
        c.ajouter("a_traiter");
        c.ajouter("remplace");
        TableauJourDto {
            jour: j(2026, 11, 16),
            absents: vec![],
            lignes: vec![
                LigneTableauDto {
                    heure_debut: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                    impacts: vec![impact("a_traiter", 8), impact("remplace", 8)],
                },
                LigneTableauDto {
                    heure_debut: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    impacts: vec![impact("a_traiter", 9)],
                },
            ],
            compteurs: c,
        }
    }

    #[test]
    fn le_tableau_compte_l_urgence_par_heure() {
        let t = tableau();
        assert_eq!(a_couvrir(&t), 2);
        let c = compteurs_non_nuls(&t);
        assert_eq!(c[0], ("a_traiter", "À traiter", 2));
        assert_eq!(c.len(), 2);
        let l = lignes_avec_urgence(&t);
        assert_eq!((l[0].1, l[1].1), (1, 1));
        assert_eq!(impacts_planifiables(&t).len(), 2);
        assert_eq!(
            libelle_seance(&t.lignes[0].impacts[0].impact),
            "4e B · Mathématiques · salle 12 · 08:00–09:00"
        );
    }

    fn proposition(
        nom: &str,
        discipline: bool,
        ce_jour: u32,
        faits: u32,
        repond_rarement: bool,
    ) -> PropositionDto {
        PropositionDto {
            agent_id: Uuid::from_u128(nom.bytes().fold(7u128, |a, b| a * 31 + u128::from(b))),
            nom: Some(nom.into()),
            meme_discipline: discipline,
            seances_ce_jour: ce_jour,
            remplacements_faits: faits,
            repond_rarement,
        }
    }

    #[test]
    fn les_raisons_d_une_proposition_ne_disent_rien_de_la_personne() {
        let r = raisons_proposition(&proposition("M. Léon", true, 2, 0, true));
        assert_eq!(
            r,
            vec![
                "Même discipline",
                "2 séances ce jour",
                "Aucun remplacement fait cette année",
                "Répond rarement aux sollicitations"
            ]
        );
        let texte = r.join(" ").to_lowercase();
        for interdit in ["âge", "santé", "maladie", "ancienneté", "absence"] {
            assert!(!texte.contains(interdit), "{interdit}");
        }
    }

    #[test]
    fn le_plan_prend_la_premiere_proposition_et_evite_les_doublons_a_la_meme_heure() {
        let t = tableau();
        let planifiables = impacts_planifiables(&t);
        let impacts: Vec<(Uuid, String)> = planifiables
            .iter()
            .map(|i| (i.impact.id, libelle_seance(&i.impact)))
            .collect();
        let leon = proposition("M. Léon", true, 0, 0, false);
        let propositions = vec![
            (
                impacts[0].0,
                vec![leon.clone(), proposition("Mme Roux", false, 0, 0, false)],
            ),
            (impacts[1].0, vec![]),
        ];
        let plan = composer_plan(&impacts, &propositions);
        assert_eq!(plan[0].candidat_id, Some(leon.agent_id));
        assert_eq!(
            plan[1].candidat_id, None,
            "aucun remplaçant libre : sans action"
        );
        let heures = vec![
            (impacts[0].0, NaiveTime::from_hms_opt(8, 0, 0).unwrap()),
            (impacts[1].0, NaiveTime::from_hms_opt(8, 0, 0).unwrap()),
        ];
        assert_eq!(lignes_applicables(&plan, &heures).len(), 1);
        // Même agent sur la même heure : une seule demande.
        let double = vec![
            plan[0].clone(),
            LignePlan {
                impact_id: impacts[1].0,
                ..plan[0].clone()
            },
        ];
        assert_eq!(lignes_applicables(&double, &heures).len(), 1);
    }

    #[test]
    fn chaque_decision_demande_ses_champs() {
        use ent_web_api::ActionDecision::*;
        assert!(spec_decision(Valider).avec_traitement);
        assert!(spec_decision(Traiter).avec_voie);
        assert!(!spec_decision(Refuser).raisons.is_empty() && spec_decision(Refuser).danger);
        assert!(spec_decision(Annuler).danger && !spec_decision(Clore).danger);
        for a in [
            Valider, Refuser, Precision, Traiter, Clore, Rouvrir, Annuler,
        ] {
            let s = spec_decision(a);
            assert!(!s.titre.is_empty() && !s.bouton.is_empty() && !s.message_ok.is_empty());
            for (code, _) in s.raisons {
                assert!(
                    code.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                    "{code}"
                );
            }
        }
    }

    #[test]
    fn un_lot_s_arrete_sur_401_403_429_et_continue_sur_un_refus_metier() {
        use ent_web_api::ApiError::*;
        assert!(arret_sur_erreur(&NonConnecte));
        assert!(arret_sur_erreur(&Interdit("x".into())));
        assert!(arret_sur_erreur(&TropDeRequetes));
        assert!(!arret_sur_erreur(&Conflit("état".into())));
        assert!(!arret_sur_erreur(&Invalide("traitement".into())));
        assert!(!arret_sur_erreur(&Introuvable("x".into())));
    }

    #[test]
    fn un_signalement_n_a_ni_nature_ni_motif() {
        let agent = Uuid::from_u128(9).to_string();
        let b = composer_signalement(&SaisieSignalement {
            agent: agent.clone(),
            debut: "2026-11-16".into(),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(b.debut, b.fin, "sans dernier jour : la même journée");
        let json = serde_json::to_string(&b).unwrap();
        assert!(
            !json.contains("nature") && !json.contains("motif"),
            "{json}"
        );
        let e = composer_signalement(&SaisieSignalement {
            agent: String::new(),
            debut: "2026-11-18".into(),
            fin: "2026-11-16".into(),
            heure_debut: "25:00".into(),
            ..Default::default()
        })
        .unwrap_err();
        let champs: Vec<&str> = e.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(champs, ["sig-agent", "sig-fin", "sig-heure-debut"]);
    }

    #[test]
    fn le_tableau_couvre_de_la_veille_a_quatorze_jours() {
        let ajd = j(2026, 11, 16);
        assert!(jour_dans_fenetre(j(2026, 11, 15), ajd));
        assert!(jour_dans_fenetre(j(2026, 11, 30), ajd));
        assert!(!jour_dans_fenetre(j(2026, 11, 14), ajd));
        assert!(!jour_dans_fenetre(j(2026, 12, 1), ajd));
    }

    #[test]
    fn sollicitation_ouverte_attend_une_action() {
        assert!(sollicitation_ouverte("envoyee") && sollicitation_ouverte("designee"));
        assert!(!sollicitation_ouverte("acceptee") && !sollicitation_ouverte("expiree"));
        assert_eq!(
            REFUS_CODES[0].0, "",
            "le refus sans motif existe : un collègue hors RCD n'a aucune obligation"
        );
    }
}
