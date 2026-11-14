//! Logique d'écran pure du lot C2 (saisie et publication) : sans Leptos ni réseau, testée en natif.
//!
//! **Aucune règle métier ici** : droits, plage calme, échéances et versions restent décidés par le serveur
//! (`ent-db`). Ce module ne fait que des contrôles de forme (champ vide, nombre, date lisible) pour épargner un
//! aller-retour, et formate ce que le serveur annonce (« visible lundi à 7 h »). Les heures affichées sont celles
//! **de l'établissement**, données par le serveur : aucun fuseau n'est deviné ici.

use chrono::{NaiveDate, NaiveTime};
use ent_web_api::cahier_c2_types::{EntreeBody, EntreeDto, ModifierTravailC2Body, TravailC2Body};
use ent_web_ui::TonEtat;
use uuid::Uuid;

use super::modele::Erreurs;
use crate::noyau::temps::date_longue;

/// Saisie de l'entrée d'une séance telle que lue dans le formulaire.
#[derive(Clone, Debug, Default)]
pub struct SaisieEntree {
    pub titre: String,
    pub contenu: String,
    /// Une notion par ligne.
    pub notions: String,
    /// `brouillon`, `maintenant` ou `programmer`.
    pub publication: String,
    pub jour: String,
    pub heure: String,
    pub sans_avis: bool,
    pub motif: String,
}

fn nb_car(s: &str) -> usize {
    s.chars().count()
}

/// Date `AAAA-MM-JJ` d'un champ `date` natif.
pub fn lire_jour(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

/// Heure `HH:MM` d'un champ `time` natif.
pub fn lire_heure_champ(s: &str) -> Option<NaiveTime> {
    let s = s.trim();
    NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .ok()
}

/// « 07:00 » → « 7 h » ; « 18:30 » → « 18 h 30 ».
pub fn heure_en_clair(hhmm: &str) -> String {
    match hhmm.split_once(':') {
        Some((h, m)) => {
            let h = h.trim_start_matches('0');
            let h = if h.is_empty() { "0" } else { h };
            if m == "00" {
                format!("{h} h")
            } else {
                format!("{h} h {m}")
            }
        }
        None => hhmm.to_string(),
    }
}

pub fn valider_entree(s: &SaisieEntree) -> Result<EntreeBody, Erreurs> {
    let mut e: Erreurs = Vec::new();
    let contenu = s.contenu.trim();
    if contenu.is_empty() {
        e.push(("contenu", "Décrivez le contenu de la séance.".into()));
    } else if nb_car(contenu) > 20_000 {
        e.push(("contenu", "Au plus 20 000 caractères.".into()));
    }
    if nb_car(s.titre.trim()) > 200 {
        e.push(("titre", "Au plus 200 caractères.".into()));
    }
    let notions: Vec<String> = s
        .notions
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if notions.len() > 20 {
        e.push(("notions", "Vingt notions au plus.".into()));
    } else if notions.iter().any(|n| nb_car(n) > 200) {
        e.push(("notions", "Chaque notion : 200 caractères au plus.".into()));
    }
    if !matches!(s.publication.as_str(), "brouillon" | "maintenant" | "programmer") {
        e.push(("publication", "Choisissez : brouillon, publier maintenant ou programmer.".into()));
    }
    let (mut publie_jour, mut publie_heure) = (None, None);
    if s.publication == "programmer" {
        match lire_jour(&s.jour) {
            Some(j) => publie_jour = Some(j),
            None => e.push(("jour", "Indiquez le jour de publication.".into())),
        }
        if !s.heure.trim().is_empty() {
            match lire_heure_champ(&s.heure) {
                Some(h) => publie_heure = Some(h),
                None => e.push(("heure", "Heure illisible (exemple : 07:30).".into())),
            }
        }
    }
    if !e.is_empty() {
        return Err(e);
    }
    let t = s.titre.trim();
    let m = s.motif.trim();
    Ok(EntreeBody {
        titre: (!t.is_empty()).then(|| t.to_string()),
        contenu: contenu.to_string(),
        notions,
        publication: s.publication.clone(),
        publie_a: None,
        publie_jour,
        publie_heure,
        sans_avis: s.sans_avis,
        motif: (!m.is_empty()).then(|| m.to_string()),
    })
}

/// Libellé d'état d'une entrée et ton du badge.
pub fn etat_entree(etat: &str) -> (&'static str, TonEtat) {
    match etat {
        "publiee" => ("Publiée", TonEtat::Ok),
        "programmee" => ("Programmée", TonEtat::Info),
        "retiree" => ("Retirée", TonEtat::Neutre),
        _ => ("Brouillon", TonEtat::Avertissement),
    }
}

/// « Visible Lundi 17 mars à 7 h » quand la publication est décalée ou programmée ; vide sinon.
pub fn libelle_visibilite(e: &EntreeDto) -> Option<String> {
    if e.etat != "publiee" && e.etat != "programmee" {
        return None;
    }
    let (Some(j), Some(h)) = (e.visible_jour, e.visible_heure.as_deref()) else {
        return None;
    };
    if e.etat == "programmee" || e.differee_par_plage_calme {
        Some(format!("Visible des élèves : {} à {}", date_longue(j).to_lowercase(), heure_en_clair(h)))
    } else {
        None
    }
}

/// Rappel affiché à l'auteur quand la plage calme décale sa publication.
pub const RAPPEL_PLAGE_CALME: &str = "Publié pendant la plage calme (20 h – 7 h, week-end, vacances) : la publication et l'avis aux familles partent à la fin de la plage, dans le respect du droit à la déconnexion.";

/// Phrase d'un avertissement non bloquant renvoyé à la création d'un travail.
pub fn message_avertissement(code: &str) -> &'static str {
    match code {
        "donne_moins_de_24h" => "Ce travail est donné moins de 24 h avant son échéance.",
        "differe_par_plage_calme" => "Publication différée par la plage calme : visible à la fin de la plage.",
        _ => "Avertissement.",
    }
}

/// Saisie du travail à donner (écran 2).
#[derive(Clone, Debug, Default)]
pub struct SaisieTravailC2 {
    pub cours: Vec<Uuid>,
    pub seance_donnee: Option<Uuid>,
    /// `prochaine` ou `date`.
    pub echeance: String,
    pub jour: String,
    pub heure: String,
    pub nature: String,
    pub titre: String,
    pub consigne: String,
    pub duree: String,
    /// `aucun`, `en_ligne`, `en_classe`.
    pub rendu: String,
    pub retard_accepte: bool,
    pub formats: String,
    pub ia: String,
    pub publication: String,
    pub publie_jour: String,
    pub publie_heure: String,
}

pub fn valider_travail_c2(s: &SaisieTravailC2) -> Result<TravailC2Body, Erreurs> {
    let mut e: Erreurs = Vec::new();
    if s.cours.is_empty() {
        e.push(("cours", "Choisissez au moins un cours.".into()));
    }
    if s.titre.trim().is_empty() {
        e.push(("titre", "Donnez un titre au travail.".into()));
    } else if nb_car(s.titre.trim()) > 200 {
        e.push(("titre", "Au plus 200 caractères.".into()));
    }
    if s.consigne.trim().is_empty() {
        e.push(("consigne", "Écrivez la consigne.".into()));
    } else if nb_car(s.consigne.trim()) > 20_000 {
        e.push(("consigne", "Au plus 20 000 caractères.".into()));
    }
    if s.nature.trim().is_empty() {
        e.push(("nature", "Choisissez la nature du travail.".into()));
    }
    let duree = if s.duree.trim().is_empty() {
        None
    } else {
        match s.duree.trim().parse::<i16>() {
            Ok(d) if (1..=600).contains(&d) => Some(d),
            _ => {
                e.push(("duree", "Durée en minutes, de 1 à 600.".into()));
                None
            }
        }
    };
    let (mut echeance_jour, mut echeance_heure, prochaine) = (None, None, s.echeance != "date");
    if s.echeance == "date" {
        match lire_jour(&s.jour) {
            Some(j) => echeance_jour = Some(j),
            None => e.push(("jour", "Indiquez le jour de l'échéance.".into())),
        }
        if !s.heure.trim().is_empty() {
            match lire_heure_champ(&s.heure) {
                Some(h) => echeance_heure = Some(h),
                None => e.push(("heure", "Heure illisible (exemple : 18:00).".into())),
            }
        }
    }
    let formats: Vec<String> = s
        .formats
        .split(',')
        .map(|f| f.trim().trim_start_matches('.').to_lowercase())
        .filter(|f| !f.is_empty())
        .collect();
    if !matches!(s.publication.as_str(), "brouillon" | "maintenant" | "programmer") {
        e.push(("publication", "Choisissez : brouillon, publier maintenant ou programmer.".into()));
    }
    let (mut publie_jour, mut publie_heure) = (None, None);
    if s.publication == "programmer" {
        match lire_jour(&s.publie_jour) {
            Some(j) => publie_jour = Some(j),
            None => e.push(("publie_jour", "Indiquez le jour de publication.".into())),
        }
        if !s.publie_heure.trim().is_empty() {
            match lire_heure_champ(&s.publie_heure) {
                Some(h) => publie_heure = Some(h),
                None => e.push(("publie_heure", "Heure illisible (exemple : 07:30).".into())),
            }
        }
    }
    if !e.is_empty() {
        return Err(e);
    }
    Ok(TravailC2Body {
        cours_ids: s.cours.clone(),
        seance_donnee_id: s.seance_donnee,
        echeance_seance_id: None,
        echeance_prochaine_seance: prochaine,
        echeance_jour,
        echeance_heure,
        nature: s.nature.trim().to_string(),
        titre: s.titre.trim().to_string(),
        consigne: s.consigne.trim().to_string(),
        duree_estimee_min: duree,
        rendu: if s.rendu.is_empty() { "aucun".into() } else { s.rendu.clone() },
        retard_accepte: s.retard_accepte,
        formats,
        eleve_ids: Vec::new(),
        ia_autorisee: if s.ia.is_empty() { "non".into() } else { s.ia.clone() },
        publication: s.publication.clone(),
        publie_a: None,
        publie_jour,
        publie_heure,
    })
}

/// Modification d'un travail : seuls les champs changés sont envoyés.
#[derive(Clone, Debug, Default)]
pub struct SaisieModification {
    pub titre: String,
    pub consigne: String,
    pub jour: String,
    pub heure: String,
    pub sans_avis: bool,
    pub motif: String,
}

/// Valeurs d'origine du travail (pour ne renvoyer que ce qui change).
#[derive(Clone, Debug)]
pub struct OrigineTravail {
    pub titre: String,
    pub consigne: String,
    pub jour: NaiveDate,
    pub heure: String,
}

/// Corps de modification et indicateur « échéance avancée » (confirmation demandée **avant** l'envoi, le serveur
/// la réclame de toute façon).
pub fn valider_modification(
    s: &SaisieModification,
    o: &OrigineTravail,
) -> Result<(ModifierTravailC2Body, bool), Erreurs> {
    let mut e: Erreurs = Vec::new();
    if s.titre.trim().is_empty() {
        e.push(("titre", "Le titre ne peut pas être vide.".into()));
    }
    if s.consigne.trim().is_empty() {
        e.push(("consigne", "La consigne ne peut pas être vide.".into()));
    }
    let jour = match lire_jour(&s.jour) {
        Some(j) => j,
        None => {
            e.push(("jour", "Indiquez le jour de l'échéance.".into()));
            o.jour
        }
    };
    let heure = if s.heure.trim().is_empty() {
        None
    } else {
        match lire_heure_champ(&s.heure) {
            Some(h) => Some(h),
            None => {
                e.push(("heure", "Heure illisible (exemple : 18:00).".into()));
                None
            }
        }
    };
    if !e.is_empty() {
        return Err(e);
    }
    let heure_txt = heure.map(|h| h.format("%H:%M").to_string()).unwrap_or_else(|| o.heure.clone());
    let echeance_change = jour != o.jour || heure_txt != o.heure;
    let avancee = jour < o.jour || (jour == o.jour && heure_txt < o.heure);
    let m = s.motif.trim();
    Ok((
        ModifierTravailC2Body {
            titre: (s.titre.trim() != o.titre).then(|| s.titre.trim().to_string()),
            consigne: (s.consigne.trim() != o.consigne).then(|| s.consigne.trim().to_string()),
            echeance_jour: echeance_change.then_some(jour),
            echeance_heure: echeance_change.then(|| heure.or_else(|| lire_heure_champ(&o.heure))).flatten(),
            confirme_echeance_avancee: false,
            sans_avis: s.sans_avis,
            motif: (!m.is_empty()).then(|| m.to_string()),
            ..Default::default()
        },
        avancee,
    ))
}

/// Libellé d'un état de travail.
pub fn etat_travail(etat: &str) -> (&'static str, TonEtat) {
    match etat {
        "publie" => ("Publié", TonEtat::Ok),
        "programme" => ("Programmé", TonEtat::Info),
        "retire" => ("Retiré", TonEtat::Neutre),
        _ => ("Brouillon", TonEtat::Avertissement),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entree(pub_: &str) -> SaisieEntree {
        SaisieEntree { contenu: "Fractions".into(), publication: pub_.into(), ..Default::default() }
    }

    #[test]
    fn contenu_vide_refuse_et_notions_une_par_ligne() {
        let mut s = entree("maintenant");
        s.contenu = "  ".into();
        assert!(valider_entree(&s).is_err_and(|e| e.iter().any(|(c, _)| *c == "contenu")));
        let mut s = entree("brouillon");
        s.notions = "fractions\n\n  proportions \n".into();
        let b = valider_entree(&s).unwrap();
        assert_eq!(b.notions, vec!["fractions", "proportions"]);
        assert_eq!(b.publication, "brouillon");
    }

    #[test]
    fn programmer_exige_un_jour_lisible_et_n_envoie_jamais_d_utc() {
        let mut s = entree("programmer");
        assert!(valider_entree(&s).is_err_and(|e| e.iter().any(|(c, _)| *c == "jour")));
        s.jour = "2031-03-11".into();
        s.heure = "22:00".into();
        let b = valider_entree(&s).unwrap();
        assert_eq!(b.publie_jour, lire_jour("2031-03-11"));
        assert_eq!(b.publie_heure, lire_heure_champ("22:00"));
        assert!(b.publie_a.is_none(), "l'heure est celle de l'établissement, jamais un instant UTC");
        s.heure = "vingt".into();
        assert!(valider_entree(&s).is_err_and(|e| e.iter().any(|(c, _)| *c == "heure")));
        s.heure.clear();
        assert_eq!(valider_entree(&s).unwrap().publie_heure, None, "heure absente : le serveur applique 7 h");
    }

    #[test]
    fn heure_en_clair_et_visibilite_annoncee() {
        assert_eq!(heure_en_clair("07:00"), "7 h");
        assert_eq!(heure_en_clair("18:30"), "18 h 30");
        assert_eq!(heure_en_clair("00:00"), "0 h");
        let mut e = EntreeDto {
            id: Uuid::nil(),
            cours_id: Uuid::nil(),
            seance_id: Uuid::nil(),
            titre: None,
            contenu: "x".into(),
            notions: vec![],
            etat: "programmee".into(),
            publie_a: None,
            visible_a: None,
            visible_jour: lire_jour("2031-03-17"),
            visible_heure: Some("07:00".into()),
            differee_par_plage_calme: true,
            auteur: "A".into(),
            version: 1,
            modifie_le: chrono::DateTime::UNIX_EPOCH,
            renseignee_le: None,
            retire_motif: None,
            copie_de_id: None,
            contributions: vec![],
        };
        assert_eq!(libelle_visibilite(&e).as_deref(), Some("Visible des élèves : lundi 17 mars à 7 h"));
        e.etat = "brouillon".into();
        assert_eq!(libelle_visibilite(&e), None);
        e.etat = "publiee".into();
        e.differee_par_plage_calme = false;
        assert_eq!(libelle_visibilite(&e), None, "publiée et visible : rien à annoncer");
    }

    fn travail() -> SaisieTravailC2 {
        SaisieTravailC2 {
            cours: vec![Uuid::from_u128(1)],
            echeance: "prochaine".into(),
            nature: "exercices".into(),
            titre: "Exercices".into(),
            consigne: "p. 12".into(),
            publication: "maintenant".into(),
            retard_accepte: true,
            ..Default::default()
        }
    }

    #[test]
    fn travail_en_trois_gestes_echeance_prochaine_seance_par_defaut() {
        let b = valider_travail_c2(&travail()).unwrap();
        assert!(b.echeance_prochaine_seance && b.echeance_jour.is_none());
        assert_eq!((b.rendu.as_str(), b.ia_autorisee.as_str()), ("aucun", "non"));
        assert_eq!(b.duree_estimee_min, None, "durée absente : le serveur propose celle de la nature");
    }

    #[test]
    fn travail_controles_de_forme() {
        let mut s = travail();
        s.cours.clear();
        s.titre.clear();
        s.duree = "9999".into();
        let e = valider_travail_c2(&s).unwrap_err();
        for champ in ["cours", "titre", "duree"] {
            assert!(e.iter().any(|(c, _)| *c == champ), "{champ}");
        }
        let mut s = travail();
        s.echeance = "date".into();
        assert!(valider_travail_c2(&s).is_err_and(|e| e.iter().any(|(c, _)| *c == "jour")));
        s.jour = "2031-03-12".into();
        s.heure = "18:00".into();
        s.formats = ".PDF, docx ,".into();
        s.rendu = "en_ligne".into();
        let b = valider_travail_c2(&s).unwrap();
        assert!(!b.echeance_prochaine_seance);
        assert_eq!(b.formats, vec!["pdf", "docx"]);
    }

    #[test]
    fn modification_n_envoie_que_ce_qui_change_et_detecte_l_echeance_avancee() {
        let o = OrigineTravail {
            titre: "T".into(),
            consigne: "C".into(),
            jour: lire_jour("2031-03-14").unwrap(),
            heure: "18:00".into(),
        };
        let mut s = SaisieModification {
            titre: "T".into(),
            consigne: "C modifiée".into(),
            jour: "2031-03-14".into(),
            heure: "18:00".into(),
            sans_avis: true,
            motif: String::new(),
        };
        let (b, avancee) = valider_modification(&s, &o).unwrap();
        assert_eq!((b.titre, b.consigne.as_deref()), (None, Some("C modifiée")));
        assert!(b.echeance_jour.is_none() && !avancee && b.sans_avis);
        s.jour = "2031-03-12".into();
        let (b, avancee) = valider_modification(&s, &o).unwrap();
        assert!(avancee, "plus tôt : confirmation demandée avant l'envoi");
        assert_eq!(b.echeance_jour, lire_jour("2031-03-12"));
        s.jour = "2031-03-17".into();
        assert!(!valider_modification(&s, &o).unwrap().1, "reculer ne demande rien");
        s.titre.clear();
        assert!(valider_modification(&s, &o).is_err());
    }
}
