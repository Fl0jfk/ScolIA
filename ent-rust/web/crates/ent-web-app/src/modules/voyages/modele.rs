//! Logique d'écran pure des sorties et voyages (socle 42, lot V0a) : libellés, tons, regroupement
//! par mois, composition et contrôle de la saisie. **Aucune règle métier** : droits, états et
//! préconditions sont décidés par le serveur ; l'écran n'affiche que ce qu'il lui renvoie (les
//! boutons d'état viennent de `SortieDetailDto::commandes`).

use chrono::{Datelike, NaiveDate, NaiveTime};
use ent_types::voyages::{SortieResumeDto, SortieSaisieDto};
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::noyau::temps::{a_paris, nom_mois, paris_vers_utc};

pub const NATURES: [(&str, &str); 2] = [
    ("facultative", "Facultative (autorisation des responsables demandée)"),
    ("obligatoire", "Obligatoire (sur le temps scolaire, gratuite)"),
];
pub const FORMES: [(&str, &str); 2] = [
    ("sans_nuitee", "Sortie sans nuitée"),
    ("voyage", "Voyage avec nuitées"),
];
pub const DESTINATIONS: [(&str, &str); 4] = [
    ("proximite", "Proximité"),
    ("france", "France"),
    ("frontalier", "Zone frontalière"),
    ("etranger", "Étranger"),
];
pub const ETATS_FILTRE: [(&str, &str); 10] = [
    ("", "Tous les états"),
    ("brouillon", "Brouillon"),
    ("soumise", "Soumise à la direction"),
    ("a_completer", "À compléter"),
    ("preparation", "En préparation"),
    ("budget_a_valider", "Budget à valider"),
    ("a_autoriser", "À autoriser"),
    ("ouverte", "Ouverte"),
    ("prete", "Prête"),
    ("annulee", "Annulée"),
];
pub const ROLES_ENCADRANT: [(&str, &str); 4] = [
    ("responsable", "Responsable"),
    ("encadrant", "Encadrant"),
    ("non_compte", "Accompagnant non compté"),
    ("intervenant", "Intervenant"),
];
pub const MOTIFS_SORTIE: [(&str, &str); 5] = [
    ("sante", "Santé"),
    ("famille", "Raison familiale"),
    ("decision_personnelle", "Décision personnelle"),
    ("exigence_manquante", "Exigence non remplie"),
    ("autre", "Autre"),
];

pub fn libelle_etat(etat: &str) -> &'static str {
    match etat {
        "brouillon" => "Brouillon",
        "soumise" => "Soumise à la direction",
        "a_completer" => "À compléter",
        "preparation" => "En préparation",
        "a_signer" => "Devis à signer",
        "budget_a_valider" => "Budget à valider",
        "vote" => "En attente du conseil",
        "a_autoriser" => "À autoriser",
        "ouverte" => "Ouverte",
        "prete" => "Prête",
        "en_cours" => "En cours",
        "rentree" => "Rentrée",
        "close" => "Close",
        "refusee" => "Refusée",
        "annulee" => "Annulée",
        _ => "État inconnu",
    }
}

pub fn ton_etat(etat: &str) -> TonEtat {
    match etat {
        "prete" | "en_cours" | "close" | "rentree" => TonEtat::Ok,
        "ouverte" | "preparation" | "a_signer" => TonEtat::Info,
        "soumise" | "a_completer" | "budget_a_valider" | "a_autoriser" | "vote" => {
            TonEtat::Avertissement
        }
        "refusee" | "annulee" => TonEtat::Mal,
        _ => TonEtat::Neutre,
    }
}

pub fn libelle_destination(code: &str) -> &'static str {
    DESTINATIONS
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| *l)
        .unwrap_or("Destination inconnue")
}

pub fn libelle_role_encadrant(code: &str) -> &'static str {
    ROLES_ENCADRANT
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| *l)
        .unwrap_or("Encadrant")
}

pub fn libelle_etat_participant(etat: &str) -> &'static str {
    match etat {
        "inscrit" => "Inscrit",
        "pret" => "Prêt",
        "bloque" => "Bloqué",
        "parti" => "Parti",
        "rentre" => "Rentré",
        "remis" => "Remis",
        "desiste" => "Désisté",
        "retire" => "Retiré",
        _ => "—",
    }
}

/// Qui doit agir (champ `qui` de la prochaine étape).
pub fn libelle_qui(qui: &str) -> &'static str {
    match qui {
        "porteur" => "À vous, porteur de la sortie",
        "direction" => "À la direction",
        "comptabilite" => "À la comptabilité",
        "secretariat" => "Au secrétariat",
        _ => "Rien à faire pour le moment",
    }
}

pub fn ton_gravite(gravite: &str) -> TonEtat {
    match gravite {
        "bloquant" => TonEtat::Mal,
        "avertissement" => TonEtat::Avertissement,
        _ => TonEtat::Info,
    }
}

pub fn libelle_gravite(gravite: &str) -> &'static str {
    match gravite {
        "bloquant" => "À traiter",
        "avertissement" => "À surveiller",
        _ => "Conseil",
    }
}

/// Phrase lisible d'une ligne du journal.
pub fn libelle_journal(action: &str) -> &'static str {
    match action {
        "creee" => "Sortie créée",
        "modifiee" => "Projet modifié",
        "modifiee_apres_ouverture" => "Modification après l'ouverture : avis aux familles à proposer",
        "soumise" => "Soumise à la direction",
        "validee_pedagogie" => "Pédagogie validée",
        "a_completer" => "Modifications demandées",
        "demande_annulee" => "Demande de modifications annulée",
        "resoumise" => "Renvoyée après modification",
        "refusee" => "Refusée",
        "signature_demandee" => "Devis présenté à la signature",
        "passee_aux_finances" => "Passée à l'étape suivante",
        "budget_valide" => "Budget validé",
        "autorisee" => "Autorisée",
        "liste_confirmee" => "Liste des élèves confirmée",
        "liste_rouverte" => "Liste rouverte : à reconfirmer",
        "rouverte" => "Rouverte",
        "annulee" => "Annulée",
        "requalifiee" => "Requalifiée en sortie avec car",
        "porteur_change" => "Porteur changé",
        "programme_modifie" => "Programme modifié",
        "participant_inscrit" => "Élèves inscrits",
        "participant_retire" => "Élève retiré",
        "participant_desiste" => "Désistement",
        "encadrant_ajoute" => "Encadrant ajouté",
        "encadrant_retire" => "Encadrant retiré",
        "registre_consulte" => "Registre des élèves consulté",
        "liste_consultee" => "Liste nominative consultée",
        "liste_exportee" => "Liste exportée",
        "circulaire_generee" => "Circulaire préparée",
        _ => "Action enregistrée",
    }
}

/// Dates d'une sortie en heure de Paris : « 12/03/2027 08:00 → 17:30 » ou sur plusieurs jours.
pub fn periode(debut: chrono::DateTime<chrono::Utc>, fin: chrono::DateTime<chrono::Utc>) -> String {
    let (d, f) = (a_paris(debut), a_paris(fin));
    if d.date() == f.date() {
        format!("{} {} → {}", d.format("%d/%m/%Y"), d.format("%H:%M"), f.format("%H:%M"))
    } else {
        format!("{} → {}", d.format("%d/%m/%Y %H:%M"), f.format("%d/%m/%Y %H:%M"))
    }
}

fn majuscule(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(p) => p.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Regroupe les sorties par mois (frise, NX-17). L'ordre d'entrée (déjà trié par le serveur) est conservé.
pub fn grouper_par_mois(liste: &[SortieResumeDto]) -> Vec<(String, Vec<&SortieResumeDto>)> {
    let mut groupes: Vec<((i32, u32), Vec<&SortieResumeDto>)> = Vec::new();
    for s in liste {
        let d = a_paris(s.debut).date();
        let cle = (d.year(), d.month());
        match groupes.iter_mut().find(|(c, _)| *c == cle) {
            Some((_, v)) => v.push(s),
            None => groupes.push((cle, vec![s])),
        }
    }
    groupes
        .into_iter()
        .map(|((a, m), v)| (format!("{} {a}", majuscule(nom_mois(m))), v))
        .collect()
}

// ───────────────────────── Saisie ─────────────────────────

/// Champs du formulaire de création, tels que saisis (chaînes).
#[derive(Clone, Debug, Default)]
pub struct SaisieVoyage {
    pub titre: String,
    pub nature: String,
    pub forme: String,
    pub destination: String,
    pub pays: String,
    pub lieu: String,
    pub objectifs: String,
    pub date_debut: String,
    pub heure_debut: String,
    pub date_fin: String,
    pub heure_fin: String,
    pub effectif: String,
    pub adultes: String,
    pub point_rendez_vous: String,
    pub besoin_car: bool,
    pub classes: Vec<Uuid>,
}

type Erreurs = Vec<(String, String)>;

fn erreur(e: &mut Erreurs, champ: &str, msg: &str) {
    e.push((champ.to_string(), msg.to_string()));
}

fn entier(v: &str) -> Result<Option<i32>, ()> {
    let v = v.trim();
    if v.is_empty() {
        return Ok(None);
    }
    v.parse::<i32>().ok().filter(|n| (0..=2000).contains(n)).map(Some).ok_or(())
}

fn opt(v: &str) -> Option<String> {
    let t = v.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Contrôle de confort avant l'envoi (le serveur revérifie tout) : champs vides, dates, nombres.
pub fn composer(s: &SaisieVoyage) -> Result<SortieSaisieDto, Erreurs> {
    let mut e: Erreurs = Vec::new();
    if s.titre.trim().is_empty() {
        erreur(&mut e, "voy-titre", "Donnez un titre à la sortie.");
    }
    if s.nature.is_empty() {
        erreur(&mut e, "voy-nature", "Choisissez la nature de la sortie.");
    }
    if s.forme.is_empty() {
        erreur(&mut e, "voy-forme", "Choisissez la forme de la sortie.");
    }
    if s.destination.is_empty() {
        erreur(&mut e, "voy-destination", "Choisissez la destination.");
    }
    if s.destination == "etranger" && s.pays.trim().len() < 2 {
        erreur(&mut e, "voy-pays", "Le pays est obligatoire à l'étranger.");
    }
    let date = |v: &str| NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d").ok();
    let heure = |v: &str| NaiveTime::parse_from_str(v.trim(), "%H:%M").ok();
    let (dd, hd) = (date(&s.date_debut), heure(&s.heure_debut));
    let (df, hf) = (date(&s.date_fin), heure(&s.heure_fin));
    if dd.is_none() {
        erreur(&mut e, "voy-date-debut", "Indiquez la date de départ.");
    }
    if hd.is_none() {
        erreur(&mut e, "voy-heure-debut", "Indiquez l'heure de départ.");
    }
    if df.is_none() {
        erreur(&mut e, "voy-date-fin", "Indiquez la date de retour.");
    }
    if hf.is_none() {
        erreur(&mut e, "voy-heure-fin", "Indiquez l'heure de retour.");
    }
    let eleves = entier(&s.effectif);
    let adultes = entier(&s.adultes);
    if eleves.is_err() {
        erreur(&mut e, "voy-effectif", "Nombre d'élèves : un entier entre 0 et 2000.");
    }
    if adultes.is_err() {
        erreur(&mut e, "voy-adultes", "Nombre d'adultes : un entier entre 0 et 2000.");
    }
    if s.forme == "voyage" && matches!(eleves, Ok(None)) {
        erreur(&mut e, "voy-effectif", "Un voyage demande l'effectif d'élèves prévu (devis, budget).");
    }
    if s.classes.is_empty() {
        erreur(&mut e, "voy-classes", "Choisissez au moins une classe concernée.");
    }
    let (Some(dd), Some(hd), Some(df), Some(hf)) = (dd, hd, df, hf) else {
        return Err(e);
    };
    let debut = paris_vers_utc(dd.and_time(hd));
    let fin = paris_vers_utc(df.and_time(hf));
    if fin <= debut {
        erreur(&mut e, "voy-date-fin", "Le retour doit suivre le départ.");
    }
    if !e.is_empty() {
        return Err(e);
    }
    Ok(SortieSaisieDto {
        titre: s.titre.trim().to_string(),
        objectifs: opt(&s.objectifs),
        nature: s.nature.clone(),
        forme: s.forme.clone(),
        destination: s.destination.clone(),
        pays: opt(&s.pays),
        destination_libelle: opt(&s.lieu),
        programme_libelle: None,
        debut,
        fin,
        classes: s.classes.clone(),
        groupes: Vec::new(),
        effectif_mode: None,
        places: None,
        effectif_eleves_declare: eleves.unwrap_or(None),
        effectif_adultes_declare: adultes.unwrap_or(None),
        besoin_car: s.besoin_car,
        car_sur_place: false,
        point_rendez_vous: opt(&s.point_rendez_vous),
        consignes_transport: None,
        places_encadrant_a_pourvoir: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn resume(titre: &str, jour: u32, mois: u32) -> SortieResumeDto {
        let debut = Utc.with_ymd_and_hms(2027, mois, jour, 8, 0, 0).unwrap();
        SortieResumeDto {
            id: Uuid::nil(),
            reference: "VOY-2027-AAAAAAAA".into(),
            titre: titre.into(),
            debut,
            fin: debut + chrono::Duration::hours(8),
            destination: "france".into(),
            destination_libelle: None,
            nature: "facultative".into(),
            forme: "sans_nuitee".into(),
            etat: "ouverte".into(),
            serie_id: None,
            porteur_nom: None,
            nb_inscrits: None,
            acces_detail: false,
            est_porteur: false,
        }
    }

    fn valide() -> SaisieVoyage {
        SaisieVoyage {
            titre: "Voyage à Rome".into(),
            nature: "facultative".into(),
            forme: "voyage".into(),
            destination: "etranger".into(),
            pays: "Italie".into(),
            date_debut: "2027-03-10".into(),
            heure_debut: "07:00".into(),
            date_fin: "2027-03-14".into(),
            heure_fin: "19:30".into(),
            effectif: "42".into(),
            classes: vec![Uuid::nil()],
            ..Default::default()
        }
    }

    #[test]
    fn tous_les_etats_ont_un_libelle_et_un_ton() {
        for e in [
            "brouillon", "soumise", "a_completer", "preparation", "a_signer", "budget_a_valider",
            "vote", "a_autoriser", "ouverte", "prete", "en_cours", "rentree", "close", "refusee",
            "annulee",
        ] {
            assert_ne!(libelle_etat(e), "État inconnu", "{e}");
        }
        assert_eq!(libelle_etat("nimporte"), "État inconnu");
        assert_eq!(ton_etat("annulee"), TonEtat::Mal);
        assert_eq!(ton_etat("prete"), TonEtat::Ok);
        assert_eq!(ton_etat("soumise"), TonEtat::Avertissement);
    }

    #[test]
    fn periode_en_heure_de_paris() {
        // 12 mars 2027 : heure d'hiver (UTC+1).
        let d = Utc.with_ymd_and_hms(2027, 3, 12, 7, 0, 0).unwrap();
        assert_eq!(periode(d, d + chrono::Duration::hours(9)), "12/03/2027 08:00 → 17:00");
        let long = periode(d, d + chrono::Duration::days(3));
        assert!(long.contains("12/03/2027 08:00") && long.contains("15/03/2027 08:00"), "{long}");
    }

    #[test]
    fn frise_par_mois_dans_l_ordre_du_serveur() {
        let l = vec![
            resume("A", 5, 3),
            resume("B", 20, 3),
            resume("C", 2, 4),
        ];
        let g = grouper_par_mois(&l);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].0, "Mars 2027");
        assert_eq!(g[0].1.iter().map(|s| s.titre.as_str()).collect::<Vec<_>>(), vec!["A", "B"]);
        assert_eq!(g[1].0, "Avril 2027");
        assert!(grouper_par_mois(&[]).is_empty());
    }

    #[test]
    fn composition_valide_en_utc() {
        let s = composer(&valide()).expect("saisie valide");
        assert_eq!(s.titre, "Voyage à Rome");
        assert_eq!(s.pays.as_deref(), Some("Italie"));
        assert_eq!(s.effectif_eleves_declare, Some(42));
        // 07:00 à Paris le 10 mars 2027 (hiver) = 06:00 UTC.
        assert_eq!(s.debut, Utc.with_ymd_and_hms(2027, 3, 10, 6, 0, 0).unwrap());
        assert!(s.fin > s.debut);
    }

    #[test]
    fn composition_refuse_et_dit_pourquoi() {
        let mut s = valide();
        s.titre = "  ".into();
        s.pays = String::new();
        s.effectif = String::new();
        s.classes.clear();
        s.date_fin = "2027-03-09".into();
        let e = composer(&s).unwrap_err();
        let champs: Vec<&str> = e.iter().map(|(c, _)| c.as_str()).collect();
        for attendu in ["voy-titre", "voy-pays", "voy-effectif", "voy-classes", "voy-date-fin"] {
            assert!(champs.contains(&attendu), "{attendu} manquant : {champs:?}");
        }
        // Heure illisible et nombre hors limites.
        let mut s = valide();
        s.heure_debut = "7h".into();
        s.adultes = "-3".into();
        let champs: Vec<String> = composer(&s).unwrap_err().into_iter().map(|(c, _)| c).collect();
        assert!(champs.contains(&"voy-heure-debut".to_string()));
        assert!(champs.contains(&"voy-adultes".to_string()));
    }

    #[test]
    fn sortie_sans_nuitee_n_impose_pas_d_effectif() {
        let mut s = valide();
        s.forme = "sans_nuitee".into();
        s.destination = "proximite".into();
        s.pays = String::new();
        s.effectif = String::new();
        assert!(composer(&s).is_ok());
    }

    #[test]
    fn journal_et_gravites_lisibles() {
        assert_eq!(libelle_journal("autorisee"), "Autorisée");
        assert_eq!(libelle_journal("inconnue"), "Action enregistrée");
        assert_eq!(ton_gravite("bloquant"), TonEtat::Mal);
        assert_eq!(libelle_gravite("conseil"), "Conseil");
        assert_eq!(libelle_qui("direction"), "À la direction");
        assert_eq!(libelle_qui("personne"), "Rien à faire pour le moment");
    }
}
