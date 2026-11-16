//! Logique d'écran du **tableau de bord de la vie scolaire en panneaux** (socle 12, AS14b-bis) : pure, sans
//! Leptos ni réseau, testée en natif.
//!
//! Le serveur reste l'autorité (droits, ordre de la file AS6, heure de bascule, mémorisation). Ici on ne fait que
//! **ranger** les lignes de la file du matin dans des panneaux, calculer la **disposition** (selon l'heure ou selon
//! le choix épinglé de la personne) et préparer les changements de disposition à envoyer.
//!
//! Règles (décisions du pilote, 2026-11-16) :
//! - le bandeau « Urgent » regroupe les niveaux de la file où l'on ne sait pas où est l'élève (prévenir maintenant
//!   sans contact joint, exclu non arrivé, absence après présence) ; il n'est **ni un panneau, ni masquable** ;
//! - cinq panneaux : absents à traiter, exclus de cours, appels non faits, incohérences d'appel, familles à rappeler ;
//! - avant l'heure de bascule : absents en premier et en grand ; après : exclus et appels non faits ;
//! - la disposition épinglée (ordre, repliés, masqués) remplace l'automatique ; « revenir à automatique » l'efface.

use chrono::NaiveTime;
use ent_types::assiduite::{
    EntreeFileDto, ExcluDuJourDto, ExclusDuJourDto, FileDuMatinDto, PreferencePanneauDto,
};
use std::cmp::Reverse;

/// Les cinq panneaux (liste fermée, identique à la contrainte du serveur).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panneau {
    Absents,
    Exclus,
    Appels,
    Incoherences,
    Familles,
}

impl Panneau {
    pub const TOUS: [Panneau; 5] = [
        Panneau::Absents,
        Panneau::Exclus,
        Panneau::Appels,
        Panneau::Incoherences,
        Panneau::Familles,
    ];

    /// Code échangé avec le serveur.
    pub fn code(self) -> &'static str {
        match self {
            Panneau::Absents => "absents",
            Panneau::Exclus => "exclus",
            Panneau::Appels => "appels",
            Panneau::Incoherences => "incoherences",
            Panneau::Familles => "familles",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Panneau> {
        Self::TOUS.into_iter().find(|p| p.code() == code)
    }

    pub fn titre(self) -> &'static str {
        match self {
            Panneau::Absents => "Absents du jour à traiter",
            Panneau::Exclus => "Exclus de cours",
            Panneau::Appels => "Appels non faits",
            Panneau::Incoherences => "Incohérences d'appel",
            Panneau::Familles => "Familles à rappeler",
        }
    }

    /// Phrase de l'état vide (jamais un panneau muet).
    pub fn vide(self) -> &'static str {
        match self {
            Panneau::Absents => "Aucun absent sans nouvelles.",
            Panneau::Exclus => "Aucun exclu de cours en attente d'arrivée.",
            Panneau::Appels => "Tous les appels sont faits.",
            Panneau::Incoherences => "Aucune incohérence d'appel.",
            Panneau::Familles => "Aucune famille à rappeler.",
        }
    }
}

// ─── Regroupement ─────────────────────────────────────────────────────────────────────────

/// Catégories de la file AS6 qui vont dans le bandeau « Urgent » : on ne sait pas où est l'élève.
pub fn est_urgent(categorie: &str) -> bool {
    matches!(categorie, "urgence" | "non_arrivee" | "apres_presence")
}

/// Panneau d'une ligne de la file (`None` pour le bandeau « Urgent » et pour une catégorie inconnue : on ne
/// range jamais une ligne qu'on ne comprend pas dans un panneau au hasard).
pub fn panneau_de(categorie: &str) -> Option<Panneau> {
    match categorie {
        "sans_nouvelles" => Some(Panneau::Absents),
        "non_jointe" => Some(Panneau::Familles),
        "appel_non_fait" => Some(Panneau::Appels),
        "incoherence" => Some(Panneau::Incoherences),
        _ => None,
    }
}

/// Lignes de la file rangées par panneau, plus les exclus du jour. Rien n'est inventé : une ligne de catégorie
/// inconnue est comptée dans `ignorees` (l'écran le dit).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Regroupement {
    pub urgent: Vec<EntreeFileDto>,
    pub absents: Vec<EntreeFileDto>,
    pub appels: Vec<EntreeFileDto>,
    pub incoherences: Vec<EntreeFileDto>,
    pub familles: Vec<EntreeFileDto>,
    pub exclus: Vec<ExcluDuJourDto>,
    pub ignorees: usize,
}

impl Regroupement {
    /// Nombre de lignes d'un panneau.
    pub fn compte(&self, p: Panneau) -> usize {
        match p {
            Panneau::Absents => self.absents.len(),
            Panneau::Exclus => self.exclus.len(),
            Panneau::Appels => self.appels.len(),
            Panneau::Incoherences => self.incoherences.len(),
            Panneau::Familles => self.familles.len(),
        }
    }
}

/// Plus ancien d'abord (heure du premier constat) ; une ligne sans heure passe après, puis ordre alphabétique.
fn cle_ancien(e: &EntreeFileDto) -> (bool, Option<NaiveTime>, String, String) {
    (
        e.depuis.is_none(),
        e.depuis,
        e.nom.clone().unwrap_or_default().to_lowercase(),
        e.prenom.clone().unwrap_or_default().to_lowercase(),
    )
}

pub fn regrouper(file: &FileDuMatinDto, exclus: Option<&ExclusDuJourDto>) -> Regroupement {
    let mut r = Regroupement::default();
    for e in &file.entrees {
        if est_urgent(&e.categorie) {
            r.urgent.push(e.clone());
            continue;
        }
        match panneau_de(&e.categorie) {
            Some(Panneau::Absents) => r.absents.push(e.clone()),
            Some(Panneau::Familles) => r.familles.push(e.clone()),
            Some(Panneau::Appels) => r.appels.push(e.clone()),
            Some(Panneau::Incoherences) => r.incoherences.push(e.clone()),
            _ => r.ignorees += 1,
        }
    }
    // Bandeau : le plus urgent (niveau de la file), puis le plus ancien.
    r.urgent.sort_by(|a, b| {
        a.niveau
            .cmp(&b.niveau)
            .then_with(|| cle_ancien(a).cmp(&cle_ancien(b)))
    });
    // Absents : sous vigilance d'abord, mineurs avant majeurs, puis le plus ancien.
    r.absents.sort_by(|a, b| {
        (Reverse(a.vigilance), Reverse(a.mineur.unwrap_or(true)))
            .cmp(&(Reverse(b.vigilance), Reverse(b.mineur.unwrap_or(true))))
            .then_with(|| cle_ancien(a).cmp(&cle_ancien(b)))
    });
    r.familles.sort_by_key(cle_ancien);
    r.appels.sort_by_key(cle_ancien);
    r.incoherences.sort_by_key(cle_ancien);
    if let Some(x) = exclus {
        r.exclus = x.exclus.clone();
        // Plus ancien d'abord = le plus de minutes d'attente.
        r.exclus.sort_by(|a, b| {
            b.minutes_depuis
                .cmp(&a.minutes_depuis)
                .then_with(|| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()))
        });
    }
    r
}

// ─── Disposition ──────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    AvantBascule,
    ApresBascule,
}

/// Avant ou après l'heure de bascule (heure **locale de l'établissement**, donnée par le serveur). À l'heure
/// exacte de bascule, on est déjà « après ».
pub fn moment(heure_locale: NaiveTime, bascule: NaiveTime) -> Moment {
    if heure_locale < bascule {
        Moment::AvantBascule
    } else {
        Moment::ApresBascule
    }
}

/// Un panneau tel qu'il s'affiche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanneauAffiche {
    pub panneau: Panneau,
    /// « En grand » : pleine largeur, plus de lignes visibles. Réservé à la disposition automatique.
    pub grand: bool,
    pub replie: bool,
    pub masque: bool,
}

/// Disposition automatique : avant la bascule, les absents en premier et en grand ; après, les exclus et les appels
/// non faits en premier et en grand.
pub fn disposition_auto(m: Moment) -> Vec<PanneauAffiche> {
    let ordre: [(Panneau, bool); 5] = match m {
        Moment::AvantBascule => [
            (Panneau::Absents, true),
            (Panneau::Familles, false),
            (Panneau::Exclus, false),
            (Panneau::Appels, false),
            (Panneau::Incoherences, false),
        ],
        Moment::ApresBascule => [
            (Panneau::Exclus, true),
            (Panneau::Appels, true),
            (Panneau::Absents, false),
            (Panneau::Familles, false),
            (Panneau::Incoherences, false),
        ],
    };
    ordre
        .into_iter()
        .map(|(panneau, grand)| PanneauAffiche {
            panneau,
            grand,
            replie: false,
            masque: false,
        })
        .collect()
}

/// Une disposition épinglée est utilisable si elle liste chacun des cinq panneaux une fois (comme le serveur).
pub fn preference_utilisable(p: &[PreferencePanneauDto]) -> bool {
    p.len() == Panneau::TOUS.len()
        && Panneau::TOUS
            .iter()
            .all(|t| p.iter().filter(|x| x.panneau == t.code()).count() == 1)
}

/// Disposition affichée : celle que la personne a épinglée, sinon l'automatique de l'heure. Une disposition
/// inutilisable (réponse inattendue) retombe sur l'automatique plutôt que de cacher des panneaux.
pub fn disposition(m: Moment, preference: Option<&[PreferencePanneauDto]>) -> Vec<PanneauAffiche> {
    match preference {
        Some(p) if preference_utilisable(p) => p
            .iter()
            .filter_map(|x| {
                Panneau::depuis_code(&x.panneau).map(|panneau| PanneauAffiche {
                    panneau,
                    grand: false,
                    replie: x.replie,
                    masque: x.masque,
                })
            })
            .collect(),
        _ => disposition_auto(m),
    }
}

/// Mode affiché : « Automatique » ou « Épinglée ».
pub fn est_epinglee(preference: Option<&[PreferencePanneauDto]>) -> bool {
    preference.is_some_and(preference_utilisable)
}

/// Corps à envoyer pour épingler une disposition.
pub fn vers_preference(d: &[PanneauAffiche]) -> Vec<PreferencePanneauDto> {
    d.iter()
        .map(|x| PreferencePanneauDto {
            panneau: x.panneau.code().to_string(),
            replie: x.replie,
            masque: x.masque,
        })
        .collect()
}

/// Remonte (`delta = -1`) ou descend (`+1`) le panneau d'indice `i` ; sans effet aux extrémités.
pub fn deplacer(mut d: Vec<PanneauAffiche>, i: usize, delta: i32) -> Vec<PanneauAffiche> {
    let j = i as i64 + i64::from(delta);
    if i < d.len() && j >= 0 && (j as usize) < d.len() {
        d.swap(i, j as usize);
    }
    d
}

/// Bascule le repli du panneau (la disposition épinglée est mise à jour ; l'automatique, locale).
pub fn basculer_repli(mut d: Vec<PanneauAffiche>, p: Panneau) -> Vec<PanneauAffiche> {
    if let Some(x) = d.iter_mut().find(|x| x.panneau == p) {
        x.replie = !x.replie;
    }
    d
}

/// Bascule le masquage du panneau.
pub fn basculer_masque(mut d: Vec<PanneauAffiche>, p: Panneau) -> Vec<PanneauAffiche> {
    if let Some(x) = d.iter_mut().find(|x| x.panneau == p) {
        x.masque = !x.masque;
        // Un panneau masqué n'est pas « replié » : il revient déplié.
        x.replie = false;
    }
    d
}

/// Un changement de disposition demandé par la personne (données seulement : le serveur reçoit le résultat).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modif {
    Monter(usize),
    Descendre(usize),
    Repli(Panneau),
    Masque(Panneau),
}

pub fn appliquer(d: Vec<PanneauAffiche>, m: Modif) -> Vec<PanneauAffiche> {
    match m {
        Modif::Monter(i) => deplacer(d, i, -1),
        Modif::Descendre(i) => deplacer(d, i, 1),
        Modif::Repli(p) => basculer_repli(d, p),
        Modif::Masque(p) => basculer_masque(d, p),
    }
}

/// Une disposition épinglée peut masquer tous les panneaux : l'écran le dit au lieu de rester muet.
pub fn tout_masque(d: &[PanneauAffiche]) -> bool {
    d.iter().all(|x| x.masque)
}

// ─── Libellés ─────────────────────────────────────────────────────────────────────────────

pub fn nom_complet(prenom: Option<&str>, nom: Option<&str>) -> String {
    let t = format!(
        "{} {}",
        prenom.unwrap_or("").trim(),
        nom.unwrap_or("").trim()
    );
    let t = t.trim();
    if t.is_empty() {
        "Élève".to_string()
    } else {
        t.to_string()
    }
}

pub fn heure_courte(t: NaiveTime) -> String {
    t.format("%H:%M").to_string()
}

/// « depuis 08:05 », « depuis 08:05 (1 h 20) » : durée dite en toutes lettres.
pub fn depuis(h: Option<NaiveTime>, minutes: Option<i64>) -> Option<String> {
    let h = h?;
    let reste = match minutes {
        Some(m) if m >= 60 => format!(" ({} h {:02})", m / 60, m % 60),
        Some(m) if m >= 1 => format!(" ({m} min)"),
        _ => String::new(),
    };
    Some(format!("depuis {}{}", heure_courte(h), reste))
}

/// Libellé de la raison d'une ligne de la file : texte lisible, jamais un code brut.
pub fn libelle_categorie(c: &str) -> &'static str {
    match c {
        "urgence" => "Prévenu en urgence, famille non jointe",
        "non_arrivee" => "Exclu de cours, non arrivé",
        "apres_presence" => "Absent après avoir été présent",
        "non_jointe" => "Famille tentée sans succès",
        "sans_nouvelles" => "Sans nouvelles",
        "appel_non_fait" => "Appel non fait",
        "incoherence" => "Incohérence d'appel",
        _ => "À vérifier",
    }
}

pub fn libelle_detail(code: Option<&str>) -> Option<&'static str> {
    match code? {
        "non_fait" => Some("Appel non fait"),
        "non_clos" => Some("Appel non clos"),
        "absent_puis_present" => Some("Absent puis présent"),
        "present_malgre_declaration" => Some("Présent malgré une absence annoncée"),
        "present_alors_quen_sortie" => Some("Présent alors qu'en sortie"),
        _ => None,
    }
}

/// Titre d'une ligne : le nom de l'élève, ou, pour un appel non fait (aucun élève), la classe de la séance.
pub fn titre_ligne(e: &EntreeFileDto) -> String {
    if e.eleve_id.is_none() {
        match e.classe_seance.as_deref() {
            Some(c) => format!("Séance de {c}"),
            None => "Séance".to_string(),
        }
    } else {
        nom_complet(e.prenom.as_deref(), e.nom.as_deref())
    }
}

/// Phrase complète d'une ligne, lue par le lecteur d'écran (la couleur ne porte jamais seule l'information).
pub fn phrase_ligne(e: &EntreeFileDto) -> String {
    let raison =
        libelle_detail(e.detail_code.as_deref()).unwrap_or_else(|| libelle_categorie(&e.categorie));
    let mut p = vec![titre_ligne(e), raison.to_string()];
    if let Some(c) = e.classe.as_deref() {
        p.push(format!("classe {c}"));
    }
    if e.vigilance {
        p.push("sous vigilance".into());
    }
    match e.mineur {
        Some(true) => p.push("mineur".into()),
        Some(false) => p.push("majeur".into()),
        None => {}
    }
    if let Some(d) = depuis(e.depuis, e.minutes_depuis) {
        p.push(d);
    }
    p.join(", ")
}

/// Une ligne verrouillée par « Je prends » (verrou visible encore valide) : le serveur ne renvoie le verrou que
/// s'il est valide ; l'écran n'a donc qu'à le dire.
pub fn est_prise(e: &EntreeFileDto) -> bool {
    e.pris_par.is_some() && e.pris_jusqua.is_some()
}

/// Nombre de lignes montrées d'emblée (le reste derrière « Voir tout »).
pub fn lignes_visibles(grand: bool) -> usize {
    if grand {
        8
    } else {
        5
    }
}

/// Texte du compteur d'un panneau : « 3 à traiter » / « Rien à traiter ».
pub fn libelle_compteur(n: usize) -> String {
    match n {
        0 => "Rien à traiter".to_string(),
        1 => "1 à traiter".to_string(),
        n => format!("{n} à traiter"),
    }
}

// ─── Issues codées proposées dans le bandeau ──────────────────────────────────────────────

/// Issues d'une alerte (liste fermée du serveur, `CLOTURES_ALERTE`), libellé en français.
pub const ISSUES_ALERTE: [(&str, &str); 6] = [
    ("localise", "Élève localisé"),
    ("arrive", "Élève arrivé"),
    ("famille_jointe", "Famille jointe"),
    ("famille_deja_informee", "Famille déjà informée"),
    ("sans_suite", "Sans suite"),
    ("fausse_alerte", "Fausse alerte"),
];

/// Raisons de « prévenir maintenant » (liste fermée du serveur, `RAISONS_PREVENIR`).
pub const RAISONS_PREVENIR: [(&str, &str); 5] = [
    ("fugue_suspectee", "Fugue suspectée"),
    ("eleve_en_danger", "Élève en danger"),
    ("absence_inexpliquee_grave", "Absence inexpliquée grave"),
    ("demande_de_la_famille", "Demande de la famille"),
    ("autre_urgence", "Autre urgence"),
];

/// Raisons de « faire l'appel à la place » (liste fermée du serveur, `RAISONS_A_LA_PLACE`).
pub const RAISONS_A_LA_PLACE: [(&str, &str); 5] = [
    ("oubli", "Oubli de l'intervenant"),
    ("intervenant_injoignable", "Intervenant injoignable"),
    ("intervenant_absent", "Intervenant absent"),
    ("panne", "Panne"),
    ("autre", "Autre raison"),
];

/// Issues de contact tracées en un geste (liste fermée du serveur, `ISSUES_CONTACT`, canal téléphone).
pub const ISSUES_CONTACT: [(&str, &str); 3] = [
    ("joint", "Famille jointe"),
    ("message_laisse", "Message laissé"),
    ("ne_repond_pas", "Ne répond pas"),
];

// ─── Tests ────────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn h(a: u32, b: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(a, b, 0).unwrap()
    }

    fn entree(
        cat: &str,
        nom: &str,
        depuis: Option<NaiveTime>,
        vigilance: bool,
        mineur: bool,
    ) -> EntreeFileDto {
        EntreeFileDto {
            cle: format!("{cat}:{nom}"),
            categorie: cat.into(),
            raisons: vec![],
            niveau: match cat {
                "urgence" => 1,
                "non_arrivee" => 2,
                "apres_presence" => 3,
                "non_jointe" => 5,
                "sans_nouvelles" if vigilance => 4,
                "sans_nouvelles" if mineur => 6,
                "sans_nouvelles" => 7,
                "appel_non_fait" => 8,
                _ => 9,
            },
            rang: 0,
            eleve_id: Some(Uuid::nil()),
            nom: Some(nom.into()),
            prenom: Some("Prénom".into()),
            classe: Some("4B".into()),
            mineur: Some(mineur),
            vigilance,
            depuis,
            minutes_depuis: None,
            nb_seances: None,
            absence_id: None,
            alerte_id: None,
            creneau_id: None,
            classe_seance: None,
            intervenant_user_id: None,
            detail_code: None,
            contacts_joints: 0,
            contacts_sans_succes: 0,
            pris_par: None,
            pris_jusqua: None,
        }
    }

    fn file(entrees: Vec<EntreeFileDto>) -> FileDuMatinDto {
        FileDuMatinDto {
            jour: NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
            heure_locale: h(9, 0),
            entrees,
            compteurs: BTreeMap::new(),
        }
    }

    fn exclu(nom: &str, minutes: i64) -> ExcluDuJourDto {
        ExcluDuJourDto {
            appel_id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            nom: nom.into(),
            prenom: "X".into(),
            classe: None,
            mineur: true,
            depuis: h(8, 0),
            minutes_depuis: minutes,
            urgent: minutes >= 10,
            creneau_id: None,
        }
    }

    fn noms(v: &[EntreeFileDto]) -> Vec<&str> {
        v.iter().map(|e| e.nom.as_deref().unwrap()).collect()
    }

    #[test]
    fn le_bandeau_urgent_regroupe_les_trois_niveaux_ou_l_on_ne_sait_pas_ou_est_l_eleve() {
        let f = file(vec![
            entree("urgence", "U", Some(h(8, 0)), false, true),
            entree("non_arrivee", "N", Some(h(8, 5)), false, true),
            entree("apres_presence", "P", Some(h(8, 10)), false, true),
            entree("sans_nouvelles", "S", Some(h(8, 0)), false, true),
            entree("non_jointe", "J", Some(h(8, 0)), false, true),
            entree("appel_non_fait", "A", Some(h(9, 0)), false, false),
            entree("incoherence", "I", Some(h(9, 0)), false, false),
        ]);
        let r = regrouper(&f, None);
        assert_eq!(
            noms(&r.urgent),
            ["U", "N", "P"],
            "niveaux 1, 2, 3 de la file, dans cet ordre"
        );
        assert_eq!(noms(&r.absents), ["S"]);
        assert_eq!(noms(&r.familles), ["J"]);
        assert_eq!(noms(&r.appels), ["A"]);
        assert_eq!(noms(&r.incoherences), ["I"]);
        assert_eq!(r.ignorees, 0);
    }

    #[test]
    fn une_categorie_inconnue_n_est_rangee_nulle_part_et_est_comptee() {
        let r = regrouper(
            &file(vec![entree("nouveaute", "Z", None, false, false)]),
            None,
        );
        assert_eq!(r.ignorees, 1);
        assert!(r.urgent.is_empty() && r.absents.is_empty() && r.appels.is_empty());
        assert_eq!(panneau_de("nouveaute"), None);
        assert!(
            !est_urgent("nouveaute"),
            "dans le doute, ce n'est pas un faux urgent"
        );
    }

    #[test]
    fn absents_sous_vigilance_d_abord_puis_mineurs_avant_majeurs_puis_le_plus_ancien() {
        let f = file(vec![
            entree("sans_nouvelles", "Major_tot", Some(h(8, 0)), false, false),
            entree("sans_nouvelles", "Mineur_tard", Some(h(9, 30)), false, true),
            entree("sans_nouvelles", "Mineur_tot", Some(h(8, 15)), false, true),
            entree("sans_nouvelles", "Vigil_tard", Some(h(10, 0)), true, false),
        ]);
        let r = regrouper(&f, None);
        assert_eq!(
            noms(&r.absents),
            ["Vigil_tard", "Mineur_tot", "Mineur_tard", "Major_tot"]
        );
    }

    #[test]
    fn familles_appels_et_incoherences_le_plus_ancien_en_premier_sans_heure_a_la_fin() {
        let f = file(vec![
            entree("non_jointe", "C", None, false, true),
            entree("non_jointe", "B", Some(h(9, 0)), false, true),
            entree("non_jointe", "A", Some(h(8, 0)), false, true),
        ]);
        assert_eq!(noms(&regrouper(&f, None).familles), ["A", "B", "C"]);
    }

    #[test]
    fn exclus_le_plus_ancien_en_premier_et_aucune_donnee_inventee_sans_source() {
        let x = ExclusDuJourDto {
            heure_locale: h(10, 0),
            exclus: vec![exclu("Récent", 3), exclu("Ancien", 25)],
        };
        let r = regrouper(&file(vec![]), Some(&x));
        assert_eq!(
            r.exclus.iter().map(|e| e.nom.as_str()).collect::<Vec<_>>(),
            ["Ancien", "Récent"]
        );
        assert!(regrouper(&file(vec![]), None).exclus.is_empty());
        assert_eq!(r.compte(Panneau::Exclus), 2);
    }

    #[test]
    fn bascule_a_onze_heures_avant_et_apres() {
        let b = h(11, 0);
        assert_eq!(moment(h(8, 0), b), Moment::AvantBascule);
        assert_eq!(
            moment(NaiveTime::from_hms_opt(10, 59, 59).unwrap(), b),
            Moment::AvantBascule
        );
        assert_eq!(
            moment(h(11, 0), b),
            Moment::ApresBascule,
            "à l'heure exacte, on est déjà après"
        );
        assert_eq!(moment(h(14, 30), b), Moment::ApresBascule);
        // Réglable : un établissement qui bascule à 10 h.
        assert_eq!(moment(h(10, 30), h(10, 0)), Moment::ApresBascule);
        assert_eq!(moment(h(10, 30), h(11, 30)), Moment::AvantBascule);
    }

    #[test]
    fn disposition_automatique_le_matin_absents_en_premier_et_en_grand() {
        let d = disposition(Moment::AvantBascule, None);
        assert_eq!(d[0].panneau, Panneau::Absents);
        assert!(d[0].grand && d.iter().skip(1).all(|x| !x.grand));
        assert_eq!(
            d.len(),
            5,
            "tous les panneaux restent visibles, seul l'ordre change"
        );
        assert!(d.iter().all(|x| !x.masque && !x.replie));
    }

    #[test]
    fn disposition_automatique_l_apres_midi_exclus_et_appels_en_premier_et_en_grand() {
        let d = disposition(Moment::ApresBascule, None);
        assert_eq!(
            (d[0].panneau, d[1].panneau),
            (Panneau::Exclus, Panneau::Appels)
        );
        assert!(d[0].grand && d[1].grand && d.iter().skip(2).all(|x| !x.grand));
        assert_eq!(d.len(), 5);
        // Chaque panneau y est une seule fois.
        for p in Panneau::TOUS {
            assert_eq!(d.iter().filter(|x| x.panneau == p).count(), 1);
        }
    }

    fn epinglee() -> Vec<PreferencePanneauDto> {
        ["incoherences", "appels", "absents", "exclus", "familles"]
            .iter()
            .map(|c| PreferencePanneauDto {
                panneau: (*c).into(),
                replie: *c == "appels",
                masque: *c == "familles",
            })
            .collect()
    }

    #[test]
    fn la_disposition_epinglee_remplace_l_automatique_a_toute_heure() {
        let p = epinglee();
        for m in [Moment::AvantBascule, Moment::ApresBascule] {
            let d = disposition(m, Some(&p));
            assert_eq!(
                d.iter().map(|x| x.panneau.code()).collect::<Vec<_>>(),
                ["incoherences", "appels", "absents", "exclus", "familles"]
            );
            assert!(d[1].replie && d[4].masque && !d[0].replie);
            assert!(
                d.iter().all(|x| !x.grand),
                "aucun « grand » dans une disposition choisie"
            );
        }
        assert!(est_epinglee(Some(&p)) && !est_epinglee(None));
    }

    #[test]
    fn une_preference_inutilisable_retombe_sur_l_automatique() {
        let mut p = epinglee();
        p.pop();
        assert!(!preference_utilisable(&p) && !est_epinglee(Some(&p)));
        assert_eq!(
            disposition(Moment::AvantBascule, Some(&p)),
            disposition_auto(Moment::AvantBascule)
        );
        let mut doublon = epinglee();
        doublon[4].panneau = "absents".into();
        assert!(!preference_utilisable(&doublon));
        let mut inconnu = epinglee();
        inconnu[0].panneau = "urgent".into();
        assert!(
            !preference_utilisable(&inconnu),
            "le bandeau Urgent n'est pas un panneau"
        );
    }

    #[test]
    fn deplacer_replier_masquer_et_envoi() {
        let d = disposition_auto(Moment::AvantBascule);
        let d2 = deplacer(d.clone(), 0, 1);
        assert_eq!((d2[0].panneau, d2[1].panneau), (d[1].panneau, d[0].panneau));
        assert_eq!(deplacer(d.clone(), 0, -1), d, "sans effet en haut");
        assert_eq!(deplacer(d.clone(), 4, 1), d, "sans effet en bas");
        assert_eq!(deplacer(d.clone(), 9, 1), d, "indice hors liste");
        let d3 = basculer_repli(d.clone(), Panneau::Appels);
        assert!(
            d3.iter()
                .find(|x| x.panneau == Panneau::Appels)
                .unwrap()
                .replie
        );
        let d4 = basculer_masque(d3, Panneau::Appels);
        let a = d4.iter().find(|x| x.panneau == Panneau::Appels).unwrap();
        assert!(a.masque && !a.replie);
        assert!(!tout_masque(&d4));
        let tous = Panneau::TOUS.into_iter().fold(d, basculer_masque);
        assert!(tout_masque(&tous));
        // Ce qui part au serveur : cinq panneaux, ordre conservé, valide côté serveur.
        let envoi = vers_preference(&tous);
        assert!(preference_utilisable(&envoi) && envoi.iter().all(|x| x.masque));
        assert_eq!(envoi[0].panneau, tous[0].panneau.code());
    }

    #[test]
    fn appliquer_modifications_et_epinglage_depuis_l_automatique() {
        // Épingler depuis l'automatique du matin : l'ordre affiché devient l'ordre choisi, tous panneaux listés.
        let auto = disposition_auto(Moment::AvantBascule);
        let apres = appliquer(auto.clone(), Modif::Descendre(0));
        assert_eq!(apres[1].panneau, auto[0].panneau);
        assert_eq!(appliquer(apres.clone(), Modif::Monter(1)), auto);
        assert!(appliquer(auto.clone(), Modif::Repli(Panneau::Exclus))
            .iter()
            .any(|x| x.replie));
        let envoi = vers_preference(&apres);
        assert!(
            preference_utilisable(&envoi),
            "ce qui part au serveur est accepté par le serveur"
        );
        // Une fois épinglée, la disposition ne bouge plus avec l'heure.
        assert_eq!(
            disposition(Moment::ApresBascule, Some(&envoi))
                .iter()
                .map(|x| x.panneau)
                .collect::<Vec<_>>(),
            apres.iter().map(|x| x.panneau).collect::<Vec<_>>()
        );
    }

    #[test]
    fn phrase_lisible_sans_couleur_ni_code_brut() {
        let mut e = entree("sans_nouvelles", "Martin", Some(h(8, 5)), true, true);
        e.prenom = Some("Léa".into());
        e.minutes_depuis = Some(80);
        let p = phrase_ligne(&e);
        assert_eq!(
            p,
            "Léa Martin, Sans nouvelles, classe 4B, sous vigilance, mineur, depuis 08:05 (1 h 20)"
        );
        assert!(!p.contains("sans_nouvelles"));
        assert_eq!(depuis(None, Some(5)), None);
        assert_eq!(
            depuis(Some(h(9, 0)), Some(7)).as_deref(),
            Some("depuis 09:00 (7 min)")
        );
        assert_eq!(nom_complet(None, None), "Élève");
        // Un appel non fait n'a pas d'élève : on dit la séance.
        let mut a = entree("appel_non_fait", "", Some(h(8, 0)), false, false);
        a.eleve_id = None;
        a.nom = None;
        a.prenom = None;
        a.mineur = None;
        a.classe = None;
        a.classe_seance = Some("5A".into());
        a.detail_code = Some("non_fait".into());
        assert_eq!(titre_ligne(&a), "Séance de 5A");
        assert_eq!(
            phrase_ligne(&a),
            "Séance de 5A, Appel non fait, depuis 08:00"
        );
    }

    #[test]
    fn compteur_et_pagination() {
        assert_eq!(libelle_compteur(0), "Rien à traiter");
        assert_eq!(libelle_compteur(1), "1 à traiter");
        assert_eq!(libelle_compteur(12), "12 à traiter");
        assert!(lignes_visibles(true) > lignes_visibles(false));
    }

    #[test]
    fn listes_fermees_alignees_sur_le_serveur() {
        // Copie des listes d'`ent-db` : un écart serait un 400 pour l'utilisateur.
        let issues = [
            "localise",
            "arrive",
            "famille_jointe",
            "famille_deja_informee",
            "sans_suite",
            "fausse_alerte",
        ];
        assert_eq!(ISSUES_ALERTE.map(|(c, _)| c), issues);
        let raisons = [
            "fugue_suspectee",
            "eleve_en_danger",
            "absence_inexpliquee_grave",
            "demande_de_la_famille",
            "autre_urgence",
        ];
        assert_eq!(RAISONS_PREVENIR.map(|(c, _)| c), raisons);
        assert_eq!(
            RAISONS_A_LA_PLACE.map(|(c, _)| c),
            [
                "oubli",
                "intervenant_injoignable",
                "intervenant_absent",
                "panne",
                "autre"
            ]
        );
        assert_eq!(
            ISSUES_CONTACT.map(|(c, _)| c),
            ["joint", "message_laisse", "ne_repond_pas"]
        );
        for c in [
            "urgence",
            "non_arrivee",
            "apres_presence",
            "non_jointe",
            "sans_nouvelles",
            "appel_non_fait",
            "incoherence",
        ] {
            assert_ne!(libelle_categorie(c), "À vérifier", "{c} a un libellé");
        }
    }
}
