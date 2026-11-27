//! Logique de l'écran « Absences et remplacements » (pure, sans Leptos ni réseau, testée en
//! natif) : période, libellés d'état, remplaçants proposés, textes de confirmation, saisie de
//! l'annulation en masse (aperçu obligatoire, invalidé par toute modification) et clé
//! d'idempotence. Le serveur reste l'autorité (droits, conflits, disponibilités).

use chrono::{NaiveDate, NaiveTime};
use ent_types::edt_s2::{
    AbsenceSeancesDto, CandidatDto, ClasseToucheeDto, CmdAnnulationMasseDto, CmdRemplacementDto,
    RapportMasseDto, SeanceToucheeDto,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::noyau::temps::{date_complete, date_longue};

/// Erreurs de saisie : (identifiant du champ, message), pour `ResumeErreurs`.
pub type Erreurs = Vec<(String, String)>;

/// Plage maximale des routes `/edt/absences` et `/edt/annulation-masse` (écart en jours).
pub const JOURS_MAX: i64 = 62;

/// Longueur maximale du motif public (caractères), comme le serveur.
pub const MOTIF_PUBLIC_MAX: usize = 200;

/// Rappel affiché avant tout remplacement ou annulation.
pub const RAPPEL_PREVENUS: &str =
    "Les élèves et leurs familles seront prévenus, sans indication de motif.";

// ───────────────────────────── Période ─────────────────────────────

/// Période par défaut : du jour aux 13 jours suivants (deux semaines).
pub fn periode_par_defaut(aujourdhui: NaiveDate) -> (NaiveDate, NaiveDate) {
    (
        aujourdhui,
        aujourdhui
            .checked_add_signed(chrono::Duration::days(13))
            .unwrap_or(aujourdhui),
    )
}

fn lire_date(texte: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texte.trim(), "%Y-%m-%d").ok()
}

fn lire_heure(texte: &str) -> Option<NaiveTime> {
    let t = texte.trim();
    NaiveTime::parse_from_str(t, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(t, "%H:%M:%S"))
        .ok()
}

/// Période saisie : dates lisibles, fin après le début, 62 jours au plus.
pub fn valider_periode(du: &str, au: &str) -> Result<(NaiveDate, NaiveDate), String> {
    let (Some(d), Some(a)) = (lire_date(du), lire_date(au)) else {
        return Err("Indiquez une date de début et une date de fin.".into());
    };
    if a < d {
        return Err("La date de fin doit suivre la date de début.".into());
    }
    if (a - d).num_days() > JOURS_MAX {
        return Err(format!(
            "La période ne peut pas dépasser {JOURS_MAX} jours."
        ));
    }
    Ok((d, a))
}

// ───────────────────────────── Absences ─────────────────────────────

pub fn libelle_statut_absence(statut: &str) -> &'static str {
    match statut {
        "validee" => "Validée",
        "signalee" => "Signalée, à valider",
        _ => "Statut inconnu",
    }
}

pub fn ton_statut_absence(statut: &str) -> TonEtat {
    match statut {
        "validee" => TonEtat::Ok,
        "signalee" => TonEtat::Avertissement,
        _ => TonEtat::Neutre,
    }
}

/// Dates d'une absence : « Le lundi 9 novembre » ou « Du lundi 9 novembre au mercredi 11 novembre ».
pub fn libelle_dates(debut: NaiveDate, fin: NaiveDate) -> String {
    if debut == fin {
        format!("Le {}", date_longue(debut).to_lowercase())
    } else {
        format!(
            "Du {} au {}",
            date_longue(debut).to_lowercase(),
            date_longue(fin).to_lowercase()
        )
    }
}

/// Ce qu'il reste à faire pour une absence.
pub fn resume_traitement(a: &AbsenceSeancesDto) -> String {
    if a.absence.statut != "validee" {
        return "Les séances sont proposées au traitement dès que l'absence est validée.".into();
    }
    match (a.seances.len(), a.a_traiter) {
        (0, _) => "Aucune séance touchée sur la période.".into(),
        (_, 0) => "Toutes les séances sont traitées.".into(),
        (_, 1) => "1 séance à traiter.".into(),
        (_, n) => format!("{n} séances à traiter."),
    }
}

/// Nombre total de séances à traiter sur l'ensemble des absences (pour l'annonce).
pub fn total_a_traiter(absences: &[AbsenceSeancesDto]) -> usize {
    absences.iter().map(|a| a.a_traiter).sum()
}

// ───────────────────────────── Séances touchées ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatSeance {
    ATraiter,
    Annulee,
    Remplacee,
    Deplacee,
    Inconnu,
}

impl EtatSeance {
    pub fn depuis(statut: &str) -> EtatSeance {
        match statut {
            "prevue" => EtatSeance::ATraiter,
            "annulee" => EtatSeance::Annulee,
            "remplacee" => EtatSeance::Remplacee,
            "deplacee" => EtatSeance::Deplacee,
            _ => EtatSeance::Inconnu,
        }
    }

    pub fn ton(self) -> TonEtat {
        match self {
            EtatSeance::ATraiter => TonEtat::Avertissement,
            EtatSeance::Annulee => TonEtat::Mal,
            EtatSeance::Remplacee | EtatSeance::Deplacee => TonEtat::Ok,
            EtatSeance::Inconnu => TonEtat::Neutre,
        }
    }
}

/// État en toutes lettres (jamais la couleur seule).
pub fn libelle_etat_seance(s: &SeanceToucheeDto) -> String {
    match EtatSeance::depuis(&s.statut) {
        EtatSeance::ATraiter => "À traiter".into(),
        EtatSeance::Annulee => "Cours annulé".into(),
        EtatSeance::Remplacee => {
            match s
                .remplacant_nom
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty())
            {
                Some(nom) => format!("Remplacé par {nom}"),
                None => "Remplaçant désigné".into(),
            }
        }
        EtatSeance::Deplacee => "Cours déplacé".into(),
        EtatSeance::Inconnu => "État inconnu".into(),
    }
}

/// Seule une séance « prévue » se traite (remplacement ou annulation).
pub fn peut_traiter(s: &SeanceToucheeDto) -> bool {
    EtatSeance::depuis(&s.statut) == EtatSeance::ATraiter
}

/// Une annulation, un remplacement ou un déplacement peut être levé (« Rétablir »).
pub fn peut_retablir(s: &SeanceToucheeDto) -> bool {
    matches!(
        EtatSeance::depuis(&s.statut),
        EtatSeance::Annulee | EtatSeance::Remplacee | EtatSeance::Deplacee
    )
}

/// Clé stable d'une séance touchée (liste et sélection).
pub fn cle_seance(s: &SeanceToucheeDto) -> String {
    format!("{}|{}", s.creneau_id, s.jour)
}

/// « Lundi 9 novembre · 08:00 – 09:00 ».
pub fn libelle_horaire(s: &SeanceToucheeDto) -> String {
    format!(
        "{} · {} – {}",
        date_longue(s.jour),
        s.heure_debut.format("%H:%M"),
        s.heure_fin.format("%H:%M")
    )
}

/// « Maths · 4B · Salle B12 ».
pub fn libelle_cours(s: &SeanceToucheeDto) -> String {
    let mut parties = vec![
        s.matiere_libelle.trim().to_string(),
        s.classe.trim().to_string(),
    ];
    if let Some(salle) = s.salle.as_deref().map(str::trim).filter(|x| !x.is_empty()) {
        parties.push(format!("Salle {salle}"));
    }
    parties.retain(|p| !p.is_empty());
    parties.join(" · ")
}

/// Phrase décrivant la séance dans une confirmation : « Maths avec la classe 4B, le lundi 9
/// novembre de 08:00 à 09:00 ».
pub fn phrase_seance(s: &SeanceToucheeDto) -> String {
    format!(
        "{} avec la classe {}, le {} de {} à {}",
        s.matiere_libelle.trim(),
        s.classe.trim(),
        date_longue(s.jour).to_lowercase(),
        s.heure_debut.format("%H:%M"),
        s.heure_fin.format("%H:%M")
    )
}

// ───────────────────────────── Remplaçants ─────────────────────────────

/// Candidats dans l'ordre d'affichage : compétents d'abord, puis moins chargés, puis par nom.
pub fn trier_candidats(mut candidats: Vec<CandidatDto>) -> Vec<CandidatDto> {
    candidats.sort_by(|a, b| {
        (
            !a.competent,
            a.seances_ce_jour,
            a.nom.clone().unwrap_or_default(),
        )
            .cmp(&(
                !b.competent,
                b.seances_ce_jour,
                b.nom.clone().unwrap_or_default(),
            ))
    });
    candidats
}

pub fn nom_candidat(c: &CandidatDto) -> String {
    c.nom
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("Nom non renseigné")
        .to_string()
}

/// « Mme Chourave — enseigne cette matière, 2 séances ce jour-là ».
pub fn libelle_candidat(c: &CandidatDto) -> String {
    let competence = if c.competent {
        "enseigne cette matière"
    } else {
        "n'enseigne pas cette matière"
    };
    let charge = match c.seances_ce_jour {
        0 => "libre ce jour-là".to_string(),
        1 => "1 séance ce jour-là".to_string(),
        n => format!("{n} séances ce jour-là"),
    };
    format!("{} — {competence}, {charge}", nom_candidat(c))
}

/// Options de la liste de choix : (identifiant, libellé).
pub fn options_candidats(candidats: &[CandidatDto]) -> Vec<(String, String)> {
    candidats
        .iter()
        .map(|c| (c.user_id.to_string(), libelle_candidat(c)))
        .collect()
}

/// Corps de `POST /edt/remplacements` ; `None` si le choix est illisible.
pub fn cmd_remplacement(
    absence: Uuid,
    seance: &SeanceToucheeDto,
    choix: &str,
) -> Option<CmdRemplacementDto> {
    Some(CmdRemplacementDto {
        creneau_id: seance.creneau_id,
        jour: seance.jour,
        remplacant_user_id: choix.trim().parse().ok()?,
        absence_rh_id: Some(absence),
        nouvelle_salle: None,
    })
}

pub fn message_remplacement(s: &SeanceToucheeDto, remplacant: &str) -> String {
    format!("Confier à {remplacant} le cours de {} ?", phrase_seance(s))
}

pub fn message_annulation(s: &SeanceToucheeDto) -> String {
    format!("Annuler le cours de {} ?", phrase_seance(s))
}

pub fn message_retablissement(s: &SeanceToucheeDto) -> String {
    format!(
        "Rétablir le cours de {} ? Il reprend sa place normale au programme.",
        phrase_seance(s)
    )
}

// ───────────────────────────── Annulation en masse ─────────────────────────────

pub const PORTEE_ETABLISSEMENT: &str = "etablissement";
pub const PORTEE_CLASSES: &str = "classes";

/// Champs saisis de l'annulation en masse.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieMasse {
    pub du: String,
    pub au: String,
    /// `etablissement` ou `classes`.
    pub portee: String,
    pub classes: Vec<Uuid>,
    /// Facultatif (« HH:MM »).
    pub heure_debut: String,
    pub heure_fin: String,
    /// Motif **public** : il sera lu par les élèves et les familles.
    pub motif: String,
}

fn erreur(champ: &str, message: &str) -> (String, String) {
    (champ.to_string(), message.to_string())
}

/// Contrôle local (confort : le serveur revérifie tout) et composition du corps. L'aperçu n'a
/// pas de clé ; l'application porte la clé d'idempotence dérivée des paramètres.
pub fn composer_masse(s: &SaisieMasse, apercu: bool) -> Result<CmdAnnulationMasseDto, Erreurs> {
    let mut erreurs: Erreurs = Vec::new();
    let du = lire_date(&s.du);
    let au = lire_date(&s.au);
    match (du, au) {
        (Some(d), Some(a)) if a < d => {
            erreurs.push(erreur(
                "rpl-masse-au",
                "La date de fin doit suivre la date de début.",
            ));
        }
        (Some(d), Some(a)) if (a - d).num_days() > JOURS_MAX => {
            erreurs.push(erreur(
                "rpl-masse-au",
                &format!("La période ne peut pas dépasser {JOURS_MAX} jours."),
            ));
        }
        (Some(_), Some(_)) => {}
        _ => erreurs.push(erreur(
            "rpl-masse-du",
            "Indiquez une date de début et une date de fin.",
        )),
    }
    let portee = s.portee.trim();
    let cibles = match portee {
        PORTEE_ETABLISSEMENT => Vec::new(),
        PORTEE_CLASSES => {
            if s.classes.is_empty() {
                erreurs.push(erreur(
                    "rpl-masse-classes",
                    "Choisissez au moins une classe.",
                ));
            }
            let mut c = s.classes.clone();
            c.sort();
            c.dedup();
            c
        }
        _ => {
            erreurs.push(erreur(
                "rpl-masse-portee",
                "Choisissez ce qui est concerné.",
            ));
            Vec::new()
        }
    };
    let lire_facultative = |champ: &str, texte: &str, erreurs: &mut Erreurs| {
        if texte.trim().is_empty() {
            None
        } else {
            let h = lire_heure(texte);
            if h.is_none() {
                erreurs.push(erreur(champ, "Heure illisible (format HH:MM)."));
            }
            h
        }
    };
    let heure_debut = lire_facultative("rpl-masse-hd", &s.heure_debut, &mut erreurs);
    let heure_fin = lire_facultative("rpl-masse-hf", &s.heure_fin, &mut erreurs);
    if let (Some(d), Some(f)) = (heure_debut, heure_fin) {
        if f <= d {
            erreurs.push(erreur(
                "rpl-masse-hf",
                "L'heure de fin doit suivre l'heure de début.",
            ));
        }
    }
    let motif = s.motif.trim();
    if motif.is_empty() {
        erreurs.push(erreur(
            "rpl-masse-motif",
            "Indiquez le motif public, lu par les familles.",
        ));
    } else if motif.chars().count() > MOTIF_PUBLIC_MAX {
        erreurs.push(erreur(
            "rpl-masse-motif",
            &format!("Le motif ne peut pas dépasser {MOTIF_PUBLIC_MAX} caractères."),
        ));
    }
    let (Some(du), Some(au), true) = (du, au, erreurs.is_empty()) else {
        return Err(erreurs);
    };
    let mut cmd = CmdAnnulationMasseDto {
        du,
        au,
        portee: portee.to_string(),
        cibles,
        heure_debut,
        heure_fin,
        motif_public: motif.to_string(),
        cle: None,
        apercu,
    };
    if !apercu {
        cmd.cle = Some(cle_idempotence(&cmd));
    }
    Ok(cmd)
}

fn fnv1a(octets: &[u8], graine: u64) -> u64 {
    let mut h = graine;
    for o in octets {
        h ^= u64::from(*o);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Description canonique des paramètres : `du|au|portée|cibles triées|début|fin|motif`.
pub fn description_canonique(cmd: &CmdAnnulationMasseDto) -> String {
    let mut cibles: Vec<String> = cmd.cibles.iter().map(Uuid::to_string).collect();
    cibles.sort();
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        cmd.du,
        cmd.au,
        cmd.portee,
        cibles.join(","),
        cmd.heure_debut
            .map(|h| h.format("%H:%M").to_string())
            .unwrap_or_default(),
        cmd.heure_fin
            .map(|h| h.format("%H:%M").to_string())
            .unwrap_or_default(),
        cmd.motif_public.trim()
    )
}

/// Clé d'idempotence **déterministe** dérivée des paramètres (le serveur limite la clé à 80
/// caractères : on envoie une empreinte de 128 bits, non cryptographique, préfixée `web-`).
/// Envoyer deux fois les mêmes paramètres ne crée pas un second lot.
pub fn cle_idempotence(cmd: &CmdAnnulationMasseDto) -> String {
    let canon = description_canonique(cmd);
    let a = fnv1a(canon.as_bytes(), 0xcbf2_9ce4_8422_2325);
    let b = fnv1a(canon.as_bytes(), 0x9e37_79b9_7f4a_7c15);
    format!("web-{a:016x}{b:016x}")
}

/// Signature de la saisie : l'aperçu n'est valable que pour la saisie qui l'a produit.
/// Toute modification (dates, portée, classes, heures, motif) change la signature.
pub fn signature(s: &SaisieMasse) -> String {
    match composer_masse(s, true) {
        Ok(cmd) => description_canonique(&cmd),
        Err(_) => String::new(),
    }
}

/// Peut-on confirmer ? Seulement si un aperçu existe pour exactement la saisie actuelle.
pub fn apercu_valide(signature_apercu: Option<&str>, s: &SaisieMasse) -> bool {
    match signature_apercu {
        Some(sig) if !sig.is_empty() => sig == signature(s),
        _ => false,
    }
}

/// Résumé du rapport d'aperçu ou d'application.
pub fn resume_rapport(r: &RapportMasseDto) -> String {
    let seances = match r.nb_seances {
        0 => "aucune séance".to_string(),
        1 => "1 séance".to_string(),
        n => format!("{n} séances"),
    };
    let classes = match r.nb_classes {
        0 => "aucune classe".to_string(),
        1 => "1 classe".to_string(),
        n => format!("{n} classes"),
    };
    let mut texte = format!("{seances} dans {classes}");
    if r.deja_annulees > 0 {
        texte.push_str(&format!(
            " ; {} déjà annulée(s) ou déplacée(s), ignorée(s)",
            r.deja_annulees
        ));
    }
    texte.push('.');
    texte
}

/// « 4B : 12 séances ».
pub fn libelle_classe_touchee(c: &ClasseToucheeDto) -> String {
    match c.nb_seances {
        1 => format!("{} : 1 séance", c.classe),
        n => format!("{} : {n} séances", c.classe),
    }
}

/// Message de l'application d'un lot.
pub fn message_application(r: &RapportMasseDto) -> String {
    if r.deja_applique {
        "Cette annulation en masse avait déjà été enregistrée : rien n'a été ajouté.".into()
    } else {
        format!(
            "Annulation enregistrée : {} Les élèves et les familles sont prévenus.",
            resume_rapport(r)
        )
    }
}

/// Phrase de confirmation de l'annulation en masse.
pub fn message_confirmation_masse(cmd: &CmdAnnulationMasseDto, r: &RapportMasseDto) -> String {
    format!(
        "Annuler {} du {} au {} inclus ({}) ?",
        if cmd.portee == PORTEE_ETABLISSEMENT {
            "les cours de tout l'établissement".to_string()
        } else {
            "les cours des classes choisies".to_string()
        },
        date_complete(cmd.du).to_lowercase(),
        date_complete(cmd.au).to_lowercase(),
        resume_rapport(r).trim_end_matches('.')
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_types::edt_s2::AbsenceVueDto;

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn h(heure: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(heure, 0, 0).unwrap()
    }

    fn touchee(statut: &str) -> SeanceToucheeDto {
        SeanceToucheeDto {
            creneau_id: Uuid::from_u128(1),
            jour: d(2026, 11, 9),
            classe: "4B".into(),
            classe_id: Uuid::from_u128(2),
            groupe_id: None,
            matiere_libelle: "Maths".into(),
            heure_debut: h(8),
            heure_fin: h(9),
            salle: Some("B12".into()),
            statut: statut.into(),
            role_absent: "titulaire".into(),
            remplacant_user_id: None,
            remplacant_nom: None,
        }
    }

    fn absence(statut: &str, seances: Vec<SeanceToucheeDto>) -> AbsenceSeancesDto {
        let a_traiter = seances.iter().filter(|s| s.statut == "prevue").count();
        AbsenceSeancesDto {
            absence: AbsenceVueDto {
                id: Uuid::from_u128(9),
                user_id: Uuid::from_u128(10),
                nom: Some("Mme Chourave".into()),
                debut: d(2026, 11, 9),
                fin: d(2026, 11, 10),
                statut: statut.into(),
            },
            seances,
            a_traiter,
            du: d(2026, 11, 9),
            au: d(2026, 11, 10),
        }
    }

    fn saisie() -> SaisieMasse {
        SaisieMasse {
            du: "2026-11-12".into(),
            au: "2026-11-13".into(),
            portee: PORTEE_ETABLISSEMENT.into(),
            classes: vec![],
            heure_debut: String::new(),
            heure_fin: String::new(),
            motif: "Grève des transports".into(),
        }
    }

    #[test]
    fn periode() {
        assert_eq!(
            periode_par_defaut(d(2026, 11, 9)),
            (d(2026, 11, 9), d(2026, 11, 22))
        );
        assert!(valider_periode("2026-11-09", "2026-11-22").is_ok());
        assert!(valider_periode("2026-11-22", "2026-11-09").is_err());
        assert!(
            valider_periode("2026-11-01", "2027-01-03").is_err(),
            "plus de 62 jours"
        );
        assert!(valider_periode("2026-11-01", "2027-01-03")
            .unwrap_err()
            .contains("62"));
        assert!(
            valider_periode("2026-11-01", "2027-01-02").is_ok(),
            "exactement 62 jours"
        );
        assert!(valider_periode("", "").is_err());
    }

    #[test]
    fn libelles_d_absence() {
        assert_eq!(libelle_statut_absence("validee"), "Validée");
        assert_eq!(libelle_statut_absence("signalee"), "Signalée, à valider");
        assert_eq!(ton_statut_absence("validee"), TonEtat::Ok);
        assert_eq!(
            libelle_dates(d(2026, 11, 9), d(2026, 11, 9)),
            "Le lundi 9 novembre"
        );
        assert_eq!(
            libelle_dates(d(2026, 11, 9), d(2026, 11, 11)),
            "Du lundi 9 novembre au mercredi 11 novembre"
        );
    }

    #[test]
    fn resume_du_traitement() {
        let signalee = absence("signalee", vec![]);
        assert!(resume_traitement(&signalee).contains("validée"));
        let validee = absence("validee", vec![touchee("prevue"), touchee("annulee")]);
        assert_eq!(resume_traitement(&validee), "1 séance à traiter.");
        let deux = absence("validee", vec![touchee("prevue"), touchee("prevue")]);
        assert_eq!(resume_traitement(&deux), "2 séances à traiter.");
        let fini = absence("validee", vec![touchee("remplacee")]);
        assert_eq!(
            resume_traitement(&fini),
            "Toutes les séances sont traitées."
        );
        assert_eq!(
            resume_traitement(&absence("validee", vec![])),
            "Aucune séance touchée sur la période."
        );
        assert_eq!(total_a_traiter(&[validee, deux, fini]), 3);
    }

    #[test]
    fn etat_des_seances_en_toutes_lettres() {
        assert_eq!(libelle_etat_seance(&touchee("prevue")), "À traiter");
        assert_eq!(libelle_etat_seance(&touchee("annulee")), "Cours annulé");
        let mut r = touchee("remplacee");
        assert_eq!(libelle_etat_seance(&r), "Remplaçant désigné");
        r.remplacant_nom = Some("M. Rogue".into());
        assert_eq!(libelle_etat_seance(&r), "Remplacé par M. Rogue");
        assert_eq!(libelle_etat_seance(&touchee("deplacee")), "Cours déplacé");
        assert_eq!(libelle_etat_seance(&touchee("zzz")), "État inconnu");
        assert!(peut_traiter(&touchee("prevue")) && !peut_traiter(&touchee("annulee")));
        assert!(!peut_retablir(&touchee("prevue")));
        for s in ["annulee", "remplacee", "deplacee"] {
            assert!(peut_retablir(&touchee(s)), "{s}");
        }
        assert_eq!(EtatSeance::depuis("annulee").ton(), TonEtat::Mal);
    }

    #[test]
    fn textes_de_confirmation() {
        let s = touchee("prevue");
        assert_eq!(libelle_horaire(&s), "Lundi 9 novembre · 08:00 – 09:00");
        assert_eq!(libelle_cours(&s), "Maths · 4B · Salle B12");
        assert_eq!(
            message_annulation(&s),
            "Annuler le cours de Maths avec la classe 4B, le lundi 9 novembre de 08:00 à 09:00 ?"
        );
        assert!(message_remplacement(&s, "M. Rogue")
            .starts_with("Confier à M. Rogue le cours de Maths"));
        assert!(message_retablissement(&s).contains("Rétablir"));
        assert!(RAPPEL_PREVENUS.contains("élèves") && RAPPEL_PREVENUS.contains("familles"));
        assert!(RAPPEL_PREVENUS.contains("sans indication de motif"));
    }

    fn candidat(n: u128, nom: Option<&str>, competent: bool, charge: usize) -> CandidatDto {
        CandidatDto {
            user_id: Uuid::from_u128(n),
            nom: nom.map(str::to_string),
            competent,
            seances_ce_jour: charge,
        }
    }

    #[test]
    fn candidats_competents_d_abord() {
        let l = trier_candidats(vec![
            candidat(1, Some("B"), false, 0),
            candidat(2, Some("C"), true, 3),
            candidat(3, Some("A"), true, 3),
            candidat(4, Some("D"), true, 1),
        ]);
        let ordre: Vec<u128> = l.iter().map(|c| c.user_id.as_u128()).collect();
        assert_eq!(ordre, [4, 3, 2, 1]);
        assert_eq!(
            libelle_candidat(&l[0]),
            "D — enseigne cette matière, 1 séance ce jour-là"
        );
        assert_eq!(
            libelle_candidat(&l[3]),
            "B — n'enseigne pas cette matière, libre ce jour-là"
        );
        assert_eq!(
            nom_candidat(&candidat(5, None, true, 0)),
            "Nom non renseigné"
        );
        assert_eq!(options_candidats(&l).len(), 4);
    }

    #[test]
    fn commande_de_remplacement() {
        let s = touchee("prevue");
        let absence = Uuid::from_u128(9);
        let choix = Uuid::from_u128(77).to_string();
        let c = cmd_remplacement(absence, &s, &choix).unwrap();
        assert_eq!(c.absence_rh_id, Some(absence));
        assert_eq!(c.remplacant_user_id, Uuid::from_u128(77));
        assert_eq!((c.creneau_id, c.jour), (s.creneau_id, s.jour));
        assert!(cmd_remplacement(absence, &s, "").is_none());
        assert!(cmd_remplacement(absence, &s, "pas un uuid").is_none());
    }

    #[test]
    fn saisie_de_masse_valide() {
        let apercu = composer_masse(&saisie(), true).unwrap();
        assert!(apercu.apercu && apercu.cle.is_none());
        let appli = composer_masse(&saisie(), false).unwrap();
        assert!(!appli.apercu);
        let cle = appli.cle.clone().unwrap();
        assert!(cle.starts_with("web-") && cle.len() <= 80, "{cle}");
        assert_eq!(appli.motif_public, "Grève des transports");
    }

    #[test]
    fn saisie_de_masse_refusee() {
        let champs = |s: &SaisieMasse| -> Vec<String> {
            composer_masse(s, true)
                .unwrap_err()
                .into_iter()
                .map(|(c, _)| c)
                .collect()
        };
        let mut s = saisie();
        s.du = String::new();
        assert_eq!(champs(&s), ["rpl-masse-du"]);
        let mut s = saisie();
        s.au = "2026-11-01".into();
        assert_eq!(champs(&s), ["rpl-masse-au"]);
        let mut s = saisie();
        s.au = "2027-03-01".into();
        assert_eq!(champs(&s), ["rpl-masse-au"]);
        let mut s = saisie();
        s.portee = PORTEE_CLASSES.into();
        assert_eq!(champs(&s), ["rpl-masse-classes"]);
        let mut s = saisie();
        s.portee = "niveau".into();
        assert_eq!(
            champs(&s),
            ["rpl-masse-portee"],
            "niveau non proposé par l'écran"
        );
        let mut s = saisie();
        s.heure_debut = "10:00".into();
        s.heure_fin = "09:00".into();
        assert_eq!(champs(&s), ["rpl-masse-hf"]);
        s.heure_fin = "abc".into();
        assert_eq!(champs(&s), ["rpl-masse-hf"]);
        let mut s = saisie();
        s.motif = "   ".into();
        assert_eq!(champs(&s), ["rpl-masse-motif"]);
        s.motif = "x".repeat(MOTIF_PUBLIC_MAX + 1);
        assert_eq!(champs(&s), ["rpl-masse-motif"]);
        s.motif = "x".repeat(MOTIF_PUBLIC_MAX);
        assert!(composer_masse(&s, true).is_ok());
    }

    #[test]
    fn heures_facultatives_et_classes_triees() {
        let mut s = saisie();
        s.portee = PORTEE_CLASSES.into();
        s.classes = vec![Uuid::from_u128(5), Uuid::from_u128(3), Uuid::from_u128(5)];
        s.heure_debut = "08:00".into();
        s.heure_fin = "12:00".into();
        let c = composer_masse(&s, true).unwrap();
        assert_eq!(
            c.cibles,
            [Uuid::from_u128(3), Uuid::from_u128(5)],
            "triées, sans doublon"
        );
        assert_eq!(c.heure_debut, Some(h(8)));
        assert_eq!(c.heure_fin, Some(h(12)));
        // Une seule heure est acceptée.
        s.heure_fin = String::new();
        assert!(composer_masse(&s, true).is_ok());
    }

    #[test]
    fn cle_stable_et_sensible_aux_parametres() {
        let a = composer_masse(&saisie(), false).unwrap();
        let b = composer_masse(&saisie(), false).unwrap();
        assert_eq!(a.cle, b.cle, "déterministe");
        // Valeur figée : un changement d'algorithme casserait l'idempotence des lots déjà envoyés.
        assert_eq!(a.cle.as_deref(), Some(CLE_ATTENDUE));
        let mut s = saisie();
        s.motif = "Autre motif".into();
        assert_ne!(composer_masse(&s, false).unwrap().cle, a.cle);
        let mut s = saisie();
        s.au = "2026-11-14".into();
        assert_ne!(composer_masse(&s, false).unwrap().cle, a.cle);
        // Les espaces autour du motif et l'ordre des classes ne changent pas la clé.
        let mut x = saisie();
        x.portee = PORTEE_CLASSES.into();
        x.classes = vec![Uuid::from_u128(2), Uuid::from_u128(1)];
        let mut y = x.clone();
        y.classes = vec![Uuid::from_u128(1), Uuid::from_u128(2)];
        y.motif = "  Grève des transports ".into();
        assert_eq!(
            composer_masse(&x, false).unwrap().cle,
            composer_masse(&y, false).unwrap().cle
        );
        // L'aperçu n'a pas de clé et la description ne dépend pas de `apercu`.
        assert_eq!(
            description_canonique(&composer_masse(&saisie(), true).unwrap()),
            description_canonique(&a)
        );
    }

    /// Empreinte attendue de `saisie()` (voir `cle_stable_et_sensible_aux_parametres`).
    const CLE_ATTENDUE: &str = "web-58bb753ce40452834966c0e3cea899f3";

    #[test]
    fn l_apercu_est_invalide_par_toute_modification() {
        let s = saisie();
        let sig = signature(&s);
        assert!(!sig.is_empty());
        assert!(apercu_valide(Some(&sig), &s));
        assert!(
            !apercu_valide(None, &s),
            "pas d'aperçu : pas de confirmation"
        );
        assert!(!apercu_valide(Some(""), &s));
        type Modification = Box<dyn Fn(&mut SaisieMasse)>;
        let modifications: Vec<Modification> = vec![
            Box::new(|s| s.du = "2026-11-11".into()),
            Box::new(|s| s.au = "2026-11-14".into()),
            Box::new(|s| s.portee = PORTEE_CLASSES.into()),
            Box::new(|s| s.heure_debut = "08:00".into()),
            Box::new(|s| s.heure_fin = "12:00".into()),
            Box::new(|s| s.motif = "Autre".into()),
        ];
        for (i, m) in modifications.iter().enumerate() {
            let mut t = s.clone();
            m(&mut t);
            assert!(!apercu_valide(Some(&sig), &t), "modification {i}");
        }
        let mut t = saisie();
        t.portee = PORTEE_CLASSES.into();
        t.classes = vec![Uuid::from_u128(1)];
        let sig_classes = signature(&t);
        t.classes.push(Uuid::from_u128(2));
        assert!(!apercu_valide(Some(&sig_classes), &t), "classe ajoutée");
        // Une saisie devenue invalide n'a jamais d'aperçu valide.
        let mut v = saisie();
        v.motif = String::new();
        assert!(!apercu_valide(Some(&sig), &v));
        // Des espaces superflus ne comptent pas comme une modification.
        let mut e = saisie();
        e.motif = " Grève des transports ".into();
        assert!(apercu_valide(Some(&sig), &e));
    }

    #[test]
    fn resume_du_rapport() {
        let r = RapportMasseDto {
            apercu: true,
            deja_applique: false,
            lot_id: None,
            nb_seances: 12,
            nb_classes: 2,
            classes: vec![
                ClasseToucheeDto {
                    classe_id: Uuid::nil(),
                    classe: "4B".into(),
                    nb_seances: 7,
                },
                ClasseToucheeDto {
                    classe_id: Uuid::nil(),
                    classe: "5A".into(),
                    nb_seances: 1,
                },
            ],
            deja_annulees: 3,
        };
        assert_eq!(
            resume_rapport(&r),
            "12 séances dans 2 classes ; 3 déjà annulée(s) ou déplacée(s), ignorée(s)."
        );
        assert_eq!(libelle_classe_touchee(&r.classes[0]), "4B : 7 séances");
        assert_eq!(libelle_classe_touchee(&r.classes[1]), "5A : 1 séance");
        let vide = RapportMasseDto {
            nb_seances: 0,
            nb_classes: 0,
            classes: vec![],
            deja_annulees: 0,
            ..r.clone()
        };
        assert_eq!(resume_rapport(&vide), "aucune séance dans aucune classe.");
        let cmd = composer_masse(&saisie(), true).unwrap();
        let m = message_confirmation_masse(&cmd, &r);
        assert!(
            m.contains("tout l'établissement") && m.contains("12 séances"),
            "{m}"
        );
        assert!(message_application(&r).contains("prévenus"));
        let deja = RapportMasseDto {
            deja_applique: true,
            apercu: false,
            ..r
        };
        assert!(message_application(&deja).contains("déjà été enregistrée"));
    }
}
