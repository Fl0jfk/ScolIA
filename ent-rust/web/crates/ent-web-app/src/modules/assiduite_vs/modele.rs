//! Modèle des écrans d'assiduité de la vie scolaire (socle 12, AS14b) : logique pure, sans Leptos ni
//! réseau. Les décisions (qui qualifie, quels motifs sont légitimes, ce que devient un avis) restent
//! au serveur ; ce module ne fait que libeller, trier, composer les corps d'écriture et contrôler la
//! saisie avant l'envoi.

use ent_types::assiduite::{
    AvisDemiJourneeDto, EpisodeDto, EpisodeSansReponseDto, NatureMotifDto, QualifierEpisodeBody,
    SelectionMasseBody,
};
use uuid::Uuid;

/// Issues d'une qualification (liste fermée du serveur) : code et libellé.
pub const QUALIFICATIONS: [(&str, &str); 4] = [
    ("reputee_legitime", "Motif légitime (L131-8)"),
    ("excuse_valable", "Excusée, motif valable"),
    ("non_valable", "Motif non valable"),
    ("sans_motif", "Sans motif"),
];

/// Plafond de qualification en masse (le serveur refuse au-delà).
pub const MAX_MASSE: usize = 200;
/// Longueur maximale de la note interne.
pub const MAX_NOTE: usize = 500;

pub fn libelle_qualification(code: &str) -> &'static str {
    QUALIFICATIONS
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| *l)
        .unwrap_or("À qualifier")
}

pub fn libelle_etat_episode(etat: &str) -> &'static str {
    match etat {
        "ouvert" => "Ouvert, à qualifier",
        "a_qualifier" => "À qualifier",
        "qualifie" => "Qualifié",
        "retire" => "Retiré",
        _ => "État inconnu",
    }
}

pub fn libelle_source(source: &str) -> &'static str {
    match source {
        "appel" => "Appel",
        "famille" => "Famille",
        "accueil" => "Accueil",
        "declaration" => "Déclaration",
        _ => "Autre",
    }
}

/// Libellés à liste fermée montrés à la famille : code et texte.
pub const LIBELLES_FAMILLE: [(&str, &str); 6] = [
    ("absence_justifiee", "Absence justifiée"),
    ("absence_excusee", "Absence excusée"),
    ("motif_non_retenu", "Motif non retenu"),
    (
        "motif_non_retenu_delai",
        "Motif non retenu : justificatif reçu hors délai",
    ),
    (
        "motif_non_retenu_piece",
        "Motif non retenu : pièce justificative manquante",
    ),
    ("motif_non_communique", "Aucun motif communiqué"),
];

pub fn libelle_famille(code: &str) -> &'static str {
    LIBELLES_FAMILLE
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, l)| *l)
        .unwrap_or("Absence à régulariser")
}

/// Libellés cohérents avec une qualification (jamais « justifiée » pour un motif non valable).
pub fn libelles_famille_pour(qualification: &str) -> Vec<(&'static str, &'static str)> {
    LIBELLES_FAMILLE
        .iter()
        .copied()
        .filter(|(c, _)| match qualification {
            "reputee_legitime" | "excuse_valable" => {
                matches!(*c, "absence_justifiee" | "absence_excusee")
            }
            "non_valable" => c.starts_with("motif_non_retenu"),
            "sans_motif" => *c == "motif_non_communique",
            _ => false,
        })
        .collect()
}

/// Les cinq motifs de L131-8 (natures légitimes) pour une qualification « motif légitime ».
pub fn natures_legitimes(natures: &[NatureMotifDto]) -> Vec<(String, String)> {
    let mut v: Vec<&NatureMotifDto> = natures.iter().filter(|n| n.legitime_l131_8).collect();
    v.sort_by_key(|n| n.ordre);
    v.into_iter()
        .map(|n| (n.code.clone(), n.libelle.clone()))
        .collect()
}

/// Séances encore comptées (une séance dont le constat a été défait reste listée, « retirée »).
pub fn nb_seances_actives(e: &EpisodeDto) -> usize {
    e.seances.iter().filter(|s| !s.retiree).count()
}

/// Plus récent d'abord, puis par nom.
pub fn trier_episodes(mut v: Vec<EpisodeDto>) -> Vec<EpisodeDto> {
    v.sort_by(|a, b| {
        b.jour_debut
            .cmp(&a.jour_debut)
            .then_with(|| a.eleve_label.cmp(&b.eleve_label))
            .then_with(|| a.id.cmp(&b.id))
    });
    v
}

/// Nom affiché d'un élève d'épisode.
pub fn nom_eleve(e: &EpisodeDto) -> String {
    e.eleve_label
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| "Élève sans nom".to_string())
}

/// Saisie du formulaire de qualification.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SaisieQualification {
    pub qualification: String,
    pub nature_code: String,
    pub libelle_famille: String,
    pub note: String,
}

/// Contrôle local (le serveur revérifie) puis corps d'écriture : une qualification choisie, une nature
/// légitime pour « motif légitime », un libellé cohérent, une note courte. La version lue est envoyée :
/// un épisode modifié entre-temps est refusé (conflit).
pub fn corps_qualification(
    s: &SaisieQualification,
    version: i32,
    proposition_id: Option<Uuid>,
) -> Result<QualifierEpisodeBody, &'static str> {
    if !QUALIFICATIONS.iter().any(|(c, _)| *c == s.qualification) {
        return Err("Choisissez une qualification.");
    }
    if s.qualification == "reputee_legitime" && s.nature_code.trim().is_empty() {
        return Err("Choisissez l'un des motifs légitimes.");
    }
    let libelle = s.libelle_famille.trim();
    if !libelle.is_empty()
        && !libelles_famille_pour(&s.qualification)
            .iter()
            .any(|(c, _)| *c == libelle)
    {
        return Err("Ce libellé ne correspond pas à la qualification choisie.");
    }
    let note = s.note.trim();
    if note.chars().count() > MAX_NOTE {
        return Err("La note interne est trop longue (500 caractères au plus).");
    }
    Ok(QualifierEpisodeBody {
        qualification: s.qualification.clone(),
        version,
        nature_code: (s.qualification == "reputee_legitime" || !s.nature_code.trim().is_empty())
            .then(|| s.nature_code.trim().to_string())
            .filter(|n| !n.is_empty()),
        motif_id: None,
        libelle_famille_code: (!libelle.is_empty()).then(|| libelle.to_string()),
        note_interne: (!note.is_empty()).then(|| note.to_string()),
        proposition_id,
    })
}

/// Corps d'une qualification en masse (aperçu sans empreinte, confirmation avec l'empreinte).
pub fn corps_masse(
    ids: &[Uuid],
    qualification: &str,
    libelle_famille: &str,
    empreinte: Option<String>,
) -> Result<SelectionMasseBody, &'static str> {
    if ids.is_empty() {
        return Err("Sélectionnez au moins une absence.");
    }
    if ids.len() > MAX_MASSE {
        return Err("Trop d'absences sélectionnées (200 au plus).");
    }
    if !QUALIFICATIONS.iter().any(|(c, _)| *c == qualification) {
        return Err("Choisissez une qualification.");
    }
    let libelle = libelle_famille.trim();
    Ok(SelectionMasseBody {
        absence_ids: ids.to_vec(),
        qualification: qualification.to_string(),
        libelle_famille_code: (!libelle.is_empty()).then(|| libelle.to_string()),
        note_interne: None,
        empreinte,
    })
}

/// Raison, en français, pour laquelle une absence n'est pas qualifiée en masse.
pub fn libelle_ecart(code: &str) -> &'static str {
    match code {
        "deja_qualifie" | "deja_qualifiee" => "déjà qualifiée",
        "retire" | "retiree" => "retirée",
        "nature_non_legitime" | "nature_requise" => "motif légitime à choisir une par une",
        "introuvable" => "introuvable",
        "version" | "modifie" => "modifiée entre-temps",
        _ => "écartée par le serveur",
    }
}

// ─── Avis aux familles (AS5) ──────────────────────────────────────────────────────────────────────

/// Ce qu'est devenu l'avis d'une demi-journée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EtatAvis {
    /// Programmé : part à la fin du délai de consolidation si rien ne l'annule.
    Prevu,
    EnCours,
    Parti,
    Annule,
    /// Un avis parti à tort a reçu son démenti (R-AS14).
    Dementi,
}

pub fn etat_avis(a: &AvisDemiJourneeDto) -> EtatAvis {
    match a.statut.as_str() {
        "dementi" => EtatAvis::Dementi,
        "annule" => EtatAvis::Annule,
        _ => match a.avis_statut.as_str() {
            "planifie" => EtatAvis::Prevu,
            "en_cours" => EtatAvis::EnCours,
            "annule" => EtatAvis::Annule,
            _ => EtatAvis::Parti,
        },
    }
}

impl EtatAvis {
    pub fn libelle(self) -> &'static str {
        match self {
            EtatAvis::Prevu => "Prévu",
            EtatAvis::EnCours => "En cours d'envoi",
            EtatAvis::Parti => "Parti",
            EtatAvis::Annule => "Annulé avant envoi",
            EtatAvis::Dementi => "Démenti envoyé",
        }
    }
}

pub fn libelle_periode_avis(p: &str) -> &'static str {
    match p {
        "matin" => "matin",
        "apres_midi" => "après-midi",
        _ => "demi-journée",
    }
}

/// Cause d'un avis annulé ou démenti, en français.
pub fn libelle_cause(code: Option<&str>) -> &'static str {
    match code {
        Some("retard") => "l'élève est arrivé dans le délai",
        Some("correction") => "l'appel a été corrigé",
        Some("sujet_gele") => "dossier gelé",
        Some("absence_levee") => "l'absence a été levée",
        Some(_) => "situation modifiée",
        None => "",
    }
}

/// Étape de la relance graduée, en français.
pub fn libelle_etape(code: &str) -> &'static str {
    match code {
        "rappel" => "Rappel (J+1)",
        "canal_externe" => "Canal externe (J+3)",
        "sans_reponse" => "Sans réponse (J+5) : courrier à décider",
        "proposition_sans_motif" => "Proposition « sans motif » (J+8)",
        _ => "Relance",
    }
}

/// File « sans réponse » : les plus anciens d'abord (ce qui attend depuis le plus longtemps).
pub fn trier_sans_reponse(mut v: Vec<EpisodeSansReponseDto>) -> Vec<EpisodeSansReponseDto> {
    v.sort_by(|a, b| {
        a.jour_debut
            .cmp(&b.jour_debut)
            .then_with(|| a.absence_id.cmp(&b.absence_id))
    });
    v
}

pub fn libelle_presentation_justificatif(p: &str) -> &'static str {
    match p {
        "texte" => "Texte",
        "depose" => "Pièce déposée",
        "vu_non_conserve" => "Certificat vu, non conservé",
        _ => "Justificatif",
    }
}

pub fn libelle_depose_par(qualite: &str) -> &'static str {
    match qualite {
        "famille" => "la famille",
        "eleve_majeur" => "l'élève majeur",
        "vie_scolaire" => "la vie scolaire",
        _ => "un tiers",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ent_types::assiduite::SeanceEpisodeDto;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn episode(n: u128, nom: &str, jour: u32) -> EpisodeDto {
        EpisodeDto {
            id: id(n),
            eleve_id: id(100 + n),
            eleve_label: Some(nom.into()),
            jour_debut: NaiveDate::from_ymd_opt(2026, 12, jour).unwrap(),
            jour_fin: NaiveDate::from_ymd_opt(2026, 12, jour).unwrap(),
            etat: "a_qualifier".into(),
            qualification: "a_qualifier".into(),
            nature_code: None,
            motif_id: None,
            prevenue: false,
            source: "appel".into(),
            libelle_famille_code: None,
            version: 3,
            seances: vec![],
            justificatifs: 0,
            propositions_en_attente: 0,
            textes: vec![],
            note_interne: None,
        }
    }

    fn avis(statut: &str, avis_statut: &str) -> AvisDemiJourneeDto {
        AvisDemiJourneeDto {
            id: id(1),
            eleve_id: id(2),
            jour: NaiveDate::from_ymd_opt(2026, 12, 3).unwrap(),
            periode: "matin".into(),
            generation: 1,
            statut: statut.into(),
            cause_code: None,
            avis_id: id(3),
            dementi_avis_id: None,
            avis_statut: avis_statut.into(),
        }
    }

    #[test]
    fn tri_recent_d_abord_puis_par_nom() {
        let t = trier_episodes(vec![
            episode(1, "Zoé", 2),
            episode(2, "Alice", 3),
            episode(3, "Bob", 3),
        ]);
        let noms: Vec<String> = t.iter().map(nom_eleve).collect();
        assert_eq!(noms, ["Alice", "Bob", "Zoé"]);
    }

    #[test]
    fn libelles_famille_coherents_avec_la_qualification() {
        let ok: Vec<&str> = libelles_famille_pour("non_valable")
            .iter()
            .map(|(c, _)| *c)
            .collect();
        assert_eq!(
            ok,
            [
                "motif_non_retenu",
                "motif_non_retenu_delai",
                "motif_non_retenu_piece"
            ]
        );
        assert!(libelles_famille_pour("sans_motif")
            .iter()
            .all(|(c, _)| *c == "motif_non_communique"));
        assert!(!libelles_famille_pour("non_valable")
            .iter()
            .any(|(c, _)| *c == "absence_justifiee"));
        assert!(libelles_famille_pour("inconnue").is_empty());
    }

    #[test]
    fn motif_legitime_exige_une_nature_des_cinq() {
        let s = SaisieQualification {
            qualification: "reputee_legitime".into(),
            ..Default::default()
        };
        assert!(corps_qualification(&s, 3, None).is_err());
        let s = SaisieQualification {
            nature_code: "maladie_enfant".into(),
            ..s
        };
        let c = corps_qualification(&s, 3, None).unwrap();
        assert_eq!(c.version, 3);
        assert_eq!(c.nature_code.as_deref(), Some("maladie_enfant"));
        assert_eq!(c.libelle_famille_code, None);
    }

    #[test]
    fn libelle_incoherent_ou_note_trop_longue_refuses() {
        let mut s = SaisieQualification {
            qualification: "non_valable".into(),
            libelle_famille: "absence_justifiee".into(),
            ..Default::default()
        };
        assert!(corps_qualification(&s, 1, None).is_err());
        s.libelle_famille = "motif_non_retenu_delai".into();
        assert!(corps_qualification(&s, 1, None).is_ok());
        s.note = "x".repeat(MAX_NOTE + 1);
        assert!(corps_qualification(&s, 1, None).is_err());
    }

    #[test]
    fn la_proposition_acceptee_est_transmise() {
        let s = SaisieQualification {
            qualification: "sans_motif".into(),
            ..Default::default()
        };
        let c = corps_qualification(&s, 2, Some(id(9))).unwrap();
        assert_eq!(c.proposition_id, Some(id(9)));
        assert_eq!(c.nature_code, None);
    }

    #[test]
    fn masse_bornee_et_avec_empreinte() {
        assert!(corps_masse(&[], "sans_motif", "", None).is_err());
        let beaucoup: Vec<Uuid> = (0..201).map(id).collect();
        assert!(corps_masse(&beaucoup, "sans_motif", "", None).is_err());
        assert!(corps_masse(&[id(1)], "n_importe_quoi", "", None).is_err());
        let c = corps_masse(&[id(1)], "sans_motif", "", Some("abc".into())).unwrap();
        assert_eq!(c.empreinte.as_deref(), Some("abc"));
        assert_eq!(c.libelle_famille_code, None);
    }

    #[test]
    fn natures_legitimes_triees_et_filtrees() {
        let n = |c: &str, l: bool, o: i16| NatureMotifDto {
            code: c.into(),
            libelle: c.into(),
            legitime_l131_8: l,
            medical: false,
            ordre: o,
        };
        let v = natures_legitimes(&[n("b", true, 2), n("x", false, 1), n("a", true, 1)]);
        assert_eq!(
            v.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn etat_d_un_avis_prevu_parti_annule_dementi() {
        assert_eq!(etat_avis(&avis("programme", "planifie")), EtatAvis::Prevu);
        assert_eq!(etat_avis(&avis("programme", "en_cours")), EtatAvis::EnCours);
        assert_eq!(etat_avis(&avis("programme", "termine")), EtatAvis::Parti);
        assert_eq!(etat_avis(&avis("annule", "annule")), EtatAvis::Annule);
        assert_eq!(etat_avis(&avis("dementi", "termine")), EtatAvis::Dementi);
        assert_eq!(
            libelle_cause(Some("retard")),
            "l'élève est arrivé dans le délai"
        );
    }

    #[test]
    fn seances_retirees_ne_comptent_pas() {
        let mut e = episode(1, "A", 3);
        let s = |r| SeanceEpisodeDto {
            id: id(5),
            jour: e.jour_debut,
            kind: "cours".into(),
            periode: Some("matin".into()),
            heure_debut: None,
            heure_fin: None,
            prevenue: false,
            retiree: r,
        };
        e.seances = vec![s(false), s(true), s(false)];
        assert_eq!(nb_seances_actives(&e), 2);
    }

    #[test]
    fn sans_reponse_les_plus_anciens_d_abord() {
        let r = |n: u128, j: u32| EpisodeSansReponseDto {
            absence_id: id(n),
            eleve_id: id(50),
            jour_debut: NaiveDate::from_ymd_opt(2026, 12, j).unwrap(),
            jour_fin: NaiveDate::from_ymd_opt(2026, 12, j).unwrap(),
            derniere_etape: "sans_reponse".into(),
            courrier_en_cours: false,
        };
        let t = trier_sans_reponse(vec![r(1, 9), r(2, 2)]);
        assert_eq!(t[0].absence_id, id(2));
    }
}
