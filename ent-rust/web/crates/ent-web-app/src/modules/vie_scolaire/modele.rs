//! Logique d'écran de la vie scolaire (pure, sans Leptos ni réseau, testée en natif) :
//! filtres, tri, fusion des décisions de la session, décisions possibles, validation des
//! notes, signaux d'absence répétée.

use std::collections::HashMap;

use chrono::NaiveDate;
use ent_web_api::{Absence, DecisionApi, SignalAbsence};
use ent_web_ui::TonEtat;
use uuid::Uuid;

use crate::composants_metier::absences::{couvre, nom_eleve, StatutAbsence};
use crate::noyau::droits::Droits;

/// Longueur maximale d'une note de décision (caractères).
pub const NOTE_MAX: usize = 500;

// ---------------------------------------------------------------------------
// Liste des absences
// ---------------------------------------------------------------------------

/// Filtre de la liste : statut, classe, recherche par nom.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filtre {
    pub statut: Option<StatutAbsence>,
    pub classe: Option<String>,
    pub recherche: String,
}

fn normaliser(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' | 'ä' => 'a',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// Absences correspondant au filtre. Sans information de classe pour un élève, il
/// n'apparaît pas quand une classe est demandée (on ne devine pas).
pub fn filtrer(
    absences: &[Absence],
    classes: &HashMap<Uuid, String>,
    filtre: &Filtre,
) -> Vec<Absence> {
    let besoin = normaliser(filtre.recherche.trim());
    absences
        .iter()
        .filter(|a| filtre.statut.is_none_or(|s| StatutAbsence::depuis(a) == s))
        .filter(|a| {
            filtre
                .classe
                .as_ref()
                .is_none_or(|c| classes.get(&a.eleve_id) == Some(c))
        })
        .filter(|a| besoin.is_empty() || normaliser(&nom_eleve(a)).contains(&besoin))
        .cloned()
        .collect()
}

/// Rang de tri : ce qui attend une décision d'abord.
fn rang(s: StatutAbsence) -> u8 {
    match s {
        StatutAbsence::JustificatifEnAttente => 0,
        StatutAbsence::NonJustifiee => 1,
        StatutAbsence::Refusee => 2,
        StatutAbsence::Justifiee => 3,
        StatutAbsence::AnnuleePresent => 4,
    }
}

/// Justificatifs en attente, puis non justifiées, puis le reste ; par nom à rang égal.
pub fn trier(mut absences: Vec<Absence>) -> Vec<Absence> {
    absences.sort_by(|a, b| {
        rang(StatutAbsence::depuis(a))
            .cmp(&rang(StatutAbsence::depuis(b)))
            .then_with(|| normaliser(&nom_eleve(a)).cmp(&normaliser(&nom_eleve(b))))
            .then(a.debut.cmp(&b.debut))
    });
    absences
}

/// Classes distinctes connues, triées.
pub fn classes_connues(classes: &HashMap<Uuid, String>) -> Vec<String> {
    let mut v: Vec<String> = classes.values().cloned().collect();
    v.sort();
    v.dedup();
    v
}

/// La réponse d'une décision ne contient pas le nom de l'élève : on garde celui de la liste.
pub fn appliquer_decision(avant: &Absence, mut apres: Absence) -> Absence {
    if apres.eleve_label.is_none() {
        apres.eleve_label = avant.eleve_label.clone();
    }
    apres
}

/// Liste du serveur (absences encore à traiter ce jour) complétée par les décisions prises
/// pendant la session : une absence décidée sort de la liste du serveur mais reste visible,
/// avec son nouveau statut, jusqu'au changement de jour ou de page.
pub fn fusionner(serveur: Vec<Absence>, decidees: &[Absence], jour: NaiveDate) -> Vec<Absence> {
    let mut v: Vec<Absence> = serveur
        .into_iter()
        .map(|a| decidees.iter().find(|d| d.id == a.id).cloned().unwrap_or(a))
        .collect();
    for d in decidees {
        if couvre(d, jour) && !v.iter().any(|a| a.id == d.id) {
            v.push(d.clone());
        }
    }
    v
}

/// Mémorise une décision (remplace l'ancienne version de la même absence).
pub fn memoriser(decidees: &mut Vec<Absence>, absence: Absence) {
    decidees.retain(|d| d.id != absence.id);
    decidees.push(absence);
}

// ---------------------------------------------------------------------------
// Décisions
// ---------------------------------------------------------------------------

/// Décision du personnel sur une absence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Accepter,
    Refuser,
    AnnulerPresent,
}

impl Decision {
    pub fn api(self) -> DecisionApi {
        match self {
            Decision::Accepter => DecisionApi::Accepter,
            Decision::Refuser => DecisionApi::Refuser,
            Decision::AnnulerPresent => DecisionApi::AnnulerPresent,
        }
    }

    /// Libellé du bouton. Sans justificatif déposé, « accepter » devient « justifier ».
    pub fn libelle(self, a_un_justificatif: bool) -> &'static str {
        match self {
            Decision::Accepter if a_un_justificatif => "Accepter le justificatif",
            Decision::Accepter => "Justifier l'absence",
            Decision::Refuser if a_un_justificatif => "Refuser le justificatif",
            Decision::Refuser => "Refuser la justification",
            Decision::AnnulerPresent => "Annuler (élève présent)",
        }
    }

    /// Titre de la demande de confirmation.
    pub fn titre_confirmation(self) -> &'static str {
        match self {
            Decision::Accepter => "Accepter la justification ?",
            Decision::Refuser => "Refuser la justification ?",
            Decision::AnnulerPresent => "Remettre l'élève présent ?",
        }
    }

    /// Texte de confirmation : dit ce qui va se passer, pour qui.
    pub fn message(self, eleve: &str, periode: &str) -> String {
        match self {
            Decision::Accepter => format!(
                "L'absence de {eleve} ({periode}) sera marquée comme justifiée."
            ),
            Decision::Refuser => format!(
                "La justification de l'absence de {eleve} ({periode}) sera refusée. L'absence restera comptée."
            ),
            Decision::AnnulerPresent => format!(
                "L'absence de {eleve} ({periode}) sera annulée : à utiliser pour une erreur d'appel. Elle ne comptera plus comme absence."
            ),
        }
    }

    /// Message de réussite (notification et annonce).
    pub fn resultat(self, eleve: &str) -> String {
        match self {
            Decision::Accepter => format!("Absence de {eleve} justifiée."),
            Decision::Refuser => format!("Justification de {eleve} refusée."),
            Decision::AnnulerPresent => format!("Absence de {eleve} annulée : élève présent."),
        }
    }

    pub fn note_obligatoire(self) -> bool {
        self == Decision::Refuser
    }

    pub fn libelle_note(self) -> &'static str {
        match self {
            Decision::Refuser => "Motif du refus",
            Decision::Accepter => "Commentaire (facultatif)",
            Decision::AnnulerPresent => "Précision (facultatif)",
        }
    }

    pub fn libelle_confirmer(self) -> &'static str {
        match self {
            Decision::Accepter => "Justifier l'absence",
            Decision::Refuser => "Refuser",
            Decision::AnnulerPresent => "Remettre présent",
        }
    }

    pub fn danger(self) -> bool {
        self != Decision::Accepter
    }
}

/// Décisions proposées pour une absence selon son statut et les droits. Le serveur
/// revérifie : ceci évite seulement de proposer ce qui serait refusé.
pub fn decisions_possibles(statut: StatutAbsence, droits: Droits) -> Vec<Decision> {
    let mut v = Vec::new();
    if droits.trancher_justificatif && statut.est_a_traiter() {
        v.push(Decision::Accepter);
        v.push(Decision::Refuser);
    }
    if droits.annuler_absence_present && statut.compte_comme_absence() {
        v.push(Decision::AnnulerPresent);
    }
    v
}

/// Note saisie : espaces retirés ; vide = absente ; obligatoire pour un refus ; bornée.
pub fn valider_note(decision: Decision, note: &str) -> Result<Option<String>, String> {
    let n = note.trim();
    if n.is_empty() {
        return if decision.note_obligatoire() {
            Err("Indiquez le motif du refus : la famille le verra.".into())
        } else {
            Ok(None)
        };
    }
    if n.chars().count() > NOTE_MAX {
        return Err(format!(
            "Le texte est trop long ({NOTE_MAX} caractères au plus)."
        ));
    }
    Ok(Some(n.to_string()))
}

// ---------------------------------------------------------------------------
// Signaux d'absence répétée
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatutSignal {
    Ouvert,
    EnCours,
    Clos,
    Inconnu,
}

impl StatutSignal {
    pub fn depuis(s: &SignalAbsence) -> StatutSignal {
        match s.statut.as_str() {
            "ouvert" => StatutSignal::Ouvert,
            "en_cours" => StatutSignal::EnCours,
            "clos" => StatutSignal::Clos,
            _ => StatutSignal::Inconnu,
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            StatutSignal::Ouvert => "À prendre en charge",
            StatutSignal::EnCours => "En cours",
            StatutSignal::Clos => "Clos",
            StatutSignal::Inconnu => "Statut inconnu",
        }
    }

    pub fn ton(self) -> TonEtat {
        match self {
            StatutSignal::Ouvert => TonEtat::Mal,
            StatutSignal::EnCours => TonEtat::Avertissement,
            StatutSignal::Clos => TonEtat::Ok,
            StatutSignal::Inconnu => TonEtat::Neutre,
        }
    }
}

/// Actions possibles sur un signal : (prendre en charge, clore).
pub fn actions_signal(s: &SignalAbsence, droits: Droits) -> (bool, bool) {
    if !droits.trancher_justificatif {
        return (false, false);
    }
    match StatutSignal::depuis(s) {
        StatutSignal::Ouvert => (true, true),
        StatutSignal::EnCours => (false, true),
        StatutSignal::Clos | StatutSignal::Inconnu => (false, false),
    }
}

/// « 12 absences depuis la rentrée (seuil : 10) ».
pub fn libelle_progression(s: &SignalAbsence) -> String {
    format!(
        "{} {} depuis la rentrée (seuil : {})",
        s.compteur,
        if s.compteur > 1 {
            "absences"
        } else {
            "absence"
        },
        s.seuil
    )
}

/// Note de clôture : obligatoire (le serveur la refuse vide), bornée.
pub fn valider_note_signal(note: &str) -> Result<String, String> {
    let n = note.trim();
    if n.is_empty() {
        return Err("Indiquez ce qui a été fait (contact avec la famille, convocation…).".into());
    }
    if n.chars().count() > NOTE_MAX {
        return Err(format!(
            "Le texte est trop long ({NOTE_MAX} caractères au plus)."
        ));
    }
    Ok(n.to_string())
}

/// Signaux non clos, les plus avancés d'abord (compteur décroissant).
pub fn signaux_actifs(mut signaux: Vec<SignalAbsence>) -> Vec<SignalAbsence> {
    signaux.retain(|s| StatutSignal::depuis(s) != StatutSignal::Clos);
    signaux.sort_by(|a, b| {
        b.compteur
            .cmp(&a.compteur)
            .then_with(|| a.eleve_label.cmp(&b.eleve_label))
    });
    signaux
}

/// Remplace un signal par sa nouvelle version ; un signal clos disparaît de la liste.
pub fn remplacer_signal(signaux: &mut Vec<SignalAbsence>, nouveau: SignalAbsence) {
    let eleve_label = signaux
        .iter()
        .find(|s| s.id == nouveau.id)
        .and_then(|s| s.eleve_label.clone());
    let mut nouveau = nouveau;
    if nouveau.eleve_label.is_none() {
        nouveau.eleve_label = eleve_label;
    }
    signaux.retain(|s| s.id != nouveau.id);
    if StatutSignal::depuis(&nouveau) != StatutSignal::Clos {
        signaux.push(nouveau);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composants_metier::absences::essais::absence;
    use crate::noyau::droits::droits_du_role;

    fn jour() -> NaiveDate {
        "2026-11-03".parse().unwrap()
    }

    fn classes() -> HashMap<Uuid, String> {
        HashMap::from([
            (Uuid::from_u128(1), "4B".to_string()),
            (Uuid::from_u128(2), "5A".to_string()),
        ])
    }

    fn jeu() -> Vec<Absence> {
        vec![
            absence(1, "a_traiter", None, "2026-11-03"),
            absence(2, "a_traiter", Some("Maladie"), "2026-11-03"),
            absence(3, "justifiee", Some("RDV"), "2026-11-03"),
        ]
    }

    #[test]
    fn filtre_par_statut_classe_et_nom() {
        let l = jeu();
        let c = classes();
        let f = |statut, classe: Option<&str>, recherche: &str| Filtre {
            statut,
            classe: classe.map(String::from),
            recherche: recherche.into(),
        };
        assert_eq!(filtrer(&l, &c, &Filtre::default()).len(), 3);
        let en_attente = filtrer(
            &l,
            &c,
            &f(Some(StatutAbsence::JustificatifEnAttente), None, ""),
        );
        assert_eq!(en_attente.len(), 1);
        assert_eq!(en_attente[0].id, Uuid::from_u128(2));
        assert_eq!(
            filtrer(&l, &c, &f(None, Some("4B"), "")).len(),
            1,
            "classe 4B : un seul élève"
        );
        // Élève 3 sans classe connue : exclu quand une classe est demandée, présent sinon.
        assert!(filtrer(&l, &c, &f(None, Some("4B"), ""))
            .iter()
            .all(|a| a.eleve_id != Uuid::from_u128(3)));
        assert_eq!(
            filtrer(&l, &c, &f(None, None, "ÉLÈVE 3")).len(),
            1,
            "casse et accents"
        );
        assert_eq!(filtrer(&l, &c, &f(None, None, "zzz")).len(), 0);
        // Filtres cumulés.
        assert_eq!(
            filtrer(&l, &c, &f(Some(StatutAbsence::Justifiee), Some("4B"), "")).len(),
            0
        );
        assert_eq!(classes_connues(&c), ["4B", "5A"]);
    }

    #[test]
    fn tri_decisions_en_attente_d_abord() {
        let mut l = jeu();
        l.push(absence(4, "refusee", None, "2026-11-03"));
        l.push(absence(5, "annulee_present", None, "2026-11-03"));
        let ids: Vec<u128> = trier(l).iter().map(|a| a.id.as_u128()).collect();
        assert_eq!(ids, [2, 1, 4, 3, 5]);
    }

    #[test]
    fn decisions_gardees_apres_rechargement() {
        let a = absence(1, "a_traiter", None, "2026-11-03");
        let mut apres = absence(1, "justifiee", None, "2026-11-03");
        apres.eleve_label = None; // la réponse du serveur n'a pas le nom
        let apres = appliquer_decision(&a, apres);
        assert_eq!(apres.eleve_label.as_deref(), Some("Élève 1"));
        let mut decidees = Vec::new();
        memoriser(&mut decidees, apres.clone());
        memoriser(&mut decidees, apres.clone());
        assert_eq!(decidees.len(), 1);

        // Le serveur ne renvoie plus l'absence décidée : elle reste visible, avec son nouveau statut.
        let serveur = vec![absence(2, "a_traiter", None, "2026-11-03")];
        let l = fusionner(serveur, &decidees, jour());
        assert_eq!(l.len(), 2);
        assert!(l
            .iter()
            .any(|x| x.id == apres.id && x.statut == "justifiee"));
        // Liste serveur périmée : la décision de la session l'emporte.
        let l = fusionner(vec![a.clone()], &decidees, jour());
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].statut, "justifiee");
        // Autre jour : une décision d'un autre jour n'apparaît pas.
        let autre_jour: NaiveDate = "2026-11-10".parse().unwrap();
        assert!(fusionner(vec![], &decidees, autre_jour).is_empty());
    }

    #[test]
    fn decisions_proposees_selon_statut_et_droits() {
        use Decision::*;
        let cpe = droits_du_role("cpe");
        let prof = droits_du_role("prof");
        let parent = droits_du_role("parent");
        assert_eq!(
            decisions_possibles(StatutAbsence::JustificatifEnAttente, cpe),
            [Accepter, Refuser, AnnulerPresent]
        );
        assert_eq!(
            decisions_possibles(StatutAbsence::NonJustifiee, cpe),
            [Accepter, Refuser, AnnulerPresent]
        );
        // Déjà décidée : seul le retour présent reste possible.
        assert_eq!(
            decisions_possibles(StatutAbsence::Justifiee, cpe),
            [AnnulerPresent]
        );
        assert_eq!(
            decisions_possibles(StatutAbsence::Refusee, cpe),
            [AnnulerPresent]
        );
        assert!(decisions_possibles(StatutAbsence::AnnuleePresent, cpe).is_empty());
        // Un enseignant ne tranche pas et ne remet pas « présent » (DA-02) : rien à proposer.
        assert!(decisions_possibles(StatutAbsence::JustificatifEnAttente, prof).is_empty());
        assert!(decisions_possibles(StatutAbsence::NonJustifiee, parent).is_empty());
        assert!(decisions_possibles(StatutAbsence::NonJustifiee, Droits::default()).is_empty());
    }

    #[test]
    fn libelles_et_messages_de_decision() {
        assert_eq!(Decision::Accepter.libelle(true), "Accepter le justificatif");
        assert_eq!(Decision::Accepter.libelle(false), "Justifier l'absence");
        assert!(Decision::Refuser.danger() && Decision::AnnulerPresent.danger());
        assert!(!Decision::Accepter.danger());
        let m = Decision::AnnulerPresent.message("Harry Potter", "3 novembre");
        assert!(m.contains("Harry Potter") && m.contains("erreur d'appel"));
        assert!(Decision::Refuser
            .message("Harry Potter", "3 novembre")
            .contains("restera comptée"));
        assert_eq!(
            Decision::Accepter.resultat("Harry Potter"),
            "Absence de Harry Potter justifiée."
        );
        assert_eq!(Decision::Refuser.api(), DecisionApi::Refuser);
    }

    #[test]
    fn note_de_decision() {
        // Refus : motif obligatoire.
        assert!(valider_note(Decision::Refuser, "  ").is_err());
        assert_eq!(
            valider_note(Decision::Refuser, "  Pièce illisible ").unwrap(),
            Some("Pièce illisible".to_string())
        );
        // Acceptation et annulation : note facultative.
        assert_eq!(valider_note(Decision::Accepter, "").unwrap(), None);
        assert_eq!(valider_note(Decision::AnnulerPresent, " ").unwrap(), None);
        // Borne.
        let longue = "é".repeat(NOTE_MAX + 1);
        assert!(valider_note(Decision::Accepter, &longue).is_err());
        assert!(valider_note(Decision::Accepter, &"é".repeat(NOTE_MAX)).is_ok());
    }

    fn signal(n: u128, statut: &str, compteur: i32) -> SignalAbsence {
        SignalAbsence {
            id: Uuid::from_u128(n),
            eleve_id: Uuid::from_u128(n),
            eleve_label: Some(format!("Élève {n}")),
            annee_scolaire: "2026-2027".into(),
            seuil: 10,
            compteur,
            statut: statut.into(),
            note_vs: None,
            traite_par_user_id: None,
            traite_par_label: None,
            fil_alerte_id: None,
            commission_dossier_id: None,
        }
    }

    #[test]
    fn signaux_actions_et_liste() {
        let cpe = droits_du_role("cpe");
        assert_eq!(actions_signal(&signal(1, "ouvert", 10), cpe), (true, true));
        assert_eq!(
            actions_signal(&signal(1, "en_cours", 10), cpe),
            (false, true)
        );
        assert_eq!(actions_signal(&signal(1, "clos", 10), cpe), (false, false));
        assert_eq!(
            actions_signal(&signal(1, "bizarre", 10), cpe),
            (false, false)
        );
        assert_eq!(
            actions_signal(&signal(1, "ouvert", 10), droits_du_role("prof")),
            (false, false),
            "un enseignant ne gère pas les signaux"
        );
        let l = signaux_actifs(vec![
            signal(1, "ouvert", 10),
            signal(2, "clos", 30),
            signal(3, "en_cours", 14),
        ]);
        let ids: Vec<u128> = l.iter().map(|s| s.id.as_u128()).collect();
        assert_eq!(ids, [3, 1], "clos retiré, plus avancé d'abord");
        assert_eq!(
            libelle_progression(&signal(1, "ouvert", 12)),
            "12 absences depuis la rentrée (seuil : 10)"
        );
        assert_eq!(
            libelle_progression(&signal(1, "ouvert", 1)),
            "1 absence depuis la rentrée (seuil : 10)"
        );
        assert_eq!(
            StatutSignal::depuis(&signal(1, "en_cours", 1)).libelle(),
            "En cours"
        );
    }

    #[test]
    fn remplacement_d_un_signal() {
        let mut l = vec![signal(1, "ouvert", 10), signal(3, "ouvert", 12)];
        let mut pris = signal(1, "en_cours", 10);
        pris.eleve_label = None;
        remplacer_signal(&mut l, pris);
        let s = l.iter().find(|s| s.id == Uuid::from_u128(1)).unwrap();
        assert_eq!(s.statut, "en_cours");
        assert_eq!(s.eleve_label.as_deref(), Some("Élève 1"), "nom conservé");
        remplacer_signal(&mut l, signal(1, "clos", 10));
        assert_eq!(l.len(), 1, "un signal clos quitte la liste");
    }

    #[test]
    fn note_de_cloture_obligatoire() {
        assert!(valider_note_signal("   ").is_err());
        assert_eq!(
            valider_note_signal(" Famille contactée ").unwrap(),
            "Famille contactée"
        );
        assert!(valider_note_signal(&"x".repeat(NOTE_MAX + 1)).is_err());
    }
}
