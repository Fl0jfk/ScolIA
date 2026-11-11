//! Logique d'écran du cahier de textes (pure, sans Leptos ni réseau, testée en natif) :
//! semaines, regroupement par jour, états d'un travail, validation des formulaires.
//!
//! Aucune règle métier ici : droits, visibilité par groupe, retard et surcharge sont décidés
//! par le serveur. L'écran valide seulement la forme de la saisie avant de l'envoyer, et
//! présente ce que le serveur renvoie.

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use ent_web_api::cahier_types::{
    AgendaItemDto, CreerTravailBody, LigneSuiviDto, RemiseDto, SeanceCahierDto, TravailDto,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

/// Taille maximale d'un fichier déposé (alignée sur la limite du serveur, contrôlée par lui).
pub const FICHIER_MAX_OCTETS: usize = 10 * 1024 * 1024;
pub const TITRE_MAX: usize = 200;
pub const CONSIGNE_MAX: usize = 8000;
pub const CONTENU_MAX: usize = 8000;
pub const RESSOURCES_MAX: usize = 4000;
pub const COMMENTAIRE_MAX: usize = 2000;
pub const DUREE_MAX_MIN: i32 = 600;

// ───────────────────────────── Semaines ─────────────────────────────

/// Lundi de la semaine contenant `d`.
pub fn lundi_de(d: NaiveDate) -> NaiveDate {
    d - Duration::days(i64::from(d.weekday().num_days_from_monday()))
}

/// Lundi demandé par `?semaine=AAAA-MM-JJ` (n'importe quel jour de la semaine), sinon la semaine courante.
pub fn semaine_demandee(param: Option<String>, aujourdhui: NaiveDate) -> NaiveDate {
    param
        .and_then(|p| NaiveDate::parse_from_str(p.trim(), "%Y-%m-%d").ok())
        .map(lundi_de)
        .unwrap_or_else(|| lundi_de(aujourdhui))
}

pub fn semaine_voisine(lundi: NaiveDate, decalage: i64) -> NaiveDate {
    lundi + Duration::days(7 * decalage)
}

pub fn fin_de_semaine(lundi: NaiveDate) -> NaiveDate {
    lundi + Duration::days(6)
}

/// « Semaine du 3 novembre au 9 novembre 2026 » (sans année répétée inutilement).
pub fn libelle_semaine(lundi: NaiveDate) -> String {
    let fin = fin_de_semaine(lundi);
    format!(
        "Semaine du {} au {}",
        crate::noyau::temps::date_longue(lundi),
        crate::noyau::temps::date_longue(fin)
    )
}

// ───────────────────────────── Séances ─────────────────────────────

/// Séances regroupées par jour, dans l'ordre chronologique.
pub fn seances_par_jour(
    mut seances: Vec<SeanceCahierDto>,
) -> Vec<(NaiveDate, Vec<SeanceCahierDto>)> {
    seances.sort_by(|a, b| (a.jour, &a.heure_debut).cmp(&(b.jour, &b.heure_debut)));
    let mut out: Vec<(NaiveDate, Vec<SeanceCahierDto>)> = Vec::new();
    for s in seances {
        match out.last_mut() {
            Some((j, l)) if *j == s.jour => l.push(s),
            _ => out.push((s.jour, vec![s])),
        }
    }
    out
}

/// Classes distinctes des séances (pour afficher travaux et charge par classe).
pub fn classes_des_seances(seances: &[SeanceCahierDto]) -> Vec<(Uuid, String)> {
    let mut out: Vec<(Uuid, String)> = Vec::new();
    for s in seances {
        if let Some(id) = s.classe_id {
            if !out.iter().any(|(i, _)| *i == id) {
                out.push((id, s.classe.clone()));
            }
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

pub fn nb_a_renseigner(seances: &[SeanceCahierDto]) -> usize {
    seances
        .iter()
        .filter(|s| s.contenu.is_none() && s.statut != "annulee")
        .count()
}

// ───────────────────────────── Agenda ─────────────────────────────

/// Un jour de l'agenda : travaux à rendre ce jour-là et temps estimé total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JourAgenda {
    pub jour: NaiveDate,
    pub items: Vec<AgendaItemDto>,
    pub minutes: i32,
    pub nb_controles: usize,
}

/// Regroupe l'agenda par échéance. Les contrôles annoncés passent en tête de leur jour.
pub fn jours_agenda(mut items: Vec<AgendaItemDto>) -> Vec<JourAgenda> {
    items.sort_by(|a, b| {
        let controle = |i: &AgendaItemDto| i.travail.type_travail != "controle";
        (a.travail.pour_le, controle(a), &a.travail.matiere_libelle).cmp(&(
            b.travail.pour_le,
            controle(b),
            &b.travail.matiere_libelle,
        ))
    });
    let mut out: Vec<JourAgenda> = Vec::new();
    for i in items {
        let minutes = i.travail.duree_estimee_min.unwrap_or(0);
        let controle = usize::from(i.travail.type_travail == "controle");
        match out.last_mut() {
            Some(j) if j.jour == i.travail.pour_le => {
                j.minutes += minutes;
                j.nb_controles += controle;
                j.items.push(i);
            }
            _ => out.push(JourAgenda {
                jour: i.travail.pour_le,
                minutes,
                nb_controles: controle,
                items: vec![i],
            }),
        }
    }
    out
}

/// « 1 h 30 », « 45 min ».
pub fn libelle_duree(minutes: i32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m:02}"),
    }
}

pub fn libelle_type(type_travail: &str) -> &'static str {
    match type_travail {
        "a_rendre" => "À rendre",
        "controle" => "Contrôle",
        _ => "À faire",
    }
}

/// État d'un travail pour l'élève ou sa famille.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatTravail {
    AFaire,
    Fait,
    /// À rendre en ligne, rien déposé, date limite pas encore passée.
    ARendre,
    /// À rendre en ligne, rien déposé, date limite dépassée.
    EnRetard,
    Rendu,
    RenduEnRetard,
    Corrige,
}

impl EtatTravail {
    pub fn libelle(self) -> &'static str {
        match self {
            Self::AFaire => "À faire",
            Self::Fait => "Fait",
            Self::ARendre => "À rendre",
            Self::EnRetard => "En retard",
            Self::Rendu => "Rendu",
            Self::RenduEnRetard => "Rendu en retard",
            Self::Corrige => "Corrigé",
        }
    }

    pub fn ton(self) -> TonEtat {
        match self {
            Self::AFaire | Self::ARendre => TonEtat::Info,
            Self::Fait | Self::Rendu | Self::Corrige => TonEtat::Ok,
            Self::EnRetard | Self::RenduEnRetard => TonEtat::Avertissement,
        }
    }
}

pub fn etat_travail(item: &AgendaItemDto, maintenant: DateTime<Utc>) -> EtatTravail {
    if let Some(r) = &item.remise {
        return match (r.statut.as_str(), r.en_retard) {
            ("corrige", _) => EtatTravail::Corrige,
            (_, true) => EtatTravail::RenduEnRetard,
            _ => EtatTravail::Rendu,
        };
    }
    if item.travail.rendu_en_ligne {
        let depasse = item.travail.date_limite.is_some_and(|l| maintenant > l);
        return if depasse {
            EtatTravail::EnRetard
        } else {
            EtatTravail::ARendre
        };
    }
    if item.fait {
        EtatTravail::Fait
    } else {
        EtatTravail::AFaire
    }
}

/// Peut-on proposer de déposer un fichier ? (le serveur tranche ; on évite le geste inutile)
pub fn peut_rendre(item: &AgendaItemDto, maintenant: DateTime<Utc>) -> bool {
    let t = &item.travail;
    if !t.rendu_en_ligne || item.remise.as_ref().is_some_and(|r| r.statut == "corrige") {
        return false;
    }
    let depasse = t.date_limite.is_some_and(|l| maintenant > l);
    !depasse || t.acceptation_retard
}

pub fn nb_a_faire(items: &[AgendaItemDto], maintenant: DateTime<Utc>) -> usize {
    items
        .iter()
        .filter(|i| {
            matches!(
                etat_travail(i, maintenant),
                EtatTravail::AFaire | EtatTravail::ARendre | EtatTravail::EnRetard
            )
        })
        .count()
}

// ───────────────────────────── Suivi des copies ─────────────────────────────

/// Copies à corriger d'abord, puis élèves n'ayant rien rendu, puis copies corrigées.
pub fn trier_suivi(mut lignes: Vec<LigneSuiviDto>) -> Vec<LigneSuiviDto> {
    let rang = |l: &LigneSuiviDto| match l.remise.as_ref().map(|r| r.statut.as_str()) {
        Some("rendu") => 0,
        None => 1,
        _ => 2,
    };
    lignes.sort_by(|a, b| (rang(a), &a.nom, &a.prenom).cmp(&(rang(b), &b.nom, &b.prenom)));
    lignes
}

pub fn libelle_remise(remise: Option<&RemiseDto>) -> (&'static str, TonEtat) {
    match remise {
        None => ("Non rendu", TonEtat::Neutre),
        Some(r) if r.statut == "corrige" => ("Corrigé", TonEtat::Ok),
        Some(r) if r.en_retard => ("Rendu en retard", TonEtat::Avertissement),
        Some(_) => ("Rendu", TonEtat::Info),
    }
}

// ───────────────────────────── Formulaires ─────────────────────────────

/// Erreurs de saisie par champ (identifiant du champ, message).
pub type Erreurs = Vec<(&'static str, String)>;

fn nb_car(s: &str) -> usize {
    s.chars().count()
}

/// Saisie du contenu d'une séance.
pub fn valider_contenu(contenu: &str, ressources: &str) -> Result<(String, String), Erreurs> {
    let (c, r) = (contenu.trim(), ressources.trim());
    let mut e: Erreurs = Vec::new();
    if c.is_empty() {
        e.push(("contenu", "Décrivez le contenu de la séance.".into()));
    } else if nb_car(c) > CONTENU_MAX {
        e.push(("contenu", format!("Au plus {CONTENU_MAX} caractères.")));
    }
    if nb_car(r) > RESSOURCES_MAX {
        e.push((
            "ressources",
            format!("Au plus {RESSOURCES_MAX} caractères."),
        ));
    }
    if e.is_empty() {
        Ok((c.to_string(), r.to_string()))
    } else {
        Err(e)
    }
}

/// Saisie brute du formulaire « donner un travail ».
#[derive(Clone, Debug, Default)]
pub struct SaisieTravail {
    pub type_travail: String,
    pub titre: String,
    pub consigne: String,
    /// `AAAA-MM-JJ` (champ date natif).
    pub pour_le: String,
    pub duree_min: String,
    /// Facultative : `AAAA-MM-JJ` ; sinon fin de journée de l'échéance (décidé par le serveur).
    pub limite: String,
    pub formats: String,
    pub retard_accepte: bool,
}

fn date_saisie(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

/// Fin de la journée (UTC) d'un jour : date limite choisie par l'enseignant.
pub fn fin_de_journee(d: NaiveDate) -> Option<DateTime<Utc>> {
    d.and_hms_opt(23, 59, 59).map(|t| t.and_utc())
}

/// Extensions saisies (« pdf, .DOCX ; jpeg ») → minuscules sans point, sans doublon.
pub fn formats_saisis(texte: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for f in texte.split([',', ';', ' ']) {
        let f = f.trim().trim_start_matches('.').to_ascii_lowercase();
        if !f.is_empty() && !out.contains(&f) {
            out.push(f);
        }
    }
    out
}

/// Valide la saisie et compose le corps de la requête. Le serveur revalide tout.
pub fn valider_travail(
    s: &SaisieTravail,
    seance_id: Option<Uuid>,
    classe_id: Option<Uuid>,
    aujourdhui: NaiveDate,
) -> Result<CreerTravailBody, Erreurs> {
    let mut e: Erreurs = Vec::new();
    let type_travail = match s.type_travail.as_str() {
        "a_rendre" | "controle" => s.type_travail.clone(),
        _ => "a_faire".to_string(),
    };
    let titre = s.titre.trim();
    if titre.is_empty() {
        e.push(("titre", "Donnez un titre au travail.".into()));
    } else if nb_car(titre) > TITRE_MAX {
        e.push(("titre", format!("Au plus {TITRE_MAX} caractères.")));
    }
    let consigne = s.consigne.trim();
    if consigne.is_empty() {
        e.push(("consigne", "Écrivez la consigne.".into()));
    } else if nb_car(consigne) > CONSIGNE_MAX {
        e.push(("consigne", format!("Au plus {CONSIGNE_MAX} caractères.")));
    }
    let pour_le = date_saisie(&s.pour_le);
    match pour_le {
        None => e.push(("pour-le", "Choisissez la date d'échéance.".into())),
        Some(d) if d < aujourdhui => e.push((
            "pour-le",
            "L'échéance ne peut pas être dans le passé.".into(),
        )),
        Some(_) => {}
    }
    let duree = match s.duree_min.trim() {
        "" => None,
        t => match t.parse::<i32>() {
            Ok(m) if (1..=DUREE_MAX_MIN).contains(&m) => Some(m),
            _ => {
                e.push((
                    "duree",
                    format!("Indiquez une durée entre 1 et {DUREE_MAX_MIN} minutes."),
                ));
                None
            }
        },
    };
    let a_rendre = type_travail == "a_rendre";
    let mut date_limite = None;
    if a_rendre && !s.limite.trim().is_empty() {
        match date_saisie(&s.limite) {
            Some(d) if pour_le.is_some_and(|p| d < p) => {
                e.push(("limite", "La date limite ne précède pas l'échéance.".into()))
            }
            Some(d) => date_limite = fin_de_journee(d),
            None => e.push(("limite", "Date limite invalide.".into())),
        }
    }
    if !e.is_empty() {
        return Err(e);
    }
    Ok(CreerTravailBody {
        seance_id,
        classe_id: if seance_id.is_some() { None } else { classe_id },
        groupe_id: None,
        matiere_libelle: None,
        type_travail: Some(type_travail),
        titre: titre.to_string(),
        consigne: consigne.to_string(),
        pour_le: pour_le.unwrap_or(aujourdhui),
        duree_estimee_min: duree,
        date_limite,
        acceptation_retard: a_rendre.then_some(s.retard_accepte),
        formats_acceptes: if a_rendre {
            formats_saisis(&s.formats)
        } else {
            Vec::new()
        },
    })
}

/// Commentaire de correction obligatoire.
pub fn valider_retour(commentaire: &str) -> Result<String, String> {
    let c = commentaire.trim();
    if c.is_empty() {
        Err("Écrivez un commentaire de correction.".into())
    } else if nb_car(c) > 4000 {
        Err("Au plus 4000 caractères.".into())
    } else {
        Ok(c.to_string())
    }
}

pub fn valider_commentaire_eleve(commentaire: &str) -> Result<String, String> {
    let c = commentaire.trim();
    if nb_car(c) > COMMENTAIRE_MAX {
        Err(format!("Au plus {COMMENTAIRE_MAX} caractères."))
    } else {
        Ok(c.to_string())
    }
}

/// Contrôle de forme d'un fichier choisi (nom, taille). Le contenu réel (signature, type,
/// fichier actif, antivirus) est contrôlé par le serveur.
pub fn valider_fichier(nom: &str, taille: usize) -> Result<String, String> {
    let nom = nom.trim();
    if nom.is_empty() || nb_car(nom) > 120 || nom.contains(['/', '\\', '\0']) {
        return Err("Ce nom de fichier n'est pas accepté.".into());
    }
    if taille == 0 {
        return Err("Ce fichier est vide.".into());
    }
    if taille > FICHIER_MAX_OCTETS {
        return Err("Ce fichier dépasse 10 Mo.".into());
    }
    Ok(nom.to_string())
}

/// Taille lisible (« 1,2 Mo », « 340 Ko »).
pub fn libelle_taille(octets: i64) -> String {
    const KO: f64 = 1024.0;
    let o = octets.max(0) as f64;
    if o < KO {
        format!("{octets} o")
    } else if o < KO * KO {
        format!("{} Ko", (o / KO).round() as i64)
    } else {
        format!("{:.1} Mo", o / (KO * KO)).replace('.', ",")
    }
}

/// Texte de l'échéance d'un travail à rendre (« avant le 12/11 à 23 h 59 UTC » serait trompeur :
/// on affiche la date seule, l'heure exacte étant celle du serveur).
pub fn libelle_limite(t: &TravailDto) -> Option<String> {
    t.date_limite
        .map(|l| format!("Date limite : {}", l.format("%d/%m/%Y")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ent_web_api::cahier_types::PieceCahierDto;

    /// Identifiants distincts sans générateur aléatoire (la feature `v4` n'est pas activée).
    fn uid() -> Uuid {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(1);
        Uuid::from_u128(u128::from(N.fetch_add(1, Ordering::Relaxed)))
    }

    fn d(a: i32, m: u32, j: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, j).unwrap()
    }

    fn travail(pour_le: NaiveDate, type_travail: &str, duree: Option<i32>) -> TravailDto {
        TravailDto {
            id: uid(),
            seance_id: None,
            classe_id: Some(uid()),
            groupe_id: None,
            cible: "4B".into(),
            matiere_libelle: "Maths".into(),
            auteur: "M. Martin".to_string(),
            type_travail: type_travail.into(),
            titre: "Exercices".into(),
            consigne: "p. 12".into(),
            donne_le: d(2026, 11, 1),
            pour_le,
            duree_estimee_min: duree,
            rendu_en_ligne: type_travail == "a_rendre",
            date_limite: None,
            acceptation_retard: true,
            formats_acceptes: vec![],
            version: 1,
            pieces: Vec::<PieceCahierDto>::new(),
        }
    }

    fn item(t: TravailDto, fait: bool, remise: Option<RemiseDto>) -> AgendaItemDto {
        AgendaItemDto {
            travail: t,
            fait,
            vu: false,
            remise,
        }
    }

    fn remise(statut: &str, en_retard: bool) -> RemiseDto {
        RemiseDto {
            id: uid(),
            travail_id: uid(),
            eleve_id: uid(),
            soumise_le: Utc::now(),
            derniere_le: Utc::now(),
            en_retard,
            commentaire_eleve: String::new(),
            statut: statut.into(),
            retour_commentaire: None,
            retour_le: None,
            note_id: None,
            pieces: vec![],
        }
    }

    #[test]
    fn semaine_commence_le_lundi() {
        // 2026-11-05 est un jeudi.
        assert_eq!(lundi_de(d(2026, 11, 5)), d(2026, 11, 2));
        assert_eq!(lundi_de(d(2026, 11, 2)), d(2026, 11, 2));
        assert_eq!(
            lundi_de(d(2026, 11, 8)),
            d(2026, 11, 2),
            "dimanche → lundi précédent"
        );
        assert_eq!(
            semaine_demandee(Some("2026-11-11".into()), d(2026, 1, 1)),
            d(2026, 11, 9)
        );
        assert_eq!(
            semaine_demandee(Some("n'importe quoi".into()), d(2026, 11, 5)),
            d(2026, 11, 2)
        );
        assert_eq!(semaine_demandee(None, d(2026, 11, 5)), d(2026, 11, 2));
        assert_eq!(semaine_voisine(d(2026, 11, 2), -1), d(2026, 10, 26));
        assert_eq!(fin_de_semaine(d(2026, 11, 2)), d(2026, 11, 8));
    }

    #[test]
    fn agenda_groupe_par_echeance_controles_en_tete() {
        let a = item(travail(d(2026, 11, 10), "a_faire", Some(30)), false, None);
        let b = item(travail(d(2026, 11, 10), "controle", None), false, None);
        let c = item(travail(d(2026, 11, 9), "a_faire", Some(45)), true, None);
        let jours = jours_agenda(vec![a.clone(), b.clone(), c.clone()]);
        assert_eq!(jours.len(), 2);
        assert_eq!(jours[0].jour, d(2026, 11, 9));
        assert_eq!((jours[0].minutes, jours[0].nb_controles), (45, 0));
        assert_eq!((jours[1].minutes, jours[1].nb_controles), (30, 1));
        assert_eq!(
            jours[1].items[0].travail.id, b.travail.id,
            "le contrôle passe en premier"
        );
        assert!(jours_agenda(vec![]).is_empty());
    }

    #[test]
    fn durees_lisibles() {
        assert_eq!(libelle_duree(45), "45 min");
        assert_eq!(libelle_duree(60), "1 h");
        assert_eq!(libelle_duree(90), "1 h 30");
        assert_eq!(libelle_duree(125), "2 h 05");
        assert_eq!(libelle_taille(0), "0 o");
        assert_eq!(libelle_taille(2048), "2 Ko");
        assert_eq!(libelle_taille(1_572_864), "1,5 Mo");
    }

    #[test]
    fn etats_d_un_travail() {
        let now = Utc::now();
        let simple = item(travail(d(2026, 11, 10), "a_faire", None), false, None);
        assert_eq!(etat_travail(&simple, now), EtatTravail::AFaire);
        assert_eq!(
            etat_travail(
                &AgendaItemDto {
                    fait: true,
                    ..simple
                },
                now
            ),
            EtatTravail::Fait
        );

        let mut t = travail(d(2026, 11, 10), "a_rendre", None);
        t.date_limite = Some(now + Duration::days(1));
        let a_rendre = item(t.clone(), false, None);
        assert_eq!(etat_travail(&a_rendre, now), EtatTravail::ARendre);
        assert!(peut_rendre(&a_rendre, now));

        t.date_limite = Some(now - Duration::hours(1));
        let echu = item(t.clone(), false, None);
        assert_eq!(etat_travail(&echu, now), EtatTravail::EnRetard);
        assert!(peut_rendre(&echu, now), "retard accepté par défaut");
        t.acceptation_retard = false;
        assert!(
            !peut_rendre(&item(t.clone(), false, None), now),
            "retard refusé : plus de dépôt"
        );

        let rendu = item(t.clone(), false, Some(remise("rendu", false)));
        let tard = item(t.clone(), false, Some(remise("rendu", true)));
        let corrige = item(t, false, Some(remise("corrige", true)));
        assert_eq!(etat_travail(&rendu, now), EtatTravail::Rendu);
        assert_eq!(etat_travail(&tard, now), EtatTravail::RenduEnRetard);
        assert_eq!(etat_travail(&corrige, now), EtatTravail::Corrige);
        assert!(!peut_rendre(&corrige, now), "corrigé : plus de dépôt");
        assert_eq!(nb_a_faire(&[a_rendre, echu, rendu, simple_fait()], now), 2);
    }

    fn simple_fait() -> AgendaItemDto {
        item(travail(d(2026, 11, 10), "a_faire", None), true, None)
    }

    #[test]
    fn suivi_copies_a_corriger_d_abord() {
        let ligne = |nom: &str, r: Option<RemiseDto>| LigneSuiviDto {
            eleve_id: uid(),
            nom: nom.into(),
            prenom: "X".into(),
            remise: r,
        };
        let v = trier_suivi(vec![
            ligne("Dupont", Some(remise("corrige", false))),
            ligne("Martin", None),
            ligne("Bernard", Some(remise("rendu", false))),
            ligne("Abel", None),
        ]);
        let noms: Vec<&str> = v.iter().map(|l| l.nom.as_str()).collect();
        assert_eq!(noms, ["Bernard", "Abel", "Martin", "Dupont"]);
        assert_eq!(libelle_remise(None).0, "Non rendu");
        assert_eq!(
            libelle_remise(Some(&remise("rendu", true))).0,
            "Rendu en retard"
        );
        assert_eq!(libelle_remise(Some(&remise("corrige", true))).0, "Corrigé");
    }

    fn saisie() -> SaisieTravail {
        SaisieTravail {
            type_travail: "a_faire".into(),
            titre: " Exercices ".into(),
            consigne: "p. 12 n° 3".into(),
            pour_le: "2026-11-10".into(),
            duree_min: "30".into(),
            ..Default::default()
        }
    }

    #[test]
    fn saisie_de_travail_valide_et_normalisee() {
        let seance = uid();
        let b = valider_travail(&saisie(), Some(seance), None, d(2026, 11, 5)).unwrap();
        assert_eq!(b.titre, "Exercices");
        assert_eq!((b.seance_id, b.classe_id), (Some(seance), None));
        assert_eq!(b.duree_estimee_min, Some(30));
        assert_eq!(b.type_travail.as_deref(), Some("a_faire"));
        assert!(
            b.formats_acceptes.is_empty()
                && b.date_limite.is_none()
                && b.acceptation_retard.is_none()
        );
        // Sans séance : la classe choisie est envoyée.
        let classe = uid();
        let b = valider_travail(&saisie(), None, Some(classe), d(2026, 11, 5)).unwrap();
        assert_eq!((b.seance_id, b.classe_id), (None, Some(classe)));
    }

    #[test]
    fn saisie_de_travail_refus_par_champ() {
        let champs = |s: SaisieTravail| -> Vec<&'static str> {
            valider_travail(&s, None, None, d(2026, 11, 5))
                .unwrap_err()
                .into_iter()
                .map(|(c, _)| c)
                .collect()
        };
        assert_eq!(
            champs(SaisieTravail::default()),
            ["titre", "consigne", "pour-le"]
        );
        assert_eq!(
            champs(SaisieTravail {
                pour_le: "2026-11-04".into(),
                ..saisie()
            }),
            ["pour-le"]
        );
        assert_eq!(
            champs(SaisieTravail {
                pour_le: "10/11/2026".into(),
                ..saisie()
            }),
            ["pour-le"]
        );
        assert_eq!(
            champs(SaisieTravail {
                duree_min: "0".into(),
                ..saisie()
            }),
            ["duree"]
        );
        assert_eq!(
            champs(SaisieTravail {
                duree_min: "601".into(),
                ..saisie()
            }),
            ["duree"]
        );
        assert_eq!(
            champs(SaisieTravail {
                duree_min: "abc".into(),
                ..saisie()
            }),
            ["duree"]
        );
        assert_eq!(
            champs(SaisieTravail {
                titre: "x".repeat(201),
                ..saisie()
            }),
            ["titre"]
        );
        let rendre = SaisieTravail {
            type_travail: "a_rendre".into(),
            ..saisie()
        };
        assert_eq!(
            champs(SaisieTravail {
                limite: "2026-11-09".into(),
                ..rendre.clone()
            }),
            ["limite"]
        );
        assert_eq!(
            champs(SaisieTravail {
                limite: "demain".into(),
                ..rendre
            }),
            ["limite"]
        );
    }

    #[test]
    fn travail_a_rendre_limite_formats_et_retard() {
        let s = SaisieTravail {
            type_travail: "a_rendre".into(),
            limite: "2026-11-12".into(),
            formats: "PDF, .docx ; pdf".into(),
            retard_accepte: false,
            ..saisie()
        };
        let b = valider_travail(&s, None, Some(uid()), d(2026, 11, 5)).unwrap();
        assert_eq!(b.formats_acceptes, ["pdf", "docx"]);
        assert_eq!(b.acceptation_retard, Some(false));
        assert_eq!(
            b.date_limite
                .unwrap()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            "2026-11-12 23:59:59"
        );
        // Un type inconnu retombe sur « à faire » (jamais de type libre envoyé au serveur).
        let b = valider_travail(
            &SaisieTravail {
                type_travail: "autre".into(),
                ..saisie()
            },
            None,
            None,
            d(2026, 11, 5),
        )
        .unwrap();
        assert_eq!(b.type_travail.as_deref(), Some("a_faire"));
    }

    #[test]
    fn contenu_retour_et_fichier() {
        assert_eq!(
            valider_contenu("  Fractions ", " p. 3 ").unwrap(),
            ("Fractions".into(), "p. 3".into())
        );
        assert_eq!(valider_contenu(" ", "").unwrap_err()[0].0, "contenu");
        assert_eq!(
            valider_contenu("ok", &"x".repeat(RESSOURCES_MAX + 1)).unwrap_err()[0].0,
            "ressources"
        );
        assert!(valider_retour("  ").is_err());
        assert_eq!(valider_retour(" Bien ").unwrap(), "Bien");
        assert!(valider_commentaire_eleve(&"x".repeat(COMMENTAIRE_MAX + 1)).is_err());
        assert_eq!(valider_fichier("copie.pdf", 10).unwrap(), "copie.pdf");
        assert!(valider_fichier("", 10).is_err());
        assert!(valider_fichier("../x.pdf", 10).is_err());
        assert!(valider_fichier("a\\b.pdf", 10).is_err());
        assert!(valider_fichier("a.pdf", 0).is_err());
        assert!(valider_fichier("a.pdf", FICHIER_MAX_OCTETS + 1).is_err());
    }

    #[test]
    fn seances_regroupees_et_classes() {
        let s = |jour: NaiveDate, h: &str, classe: &str, cid: Option<Uuid>| SeanceCahierDto {
            seance_id: uid(),
            jour,
            heure_debut: h.into(),
            heure_fin: "09:00:00".into(),
            classe_id: cid,
            classe: classe.into(),
            matiere_libelle: "Maths".into(),
            statut: "prevue".into(),
            contenu: None,
            nb_travaux: 0,
        };
        let c1 = uid();
        let v = vec![
            s(d(2026, 11, 3), "10:00:00", "4B", Some(c1)),
            s(d(2026, 11, 2), "14:00:00", "5A", Some(uid())),
            s(d(2026, 11, 3), "08:00:00", "4B", Some(c1)),
            s(d(2026, 11, 3), "11:00:00", "4B", None),
        ];
        let classes = classes_des_seances(&v);
        assert_eq!(
            classes.iter().map(|c| c.1.as_str()).collect::<Vec<_>>(),
            ["4B", "5A"]
        );
        assert_eq!(nb_a_renseigner(&v), 4);
        let par_jour = seances_par_jour(v);
        assert_eq!(par_jour.len(), 2);
        assert_eq!(
            par_jour[1].1[0].heure_debut, "08:00:00",
            "ordre chronologique dans la journée"
        );
    }
}
