//! Logique d'écran pure du lot C5 (rendu et retour) : sans Leptos ni réseau, testée en natif.
//!
//! **Aucune règle métier ici** : retard, clôture, droits de lecture d'une copie, immuabilité d'un dépôt et motif d'une
//! prolongation restent au serveur. Ce module formate ce que le serveur annonce, prépare les corps à envoyer et trie le
//! tableau de suivi. Le serveur donne les jours et heures de l'établissement : aucun fuseau n'est deviné ici.

use chrono::{NaiveDate, NaiveTime};
use ent_web_api::cahier_c5_types::{
    ClotureBody, ComptesSuiviDto, ConstatLigneBody, ConstatsBody, LigneSuiviDto, ProlongationBody,
    RemiseLueDto, CONSTAT_DISPENSE, CONSTAT_EXCUSE, CONSTAT_NON_RENDU, CONSTAT_RENDU_PAPIER,
};
use ent_web_ui::TonEtat;
use uuid::Uuid;

use super::modele_c2::{lire_heure_champ, lire_jour};

/// Motifs **internes** d'une prolongation (jamais montrés à l'élève) : (code, libellé du professeur).
pub const MOTIFS: [(&str, &str); 4] = [
    ("amenagement", "Aménagement"),
    ("absence", "Absence"),
    ("technique", "Problème technique"),
    ("autre", "Autre"),
];

/// Libellé d'un statut constaté (élève, famille ou professeur).
pub fn libelle_statut(statut: &str) -> &'static str {
    match statut {
        "a_faire" | "attendu" => "À rendre",
        "rendu" => "Rendu",
        "en_retard" => "Rendu en retard",
        "rendu_papier" => "Rendu sur papier",
        "non_rendu" => "Non rendu",
        "dispense" => "Dispensé",
        "excuse" => "Excusé",
        "retour_disponible" => "Retour disponible",
        _ => "À rendre",
    }
}

pub fn ton_statut(statut: &str) -> TonEtat {
    match statut {
        "rendu" | "rendu_papier" | "retour_disponible" => TonEtat::Ok,
        "en_retard" | "excuse" | "dispense" => TonEtat::Avertissement,
        "non_rendu" => TonEtat::Mal,
        _ => TonEtat::Info,
    }
}

/// État du retour d'une copie pour le professeur.
pub fn libelle_retour(retour: &str) -> &'static str {
    match retour {
        "rendu" => "Retour rendu",
        "brouillon" => "Retour en brouillon",
        "a_faire" => "Retour à faire",
        _ => "—",
    }
}

/// Phrase d'échéance : « délai accordé jusqu'au … » (jamais le motif) ou l'échéance normale.
pub fn phrase_echeance(r: &RemiseLueDto, jour: &str) -> String {
    let heure = r.echeance_heure.format("%H h %M").to_string().replace(" 00", "");
    if r.delai_accorde {
        format!("Délai accordé jusqu'au {jour} à {heure}")
    } else {
        format!("À rendre pour le {jour} à {heure}")
    }
}

/// Message de l'accusé de dépôt, lu tel quel par les lecteurs d'écran.
pub fn message_accuse(numero: i32, en_retard: bool, nb_fichiers: i16) -> String {
    let fichiers = match nb_fichiers {
        0 => "sans fichier".to_string(),
        1 => "avec 1 fichier".to_string(),
        n => format!("avec {n} fichiers"),
    };
    let retard = if en_retard { " (reçu en retard)" } else { "" };
    format!("Dépôt n° {numero} enregistré {fichiers}{retard}.")
}

/// Contrôle avant l'envoi : texte ou fichier requis, taille bornée. Le serveur recontrôle tout.
pub fn valider_depot(texte: &str, nb_fichiers: usize, nb_brouillon: usize) -> Result<(), String> {
    if texte.trim().is_empty() && nb_fichiers == 0 && nb_brouillon == 0 {
        return Err("Ajoutez un fichier ou écrivez un texte avant de déposer.".into());
    }
    if texte.chars().count() > 20_000 {
        return Err("Le texte dépasse 20 000 caractères.".into());
    }
    if nb_fichiers + nb_brouillon > 10 {
        return Err("Un dépôt comporte au plus 10 fichiers.".into());
    }
    Ok(())
}

/// Conseil affiché quand un dépôt échoue (CC-32, CC-43, CC-44) : jamais un message technique brut.
pub fn conseil_depot_refuse() -> &'static str {
    "Le fichier n'a pas pu être déposé. Essayez de le convertir en PDF, ou de le photographier, puis déposez-le de nouveau ; \
     si cela ne fonctionne toujours pas, utilisez « Je n'arrive pas à déposer » pour prévenir votre professeur."
}

/// Corps d'une prolongation d'après le formulaire ; `None` tant qu'il est incomplet.
pub fn corps_prolongation(
    eleves: Vec<Uuid>,
    jour: &str,
    heure: &str,
    motif: &str,
) -> Option<ProlongationBody> {
    if eleves.is_empty() || !MOTIFS.iter().any(|(c, _)| *c == motif) {
        return None;
    }
    Some(ProlongationBody {
        eleve_ids: eleves,
        echeance_jour: lire_jour(jour)?,
        echeance_heure: lire_heure_champ(heure)?,
        motif: motif.to_string(),
    })
}

/// Corps de la clôture du ramassage d'après le formulaire.
pub fn corps_cloture(jour: &str, heure: &str) -> Option<ClotureBody> {
    Some(ClotureBody {
        cloture_jour: lire_jour(jour)?,
        cloture_heure: lire_heure_champ(heure).or(NaiveTime::from_hms_opt(23, 59, 0))?,
    })
}

/// Constats en lot : seulement les lignes **sélectionnées et affichées** (un constat n'atteint jamais une ligne masquée).
pub fn corps_constats(
    selection: &[Uuid],
    affichees: &[Uuid],
    statut: &str,
) -> Option<ConstatsBody> {
    if ![
        CONSTAT_RENDU_PAPIER,
        CONSTAT_NON_RENDU,
        CONSTAT_DISPENSE,
        CONSTAT_EXCUSE,
    ]
    .contains(&statut)
    {
        return None;
    }
    let constats: Vec<ConstatLigneBody> = selection
        .iter()
        .filter(|e| affichees.contains(e))
        .map(|e| ConstatLigneBody {
            eleve_id: *e,
            statut: statut.to_string(),
        })
        .collect();
    (!constats.is_empty()).then_some(ConstatsBody { constats })
}

/// Sélections masquées par le filtre : annoncées pour qu'une action de groupe ne surprenne jamais.
pub fn nb_selections_masquees(selection: &[Uuid], affichees: &[Uuid]) -> usize {
    selection.iter().filter(|e| !affichees.contains(e)).count()
}

/// Filtres du tableau de suivi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FiltreSuivi {
    Tous,
    ARelancer,
    ACorriger,
    Signales,
}

/// Lignes affichées : filtre, puis tri par nom (le serveur trie déjà ; le filtre ne réordonne jamais).
pub fn filtrer_suivi(lignes: &[LigneSuiviDto], filtre: FiltreSuivi) -> Vec<LigneSuiviDto> {
    lignes
        .iter()
        .filter(|l| match filtre {
            FiltreSuivi::Tous => true,
            FiltreSuivi::ARelancer => l.statut.as_deref() == Some("attendu"),
            FiltreSuivi::ACorriger => l.retour == "a_faire" || l.retour == "brouillon",
            FiltreSuivi::Signales => l.signalement_le.is_some() || l.delai_demande_id.is_some(),
        })
        .cloned()
        .collect()
}

/// Résumé du tableau, lu par les lecteurs d'écran.
pub fn resume_comptes(c: &ComptesSuiviDto) -> String {
    format!(
        "{} élève(s) attendu(s) : {} rendu(s) dont {} en retard, {} sur papier, {} non rendu(s), {} dispensé(s), {} excusé(s). \
         Retours : {} rendu(s), {} à faire.",
        c.attendus,
        c.rendus,
        c.en_retard,
        c.rendu_papier,
        c.non_rendus,
        c.dispenses,
        c.excuses,
        c.retours_rendus,
        c.retours_a_faire
    )
}

/// Jour lisible « 2026-11-14 » → « 14/11/2026 » (champ natif de date côté saisie, lecture côté tableau).
pub fn jour_court(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn ligne(statut: &str, retour: &str) -> LigneSuiviDto {
        LigneSuiviDto {
            eleve_id: Uuid::nil(),
            nom: "DUPONT".into(),
            prenom: "Léa".into(),
            groupe: None,
            statut: Some(statut.into()),
            remise_id: None,
            dernier_depot_le: None,
            nb_depots: 0,
            echeance_effective_le: Utc.with_ymd_and_hms(2026, 11, 14, 18, 0, 0).unwrap(),
            prolonge: false,
            retour: retour.into(),
            delai_demande_id: None,
            signalement_le: None,
            constat_propose: None,
        }
    }

    #[test]
    fn statuts_lisibles_et_jamais_une_note() {
        assert_eq!(libelle_statut("non_rendu"), "Non rendu");
        assert_eq!(libelle_statut("rendu_papier"), "Rendu sur papier");
        assert_eq!(libelle_statut("inconnu"), "À rendre");
        assert_eq!(ton_statut("non_rendu"), TonEtat::Mal);
        assert_eq!(ton_statut("rendu"), TonEtat::Ok);
    }

    #[test]
    fn depot_texte_ou_fichier_requis_et_borne() {
        assert!(valider_depot("", 0, 0).is_err());
        assert!(valider_depot("  ", 0, 0).is_err());
        assert!(valider_depot("", 1, 0).is_ok());
        assert!(valider_depot("", 0, 2).is_ok(), "les fichiers du brouillon comptent");
        assert!(valider_depot(&"a".repeat(20_001), 0, 0).is_err());
        assert!(valider_depot("x", 8, 3).is_err());
    }

    #[test]
    fn accuse_annonce_le_numero_les_fichiers_et_le_retard() {
        assert_eq!(message_accuse(1, false, 1), "Dépôt n° 1 enregistré avec 1 fichier.");
        assert_eq!(message_accuse(2, true, 3), "Dépôt n° 2 enregistré avec 3 fichiers (reçu en retard).");
        assert_eq!(message_accuse(1, false, 0), "Dépôt n° 1 enregistré sans fichier.");
    }

    #[test]
    fn prolongation_exige_eleves_jour_heure_et_motif_connu() {
        let e = Uuid::from_u128(1);
        assert!(corps_prolongation(vec![e], "2026-11-20", "08:00", "amenagement").is_some());
        assert!(corps_prolongation(vec![], "2026-11-20", "08:00", "amenagement").is_none());
        assert!(corps_prolongation(vec![e], "demain", "08:00", "amenagement").is_none());
        assert!(corps_prolongation(vec![e], "2026-11-20", "huit", "amenagement").is_none());
        assert!(corps_prolongation(vec![e], "2026-11-20", "08:00", "maladie").is_none());
    }

    #[test]
    fn constats_seulement_sur_les_lignes_selectionnees_et_affichees() {
        let (a, b, c) = (Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));
        let corps = corps_constats(&[a, b, c], &[a, c], "excuse").unwrap();
        let ids: Vec<Uuid> = corps.constats.iter().map(|x| x.eleve_id).collect();
        assert_eq!(ids, vec![a, c]);
        assert_eq!(nb_selections_masquees(&[a, b, c], &[a, c]), 1);
        assert!(corps_constats(&[a], &[a], "zero").is_none(), "aucun statut n'est une note");
        assert!(corps_constats(&[b], &[a], "excuse").is_none());
    }

    #[test]
    fn filtres_du_suivi() {
        let lignes = vec![ligne("attendu", "aucun"), ligne("rendu", "a_faire"), ligne("rendu", "rendu")];
        assert_eq!(filtrer_suivi(&lignes, FiltreSuivi::Tous).len(), 3);
        assert_eq!(filtrer_suivi(&lignes, FiltreSuivi::ARelancer).len(), 1);
        assert_eq!(filtrer_suivi(&lignes, FiltreSuivi::ACorriger).len(), 1);
        assert_eq!(filtrer_suivi(&lignes, FiltreSuivi::Signales).len(), 0);
    }

    #[test]
    fn cloture_par_defaut_a_la_fin_de_journee() {
        let c = corps_cloture("2026-12-01", "").unwrap();
        assert_eq!(c.cloture_heure, NaiveTime::from_hms_opt(23, 59, 0).unwrap());
        assert!(corps_cloture("", "10:00").is_none());
    }

    #[test]
    fn resume_lisible() {
        let c = ComptesSuiviDto { attendus: 30, rendus: 25, en_retard: 3, ..Default::default() };
        assert!(resume_comptes(&c).starts_with("30 élève(s) attendu(s) : 25 rendu(s) dont 3 en retard"));
    }
}
