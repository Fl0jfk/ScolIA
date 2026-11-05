//! Logique de l'accueil (pure, testée) : messages récents.

use ent_web_api::FilMessages;

/// Fils non archivés, les plus récents d'abord, et total des messages non lus.
pub fn fils_recents(mut fils: Vec<FilMessages>, max: usize) -> (Vec<FilMessages>, i64) {
    fils.retain(|f| !f.archive);
    let non_lus = fils.iter().map(|f| f.nb_non_lus.max(0)).sum();
    fils.sort_by(|a, b| b.last_message_at.cmp(&a.last_message_at));
    fils.truncate(max);
    (fils, non_lus)
}

/// « 1 message non lu », « 3 messages non lus », « Aucun message non lu ».
pub fn libelle_non_lus(n: i64) -> String {
    match n {
        0 => "Aucun message non lu".into(),
        1 => "1 message non lu".into(),
        n => format!("{n} messages non lus"),
    }
}

/// « Aucun élève absent », « 1 élève absent », « 7 élèves absents ».
pub fn libelle_absents(n: usize) -> String {
    match n {
        0 => "Aucun élève absent".into(),
        1 => "1 élève absent".into(),
        n => format!("{n} élèves absents"),
    }
}

/// « Aucune absence à justifier », « 3 absences à justifier ».
pub fn libelle_a_justifier(n: usize) -> String {
    match n {
        0 => "Aucune absence à justifier".into(),
        1 => "1 absence à justifier".into(),
        n => format!("{n} absences à justifier"),
    }
}

/// « Aucun justificatif en attente », « 2 justificatifs en attente ».
pub fn libelle_en_attente(n: usize) -> String {
    match n {
        0 => "Aucun justificatif en attente".into(),
        1 => "1 justificatif en attente".into(),
        n => format!("{n} justificatifs en attente"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn messages_recents_et_non_lus() {
        let fil = |sujet: &str, date: &str, non_lus: i64, archive: bool| FilMessages {
            id: Uuid::from_u128(1),
            eleve_label: None,
            sujet: sujet.into(),
            last_message_at: date.into(),
            archive,
            nb_non_lus: non_lus,
        };
        let (fils, n) = fils_recents(
            vec![
                fil("ancien", "2026-10-01T08:00:00Z", 1, false),
                fil("récent", "2026-11-02T08:00:00Z", 2, false),
                fil("archivé", "2026-11-03T08:00:00Z", 5, true),
            ],
            1,
        );
        assert_eq!(n, 3, "les fils archivés ne comptent pas");
        assert_eq!(fils.len(), 1);
        assert_eq!(fils[0].sujet, "récent");
        assert_eq!(libelle_non_lus(0), "Aucun message non lu");
        assert_eq!(libelle_non_lus(1), "1 message non lu");
        assert_eq!(libelle_non_lus(4), "4 messages non lus");
    }

    #[test]
    fn libelles_absences() {
        assert_eq!(libelle_absents(0), "Aucun élève absent");
        assert_eq!(libelle_absents(1), "1 élève absent");
        assert_eq!(libelle_absents(7), "7 élèves absents");
        assert_eq!(libelle_a_justifier(0), "Aucune absence à justifier");
        assert_eq!(libelle_a_justifier(1), "1 absence à justifier");
        assert_eq!(libelle_a_justifier(3), "3 absences à justifier");
        assert_eq!(libelle_en_attente(0), "Aucun justificatif en attente");
        assert_eq!(libelle_en_attente(2), "2 justificatifs en attente");
    }
}
