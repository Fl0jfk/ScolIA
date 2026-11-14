//! Logique d'écran pure du lot C3 (agenda, cahier de l'élève et de la famille) : sans Leptos ni réseau, testée en natif.
//!
//! **Aucune règle métier ici** : droits du lien, public du cours, statuts constatés et coche restent décidés par le
//! serveur (`ent-db`). Ce module regroupe par jour, trie, formate les libellés et compte ce qui reste à faire ;
//! les jours et heures affichés sont ceux **de l'établissement**, donnés par le serveur.

use chrono::NaiveDate;
use ent_web_api::cahier_c3_types::{
    AgendaLigneDto, GENRE_EPREUVE, GENRE_EVALUATION, GENRE_SORTIE, GENRE_TRAVAIL, STATUT_A_FAIRE,
    STATUT_DISPENSE, STATUT_EN_RETARD, STATUT_EXCUSE, STATUT_NON_RENDU, STATUT_RENDU,
    STATUT_RETOUR,
};
use ent_web_ui::TonEtat;

pub use super::modele::{
    fin_de_semaine, libelle_duree, libelle_semaine, semaine_demandee, semaine_voisine,
};

/// Onglets de l'écran : agenda de la semaine, ce que j'ai manqué, résumé, calendrier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Onglet {
    Agenda,
    Manque,
    Semaine,
    Calendrier,
}

impl Onglet {
    pub const TOUS: [Onglet; 4] = [
        Onglet::Agenda,
        Onglet::Manque,
        Onglet::Semaine,
        Onglet::Calendrier,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::Agenda => "agenda",
            Self::Manque => "manque",
            Self::Semaine => "semaine",
            Self::Calendrier => "calendrier",
        }
    }

    pub fn libelle(self) -> &'static str {
        match self {
            Self::Agenda => "Agenda",
            Self::Manque => "Ce que j'ai manqué",
            Self::Semaine => "La semaine",
            Self::Calendrier => "Calendrier",
        }
    }

    /// Onglet de l'adresse (`?onglet=`) ; tout ce qui n'est pas reconnu donne l'agenda.
    pub fn depuis(param: Option<&str>) -> Self {
        Self::TOUS
            .into_iter()
            .find(|o| Some(o.code()) == param.map(str::trim))
            .unwrap_or(Self::Agenda)
    }
}

/// Un jour de l'agenda avec ses lignes et ses totaux.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JourC3 {
    pub jour: NaiveDate,
    pub lignes: Vec<AgendaLigneDto>,
    /// Durée estimée cumulée des travaux de ce jour.
    pub minutes: i32,
    pub nb_evaluations: usize,
}

/// Regroupe par jour d'échéance (ordre du serveur conservé dans un jour).
pub fn grouper_par_jour(lignes: Vec<AgendaLigneDto>) -> Vec<JourC3> {
    let mut jours: Vec<JourC3> = Vec::new();
    for l in lignes {
        if jours.last().map(|j| j.jour) != Some(l.jour) {
            jours.push(JourC3 {
                jour: l.jour,
                lignes: Vec::new(),
                minutes: 0,
                nb_evaluations: 0,
            });
        }
        if let Some(j) = jours.last_mut() {
            if l.genre == GENRE_TRAVAIL {
                j.minutes += i32::from(l.duree_estimee_min.unwrap_or(0));
            }
            if l.genre == GENRE_EVALUATION {
                j.nb_evaluations += 1;
            }
            j.lignes.push(l);
        }
    }
    jours
}

/// Libellé du genre d'une ligne.
pub fn libelle_genre(genre: &str) -> &'static str {
    match genre {
        GENRE_EVALUATION => "Évaluation",
        GENRE_EPREUVE => "Épreuve",
        GENRE_SORTIE => "Sortie",
        GENRE_TRAVAIL => "Travail",
        _ => "À venir",
    }
}

/// Ton d'un genre : une évaluation attire l'œil (un seul accent), le reste reste neutre.
pub fn ton_genre(genre: &str) -> TonEtat {
    match genre {
        GENRE_EVALUATION | GENRE_EPREUVE => TonEtat::Avertissement,
        GENRE_SORTIE => TonEtat::Info,
        _ => TonEtat::Neutre,
    }
}

/// Statut **constaté** : un fait, jamais un jugement (R-C12). `None` : aucune remise attendue, rien à dire.
pub fn libelle_statut(statut: Option<&str>) -> Option<(&'static str, TonEtat)> {
    Some(match statut? {
        STATUT_A_FAIRE => ("À rendre", TonEtat::Neutre),
        STATUT_RENDU => ("Rendu", TonEtat::Ok),
        STATUT_EN_RETARD => ("Rendu en retard", TonEtat::Avertissement),
        STATUT_NON_RENDU => ("Non rendu", TonEtat::Mal),
        STATUT_DISPENSE => ("Dispensé", TonEtat::Neutre),
        STATUT_EXCUSE => ("Excusé", TonEtat::Neutre),
        STATUT_RETOUR => ("Retour disponible", TonEtat::Info),
        _ => return None,
    })
}

/// Nombre de travaux encore à faire : pour l'élève, ceux qu'il n'a pas cochés ni rendus ; pour une famille (sans coche),
/// ceux dont le statut constaté n'est ni rendu ni excusé ni dispensé et qui ne sont pas des travaux à seulement lire.
pub fn nb_a_faire(lignes: &[AgendaLigneDto]) -> usize {
    lignes
        .iter()
        .filter(|l| l.genre == GENRE_TRAVAIL)
        .filter(|l| match l.fait {
            Some(fait) => {
                !fait
                    && !matches!(
                        l.statut.as_deref(),
                        Some(STATUT_RENDU | STATUT_RETOUR | STATUT_EXCUSE | STATUT_DISPENSE)
                    )
            }
            None => matches!(l.statut.as_deref(), Some(STATUT_A_FAIRE | STATUT_NON_RENDU)),
        })
        .count()
}

/// Phrase de synthèse de l'agenda (lue par les lecteurs d'écran).
pub fn synthese(lignes: &[AgendaLigneDto]) -> String {
    let n = nb_a_faire(lignes);
    let e = lignes
        .iter()
        .filter(|l| l.genre == GENRE_EVALUATION)
        .count();
    let travaux = match n {
        0 => "Aucun travail en attente".to_string(),
        1 => "1 travail encore à faire".to_string(),
        n => format!("{n} travaux encore à faire"),
    };
    match e {
        0 => format!("{travaux}."),
        1 => format!("{travaux}, 1 évaluation annoncée."),
        e => format!("{travaux}, {e} évaluations annoncées."),
    }
}

/// Début de la période « ce que j'ai manqué » : les quatorze derniers jours, ni plus ni moins (borne du serveur : 100 jours).
pub fn periode_manque(aujourdhui: NaiveDate) -> (NaiveDate, NaiveDate) {
    (aujourdhui - chrono::Duration::days(14), aujourdhui)
}

/// Adresse complète d'abonnement à partir de l'origine du site et du chemin donné par le serveur. Refuse un chemin qui
/// ne vient pas du serveur (aucune redirection ouverte, aucun autre schéma).
pub fn adresse_abonnement(origine: &str, chemin: &str) -> Option<String> {
    let origine = origine.trim_end_matches('/');
    let ok = chemin.starts_with("/cahier/c3/calendrier/")
        && chemin.len() < 200
        && !chemin.contains("..")
        && chemin
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'.' | b'_'));
    (ok && (origine.starts_with("https://") || origine.starts_with("http://")))
        .then(|| format!("{origine}{chemin}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ligne(
        genre: &str,
        jour: u32,
        duree: Option<i16>,
        fait: Option<bool>,
        statut: Option<&str>,
    ) -> AgendaLigneDto {
        AgendaLigneDto {
            genre: genre.into(),
            id: Uuid::from_u128(1),
            cours_id: None,
            matiere: "Maths".into(),
            titre: "T".into(),
            nature: None,
            consigne: None,
            jour: NaiveDate::from_ymd_opt(2026, 11, jour).unwrap(),
            heure: None,
            duree_estimee_min: duree,
            rendu: None,
            auteur: None,
            statut: statut.map(Into::into),
            fait,
            echeance_prolongee: false,
            modifie_le: None,
            version: 1,
        }
    }

    #[test]
    fn onglet_de_l_adresse_avec_repli_sur_l_agenda() {
        assert_eq!(Onglet::depuis(Some("manque")), Onglet::Manque);
        assert_eq!(Onglet::depuis(Some(" calendrier ")), Onglet::Calendrier);
        assert_eq!(Onglet::depuis(Some("n'importe quoi")), Onglet::Agenda);
        assert_eq!(Onglet::depuis(None), Onglet::Agenda);
        for o in Onglet::TOUS {
            assert_eq!(Onglet::depuis(Some(o.code())), o);
        }
    }

    #[test]
    fn regroupement_par_jour_avec_totaux() {
        let jours = grouper_par_jour(vec![
            ligne(GENRE_EVALUATION, 16, None, None, None),
            ligne(GENRE_TRAVAIL, 16, Some(30), Some(false), None),
            ligne(GENRE_TRAVAIL, 16, Some(20), Some(true), None),
            ligne(GENRE_SORTIE, 18, None, None, None),
        ]);
        assert_eq!(jours.len(), 2);
        assert_eq!(
            (
                jours[0].minutes,
                jours[0].nb_evaluations,
                jours[0].lignes.len()
            ),
            (50, 1, 3)
        );
        assert_eq!((jours[1].minutes, jours[1].nb_evaluations), (0, 0));
        assert!(grouper_par_jour(vec![]).is_empty());
    }

    #[test]
    fn a_faire_selon_la_coche_ou_le_statut_constate() {
        // Élève : la coche et le rendu comptent ; une évaluation n'est pas « à faire ».
        let eleve = [
            ligne(GENRE_TRAVAIL, 16, None, Some(false), None),
            ligne(GENRE_TRAVAIL, 16, None, Some(true), None),
            ligne(GENRE_TRAVAIL, 16, None, Some(false), Some(STATUT_RENDU)),
            ligne(GENRE_EVALUATION, 16, None, None, None),
        ];
        assert_eq!(nb_a_faire(&eleve), 1);
        // Famille (aucune coche) : seul le statut constaté parle.
        let famille = [
            ligne(GENRE_TRAVAIL, 16, None, None, Some(STATUT_A_FAIRE)),
            ligne(GENRE_TRAVAIL, 16, None, None, Some(STATUT_NON_RENDU)),
            ligne(GENRE_TRAVAIL, 16, None, None, Some(STATUT_RENDU)),
            ligne(GENRE_TRAVAIL, 16, None, None, None),
        ];
        assert_eq!(nb_a_faire(&famille), 2);
    }

    #[test]
    fn synthese_au_singulier_et_au_pluriel() {
        assert_eq!(synthese(&[]), "Aucun travail en attente.");
        let l = [
            ligne(GENRE_TRAVAIL, 16, None, Some(false), None),
            ligne(GENRE_EVALUATION, 17, None, None, None),
        ];
        assert_eq!(
            synthese(&l),
            "1 travail encore à faire, 1 évaluation annoncée."
        );
        let l = [
            ligne(GENRE_TRAVAIL, 16, None, Some(false), None),
            ligne(GENRE_TRAVAIL, 16, None, Some(false), None),
            ligne(GENRE_EVALUATION, 17, None, None, None),
            ligne(GENRE_EVALUATION, 18, None, None, None),
        ];
        assert_eq!(
            synthese(&l),
            "2 travaux encore à faire, 2 évaluations annoncées."
        );
    }

    #[test]
    fn statuts_constates_en_faits_jamais_en_jugement() {
        assert_eq!(libelle_statut(None), None);
        assert_eq!(libelle_statut(Some("inconnu")), None);
        assert_eq!(
            libelle_statut(Some(STATUT_NON_RENDU)).unwrap().0,
            "Non rendu"
        );
        assert_eq!(
            libelle_statut(Some(STATUT_EXCUSE)).unwrap().1,
            TonEtat::Neutre,
            "un excusé n'est pas un échec"
        );
        assert_eq!(
            libelle_statut(Some(STATUT_RETOUR)).unwrap().0,
            "Retour disponible"
        );
    }

    #[test]
    fn adresse_d_abonnement_refuse_tout_ce_qui_ne_vient_pas_du_serveur() {
        let j = format!("{}-{}", "a".repeat(32), "b".repeat(64));
        let chemin = format!("/cahier/c3/calendrier/{j}.ics");
        assert_eq!(
            adresse_abonnement("https://ent.example/", &chemin),
            Some(format!("https://ent.example{chemin}"))
        );
        assert_eq!(adresse_abonnement("javascript:alert(1)//", &chemin), None);
        assert_eq!(
            adresse_abonnement("https://ent.example", "/autre/chemin"),
            None
        );
        assert_eq!(
            adresse_abonnement("https://ent.example", "/cahier/c3/calendrier/../x"),
            None
        );
        assert_eq!(
            adresse_abonnement("https://ent.example", "/cahier/c3/calendrier/a b"),
            None
        );
        assert_eq!(
            adresse_abonnement("https://ent.example", "/cahier/c3/calendrier/a?x=1"),
            None
        );
    }

    #[test]
    fn periode_manque_quatorze_jours() {
        let j = NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
        let (du, au) = periode_manque(j);
        assert_eq!((au - du).num_days(), 14);
        assert_eq!(au, j);
    }
}
