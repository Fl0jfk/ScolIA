//! Modèle de l'appel v2 (socle 12, AS14a) : logique d'écran pure, sans Leptos ni réseau.
//!
//! Aucune règle métier n'est dupliquée : qui peut faire l'appel, qui peut corriger et dans quel délai,
//! ce que déclenche la validation (absences, avis aux familles) restent décidés par le serveur. Ce
//! module ne gère que l'état d'écran : choix en cours, compteurs, ce qu'il faut envoyer, libellés
//! lisibles au lecteur d'écran (l'état n'est jamais porté par la seule couleur).

use ent_types::assiduite::{AppelV2Dto, LigneAppelV2Dto, PointerLigneBody};
use uuid::Uuid;

use super::modele::normaliser;

/// Bornes des minutes de retard acceptées par le serveur.
pub const MINUTES_MIN: i32 = 1;
pub const MINUTES_MAX: i32 = 600;

/// Ce que le professeur peut saisir à l'appel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Choix {
    Present,
    Absent,
    Retard,
}

impl Choix {
    pub const TOUS: [Choix; 3] = [Choix::Present, Choix::Absent, Choix::Retard];

    pub fn libelle(self) -> &'static str {
        match self {
            Choix::Present => "Présent",
            Choix::Absent => "Absent",
            Choix::Retard => "Retard",
        }
    }

    pub fn libelle_pluriel(self) -> &'static str {
        match self {
            Choix::Present => "Présents",
            Choix::Absent => "Absents",
            Choix::Retard => "Retards",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Choix::Present => "present",
            Choix::Absent => "absent",
            Choix::Retard => "retard",
        }
    }

    pub fn touche(self) -> char {
        match self {
            Choix::Present => 'p',
            Choix::Absent => 'a',
            Choix::Retard => 'r',
        }
    }

    pub fn depuis_touche(c: char) -> Option<Choix> {
        Choix::TOUS
            .into_iter()
            .find(|s| s.touche() == c.to_ascii_lowercase())
    }

    pub fn depuis_code(code: &str) -> Option<Choix> {
        Choix::TOUS.into_iter().find(|s| s.code() == code)
    }
}

/// État d'une ligne tel que le serveur le connaît.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatLigne {
    NonPointe,
    Present,
    Absent,
    Retard,
    /// Localisé (infirmerie, vie scolaire, CDI…) : jamais une absence.
    Localise,
    /// Attendu ailleurs (sortie, examen, mission…) : posé par le système.
    AttenduAilleurs,
    Dispense,
}

impl EtatLigne {
    pub fn depuis_api(etat: &str) -> EtatLigne {
        match etat {
            "present" => EtatLigne::Present,
            "absent" => EtatLigne::Absent,
            "retard" => EtatLigne::Retard,
            "localise" => EtatLigne::Localise,
            "attendu_ailleurs" => EtatLigne::AttenduAilleurs,
            "present_dispense" => EtatLigne::Dispense,
            _ => EtatLigne::NonPointe,
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            EtatLigne::NonPointe => "Non pointé",
            EtatLigne::Present => "Présent",
            EtatLigne::Absent => "Absent",
            EtatLigne::Retard => "Retard",
            EtatLigne::Localise => "Localisé",
            EtatLigne::AttenduAilleurs => "Attendu ailleurs",
            EtatLigne::Dispense => "Présent, dispensé",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            EtatLigne::NonPointe => "non_pointe",
            EtatLigne::Present => "present",
            EtatLigne::Absent => "absent",
            EtatLigne::Retard => "retard",
            EtatLigne::Localise => "localise",
            EtatLigne::AttenduAilleurs => "attendu_ailleurs",
            EtatLigne::Dispense => "dispense",
        }
    }

    /// Le choix du professeur que cet état représente (rien pour les états posés par le système).
    pub fn choix(self) -> Option<Choix> {
        match self {
            EtatLigne::Present | EtatLigne::Dispense => Some(Choix::Present),
            EtatLigne::Absent => Some(Choix::Absent),
            EtatLigne::Retard => Some(Choix::Retard),
            _ => None,
        }
    }
}

/// État de l'appel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatAppel {
    EnCours,
    /// « Appel fait » : lecture et corrections (l'intervenant le jour même, la vie scolaire ensuite).
    Fait,
    /// Séance annulée après coup : plus rien ne compte (absences et retards retirés, démentis émis).
    NonAssuree,
}

impl EtatAppel {
    pub fn depuis_api(etat: &str) -> EtatAppel {
        match etat {
            "en_cours" => EtatAppel::EnCours,
            "neutralise" => EtatAppel::NonAssuree,
            _ => EtatAppel::Fait,
        }
    }
}

/// Lieu codé d'une localisation, en français (jamais un motif).
pub fn libelle_lieu(code: &str) -> &'static str {
    match code {
        "infirmerie" => "à l'infirmerie",
        "vie_scolaire" => "à la vie scolaire",
        "cdi" => "au CDI",
        "entretien" => "en entretien",
        "autre_activite" => "dans une autre activité",
        _ => "localisé",
    }
}

/// Élève de la feuille (données fixes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EleveV2 {
    pub eleve_id: Uuid,
    pub nom: String,
    pub cle_recherche: String,
    pub etat: EtatLigne,
    pub minutes: Option<i32>,
    pub lieu: Option<String>,
    /// « prévenu », « en sortie »… (libellé court de la situation connue, sans motif).
    pub libelle_court: Option<String>,
    pub pre_remplie: bool,
    pub version: i32,
    pub arrivee_vs_attendue: bool,
}

impl EleveV2 {
    pub fn depuis_api(l: &LigneAppelV2Dto) -> EleveV2 {
        let nom = l
            .eleve_label
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "Élève sans nom".to_string());
        EleveV2 {
            eleve_id: l.eleve_id,
            cle_recherche: normaliser(&nom),
            nom,
            etat: EtatLigne::depuis_api(&l.etat),
            minutes: l.minutes_retard,
            lieu: l.lieu_code.clone(),
            libelle_court: l.libelle_court.clone(),
            pre_remplie: l.pre_remplie,
            version: l.version,
            arrivee_vs_attendue: l.arrivee_vs_attendue,
        }
    }
}

/// Feuille d'appel telle que reçue du serveur, triée par nom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeuilleV2 {
    pub appel_id: Uuid,
    pub classe: String,
    pub matiere: Option<String>,
    pub jour: chrono::NaiveDate,
    pub etat: EtatAppel,
    /// Mode « présents seulement » : tout le monde commence non pointé.
    pub presents_seulement: bool,
    pub eleves: Vec<EleveV2>,
}

impl FeuilleV2 {
    pub fn depuis_api(a: &AppelV2Dto) -> FeuilleV2 {
        let mut eleves: Vec<EleveV2> = a.lignes.iter().map(EleveV2::depuis_api).collect();
        eleves.sort_by(|x, y| x.cle_recherche.cmp(&y.cle_recherche));
        FeuilleV2 {
            appel_id: a.id,
            classe: a.classe.clone(),
            matiere: a.matiere_libelle.clone(),
            jour: a.jour,
            etat: EtatAppel::depuis_api(&a.etat),
            presents_seulement: a.mode_pointage == "presents_seulement",
            eleves,
        }
    }
}

/// Saisie en cours pour un élève : `choix = None` laisse l'état du serveur inchangé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaisieV2 {
    pub eleve_id: Uuid,
    pub choix: Option<Choix>,
    /// Minutes de retard saisies (texte, contrôlé par [`minutes_valides`]).
    pub minutes: String,
}

/// Minutes de retard : entier de 1 à 600.
pub fn minutes_valides(texte: &str) -> Result<i32, &'static str> {
    let t = texte.trim();
    if t.is_empty() {
        return Err("Indiquez les minutes de retard.");
    }
    match t.parse::<i32>() {
        Ok(m) if (MINUTES_MIN..=MINUTES_MAX).contains(&m) => Ok(m),
        Ok(_) => Err("Le retard doit être compris entre 1 et 600 minutes."),
        Err(_) => Err("Les minutes de retard s'écrivent en chiffres."),
    }
}

/// Ce qu'on affichera pour la ligne : le choix du professeur, sinon l'état du serveur.
pub fn etat_affiche(eleve: &EleveV2, saisie: &SaisieV2) -> EtatLigne {
    match saisie.choix {
        Some(Choix::Present) => EtatLigne::Present,
        Some(Choix::Absent) => EtatLigne::Absent,
        Some(Choix::Retard) => EtatLigne::Retard,
        None => eleve.etat,
    }
}

/// La ligne diffère-t-elle de ce que le serveur connaît ?
pub fn est_modifiee(eleve: &EleveV2, saisie: &SaisieV2) -> bool {
    match saisie.choix {
        None => false,
        Some(Choix::Retard) => {
            eleve.etat != EtatLigne::Retard
                || minutes_valides(&saisie.minutes)
                    .map(|m| Some(m) != eleve.minutes)
                    .unwrap_or(true)
        }
        Some(c) => eleve.etat.choix() != Some(c),
    }
}

/// Compteurs affichés (état effectif, saisies comprises).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Compteurs {
    pub presents: usize,
    pub absents: usize,
    pub retards: usize,
    pub ailleurs: usize,
    pub non_pointes: usize,
}

impl Compteurs {
    pub fn de(etats: impl IntoIterator<Item = EtatLigne>) -> Compteurs {
        let mut c = Compteurs::default();
        for e in etats {
            match e {
                EtatLigne::Present | EtatLigne::Dispense => c.presents += 1,
                EtatLigne::Absent => c.absents += 1,
                EtatLigne::Retard => c.retards += 1,
                EtatLigne::Localise | EtatLigne::AttenduAilleurs => c.ailleurs += 1,
                EtatLigne::NonPointe => c.non_pointes += 1,
            }
        }
        c
    }

    pub fn total(&self) -> usize {
        self.presents + self.absents + self.retards + self.ailleurs + self.non_pointes
    }

    pub fn pour(&self, c: Choix) -> usize {
        match c {
            Choix::Present => self.presents,
            Choix::Absent => self.absents,
            Choix::Retard => self.retards,
        }
    }
}

/// Un envoi à faire avant la validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envoi {
    pub eleve_id: Uuid,
    pub nom: String,
    pub corps: PointerLigneBody,
}

/// Problème bloquant avant l'envoi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blocage {
    pub nom: String,
    pub message: &'static str,
}

/// Les envois à faire : seulement les lignes modifiées ; un retard sans minutes valides bloque.
pub fn envois(eleves: &[EleveV2], saisies: &[SaisieV2]) -> Result<Vec<Envoi>, Vec<Blocage>> {
    let mut sortie = Vec::new();
    let mut blocages = Vec::new();
    for (e, s) in eleves.iter().zip(saisies) {
        debug_assert_eq!(e.eleve_id, s.eleve_id);
        let Some(choix) = s.choix else { continue };
        if !est_modifiee(e, s) {
            continue;
        }
        let minutes = if choix == Choix::Retard {
            match minutes_valides(&s.minutes) {
                Ok(m) => Some(m),
                Err(message) => {
                    blocages.push(Blocage {
                        nom: e.nom.clone(),
                        message,
                    });
                    continue;
                }
            }
        } else {
            None
        };
        sortie.push(Envoi {
            eleve_id: e.eleve_id,
            nom: e.nom.clone(),
            corps: PointerLigneBody {
                eleve_id: e.eleve_id,
                etat: choix.code().to_string(),
                minutes,
                lieu_code: None,
            },
        });
    }
    if blocages.is_empty() {
        Ok(sortie)
    } else {
        Err(blocages)
    }
}

/// Ce que la validation va produire, pour la confirmation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resume {
    pub presents: usize,
    pub absents: Vec<String>,
    pub retards: Vec<String>,
    pub ailleurs: usize,
    /// Non pointés qui deviendraient absents (mode « présents seulement »).
    pub non_pointes: Vec<String>,
}

pub fn resume(eleves: &[EleveV2], saisies: &[SaisieV2]) -> Resume {
    let mut r = Resume {
        presents: 0,
        absents: vec![],
        retards: vec![],
        ailleurs: 0,
        non_pointes: vec![],
    };
    for (e, s) in eleves.iter().zip(saisies) {
        match etat_affiche(e, s) {
            EtatLigne::Present | EtatLigne::Dispense => r.presents += 1,
            EtatLigne::Absent => r.absents.push(e.nom.clone()),
            EtatLigne::Retard => r.retards.push(e.nom.clone()),
            EtatLigne::Localise | EtatLigne::AttenduAilleurs => r.ailleurs += 1,
            EtatLigne::NonPointe => r.non_pointes.push(e.nom.clone()),
        }
    }
    r
}

/// Le serveur exige la confirmation quand des élèves restent non pointés (mode « présents seulement »).
pub fn confirmation_non_pointes_requise(r: &Resume) -> bool {
    !r.non_pointes.is_empty()
}

/// Phrase lue par le lecteur d'écran pour une ligne : « Léa Martin, absente, prévenu ».
pub fn phrase_ligne(eleve: &EleveV2, etat: EtatLigne, minutes: Option<i32>) -> String {
    let mut p = format!("{}, {}", eleve.nom, etat.libelle().to_lowercase());
    if etat == EtatLigne::Retard {
        if let Some(m) = minutes {
            p.push_str(&format!(" de {m} minutes"));
        }
    }
    if matches!(etat, EtatLigne::Localise) {
        if let Some(l) = eleve.lieu.as_deref() {
            p.push(' ');
            p.push_str(libelle_lieu(l));
        }
    }
    if let Some(c) = eleve.libelle_court.as_deref() {
        p.push_str(", ");
        p.push_str(c);
    }
    p
}

/// Motifs d'une correction (liste fermée côté serveur ; le texte libre est facultatif, 200 caractères).
pub const MOTIFS_CORRECTION: [(&str, &str); 3] = [
    ("erreur_saisie", "Erreur de saisie"),
    ("billet_entree", "Billet d'entrée à la vie scolaire"),
    ("oubli", "Oubli"),
];

pub const MAX_MOTIF_TEXTE: usize = 200;

/// Libellé d'une incohérence signalée après la validation.
pub fn libelle_incoherence(code: &str) -> &'static str {
    match code {
        "present_malgre_declaration" => "marqué présent alors que sa famille avait prévenu",
        "present_alors_quen_sortie" => "marqué présent alors qu'il est en sortie",
        "absent_puis_present" => "d'abord absent, puis présent",
        "exclusion_sans_arrivee" => "exclu du cours, arrivée à la vie scolaire non confirmée",
        _ => "à vérifier",
    }
}

/// Libellé d'un état dans l'historique (« Présent → Absent »).
pub fn libelle_transition(avant: Option<&str>, apres: &str) -> String {
    let a = avant.map(EtatLigne::depuis_api).map(EtatLigne::libelle);
    let b = EtatLigne::depuis_api(apres).libelle();
    match a {
        Some(a) => format!("{a} → {b}"),
        None => b.to_string(),
    }
}

/// Libellé du canal de saisie d'une ligne d'historique.
pub fn libelle_canal(canal: &str) -> &'static str {
    match canal {
        "web" => "web",
        "mobile" => "mobile",
        "hors_ligne" => "hors ligne",
        _ => "autre",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ent_types::assiduite::RecapAppelDto;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn ligne(n: u128, nom: &str, etat: &str) -> LigneAppelV2Dto {
        LigneAppelV2Dto {
            eleve_id: id(n),
            etat: etat.into(),
            minutes_retard: None,
            lieu_code: None,
            pre_remplie: false,
            libelle_court: None,
            version: 1,
            eleve_label: Some(nom.into()),
            arrivee_vs_attendue: false,
        }
    }

    fn appel(lignes: Vec<LigneAppelV2Dto>, mode: &str, etat: &str) -> AppelV2Dto {
        AppelV2Dto {
            id: id(99),
            jour: NaiveDate::from_ymd_opt(2026, 12, 3).unwrap(),
            classe: "4B".into(),
            type_activite: "cours".into(),
            etat: etat.into(),
            mode_pointage: mode.into(),
            heure_debut: None,
            heure_fin: None,
            creneau_id: None,
            matiere_libelle: Some("Potions".into()),
            delegue_a: None,
            lignes,
            recapitulatif: RecapAppelDto::default(),
        }
    }

    fn saisies_vides(f: &FeuilleV2) -> Vec<SaisieV2> {
        f.eleves
            .iter()
            .map(|e| SaisieV2 {
                eleve_id: e.eleve_id,
                choix: None,
                minutes: String::new(),
            })
            .collect()
    }

    #[test]
    fn feuille_triee_sans_accent_et_etats_relus() {
        let a = appel(
            vec![
                ligne(1, "Élodie Zola", "present"),
                ligne(2, "Albert Dupont", "absent"),
                ligne(3, "Émile Bon", "retard"),
            ],
            "tous_presents",
            "en_cours",
        );
        let f = FeuilleV2::depuis_api(&a);
        let noms: Vec<&str> = f.eleves.iter().map(|e| e.nom.as_str()).collect();
        assert_eq!(noms, ["Albert Dupont", "Élodie Zola", "Émile Bon"]);
        assert_eq!(f.etat, EtatAppel::EnCours);
        assert!(!f.presents_seulement);
    }

    #[test]
    fn seance_non_assuree_et_appel_fait_sont_reconnus() {
        assert_eq!(EtatAppel::depuis_api("neutralise"), EtatAppel::NonAssuree);
        assert_eq!(EtatAppel::depuis_api("en_cours"), EtatAppel::EnCours);
        assert_eq!(EtatAppel::depuis_api("valide"), EtatAppel::Fait);
        assert_eq!(EtatAppel::depuis_api("clos"), EtatAppel::Fait);
    }

    #[test]
    fn rien_a_envoyer_sans_modification() {
        let f = FeuilleV2::depuis_api(&appel(
            vec![ligne(1, "A", "present"), ligne(2, "B", "absent")],
            "tous_presents",
            "en_cours",
        ));
        assert_eq!(envois(&f.eleves, &saisies_vides(&f)), Ok(vec![]));
    }

    #[test]
    fn seules_les_lignes_modifiees_partent() {
        let f = FeuilleV2::depuis_api(&appel(
            vec![ligne(1, "A", "present"), ligne(2, "B", "present")],
            "tous_presents",
            "en_cours",
        ));
        let mut s = saisies_vides(&f);
        s[1].choix = Some(Choix::Absent);
        s[0].choix = Some(Choix::Present); // identique au serveur : rien à envoyer
        let e = envois(&f.eleves, &s).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].corps.etat, "absent");
        assert_eq!(e[0].corps.minutes, None);
    }

    #[test]
    fn un_retard_sans_minutes_bloque_et_avec_minutes_part() {
        let f = FeuilleV2::depuis_api(&appel(
            vec![ligne(1, "Léa Martin", "present")],
            "tous_presents",
            "en_cours",
        ));
        let mut s = saisies_vides(&f);
        s[0].choix = Some(Choix::Retard);
        let b = envois(&f.eleves, &s).unwrap_err();
        assert_eq!(b[0].nom, "Léa Martin");
        s[0].minutes = "12".into();
        let e = envois(&f.eleves, &s).unwrap();
        assert_eq!(e[0].corps.minutes, Some(12));
        assert_eq!(e[0].corps.etat, "retard");
    }

    #[test]
    fn minutes_bornees() {
        assert!(minutes_valides("0").is_err());
        assert!(minutes_valides("601").is_err());
        assert!(minutes_valides("abc").is_err());
        assert!(minutes_valides("").is_err());
        assert_eq!(minutes_valides(" 5 "), Ok(5));
        assert_eq!(minutes_valides("600"), Ok(600));
    }

    #[test]
    fn marquer_present_un_eleve_prevenu_est_une_modification() {
        let mut l = ligne(1, "A", "absent");
        l.pre_remplie = true;
        l.libelle_court = Some("prévenu".into());
        let f = FeuilleV2::depuis_api(&appel(vec![l], "tous_presents", "en_cours"));
        let mut s = saisies_vides(&f);
        s[0].choix = Some(Choix::Present);
        assert!(est_modifiee(&f.eleves[0], &s[0]));
        assert_eq!(envois(&f.eleves, &s).unwrap().len(), 1);
    }

    #[test]
    fn un_eleve_localise_n_est_pas_absent_et_reste_inchange() {
        let mut l = ligne(1, "Hermione", "localise");
        l.lieu_code = Some("infirmerie".into());
        l.pre_remplie = true;
        l.libelle_court = Some("à l'infirmerie".into());
        let f = FeuilleV2::depuis_api(&appel(vec![l], "tous_presents", "en_cours"));
        let s = saisies_vides(&f);
        let c = Compteurs::de(f.eleves.iter().zip(&s).map(|(e, s)| etat_affiche(e, s)));
        assert_eq!((c.absents, c.ailleurs), (0, 1));
        assert_eq!(envois(&f.eleves, &s), Ok(vec![]));
        let phrase = phrase_ligne(&f.eleves[0], EtatLigne::Localise, None);
        assert_eq!(phrase, "Hermione, localisé à l'infirmerie, à l'infirmerie");
    }

    #[test]
    fn mode_presents_seulement_demande_confirmation_des_non_pointes() {
        let f = FeuilleV2::depuis_api(&appel(
            vec![ligne(1, "A", "non_pointe"), ligne(2, "B", "non_pointe")],
            "presents_seulement",
            "en_cours",
        ));
        assert!(f.presents_seulement);
        let mut s = saisies_vides(&f);
        s[0].choix = Some(Choix::Present);
        let r = resume(&f.eleves, &s);
        assert_eq!(r.presents, 1);
        assert_eq!(r.non_pointes, vec!["B".to_string()]);
        assert!(confirmation_non_pointes_requise(&r));
        s[1].choix = Some(Choix::Absent);
        assert!(!confirmation_non_pointes_requise(&resume(&f.eleves, &s)));
    }

    #[test]
    fn compteurs_et_touches() {
        let c = Compteurs::de([
            EtatLigne::Present,
            EtatLigne::Dispense,
            EtatLigne::Absent,
            EtatLigne::Retard,
            EtatLigne::AttenduAilleurs,
            EtatLigne::NonPointe,
        ]);
        assert_eq!(
            (c.presents, c.absents, c.retards, c.ailleurs, c.non_pointes),
            (2, 1, 1, 1, 1)
        );
        assert_eq!(c.total(), 6);
        assert_eq!(Choix::depuis_touche('A'), Some(Choix::Absent));
        assert_eq!(Choix::depuis_touche('x'), None);
    }

    #[test]
    fn phrase_ligne_lisible_au_lecteur_d_ecran() {
        let mut l = ligne(1, "Léa Martin", "absent");
        l.libelle_court = Some("prévenu".into());
        let e = EleveV2::depuis_api(&l);
        assert_eq!(
            phrase_ligne(&e, EtatLigne::Absent, None),
            "Léa Martin, absent, prévenu"
        );
        assert_eq!(
            phrase_ligne(&e, EtatLigne::Retard, Some(7)),
            "Léa Martin, retard de 7 minutes, prévenu"
        );
    }

    #[test]
    fn transitions_et_incoherences_en_francais() {
        assert_eq!(
            libelle_transition(Some("present"), "absent"),
            "Présent → Absent"
        );
        assert_eq!(libelle_transition(None, "present"), "Présent");
        assert!(libelle_incoherence("present_alors_quen_sortie").contains("en sortie"));
        assert_eq!(libelle_incoherence("inconnu"), "à vérifier");
    }
}
