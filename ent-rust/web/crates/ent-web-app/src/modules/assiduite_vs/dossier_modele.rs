//! Modèle du dossier d'assiduité d'un élève (AS14d) et du signalement (AS14e) : logique **pure**, sans
//! Leptos ni réseau, testée en natif. Le serveur calcule et décide (décompte par demi-journée, règle
//! datée, droits) ; ce module ne fait que regrouper, libeller, ordonner et contrôler la saisie.

use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
use ent_types::assiduite::{
    ContactDto, CourrierDto, EpisodeDto, RectificationDto, RequalifierEpisodeBody,
};
use ent_types::discipline::{DemiJourneeDto, SignalementDto};
use ent_web_ui::TonEtat;

use super::modele::{libelle_periode_avis, libelle_qualification, MAX_NOTE, QUALIFICATIONS};
use crate::composants_metier::absences::libelle_periode;
use crate::noyau::temps::{date_heure_paris, date_longue, nom_mois};

// ───────────────────────────── Année scolaire et mois ─────────────────────────────

/// Bornes de l'année scolaire qui contient `jour` : du 1er septembre au 31 août (hypothèse de
/// l'écran : l'année scolaire propre à l'établissement est au socle 5 et n'est pas lue ici).
pub fn bornes_annee_scolaire(jour: NaiveDate) -> (NaiveDate, NaiveDate) {
    let an = if jour.month() >= 9 {
        jour.year()
    } else {
        jour.year() - 1
    };
    (
        NaiveDate::from_ymd_opt(an, 9, 1).expect("1er septembre"),
        NaiveDate::from_ymd_opt(an + 1, 8, 31).expect("31 août"),
    )
}

/// « 2026-2027 ».
pub fn libelle_annee_scolaire(jour: NaiveDate) -> String {
    let (d, f) = bornes_annee_scolaire(jour);
    format!("{}-{}", d.year(), f.year())
}

pub fn premier_du_mois(d: NaiveDate) -> NaiveDate {
    d.with_day(1).expect("le 1er existe")
}

/// « novembre 2026 ».
pub fn libelle_mois(m: NaiveDate) -> String {
    format!("{} {}", nom_mois(m.month()), m.year())
}

/// « 2026-11 » (champ `month`) ou « 2026-11-17 » → premier jour du mois.
pub fn lire_mois(texte: &str) -> Option<NaiveDate> {
    let t = texte.trim();
    let (annee, reste) = t.split_once('-')?;
    let mois = reste.split('-').next()?;
    NaiveDate::from_ymd_opt(annee.parse().ok()?, mois.parse().ok()?, 1)
}

// ───────────────────────────── Décompte par mois ─────────────────────────────

/// Position d'un mois par rapport au seuil de signalement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatSeuil {
    /// Le seuil est atteint ou dépassé.
    Atteint,
    /// Il manque une demi-journée.
    Proche,
    Sous,
}

impl EtatSeuil {
    pub fn de(non_justifiees: i64, seuil: i32) -> EtatSeuil {
        let seuil = i64::from(seuil);
        if seuil > 0 && non_justifiees >= seuil {
            EtatSeuil::Atteint
        } else if seuil > 1 && non_justifiees == seuil - 1 {
            EtatSeuil::Proche
        } else {
            EtatSeuil::Sous
        }
    }

    /// Libellé complet : l'état n'est jamais porté par la seule couleur.
    pub fn libelle(self) -> &'static str {
        match self {
            EtatSeuil::Atteint => "Seuil atteint",
            EtatSeuil::Proche => "Proche du seuil",
            EtatSeuil::Sous => "Sous le seuil",
        }
    }

    pub fn ton(self) -> TonEtat {
        match self {
            EtatSeuil::Atteint => TonEtat::Mal,
            EtatSeuil::Proche => TonEtat::Avertissement,
            EtatSeuil::Sous => TonEtat::Neutre,
        }
    }
}

/// Demi-journées d'un mois, telles que le serveur les a dérivées.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoisDecompte {
    pub mois: NaiveDate,
    /// Demi-journées qui comptent.
    pub comptees: i64,
    /// Dont non justifiées (ce qui compte pour le seuil).
    pub non_justifiees: i64,
    pub etat: EtatSeuil,
    /// Toutes les lignes du mois, y compris celles qui ne comptent pas (avec leur raison).
    pub lignes: Vec<DemiJourneeDto>,
}

/// Regroupe par mois (le plus récent d'abord). Ne recalcule aucune règle : « compte » et « non
/// justifiée » viennent du serveur ; seul l'état par rapport au seuil est déduit du seuil lu.
pub fn grouper_par_mois(demi_journees: &[DemiJourneeDto], seuil: i32) -> Vec<MoisDecompte> {
    let mut mois: Vec<MoisDecompte> = Vec::new();
    for d in demi_journees {
        let m = premier_du_mois(d.jour);
        let idx = match mois.iter().position(|x| x.mois == m) {
            Some(i) => i,
            None => {
                mois.push(MoisDecompte {
                    mois: m,
                    comptees: 0,
                    non_justifiees: 0,
                    etat: EtatSeuil::Sous,
                    lignes: Vec::new(),
                });
                mois.len() - 1
            }
        };
        let x = &mut mois[idx];
        if d.comptee {
            x.comptees += 1;
            if d.non_justifiee {
                x.non_justifiees += 1;
            }
        }
        x.lignes.push(d.clone());
    }
    for x in &mut mois {
        x.etat = EtatSeuil::de(x.non_justifiees, seuil);
        x.lignes.sort_by_key(|l| (l.jour, l.periode != "matin"));
    }
    mois.sort_by_key(|x| std::cmp::Reverse(x.mois));
    mois
}

/// Totaux de l'année : demi-journées comptées et non justifiées.
pub fn total_annee(mois: &[MoisDecompte]) -> (i64, i64) {
    mois.iter()
        .fold((0, 0), |(c, n), m| (c + m.comptees, n + m.non_justifiees))
}

/// « Mardi 8 septembre, matin ».
pub fn libelle_demi_journee(d: &DemiJourneeDto) -> String {
    format!(
        "{}, {}",
        date_longue(d.jour),
        libelle_periode_avis(&d.periode)
    )
}

/// Ce que dit une ligne du décompte : l'état, jamais seulement une couleur.
pub fn etat_demi_journee(d: &DemiJourneeDto) -> (&'static str, TonEtat) {
    if !d.comptee {
        ("Ne compte pas", TonEtat::Neutre)
    } else if d.non_justifiee {
        ("Compte, non justifiée", TonEtat::Mal)
    } else {
        ("Compte, justifiée", TonEtat::Ok)
    }
}

/// Explication de la ligne : celle du serveur (champs codés), à défaut une phrase neutre.
pub fn explication_demi_journee(d: &DemiJourneeDto) -> String {
    if d.explication.trim().is_empty() {
        format!("{} minute(s) manquée(s).", d.minutes)
    } else {
        d.explication.clone()
    }
}

// ───────────────────────────── Épisodes ─────────────────────────────

/// Épisodes de l'élève, le plus récent d'abord ; les retirés en dernier.
pub fn trier_episodes_dossier(mut v: Vec<EpisodeDto>) -> Vec<EpisodeDto> {
    v.sort_by_key(|e| (e.etat == "retire", std::cmp::Reverse(e.jour_debut)));
    v
}

/// Compteurs d'épisodes pour l'en-tête du dossier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompteursEpisodes {
    pub total: usize,
    pub a_qualifier: usize,
    pub non_justifies: usize,
}

pub fn compter_episodes(v: &[EpisodeDto]) -> CompteursEpisodes {
    let actifs = v.iter().filter(|e| e.etat != "retire");
    CompteursEpisodes {
        total: actifs.clone().count(),
        a_qualifier: actifs
            .clone()
            .filter(|e| matches!(e.etat.as_str(), "ouvert" | "a_qualifier"))
            .count(),
        non_justifies: actifs
            .filter(|e| {
                e.etat == "qualifie"
                    && matches!(e.qualification.as_str(), "non_valable" | "sans_motif")
            })
            .count(),
    }
}

// ───────────────────────────── Requalification ─────────────────────────────

/// Motifs de requalification (liste fermée du serveur) : code et libellé.
pub const MOTIFS_REQUALIFICATION: [(&str, &str); 5] = [
    ("erreur_de_saisie", "Erreur de saisie"),
    ("nouvelle_piece", "Nouvelle pièce reçue"),
    ("decision_direction", "Décision de la direction"),
    ("changement_de_nature", "Changement de nature du motif"),
    ("autre", "Autre raison"),
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieRequalification {
    pub qualification: String,
    pub motif_code: String,
    pub nature_code: String,
    pub libelle_famille: String,
    pub note: String,
}

/// Contrôle local (le serveur revérifie) puis corps d'écriture. Une requalification exige un motif
/// codé ; une nouvelle qualification identique à l'actuelle n'a pas de sens ; la version lue part avec
/// le corps (conflit si l'épisode a changé).
pub fn corps_requalification(
    actuelle: &str,
    s: &SaisieRequalification,
    version: i32,
) -> Result<RequalifierEpisodeBody, &'static str> {
    if !QUALIFICATIONS.iter().any(|(c, _)| *c == s.qualification) {
        return Err("Choisissez la nouvelle qualification.");
    }
    if s.qualification == actuelle {
        return Err("Choisissez une qualification différente de l'actuelle.");
    }
    if !MOTIFS_REQUALIFICATION
        .iter()
        .any(|(c, _)| *c == s.motif_code)
    {
        return Err("Indiquez pourquoi vous requalifiez : c'est conservé dans l'historique.");
    }
    if s.qualification == "reputee_legitime" && s.nature_code.trim().is_empty() {
        return Err("Choisissez l'un des motifs légitimes.");
    }
    let note = s.note.trim();
    if note.chars().count() > MAX_NOTE {
        return Err("La note interne est trop longue (500 caractères au plus).");
    }
    let libelle = s.libelle_famille.trim();
    let nature = s.nature_code.trim();
    Ok(RequalifierEpisodeBody {
        qualification: s.qualification.clone(),
        version,
        motif_code: s.motif_code.clone(),
        nature_code: (!nature.is_empty()).then(|| nature.to_string()),
        libelle_famille_code: (!libelle.is_empty()).then(|| libelle.to_string()),
        note_interne: (!note.is_empty()).then(|| note.to_string()),
    })
}

// ───────────────────────────── Contacts, courriers, rectifications ─────────────────────────────

pub fn libelle_canal(code: &str) -> &'static str {
    match code {
        "telephone" => "Téléphone",
        "sms" => "SMS",
        "courriel" => "Courriel",
        "rencontre" => "Rencontre",
        "courrier" => "Courrier",
        _ => "Autre canal",
    }
}

pub fn libelle_issue_contact(code: &str) -> &'static str {
    match code {
        "joint" => "famille jointe",
        "message_laisse" => "message laissé",
        "ne_repond_pas" => "ne répond pas",
        "numero_faux" => "numéro erroné",
        "envoye" => "envoyé",
        "rencontre_faite" => "rencontre faite",
        _ => "issue inconnue",
    }
}

pub fn libelle_nature_courrier(code: &str) -> &'static str {
    match code {
        "relance_absence" => "Relance d'absence",
        "avis_absence" => "Avis d'absence",
        _ => "Courrier",
    }
}

pub fn libelle_statut_courrier(code: &str) -> &'static str {
    match code {
        "prepare" => "préparé, à valider",
        "valide" => "validé, à poster",
        "poste" => "posté",
        "abandonne" => "abandonné",
        _ => "statut inconnu",
    }
}

pub fn libelle_cause_rectification(code: &str) -> &'static str {
    match code {
        "recalcul" => "recalcul du décompte",
        "requalification" => "requalification d'une absence",
        "correction_appel" => "correction d'un appel",
        "neutralisation" => "séance non assurée",
        "calendrier" => "changement du calendrier",
        _ => "cause inconnue",
    }
}

/// « Le décompte de novembre 2026 passe de 5 à 3 demi-journées non justifiées (requalification…). »
pub fn phrase_rectification(r: &RectificationDto) -> String {
    format!(
        "Le décompte de {} passe de {} à {} demi-journée(s) non justifiée(s) ({} à {} comptées) : {}.",
        libelle_mois(r.mois),
        r.avant_non_justifiees,
        r.apres_non_justifiees,
        r.avant_demi_journees,
        r.apres_demi_journees,
        libelle_cause_rectification(&r.cause),
    )
}

/// Le seuil change-t-il de côté à cause de la rectification ?
pub fn franchit_le_seuil(r: &RectificationDto, seuil: i32) -> bool {
    let seuil = i64::from(seuil);
    seuil > 0
        && ((i64::from(r.avant_non_justifiees) >= seuil)
            != (i64::from(r.apres_non_justifiees) >= seuil))
}

// ───────────────────────────── Chronologie ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenreEvenement {
    Absence,
    Contact,
    Courrier,
    Rectification,
    Signalement,
}

impl GenreEvenement {
    pub fn libelle(self) -> &'static str {
        match self {
            GenreEvenement::Absence => "Absence",
            GenreEvenement::Contact => "Contact",
            GenreEvenement::Courrier => "Courrier",
            GenreEvenement::Rectification => "Rectification",
            GenreEvenement::Signalement => "Signalement",
        }
    }
}

/// Un événement du dossier annuel (R131-6 : tout ce qui concerne l'absentéisme de l'année).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvenementChrono {
    pub instant: DateTime<Utc>,
    /// Date lisible (jour pour une absence, date et heure pour le reste).
    pub quand: String,
    pub genre: GenreEvenement,
    pub titre: String,
    pub detail: Option<String>,
}

fn midi_utc(j: NaiveDate) -> DateTime<Utc> {
    Utc.from_utc_datetime(&j.and_time(NaiveTime::from_hms_opt(12, 0, 0).expect("midi")))
}

/// Chronologie unique, du plus récent au plus ancien. Un épisode est daté par son premier jour
/// (midi, pour l'ordre seulement), les autres événements par leur instant ; la note interne des
/// contacts est reprise telle que le serveur la rend (il l'omet à qui n'y a pas droit).
pub fn chronologie(
    episodes: &[EpisodeDto],
    contacts: &[ContactDto],
    courriers: &[CourrierDto],
    rectifications: &[RectificationDto],
    signalements: &[SignalementDto],
) -> Vec<EvenementChrono> {
    let mut v: Vec<EvenementChrono> = Vec::new();
    for e in episodes.iter().filter(|e| e.etat != "retire") {
        let etat = if e.etat == "qualifie" {
            libelle_qualification(&e.qualification).to_string()
        } else {
            "à qualifier".to_string()
        };
        v.push(EvenementChrono {
            instant: midi_utc(e.jour_debut),
            quand: date_longue(e.jour_debut),
            genre: GenreEvenement::Absence,
            titre: format!("Absence {}", libelle_periode(e.jour_debut, e.jour_fin)),
            detail: Some(etat),
        });
    }
    for c in contacts {
        v.push(EvenementChrono {
            instant: c.cree_le,
            quand: date_heure_paris(c.cree_le),
            genre: GenreEvenement::Contact,
            titre: format!(
                "{} : {}",
                libelle_canal(&c.canal),
                libelle_issue_contact(&c.issue)
            ),
            detail: c.note_interne.clone().filter(|n| !n.trim().is_empty()),
        });
    }
    for c in courriers {
        v.push(EvenementChrono {
            instant: c.poste_le.or(c.valide_le).unwrap_or(c.prepare_le),
            quand: date_heure_paris(c.poste_le.or(c.valide_le).unwrap_or(c.prepare_le)),
            genre: GenreEvenement::Courrier,
            titre: format!(
                "{} : {}",
                libelle_nature_courrier(&c.nature),
                libelle_statut_courrier(&c.statut)
            ),
            detail: None,
        });
    }
    for r in rectifications {
        v.push(EvenementChrono {
            instant: r.cree_le,
            quand: date_heure_paris(r.cree_le),
            genre: GenreEvenement::Rectification,
            titre: format!("Rectification du décompte de {}", libelle_mois(r.mois)),
            detail: Some(phrase_rectification(r)),
        });
    }
    for s in signalements {
        v.push(EvenementChrono {
            instant: midi_utc(s.mois),
            quand: libelle_mois(s.mois),
            genre: GenreEvenement::Signalement,
            titre: format!("Signalement de {}", libelle_mois(s.mois)),
            detail: Some(format!(
                "{} · {}",
                libelle_statut_signalement(&s.statut),
                resume_signalement(s)
            )),
        });
    }
    v.sort_by_key(|e| std::cmp::Reverse(e.instant));
    v
}

// ───────────────────────────── Signalements (AS14e) ─────────────────────────────

pub fn libelle_statut_signalement(statut: &str) -> &'static str {
    match statut {
        "propose" => "Proposé, en attente de décision",
        "valide" => "Validé, à transmettre",
        "refuse" => "Refusé",
        "emis" => "Transmis",
        _ => "Statut inconnu",
    }
}

pub fn ton_statut_signalement(statut: &str) -> TonEtat {
    match statut {
        "propose" => TonEtat::Avertissement,
        "valide" => TonEtat::Info,
        "emis" => TonEtat::Ok,
        _ => TonEtat::Neutre,
    }
}

/// Transitions proposées : proposé → valider / refuser ; validé → marquer transmis. ScolIA
/// **n'envoie rien** à l'académie (R-AS26) : « transmis » est la trace d'un geste fait par une
/// personne, hors de l'application.
pub fn decisions_signalement(statut: &str) -> Vec<(&'static str, &'static str)> {
    match statut {
        "propose" => vec![("valider", "Valider"), ("refuser", "Refuser")],
        "valide" => vec![("emettre", "Noter comme transmis")],
        _ => Vec::new(),
    }
}

pub fn resume_signalement(s: &SignalementDto) -> String {
    format!(
        "{} demi-journée(s) non justifiée(s) pour un seuil de {}",
        s.demi_journees, s.seuil
    )
}

/// Ligne du « dossier » pré-rempli d'un signalement : `jour`, `periode`, `minutes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LigneDossierSignalement {
    pub jour: NaiveDate,
    pub periode: String,
    pub minutes: i64,
}

/// Lit les demi-journées du dossier reconstruit par le serveur ; une ligne illisible est ignorée.
pub fn lignes_dossier_signalement(s: &SignalementDto) -> Vec<LigneDossierSignalement> {
    s.dossier
        .get("demi_journees")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|l| {
                    Some(LigneDossierSignalement {
                        jour: NaiveDate::parse_from_str(l.get("jour")?.as_str()?, "%Y-%m-%d")
                            .ok()?,
                        periode: l.get("periode")?.as_str()?.to_string(),
                        minutes: l.get("minutes")?.as_i64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn dj(jour: &str, periode: &str, comptee: bool, non_justifiee: bool) -> DemiJourneeDto {
        DemiJourneeDto {
            jour: d(jour),
            periode: periode.into(),
            minutes: 120,
            minutes_non_justifiees: if non_justifiee { 120 } else { 0 },
            comptee,
            non_justifiee,
            raison: String::new(),
            explication: String::new(),
        }
    }

    fn rectif(avant: i32, apres: i32) -> RectificationDto {
        RectificationDto {
            id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            mois: d("2026-10-01"),
            avant_demi_journees: avant + 1,
            avant_non_justifiees: avant,
            apres_demi_journees: apres + 1,
            apres_non_justifiees: apres,
            cause: "requalification".into(),
            touche_signalement: true,
            cree_le: Utc.with_ymd_and_hms(2026, 11, 5, 9, 0, 0).unwrap(),
            vue_par: None,
        }
    }

    #[test]
    fn annee_scolaire_de_septembre_a_aout() {
        let (a, b) = bornes_annee_scolaire(d("2026-11-16"));
        assert_eq!((a, b), (d("2026-09-01"), d("2027-08-31")));
        let (a, b) = bornes_annee_scolaire(d("2027-03-02"));
        assert_eq!((a, b), (d("2026-09-01"), d("2027-08-31")));
        assert_eq!(libelle_annee_scolaire(d("2027-08-31")), "2026-2027");
        assert_eq!(libelle_annee_scolaire(d("2027-09-01")), "2027-2028");
    }

    #[test]
    fn mois_lu_et_libelle() {
        assert_eq!(lire_mois("2026-11"), Some(d("2026-11-01")));
        assert_eq!(lire_mois("2026-11-17"), Some(d("2026-11-01")));
        assert_eq!(lire_mois("n'importe quoi"), None);
        assert_eq!(libelle_mois(d("2026-11-01")), "novembre 2026");
    }

    #[test]
    fn etat_par_rapport_au_seuil() {
        assert_eq!(EtatSeuil::de(4, 4), EtatSeuil::Atteint);
        assert_eq!(EtatSeuil::de(6, 4), EtatSeuil::Atteint);
        assert_eq!(EtatSeuil::de(3, 4), EtatSeuil::Proche);
        assert_eq!(EtatSeuil::de(2, 4), EtatSeuil::Sous);
        // Seuil de 1 : pas de « proche » (0 sur 1 n'est pas proche) ; seuil 0 : jamais atteint.
        assert_eq!(EtatSeuil::de(0, 1), EtatSeuil::Sous);
        assert_eq!(EtatSeuil::de(5, 0), EtatSeuil::Sous);
        assert!(EtatSeuil::Atteint.libelle().contains("atteint"));
    }

    #[test]
    fn regroupement_par_mois_recent_d_abord() {
        let l = vec![
            dj("2026-09-08", "matin", true, true),
            dj("2026-10-05", "matin", true, true),
            dj("2026-10-05", "apres_midi", true, false),
            dj("2026-10-06", "matin", false, false),
            dj("2026-10-07", "matin", true, true),
        ];
        let m = grouper_par_mois(&l, 3);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].mois, d("2026-10-01"));
        assert_eq!((m[0].comptees, m[0].non_justifiees), (3, 2));
        assert_eq!(m[0].etat, EtatSeuil::Proche);
        assert_eq!(
            m[0].lignes.len(),
            4,
            "la ligne qui ne compte pas reste visible"
        );
        assert_eq!(m[1].mois, d("2026-09-01"));
        assert_eq!(total_annee(&m), (4, 3));
    }

    #[test]
    fn lignes_ordonnees_matin_avant_apres_midi() {
        let l = vec![
            dj("2026-10-05", "apres_midi", true, true),
            dj("2026-10-05", "matin", true, true),
        ];
        let m = grouper_par_mois(&l, 4);
        assert_eq!(m[0].lignes[0].periode, "matin");
    }

    #[test]
    fn etat_d_une_ligne_jamais_par_la_couleur() {
        assert_eq!(
            etat_demi_journee(&dj("2026-10-05", "matin", false, false)).0,
            "Ne compte pas"
        );
        assert_eq!(
            etat_demi_journee(&dj("2026-10-05", "matin", true, true)).0,
            "Compte, non justifiée"
        );
        assert_eq!(
            etat_demi_journee(&dj("2026-10-05", "matin", true, false)).0,
            "Compte, justifiée"
        );
        assert!(
            libelle_demi_journee(&dj("2026-10-05", "apres_midi", true, true))
                .contains("après-midi")
        );
    }

    #[test]
    fn explication_du_serveur_ou_phrase_neutre() {
        let mut x = dj("2026-10-05", "matin", true, true);
        assert!(explication_demi_journee(&x).contains("120 minute"));
        x.explication = "lundi matin compte".into();
        assert_eq!(explication_demi_journee(&x), "lundi matin compte");
    }

    #[test]
    fn requalification_exige_un_motif_code_et_un_changement() {
        let mut s = SaisieRequalification {
            qualification: "excuse_valable".into(),
            ..Default::default()
        };
        assert!(
            corps_requalification("sans_motif", &s, 3).is_err(),
            "sans motif codé"
        );
        s.motif_code = "nouvelle_piece".into();
        let c = corps_requalification("sans_motif", &s, 3).unwrap();
        assert_eq!((c.version, c.motif_code.as_str()), (3, "nouvelle_piece"));
        assert!(
            corps_requalification("excuse_valable", &s, 3).is_err(),
            "même qualification"
        );
        s.qualification = "reputee_legitime".into();
        assert!(
            corps_requalification("sans_motif", &s, 3).is_err(),
            "nature obligatoire"
        );
        s.nature_code = "maladie".into();
        assert_eq!(
            corps_requalification("sans_motif", &s, 3)
                .unwrap()
                .nature_code
                .as_deref(),
            Some("maladie")
        );
        s.motif_code = "inventé".into();
        assert!(
            corps_requalification("sans_motif", &s, 3).is_err(),
            "motif hors liste"
        );
        s.motif_code = "autre".into();
        s.note = "x".repeat(501);
        assert!(
            corps_requalification("sans_motif", &s, 3).is_err(),
            "note trop longue"
        );
    }

    #[test]
    fn rectification_dite_en_clair() {
        let r = rectif(5, 3);
        let p = phrase_rectification(&r);
        assert!(p.contains("octobre 2026") && p.contains("de 5 à 3"), "{p}");
        assert!(franchit_le_seuil(&r, 4), "5 → 3 passe sous le seuil de 4");
        assert!(!franchit_le_seuil(&rectif(6, 5), 4));
        assert!(!franchit_le_seuil(&r, 0));
    }

    #[test]
    fn chronologie_unique_recent_d_abord() {
        let mut ep = EpisodeDto {
            id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            eleve_label: None,
            jour_debut: d("2026-10-05"),
            jour_fin: d("2026-10-05"),
            etat: "qualifie".into(),
            qualification: "sans_motif".into(),
            nature_code: None,
            motif_id: None,
            prevenue: false,
            source: "appel".into(),
            libelle_famille_code: None,
            version: 1,
            seances: vec![],
            justificatifs: 0,
            propositions_en_attente: 0,
            textes: vec![],
            note_interne: None,
        };
        let retire = EpisodeDto {
            etat: "retire".into(),
            jour_debut: d("2026-10-20"),
            jour_fin: d("2026-10-20"),
            ..ep.clone()
        };
        ep.id = Uuid::from_u128(1);
        let contact = ContactDto {
            id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            absence_id: None,
            canal: "telephone".into(),
            sens: "sortant".into(),
            issue: "message_laisse".into(),
            note_interne: Some("rappeler à 17 h".into()),
            cree_le: Utc.with_ymd_and_hms(2026, 10, 6, 8, 0, 0).unwrap(),
        };
        let v = chronologie(&[ep, retire], &[contact], &[], &[rectif(5, 3)], &[]);
        let genres: Vec<_> = v.iter().map(|e| e.genre).collect();
        assert_eq!(
            genres,
            vec![
                GenreEvenement::Rectification,
                GenreEvenement::Contact,
                GenreEvenement::Absence
            ],
            "l'épisode retiré n'apparaît pas ; ordre du plus récent au plus ancien"
        );
        assert_eq!(v[1].detail.as_deref(), Some("rappeler à 17 h"));
        assert!(v[1].titre.contains("Téléphone") && v[1].titre.contains("message laissé"));
    }

    #[test]
    fn compteurs_d_episodes() {
        let base = |etat: &str, q: &str| EpisodeDto {
            id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            eleve_label: None,
            jour_debut: d("2026-10-05"),
            jour_fin: d("2026-10-05"),
            etat: etat.into(),
            qualification: q.into(),
            nature_code: None,
            motif_id: None,
            prevenue: false,
            source: "appel".into(),
            libelle_famille_code: None,
            version: 1,
            seances: vec![],
            justificatifs: 0,
            propositions_en_attente: 0,
            textes: vec![],
            note_interne: None,
        };
        let v = vec![
            base("a_qualifier", "en_attente"),
            base("qualifie", "sans_motif"),
            base("qualifie", "excuse_valable"),
            base("retire", "sans_motif"),
        ];
        let c = compter_episodes(&v);
        assert_eq!((c.total, c.a_qualifier, c.non_justifies), (3, 1, 1));
        let t = trier_episodes_dossier(v);
        assert_eq!(t.last().unwrap().etat, "retire");
    }

    #[test]
    fn signalement_libelles_et_decisions() {
        assert_eq!(decisions_signalement("propose").len(), 2);
        assert_eq!(decisions_signalement("valide")[0].1, "Noter comme transmis");
        assert!(decisions_signalement("emis").is_empty());
        assert!(decisions_signalement("refuse").is_empty());
        assert_eq!(libelle_statut_signalement("emis"), "Transmis");
        assert_eq!(ton_statut_signalement("valide"), TonEtat::Info);
    }

    #[test]
    fn dossier_de_signalement_lu_sans_planter() {
        let s = SignalementDto {
            id: Uuid::nil(),
            eleve_id: Uuid::nil(),
            mois: d("2026-10-01"),
            demi_journees: 2,
            seuil: 4,
            statut: "propose".into(),
            dossier: serde_json::json!({
                "demi_journees": [
                    {"jour": "2026-10-05", "periode": "matin", "minutes": 120},
                    {"jour": "pas une date", "periode": "matin", "minutes": 1},
                    {"periode": "matin"}
                ]
            }),
        };
        let l = lignes_dossier_signalement(&s);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].minutes, 120);
        assert!(lignes_dossier_signalement(&SignalementDto {
            dossier: serde_json::Value::Null,
            ..s
        })
        .is_empty());
    }
}
